# Evidência — Ticket 02: trust explícito e SDKs separados

Implementação local em 27–28/10/2026. Fonte normativa: [ticket 02](../tickets/02-trust-e-sdks.md), [SPEC](../SPEC.md) P0.1, P0.2, “Estado, erros e processos”, “Avaliação e ferramentas” e Testing Decisions, e gates G1–G5 do [índice de tickets](../tickets/README.md).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Node | v26.10.0 |
| Bun | 1.3.9 |
| Rust | cargo 1.99.0, rustc 1.99.0 |
| Chrome | Google Chrome 155.0.8059.85 (canal `chrome`, sem download de browser) |
| Playwright | 1.64.1 |
| SDK do host | .NET SDK 10.0.100 e 8.0.416 em `/usr/local/share/dotnet` |
| SDK fixado pela fixture | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` |
| Fixtures | `fixtures/OrderDesk` (sln + console + lib + testes) e `fixtures/MissingSdk` (projeto único com `global.json` 8.0.999/`disable`) |
| Base de código | `89adba0` (ticket 01) + esta fatia |

Log: [`logs/02-environment.log`](logs/02-environment.log). O teste de inventário Rust foi marcado `cfg(target_os = "macos")` porque as versões 8.0.408/8.0.416/10.0.100 são as observadas nesta máquina; Windows não tem inventário verificado nesta fatia.

## O que foi construído

- **Gate único de host** (`require_trust`): todos os stubs executáveis (`evaluation.run`, `restore.start`, `lsp.start`, `tools.install`, `terminal.start`, `build.start`, `run.start`, `debug.start`, `tests.discover`, `tests.run`, `processes.probe`) passam por ele. Sem trust (inclusive chamada HTTP direta) → `trust_required`, nenhum processo. Depois de trust → `invalid_state` (ainda não implementado); depois de revogar → `trust_required`.
- **Contexto e resposta tardia**: cada operação pode capturar `ContextToken { workspaceId, contextId }`; `Host::accept_result(token)` devolve `stale_context` se o workspace foi trocado/fechado ou o contexto revogado/redetectado. Teste injeta token antigo e verifica que o estado atual não muda.
- **Trust em memória**: `trust.grant` só com workspace aberto, grava decisão (`decidedAt`, `workspaceId`) e resolve o SDK do projeto. `trust.revoke` zera a autorização, invalida o contexto, encerra a resolução e marca processos ativos como `awaitingDecision`; não declara limpeza. Abrir/trocar/fechar workspace limpa trust e contexto.
- **Inventário neutro de SDKs**: `dotnet` do PATH, cada subpasta imediata de `~/Library/Application Support/OpenRidr/sdks` que contenha `dotnet`, e `~/.dotnet` se presente. Cada raiz roda seu próprio `dotnet --list-sdks` com cwd em diretório temporário e sem `DOTNET_ROOT`/`DOTNET_MULTILEVEL_LOOKUP`. O estado guarda todas as raízes, versões, paths, exit e erro. Host de análise = maior SDK 10.x estável encontrado.
- **SDK do projeto somente após trust**: lê `global.json` do root do workspace como texto/JSON, aplica `version`, `rollForward` e `allowPrerelease`, escolhe a maior versão satisfatória entre todas as raízes e registra a raiz escolhida. Versão não resolvida vira estado distinto `sdk_missing`; nunca cai para o SDK 10 nem reescreve `global.json`.
- **Processos controlados**: registro mínimo com `{id, kind, pid, workspaceId, state: running|exited, exit: null|code, awaitingDecision, kept, termination}`. `try_wait` é consultado no snapshot; `processes.terminate` envia TERM, espera ~2 s, envia KILL e reporta o observado; `processes.keep` mantém visível sem alegar limpeza. O child de teste é criado por `processes.probe` (`sleep`, shell que ignora TERM, e `exit 3`), método **somente de desenvolvimento**, habilitado apenas com `OPENRIDR_ENABLE_PROBE=1`; a UI não expõe esse método.
- **UI**: tela Toolchain sempre mostra Analysis host, Project SDK, TFM declarada, debug adapter “Not checked in this slice”, nota Apple Silicon `experimental · disabled`, Redetect e árvore de raízes detectadas. Checkbox e `Trust and inspect project` só existem com workspace aberto; `Open text only` continua. Workspace tem trust compacto (checkbox/revoke), status `Host SDK … · Project SDK …`, Redetect, Open global.json quando ausente e painel de processos com decisões. `sdk-open-global-json` abre o arquivo no editor sem alterá-lo.
- **Fixture** `fixtures/MissingSdk` (csproj + `Program.cs` + `global.json` 8.0.999/`disable`) para o estado distinto de SDK ausente.

## Comandos e saídas reais

### Inventário nas raízes reais

```text
$ /usr/local/share/dotnet/dotnet --list-sdks
8.0.416 [/usr/local/share/dotnet/sdk]
10.0.100 [/usr/local/share/dotnet/sdk]

