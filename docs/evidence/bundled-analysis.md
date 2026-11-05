# Evidência — análise embutida (csharp-ls 0.28.0 + runtime .NET 10.0.14)

Data: 2026-11-05. Branch `feat/bundled-analysis-runtime`, base `eb3fe6f` (alpha.8). Spec:
[`../SPEC-BUNDLED-ANALYSIS.md`](../SPEC-BUNDLED-ANALYSIS.md). Tickets:
[`../tickets-bundled-analysis/`](../tickets-bundled-analysis/README.md).

## Pins e build

- Runtime: `dotnet-runtime-10.0.14-osx-arm64.tar.gz`, 33.251.887 bytes, SHA-512
  `19c5781dc2f87600865886b8774a4c5851caf1f5cf3f591185b922b0e51dea6b8c0ed0bf80ae4d3be2e083db662124258743e779fab6885322d641eff52b0382`
  (idêntico ao `dotnet-runtime-10.0.14-osx-arm64.tar.gz.sha512` oficial e ao
  `releases-index.json` do canal 10.0).
- csharp-ls: `csharp-ls.0.28.0.nupkg`, 21.643.814 bytes, SHA-512
  `a138b9c8f9de9dd6343e9b0d0c8f4d51637bba7e38606b7c2b08b1d91e8eefa39f12123057f6625f425812b4361e52841bd2856704ec127ff50144ce05170a99`
  (o mesmo pin de `crates/host/src/tools.rs`).
- `sh scripts/vendor-analysis.sh` → exit 0 em 1s com cache, 83 MiB + 57 MiB em
  `src-tauri/resources/{dotnet,csharp-ls}`. Hash errado falha o script (`fetch()` compara
  SHA-512 antes de extrair); marker `.vendor-state` torna a segunda execução no-op.
- `.app` release (`bun run tauri build --bundles app,dmg`, `beforeBuildCommand` roda os dois
  vendor scripts): `Contents/Resources/{dotnet,csharp-ls,netcoredbg,THIRD-PARTY-NOTICES.md}`.
  `codesign --verify --deep --strict` no `.app`: **0**.
- Identidade do que está embutido:

```
$ file Contents/Resources/dotnet/dotnet
Mach-O 64-bit executable arm64
$ file Contents/Resources/dotnet/host/fxr/10.0.14/libhostfxr.dylib
Mach-O 64-bit dynamically linked shared library arm64
$ shasum -a 256 Contents/Resources/csharp-ls/CSharpLanguageServer.dll
f941424ca54bc5fbe46db50f6283f3e06cc228696107723a34ec96d33b93c348
$ shasum -a 256 Contents/Resources/dotnet/dotnet
645e6f9631ff71f8d91b290cfc4c6d14184df71dc66bdc4fa7ed595f81bd51dc
$ codesign -d --entitlements - Contents/Resources/dotnet/dotnet
com.apple.security.cs.allow-jit = true
com.apple.security.cs.disable-library-validation = true
```

## Viabilidade provada antes da implementação

Com o runtime extraído, o payload do nupkg e um `hostroot` composto à mão (SDK 8.0.408
externo + runtime 10.0.14 do tarball), o LSP real respondeu `--version` e publicou
`CS0029` na fixture. Foi o que decidiu o desenho (DLL gerenciado + muxer do bundle, sem shim do
NuGet).

## Prova automatizada — bridge HTTP offline (host real, sem UI)

Sandbox (`sandbox-exec`) com **sem rede IP** (`deny network-outbound (remote ip "*:*")`) e sem
`/usr/local/share/dotnet`/`/opt/homebrew/opt/dotnet`. `HOME` isolado; `DOTNET_ROOT` e `PATH`
apontam para uma raiz sintética que só tem **SDK 8.0.408 + runtime 8.0.15** (muxer copiado,
`--list-sdks`/`--list-runtimes` provam que não há 10.x ali). `OPENRIDR_HOME` novo. Os
overrides de debug `OPENRIDR_BUNDLED_{CSHARP_LS,DOTNET_ROOT,NETCOREDBG}` apontam para o
`Contents/Resources` do `.app` construído.

