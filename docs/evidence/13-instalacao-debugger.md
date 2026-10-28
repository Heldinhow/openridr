# Evidência — Ticket 13: instalar debugger com confirmação e gates de plataforma

Implementação local em 28/10/2026 (revisão). Fonte normativa: [ticket 13](../tickets/13-instalacao-debugger.md), [SPEC](../SPEC.md) P0.1/P0.5, “Avaliação e ferramentas”, “Licenças” e Testing Decisions, e gates G1–G5 do [índice de tickets](../tickets/README.md). Nesta máquina o asset oficial **osx-arm64 foi instalado de verdade** com a flag experimental ligada, trust e consentimento explícitos; Windows e Mac Intel continuam não executados. DAP segue no ticket 14.

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| Versão aprovada | netcoredbg `3.2.0-1092`, release oficial `Samsung/netcoredbg` |
| Fonte de metadata | `https://api.github.com/repos/Samsung/netcoredbg/releases/tags/3.2.0-1092` (consultada em 28/10/2026; o `digest` não mudou) |
| Instalação real | `~/Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092` (app home padrão, fora do repo) |
| Base de código | `a067bae` (ticket 13 inicial) + esta revisão |
| Novas dependências | `ureq 2.12` (rustls) e `zip 2` (deflate) |

Log de ambiente e metadata: [`logs/13-release-metadata.log`](logs/13-release-metadata.log).

## Metadata oficial consultada (saída real)

```text
$ curl -sS https://api.github.com/repos/Samsung/netcoredbg/releases/tags/3.2.0-1092
3.2.0-1092 3.2.0-1092 2026-06-25T14:40:46Z prerelease=False draft=False
netcoredbg-linux-amd64.tar.gz	3510717	sha256:080eb3b2d2152465f599d3b33d1ee6e747794e11cc0a3773ec689f5e5f2c5afa
netcoredbg-linux-arm64.tar.gz	3424271	sha256:065ff49badec8a695dbea2de6ab6a330c774a191e426a217ab8cc05250627ccb
netcoredbg-osx-arm64.zip	3389504	sha256:f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea
netcoredbg-win64.zip	3524161	sha256:3c410a45fa502415203a94fcb88654af65bf8e3dac158a5527a722e7a6b9274a

$ curl -sS .../3.2.0-1092/README.md | grep -i arm64
113:*Note: the MacOS arm64 build (M1) is community supported and may not work as expected, as well as some tests possibly failing.*
```

Fatos usados pelo host: existe asset `osx-arm64` com SHA-256 publicado; **não existe** asset `osx-x64`; o build macOS arm64 é declarado *community supported* pelo upstream. O SHA-256 do plano é o `digest` do GitHub; assinatura/notarização não são verificadas nem alegadas. O zip real contém `netcoredbg/netcoredbg` (Mach-O arm64), bibliotecas gerenciadas e um diretório `__MACOSX` (AppleDouble), que novas extrações ignoram.

## Gates por plataforma

| Plataforma | Asset | Plano | Instalação |
|---|---|---|---|
| macOS arm64, flag OFF | `netcoredbg-osx-arm64.zip` disponível | disponível com aviso community-supported | recusada com `experimental_required`; nada baixado |
| macOS arm64, flag ON | idem | disponível com aviso explícito | permitida com trust + `confirm:true`; única URL é a oficial |
| macOS Intel | ausente | disponível com `blockedCode: unsupported_platform` | recusada; nenhuma versão/asset substituto |
| Windows x64 | `netcoredbg-win64.zip` | disponível | caminho existe, **não executado aqui** |
| Linux | `.tar.gz` | indisponível (`unsupported_archive` → `unsupported_platform`) | recusada; Linux fora do P0 |

A flag é opt-in de sessão, independente de trust; ligá-la replaneja o plano exibido, então o consentimento seguinte é o que autoriza a instalação experimental. Ela sozinha não instala e não prova DAP.

## Instalação real nesta máquina (host flow, app home padrão)

Fluxo executado via `POST /call` no `openridr-host-http` com a home real: `workspace.open` (fixture), `trust.grant`, `debug.flag {enabled:true, confirm:true}`, `debug.plan`, `debug.install {planId, confirm:true}`. Transcript completo e plano: [`logs/13-real-install.log`](logs/13-real-install.log), [`logs/13-real-install-plan.json`](logs/13-real-install-plan.json), [`logs/13-real-install-state.json`](logs/13-real-install-state.json).

```text
planId fnv1a64:969d01a8390f3343
asset netcoredbg-osx-arm64.zip · 3389504 bytes
expected sha256 f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea
[   0.0s] downloading · progress {"downloaded":0,"total":3389504}
[   0.8s] installing  · progress {"downloaded":3389504,"total":3389504}
[   1.3s] verifying
[   1.6s] ready · verifiedVersion 3.2.0
sha256 f4fa33b3…4e59ea matched the plan
$ …/netcoredbg/3.2.0-1092.staging/netcoredbg/netcoredbg --version
NET Core debugger 3.2.0-1 (9744e1f, Release)

Copyright (c) 2020 Samsung Electronics Co., LTD
Distributed under the MIT License.
See the LICENSE file in the project root for more information.
executable sha256 ffc51b6f716e6bb8cb93c416e352651b945269c9501f820d32f3023d4ce24ece
Verified executable promoted to the destination. DAP compatibility is not proven.
```