$ "$HOME/Library/Application Support/OpenRidr/sdks/8.0.408/dotnet" --list-sdks
8.0.408 [/Users/helder/Library/Application Support/OpenRidr/sdks/8.0.408/sdk]
```

O host devolve `sdk.host = {state:"found", version:"10.0.100", root:"/usr/local/share/dotnet"}` e `sdk.project = {state:"resolved", requestedVersion:"8.0.408", rollForward:"disable", resolvedVersion:"8.0.408", root:"/Users/helder/Library/Application Support/OpenRidr/sdks/8.0.408"}`. As duas raízes e as três versões aparecem no snapshot; o teste Rust falha se 10.0.100, 8.0.408 ou 8.0.416 não aparecerem.

### Host gate pela ponte HTTP de desenvolvimento

Log completo e comandos: [`logs/02-host-http.log`](logs/02-host-http.log). Trechos reais:

```text
$ POST /call {"method":"build.start","params":{}}
{"error":{"code":"trust_required","message":"build.start requires workspace trust; no process was started."},"ok":false}

$ POST /call {"method":"trust.grant","params":{}}  [selected fields]
"trust": {"decidedAt": 1793154417483, "granted": true, "revokedAt": null, "workspaceId": 1}
"host": {"state":"found","version":"10.0.100","root":"/usr/local/share/dotnet"}
"project": {"state":"resolved","requestedVersion":"8.0.408","rollForward":"disable","resolvedVersion":"8.0.408","root":"/Users/helder/Library/Application Support/OpenRidr/sdks/8.0.408"}

$ POST /call {"method":"build.start","params":{}}  (gate passes, not implemented in this slice)
{"error":{"code":"invalid_state","message":"build.start is not implemented in this build."},"ok":false}

$ POST /call {"method":"processes.probe","params":{"kind":"sleep"}}  [tracked process]
"processes": [{"id":1,"kind":"probe:sleep","pid":79783,"state":"running","exit":null,"awaitingDecision":false,"kept":false,"termination":null}]

$ POST /call {"method":"trust.revoke","params":{}}  [selected fields]
"trust": {"granted": false, "revokedAt": 1793154417562}
"mode": "restricted"
"project": "not_inspected"
"processes": [{"id":1,"state":"running","exit":null,"awaitingDecision":true,"kept":false}]

$ POST /call {"method":"build.start","params":{}}  (after revoke)
{"error":{"code":"trust_required","message":"build.start requires workspace trust; no process was started."},"ok":false}

$ POST /call {"method":"processes.terminate","params":{"id":1}}  [selected fields]
"processes": [{"id":1,"state":"exited","exit":null,"awaitingDecision":false,
  "termination":{"exit":null,"killedAfterGrace":false,"observed":"exited","requested":"SIGTERM"}}]