Saída de [`host_bridge_proof.py`](bundled-analysis/host_bridge_proof.py) (também em
`host-bridge.stderr.log` o log do bridge):

```
setup: ready
tool: {"state": "ready", "bundled": true, "destination": ".../OpenRidr.app/Contents/Resources/csharp-ls",
       "verifiedVersion": "0.28.0", "sha256": "f941424c…93c348", "plan": null, "progress": null}
runtime: {"state": "found", "bundled": true, "version": "10.0.14",
          "root": ".../OpenRidr.app/Contents/Resources/dotnet"}
project sdk: not_inspected 8.0.408
app home tools/ exists: False
trusted: True project sdk: resolved
restore: ok ['restored', 'restored', 'restored']
lsp: partial | host root: /tmp/openridr-bundled-sim/home/app/hostroot
diagnostics: [{"category": "unknown", "code": "CS0029", "severity": 1,
  "message": "Cannot implicitly convert type 'string' to 'int'",
  "position": {"line": 17, "character": 36}, "source": "lsp"}]
csharp-ls pids: ['42882']
pid 42882 : /tmp/openridr-bundled-sim/home/app/hostroot/dotnet
             .../OpenRidr.app/Contents/Resources/csharp-ls/CSharpLanguageServer.dll
home listing tools/: hostroot, recent.json, toolhome   (sem tools/)
RESULT: ok
```

O que isso prova: setup e análise sem rede, sem runtime .NET 10 externo em nenhuma raiz
detectada, sem nada instalado no app home, com o muxer copiado do bundle no `hostroot` e o DLL
lido direto de `Contents/Resources` (lsof).

## Prova automatizada — Debug com breakpoint, offline

Mesmo sandbox e mesma raiz SDK-8-só. [`host_bridge_debug.py`](bundled-analysis/host_bridge_debug.py):

```
evaluation: ok context: {'configuration': 'Debug', 'targetFramework': None}
breakpoints: [{"line": 4, "path": "src/OrderDesk.Cli/Program.cs", "verified": false}]
dap after start: paused | stopped: {"reason": "breakpoint", "threadId": 210433}
adapter: .../OpenRidr.app/Contents/Resources/netcoredbg/netcoredbg
frames: [{"id": 0, "line": 4, "name": "Program.<Main>$()", "path": ".../Program.cs"}]
variables: {"scopes": [{"name": "Locals", "variables": [
  {"name": "args", "type": "string[]"}, {"name": "order", "type": "OrderDesk.Core.Order"}]}]}
debug output tail: Order A-104: R$ 42,00
dap final: terminated exit: 0
RESULT: ok
```

Debug embutido (netcoredbg 3.2.0-1092) continua funcionando junto com o runtime privado e o SDK
externo; breakpoint, locals, continue e saída do debuggee.

## Prova desktop (app Tauri real, verify-openridr superfície C)

App de **debug** (`src-tauri/target/debug/.../OpenRidr.app`, versão 0.3.0-alpha.8),
`OPENRIDR_HOME` isolado, raiz SDK-8-só, uma instância por vez.

1. Abrir `OrderDesk.sln` → **nenhum gate de setup**: direto ao Trust
   ([`desktop-gate-skipped.jpg`](bundled-analysis/desktop-gate-skipped.jpg)). O app chama
   `setup.start` e recebe `ready` na mesma resposta.
2. Trust → restore, análise e `Problems (1)`:
   `error CS0029: Cannot implicitly convert type 'string' to 'int' · LSP · csharp-ls`
   ([`desktop-problems-cs0029.jpg`](bundled-analysis/desktop-problems-cs0029.jpg)). Barra de
   status: `Analysis runtime 10.0.14 · Project SDK 8.0.408 · Trusted mode`.