Redirect real observado: `https://github.com/.../netcoredbg-osx-arm64.zip` → `https://release-assets.githubusercontent.com/...` (host na allowlist). A query assinada desse redirect não é gravada: o host loga só esquema/host/caminho (`loggable_url`) e a evidência está com `?<redacted>`.

Executável instalado, conferido diretamente:

```text
$ "$DEST/netcoredbg/netcoredbg" --version
NET Core debugger 3.2.0-1 (9744e1f, Release)
… Distributed under the MIT License.
exit=0
$ shasum -a 256 "$DEST/netcoredbg/netcoredbg"
ffc51b6f716e6bb8cb93c416e352651b945269c9501f820d32f3023d4ce24ece
$ file "$DEST/netcoredbg/netcoredbg"
Mach-O 64-bit executable arm64
$ ls -la "$DEST" "$DEST/netcoredbg"
… netcoredbg/ (ManagedPart.dll, Microsoft.CodeAnalysis*.dll, libdbgshim.dylib, netcoredbg -rwxr-xr-x)
```

O `__MACOSX` extraído na primeira instalação permanece no destino (nada foi apagado); extrações novas pulam essas entradas. Reuso (plano novo + confirmação, sem download): [`logs/13-real-install-reuse.json`](logs/13-real-install-reuse.json).

```text
after plan: state planned · existing present
  binary …/tools/netcoredbg/3.2.0-1092/netcoredbg/netcoredbg
reuse: state ready · reused True · verifiedVersion 3.2.0
  sha256 ffc51b6f…24ece
log: Existing netcoredbg installation verified; reused without downloading.
```

## UI real

Card no Toolchain e faixa de avisos do workspace, com estado, fonte/versão, asset/availability, aviso community-supported, flag Apple Silicon, plano completo (incluindo `blockedCode`), limites, log e SHA/versão verificada. Capturas do mesmo build:

- [`screenshots/13-debugger-plan.png`](screenshots/13-debugger-plan.png) — flag OFF: `Experimental · disabled`, asset disponível, instalação recusada com motivo.
- [`screenshots/13-debugger-flag.png`](screenshots/13-debugger-flag.png) — flag ON: `Plan ready — nothing downloaded yet`, aviso explícito e botão de confirmação habilitado (o E2E não baixa).
- [`screenshots/13-debugger-ready.png`](screenshots/13-debugger-ready.png) — app home real: `Adapter verified — DAP not proven`, versão/SHA reais e “Reused the verified installation” (`logs/13-ui-real-ready.log`).

`toolbar-debug` fica desabilitado com o motivo (flag OFF: instalação experimental exigida; flag ON: DAP não implementado). Análise, Run, build e editor permanecem disponíveis.

## Testes de fronteira (servidor local + zip controlado)

`crates/host/tests/debugger.rs` sobe um servidor HTTP determinístico em `127.0.0.1:0` e usa zips controlados cujo executável é um script rotulado `NET Core debugger 3.2.0-1 (controlled test artifact, Release)`. O asset oficial não é baixado por nenhum teste automatizado. Seam: `OPENRIDR_ENABLE_PROBE=1` + `OPENRIDR_DEBUG_PLATFORM` + `OPENRIDR_DEBUG_ASSET_URL` + `OPENRIDR_DEBUG_ASSET_SHA256`; o plano marca `asset.testOverride: true`.

```text
$ cargo test -p openridr-host
running 83 tests   (unit; 9 novos do debugger)
test result: ok. 83 passed; 0 failed; 0 ignored
running 9 tests    (boundary do debugger)
test result: ok. 9 passed; 0 failed; 0 ignored
running 49 tests   (boundary dos tickets 01–12)
test result: ok. 49 passed; 0 failed; 0 ignored
```

Logs: [`13-cargo-test.log`](logs/13-cargo-test.log), [`13-debugger-states.log`](logs/13-debugger-states.log). Casos: Mac ARM64 flag OFF → `experimental_required` sem download; flag ON + artefato controlado → `downloading/installing/verifying/ready` (layout aninhado `netcoredbg/`, `__MACOSX` ignorado) e reuso sem segunda requisição; trust/consentimento/plano trocado; hash/versão errados; zip corrompido; traversal/absoluto/symlink; origem e redirect inválidos; cancelamento/retry; URL de log sem query assinada.

## E2E de UI (Playwright + Chrome)