$ POST /call {"method":"processes.probe","params":{"kind":"sleep"}}  (closed workspace)
{"error":{"code":"trust_required","message":"processes.probe requires workspace trust; no process was started."},"ok":false}
```

`exit: null` é mostrado na UI como `exit unknown`; não é convertido em zero.

### testes Rust

```text
$ cargo test -p openridr-host
running 9 tests   (unit: inventário/parsing e políticas de resolução)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 15 tests  (boundary: gate, stale context, SDK real, fixture ausente, redetect, processos)
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Log: [`logs/02-cargo-test.log`](logs/02-cargo-test.log). Cobertura de resolução: exato+`disable`, `patch`, `feature`, `minor`, `latestMajor`, `latestPatch` com pré-release excluída/liberada, maior versão entre raízes, ausência. Processos: TERM observado, TERM ignorado → KILL após ~2 s (`killedAfterGrace:true`), keep, exit code 3, revogação e troca de workspace.

### E2E Playwright (Chrome real + Vite + host + arquivos reais)

```text
$ bun run test:e2e
Running 10 tests using 1 worker
  ✓ trust resolves the project SDK, revoke blocks calls and editing still works
  ✓ missing project SDK is distinct and global.json is never rewritten
  ✓ revoked workspace shows the remaining process and terminate observes the exit
  ✓ opens the fixture in restricted mode with trust available
  ✓ … 6 testes do ticket 01 continuam verdes
  10 passed (10.2s)
```

Log: [`logs/02-playwright.log`](logs/02-playwright.log). O teste de processo sobe o host com `OPENRIDR_ENABLE_PROBE=1` (somente teste); o probe não aparece na UI. O `global.json` do `MissingSdk` tem SHA-256 `f1b3d83e…d9859252` antes e depois de trust/redetect/E2E, e nenhum `bin`/`obj` foi criado ([`logs/02-environment.log`](logs/02-environment.log)).

### Frontend

```text
$ bun run build
✓ 52 modules transformed.
✓ built in 768ms
```

Log: [`logs/02-build.log`](logs/02-build.log). O aviso de chunk >500 kB é do CodeMirror, não é erro.

### Capturas da UI real

- [`screenshots/02-toolchain-restricted.png`](screenshots/02-toolchain-restricted.png) — workspace aberto, Restricted mode, host 10.0.100, projeto “Not inspected yet”, TFM declarada, debug não verificado, Apple Silicon OFF, Redetect.
- [`screenshots/02-toolchain-trusted.png`](screenshots/02-toolchain-trusted.png) — após trust: decisão no host, projeto resolvido 8.0.408 na raiz do usuário e host 10.0.100 separados.
- [`screenshots/02-toolchain-missing-sdk.png`](screenshots/02-toolchain-missing-sdk.png) — `fixtures/MissingSdk`: projeto 8.0.999 ausente com instruções e `Open global.json`, sem fallback para o host 10.0.100.
- [`screenshots/02-workspace-trusted.png`](screenshots/02-workspace-trusted.png) — workspace com status `Host SDK 10.0.100 · Project SDK 8.0.408`.
- [`screenshots/02-workspace-revoked.png`](screenshots/02-workspace-revoked.png) — após revoke: Restricted mode, projeto não inspecionado, edição preservada.

As imagens vêm do mesmo `dist/`/Vite servido ao Chrome real via ponte HTTP, que chama o mesmo `dispatch` do host; não são captura da janela Tauri nativa (limitação de TCC registrada no ticket 01).

## Cobertura do ticket e dos gates