3. `OPENRIDR_HOME` depois da sessão: `hostroot`, `recent.json`, `run-configs`, `toolhome`,
   `trusted.json` — **sem `tools/`**; nenhum download.
4. Fechar e reabrir o mesmo binário com o mesmo home/solução → entra direto no workspace
   (Trusted mode, `Order.cs` aberto, análise rodando): [`desktop-reopen.jpg`](bundled-analysis/desktop-reopen.jpg).

## DMG local (somente leitura) e tamanho

Build: `bun run tauri build --bundles app,dmg` (bundler Tauri, `bundle_dmg.sh`/`hdiutil`, UDZO
HFS+), assinatura ad-hoc (`signingIdentity: "-"`), sem notarização.

| Item | Alpha.8 (baseline) | Esta build | Delta |
|---|---:|---:|---:|
| `.app` release | 24,25 MiB | **163,29 MiB** | +139,04 MiB |
| DMG | 10.699.101 B (10,20 MiB) | **61.774.547 B (58,91 MiB)** | **+51.075.446 B (+51,0 MiB, ~5,8×)** |
| `Contents/Resources/dotnet` | — | 83 MiB | — |
| `Contents/Resources/csharp-ls` | — | 57 MiB | — |

DMG montado com `hdiutil attach -readonly` em `/tmp/OpenRidrDmg`:

```
$ codesign --verify --deep --strict /tmp/OpenRidrDmg/OpenRidr.app   # exit 0
$ spctl -a -vv /tmp/OpenRidrDmg/OpenRidr.app
/tmp/OpenRidrDmg/OpenRidr.app: rejected      # ad-hoc, sem Developer ID/notarização nesta sessão
$ stapler validate ...                       # não roda: stapler exige Xcode completo
```

App executado do volume somente leitura com home novo e raiz SDK-8-só: trust → restore →
`Problems (1)` com o mesmo `CS0029 · LSP · csharp-ls`, barra de status com
`Analysis runtime 10.0.14 · Project SDK 8.0.4…`
([`dmg-readonly-problems.jpg`](bundled-analysis/dmg-readonly-problems.jpg)). App home sem
`tools/`. DMG desmontado e nenhum `openridr` deixado rodando.

## Testes

- `cargo test -p openridr-host`: lib 266, bundled_analysis 4, debugger 14, host 98,
  test_discovery 10, test_run 7, trust_model 1 — todos ok.
- `bun run check` e `bun run build` ok.
- `scripts/vendor-analysis.sh` idempotente; `codesign --verify --strict` nos 13 dylibs + muxer.

## Não verificado / limites

- **Notarização e Developer ID**: não há credencial nesta sessão; o `.app`/DMG são ad-hoc,
  `spctl` rejeita e `stapler` não roda (sem Xcode completo). O caminho de assinatura existe
  (`APPLE_SIGNING_IDENTITY` no vendor script) mas não foi executado.
- **Quarentena real / Gatekeeper**: o DMG foi gerado localmente, sem `com.apple.quarantine`.
- **Mac limpo real**: a ausência de runtime .NET 10 foi simulada (sandbox sem rede + raiz
  sintética com SDK 8 e runtime 8 apenas). A máquina ainda tem `/usr/local/share/dotnet` e
  `~/.dotnet` com 10.x, bloqueados/ignorados na simulação. Não é a mesma coisa que um Mac sem
  .NET 10.
- **`~/.dotnet` na prova desktop**: a raiz sintética também é simulação; a prova de que o bundle
  vence o runtime externo é o campo `bundled: true`/`root` em `Contents/Resources` no bridge e a
  barra de status da desktop (runtime 10.0.14 do bundle).
- **Windows, macOS Intel, Linux**: fora do escopo; só osx-arm64 foi construído e executado.
- **Reabertura após atualização do app**: o `hostroot` composto fica no app home e é recomposto
  quando os inputs (incluindo `runtimeRoot`) mudam; troca de caminho do `.app` não foi testada.
