# Evidência — netcoredbg embutido

Data: 2026-11-04. Branch `fix/e2e-alpha5`. Spec: [`../SPEC-EMBED-NETCOREDBG.md`](../SPEC-EMBED-NETCOREDBG.md). Ticket: [`../tickets-embed-netcoredbg/01-embed-netcoredbg.md`](../tickets-embed-netcoredbg/01-embed-netcoredbg.md). Implementação: `f804cd8`.

## Pin e build

- Zip oficial `netcoredbg-osx-arm64-3.2.0-1092.zip`: **3.389.504 bytes**, SHA-256
  `f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea`, versionado em
  `vendor/netcoredbg/`.
- `scripts/vendor-netcoredbg.sh` verifica o hash e falha o build quando diverge (testado com
  zip alterado: recusou e o original foi restaurado). Extrai para
  `src-tauri/resources/netcoredbg/` (não versionado), assina `libdbgshim.dylib` e
  `netcoredbg` com Developer ID + hardened runtime + timestamp quando há identidade, senão
  ad-hoc.
- Árvore embutida: **9.587.448 bytes** (9,14 MiB); 7 arquivos (netcoredbg, dbgshim e DLLs
  Roslyn).
- `bundle.resources` copia para `Contents/Resources/netcoredbg/` e o
  `THIRD-PARTY-NOTICES.md` para `Contents/Resources/`.

## Assinatura, entitlements e notarização

- `codesign --verify --deep --strict` no `.app` de release: **valid on disk / satisfies its
  Designated Requirement**.
- Binário aninhado: `Authority=Developer ID Application: Helder Santos (K6M85697D4)`,
  `flags=0x10000(runtime)`, `Timestamp=4 Nov 2026 11:13:24`, `TeamIdentifier=K6M85697D4`.
  A `libdbgshim.dylib` também está Developer ID + runtime.
- Entitlements necessários, provados por eliminação: sem entitlements a sessão DAP não
  conclui; só `disable-library-validation` devolve `configurationDone` 0x80004005; só
  `allow-jit` cai com SIGSEGV; com **`allow-jit` + `disable-library-validation`** a sessão
  termina e imprime a saída do debuggee. O app em si não recebeu entitlements.
- Notarização do `.app`: `status: Accepted` (id `fa7be590-817a-4322-a9f8-12b399d54381`);
  `stapler validate` ok; `spctl` = **accepted, source=Notarized Developer ID**.
- DMG: `status: Accepted` (id `4e9d6b8c-abe9-44a6-8a6d-9ed91ecda0b1`); assinado, grampeado,
  `spctl` = **accepted, Notarized Developer ID**; `stapler validate` ok.

## Tamanho antes/depois (mesmo método)

Método: build release `aarch64-apple-darwin` + `hdiutil create -fs HFS+ -format UDZO`.

| Item | Antes | Depois | Delta |
|---|---:|---:|---:|
| `.app` release | 14,68 MiB | 23,85 MiB | **+9,17 MiB** |
| DMG | 5.671.389 B (5,41 MiB) | 10.454.440 B (9,97 MiB) | **+4.783.051 B (+4,56 MiB, +84,3%)** |
| `Contents/Resources/netcoredbg` | — | 9,14 MiB | +9,14 MiB |

O DMG ficou ~1,16 MiB acima da estimativa da proposta (~8,6 MiB): o UDZO/HFS+ comprime a
árvore um pouco pior que o zip oficial. O debug bundle é 53 MiB (símbolos de debug), não
serve para comparação de tamanho.

## App real (verify-openridr, superfície C)

**Debug bundle, home isolada vazia** (`OPENRIDR_HOME` nova, `tools/` inexistente):

1. Trust no `OrderDesk.sln`.
2. Debug: build implícito e sessão; saída `Order A-104: R$ 42,00`; sessão terminou.
3. `tools/` continuou **inexistente** — nenhum download, nenhuma instalação.
4. Workspace tools: "Bundled with OpenRidr", versão 3.2.0-1092, SHA-256
   `18157e59…0467493` do executável, caminho dentro de `Contents/Resources`, e o texto
   "nothing is downloaded"; sem plano/progresso/retry.
5. `debug.install`/`debug.retry` são recusados quando embutido (teste de host).

**DMG montado somente leitura** (o risco de rodar de `Contents/Resources` read-only):

1. `hdiutil attach -readonly` em `/tmp/OpenRidrDmg`; `codesign --deep --strict`, `spctl` e
   `stapler` ok; versão 0.3.0-alpha.5.
2. App executado do DMG com `OPENRIDR_HOME=/tmp/embed-dmg-home` e fixture copiada para
   `/tmp/embed-dmg-work`.
3. Trust + Debug: `Order A-104: R$ 42,00`; `tools/` vazio. O adaptador rodou do volume
   somente leitura.
4. DMG desmontado; nenhum processo `openridr` restante.

## Testes

- Host: `cargo test -p openridr-host --lib` 216/216; `--test debugger` 14/14 (bundle vence
  tools, fallback, ausência dos dois, view bundled/versão/hash, recusa de install/retry,
  `--version` do embutido).
- `bun run check` e `bun run build` ok.
- `bun run tauri build --debug --bundles app` com Resources + THIRD-PARTY-NOTICES.

## Não verificado

- **Quarentena/translocation**: o DMG foi gerado localmente (sem `com.apple.quarantine`); a
  prova de Gatekeeper é `spctl`/stapler no DMG e no app, não uma abertura com quarentena real.
- **macOS Intel e Windows/Linux**: o release 3.2.0-1092 não publica osx-x64; só arm64 foi
  embutido e executado.
- **Atualização do adaptador**: trocar a versão exige nova release do app (decisão).
- csharp-ls, plano revisável e download da análise não foram tocados nesta rodada.