```text
$ bun run test:e2e
  ✓ macOS arm64 requires the experimental flag and shows the real asset
  ✓ host gates consent, trust and plan identity before any download
  ✓ … 36 testes dos tickets 01–12 continuam verdes
  38 passed (3.4m)
```

Log: [`13-playwright.log`](logs/13-playwright.log). O E2E usa `OPENRIDR_HOME` isolado e **não** baixa o asset; a instalação real foi feita à parte na app home padrão, como registrado acima.

```text
$ bun run build
✓ built in 807ms

$ bun run tauri build --debug --no-bundle
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.29s
Built application at: /Users/helder/Projetos/openridr/src-tauri/target/debug/openridr
```

## Cobertura do ticket e dos gates

| Item / gate | Resultado | Evidência |
|---|---|---|
| Plano com versão/fonte/URL/asset/OS/arch/destino/operação/permissões; sem consentimento/origem não baixa; sem troca silenciosa | Plano completo com digest oficial; `experimental_required` com flag OFF; `confirmation_required` para plano trocado | `13-real-install-plan.json`, boundary tests, E2E |
| G3 origem/redirect/destino/extração/integridade/limites/sem global/elevação | HTTPS + hosts oficiais, redirect real exercitado, destino no app home, zip sem traversal/symlink/absoluto com caps, SHA-256 oficial conferido; assinatura/notarização declaradas não verificadas | `13-real-install.log`, boundary tests |
| Estados/logs/cancel/retry/parcial não pronta/ferramenta válida preservada/progresso medido/sem DAP | Instalação real `downloading→installing→verifying→ready` em 1,6 s, log durável, reuso sem download, bytes medidos, `dapProven:false` | `13-real-install-state.json`, `13-real-install-reuse.json` |
| Mac ARM64 flag OFF → bloqueado; flag ON + trust + consentimento → instala asset oficial; Debug bloqueado até DAP | Flag OFF recusa com `experimental_required`; flag ON instala e verifica 3.2.0; toolbar Debug desabilitado | E2E 1, `13-debugger-{plan,flag}.png`, instalação real |
| Mac Intel/Windows/Linux | Intel e Linux `unsupported_platform`; Windows com asset e plano, não executado | unit `gate_states_are_honest_per_platform`; limitações abaixo |
| Host aplica consentimento/plataforma/flag/trust em chamadas diretas; G1–G5; MIT e UI acessível | `debug.install`/`debug.retry` atrás de `require_trust`; consentimento/plataforma/flag no host; controles nativos com nome/motivo; MIT registrado | E2E 2, cargo tests, esta página |
| AC-04/05/06/22/23/24 (fatia), AC-01/07, AC-32/33/34 | Plano/instalação/gates/flag/limites e UI acessível; DAP e auditoria de release ficam em 14/18 | tabela acima |

## Limitações e pendências honestas

- **DAP não foi executado.** A instalação real prova executável, SHA-256 e `--version` (`3.2.0-1`); não há launch, breakpoint, stepping, stop, async nem multi-thread. A flag ARM64 só pode ser retirada com a prova do ticket 14; o relato upstream #256 continua risco não reproduzido.
- **Windows x64 e macOS Intel não executados.** Windows tem asset oficial e o caminho existe, mas instalação/`--version`/smoke DAP não foram rodados aqui. Mac Intel não tem `osx-x64` na release e fica bloqueado; nenhuma outra versão é baixada.
- **Linux** fora do P0: assets `.tar.gz` são metadata-only (`unsupported_archive`); o host não chama `tar` externo.
- **Integridade**: SHA-256 vem do `digest` do GitHub; não há assinatura de código ou notarização verificada e o host não alega uma. O redirect real foi exercitado nesta instalação (`release-assets.githubusercontent.com`), mas os demais saltos possíveis não.
- **Layout do archive**: o zip oficial osx-arm64 foi inspecionado nesta instalação (executável em `netcoredbg/netcoredbg`); o layout do zip win64 não foi. O host localiza exatamente um executável pelo nome e falha se houver zero ou mais de um.
- **Teste controlado**: o executável do zip de teste é um script rotulado e só roda no harness; o seam exige `OPENRIDR_ENABLE_PROBE=1`.
- **Privacidade**: a URL assinada do redirect do GitHub não é logada (somente esquema/host/caminho); a evidência antiga desta sessão foi sanitizada com `?<redacted>`. A primeira instalação deixou um diretório `__MACOSX` no destino (nada foi apagado); extrações novas o ignoram.
- **Licença**: netcoredbg é MIT (`LICENSE` da tag 3.2.0-1092, “Copyright (c) 2017 Samsung Electronics Co., LTD”); o binário instalado imprime o aviso MIT. Nenhum código-fonte foi incorporado; a auditoria de distribuição permanece no ticket 18 e nada aqui é parecer jurídico.
- **E2E** usa Chrome real contra Vite + host-http, com `OPENRIDR_HOME` isolado; a captura “ready” usou o host com a app home real. A janela Tauri nativa não foi capturada (limitação de TCC já registrada).