| Item do ticket / gate | Resultado | Evidência |
|---|---|---|
| Inventário neutro fora do workspace; SDK do projeto só após trust; não substituir SDK fixado | Raízes PATH + `OpenRidr/sdks/*` + `~/.dotnet`; host 10.0.100, projeto 8.0.408; antes de trust `not_inspected` | ambiente, HTTP, `cargo-test`, E2E 1/2 |
| Host valida trust/workspace/contexto em toda chamada executável, inclusive direta | `require_trust` único; 11 métodos negados antes/depois de revoke; HTTP direto negado; nenhum `bin/obj` | HTTP, `cargo-test`, E2E 10 |
| SDK host/projeto ausentes distintos, instruções e redetecção; sem instalar SDK/alterar global.json/TFM | Estados `missing` (host) e `sdk_missing` (projeto); links oficiais; Redetect antes/depois; hash do `global.json` inalterado | E2E 2, `cargo-test`, ambiente |
| Revogar bloqueia imediatamente, encerra serviços e exige decisão para processos; lista e pending visíveis | Revoke zera trust/contexto, projeto volta a `not_inspected`, processo `running/awaitingDecision`; terminate/keep explícitos; sem declaração de limpeza | HTTP, `cargo-test`, E2E 3 |
| Troca/fechamento preserva edições, encerra/relata processos; trust não vaza nem ressuscita por resposta tardia | Close limpa trust/contexto e mantém processo visível; `accept_result` antigo → `stale_context`; confirmação de buffer do ticket 01 continua verde | `cargo-test`, E2E 3/8 |
| G1 — trust e fronteira | Stubs sem trust não iniciam processo; child controlado só nasce após trust; revoke bloqueia de novo; close/switch invalidam contexto | HTTP, `cargo-test` |
| G2 — contexto e integridade | `ContextToken`/`stale_context`; processo tem state/exit/awaiting separados; `exit null` = unknown; argumentos do probe separados/cwd workspace | `cargo-test`, E2E 3 |
| G3 — consentimento/instaladores | Nenhum download/instalação de SDK ou ferramenta; apenas inventário e links oficiais; nenhuma elevação | `cargo-test`, E2E 2 |
| G4 — UI/privacidade | UI inglesa, labels/foco/reason textual, estados com texto, sem telemetria; screenshots e asserts Playwright | screenshots, E2E |
| G5 — validação/proveniência | OS/arch/versões, comandos, logs, SDKs, TFM, exit observado/unknown e limitações abaixo | esta página + logs |

ACs tocados: **AC-01** (host-enforcement), **AC-02** (inventário, SDKs separados, regras e ausência), **AC-07** (revogação/processos; serviços reais ainda inexistentes nesta fatia), **AC-16 parcial** (pré-requisitos distintos; coexistência LSP real em 05), **AC-32/33** (superfície e evidência desta fatia).

## Limitações e pendências honestas

- **Windows não foi executado**. O término usa `TerminateProcess` (sem TERM gracioso) no branch não-Unix; a descoberta de SDKs usa layout macOS + `PATH` + `~/.dotnet`. `processes.probe` é Unix-only. Nenhum comportamento Windows é comprovado por esta evidência.
- **Desktop nativo não foi dirigido**: a janela Tauri não foi capturada/automatizada por TCC (ticket 01). A UI foi exercitada no Chrome real contra o mesmo host.
- **Nenhum serviço real para encerrar**: LSP/restore/evaluation ainda não existem. “Encerra serviços” nesta fatia = invalidar contexto e resolver o SDK; consumidores futuros repetirão os negativos.
- **Processos reais são controlados pelo probe de desenvolvimento**, não PTY (ticket 15) nem build/run (12). O probe exige `OPENRIDR_ENABLE_PROBE=1` e não tem UI; em uso normal, o painel de processos só mostra o que tickets posteriores iniciarem.
- **Resolução de projeto** lê apenas `global.json` no root do workspace; não sobe diretórios. JSON com comentários/trailing comma não é aceito pelo parser. Versão de 2 partes é normalizada com patch 0; ordenação de pré-release é lexical simplificada (`shortcut:` registrado no código se necessário).
- **MSBuild, restore, LSP, csharp-ls, debugger e TFM avaliada** pertencem a 03–06/13; aqui TFM é apenas “declared · not evaluated” e o adaptador é “not checked”.
- **Acessibilidade**: labels, foco visível e motivos textuais implementados/verificados por Playwright; não houve teste com leitor de tela nem auditoria WCAG.
- **Fixture `MissingSdk`** foi criada para o estado de SDK ausente e não é restaurada/compilada pelo app; nenhum SDK foi instalado nesta tarefa.
