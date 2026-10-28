# Evidência — Ticket 16: descobrir testes VSTest com identidade e escopo

Implementação local em 28/10/2026, base `5864bae` (ticket 15). Fonte normativa: [ticket 16](../tickets/16-descoberta-vstest.md), [SPEC](../SPEC.md) AC-03/27/29/30/31 (parciais), AC-01/07/32/33, P0.7 e gates G1–G5 do [índice de tickets](../tickets/README.md). Decisões: [decisions.md](../decisions.md) (Ticket 16) e [implementation-notes.md](../implementation-notes.md) (Descoberta de testes).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| Chrome / Playwright | Google Chrome 155.0.8059.85 / 1.64.1 |
| SDK do host / do projeto | 10.0.100 em `/usr/local/share/dotnet` / 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` |
| Fixture | `fixtures/OrderDesk`, copiada com espaço no caminho; `bin/obj` nunca copiados |
| Testes da fixture | 2 aprovados, 1 falha deliberada, 1 `[Fact(Skip)]` (4 identidades) |
| UI | Chrome real contra Vite + host-http; a janela Tauri não foi dirigida (TCC) |

Logs: [`16-environment.log`](logs/16-environment.log), [`16-drive.log`](logs/16-drive.log), [`16-trust-gate.json`](logs/16-trust-gate.json), [`16-restore-required.json`](logs/16-restore-required.json), [`16-discovery.json`](logs/16-discovery.json) + [`16-discovery.log`](logs/16-discovery.log), [`16-mtp.json`](logs/16-mtp.json) + [`16-mtp.log`](logs/16-mtp.log), [`16-adapter-missing.json`](logs/16-adapter-missing.json) + [`16-adapter-missing.log`](logs/16-adapter-missing.log), [`16-empty.json`](logs/16-empty.json) + [`16-empty.log`](logs/16-empty.log), [`16-failed.json`](logs/16-failed.json) + [`16-failed.log`](logs/16-failed.log), [`16-cancel.json`](logs/16-cancel.json) + [`16-cancel.log`](logs/16-cancel.log), [`16-revoke.json`](logs/16-revoke.json) + [`16-revoke-late.json`](logs/16-revoke-late.json), [`16-cargo-test.log`](logs/16-cargo-test.log), [`16-playwright.log`](logs/16-playwright.log), [`16-build.log`](logs/16-build.log), [`16-tauri-build.log`](logs/16-tauri-build.log). Nada em `fixtures/` foi restaurado/alterado pelas execuções (cópias temporárias); nenhum package/SDK foi instalado ou editado.

## O que foi construído

- **Host** (`crates/host/src/test_discovery.rs` + `lib.rs`): `tests.discover {project, targetFramework?}` e `tests.discover_cancel`, com `tests.discovery` no snapshot (`state`, `project{name,path}`, `targetFramework`, `command`, `cwd`, `sdkRoot`/`sdkVersion`, `log`/`logTruncated`, `exit`, `termination`, `processId`, `awaitingDecision`, `revision`, `discoveryId`, `workspaceId`/`contextId`, tempos, `items`, `partial`, `error`, `mtpMessage`, `stale`/`staleReason`, `sealed`). `tests` fica no nível do snapshot para o ticket 17 acrescentar execução sem quebrar campos.
- **Gate**: trust → `restore::require_restored` (mesma resposta `restore_required`/`action`) → SDK do projeto → avaliação atual (`ok|partial`, não stale, mesmo workspace/contexto) → projeto de teste explícito → TFM avaliado. Sem projeto, console/biblioteca, avaliação velha ou TFM não avaliado a resposta é `invalid_state` com motivo; nunca há restauração, instalação ou edição de package.
- **MTP antes de argv**: texto do `.csproj` (`Microsoft.Testing.Platform`, `UseMicrosoftTestingPlatformRunner`) e packageRefs MTP são verificados antes de qualquer comando. O estado é `mtp_unsupported` com a mensagem exata `Microsoft.Testing.Platform is not supported in this version. Use the terminal.`, `command: []`, `exit:null` e nenhum processo. Nada é convertido.
- **Adapter**: exige `Microsoft.NET.Test.Sdk` + `xunit.runner.visualstudio`/`NUnit3TestAdapter`/`MSTest.TestAdapter` nas referências avaliadas; ausência vira `adapter_missing` com orientação e `command: []`, distinto de vazio/falha. A UI mostra a orientação; nenhum package é adicionado.
- **Comando**: `[<sdk-root>/dotnet, "test", <csproj absoluto>, "-c", <Configuration>, "--no-restore", "--list-tests", "--nologo", "-v:minimal"]` + `["-f",<tfm>]` quando há TFM; cwd na raiz, `DOTNET_ROOT` do SDK do projeto, `DOTNET_CLI_UI_LANGUAGE=en`, grupo próprio no `Registry` (`kind:"tests:<Project>"`). Duas threads drenam stdout/stderr para o `LogBuffer` (stderr `[stderr]`, truncamento declarado).
- **Parser/identidade**: marcador exato `The following Tests are available:`; apenas linhas indentadas seguintes; a primeira linha não indentada (`No test is available …`) encerra a lista. Identidade = nome completamente qualificado trim; `id = test:<fnv1a64(project|tfm|fqn)>` estável, independente da ordem. `displayName` é o mesmo texto porque o VSTest só imprime um nome; nunca é usado sozinho como identidade. Repetidos deduplicam; nomes aninhados (`Outer+Inner`) e com espaços são preservados.
- **Estados**: `idle|discovering|completed|empty|failed|cancelled|adapter_missing|mtp_unsupported`. Exit 0 + marcador sem linhas = `empty`; exit 0 sem marcador ou exit ≠ 0 = `failed` com log; `exit:null` nunca vira sucesso. `partial:true` em cancelamento (itens já impressos preservados) e em falha após o marcador.
- **Cancelamento/fronteiras**: `tests.discover_cancel` é segurança sem trust, TERM→KILL no grupo, `termination` requested/observed separados, exit só observado. Uma descoberta por vez. Cada operação troca o `Arc<Tests>`; `sealed` descarta linhas/exit tardios, então discoveryId/revisão/contexto antigos não trocam itens/seleção. Revogar/fechar/trocar encerra e sela (revogar mantém o registro visível; fechar/trocar voltam a `idle`). `evaluation.run`, restore concluído, `sdk.redetect` e avaliação stale marcam **stale**, nunca `failed`.
- **UI** (`src/TestsPanel.tsx`): projeto de teste avaliado e TFM ligados ao contexto, Discover/Cancel/Retry, estado/`discoveryId`/revisão/contexto, escopo/comando/cwd/SDK/exit, log rolável, lista com checkbox e identidade + display name, contagens presas a `discoveryId`/revisão. MTP com a mensagem exata; `adapter_missing`, `empty`, `failed` e `cancelled` com textos distintos; `Restore now` explícito; editar buffer depois do início marca stale local (`editedAt` vs `startedAt`) e desabilita a seleção até nova descoberta; redescobrir sobre o disco salvo libera a seleção e mantém o buffer sujo para o ticket 17. Sem instalar/editar packages.

## Comandos e saídas reais

Driver real pela ponte HTTP de desenvolvimento, cópias temporárias, dotnet real. Caminhos encurtados só na prosa; logs guardam os completos.

### Antes de trust e sem restore

```text
$ tests.discover (Restricted)
{"ok":false,"error":{"code":"trust_required",
 "message":"tests.discover requires workspace trust; no process was started."}}

$ tests.discover (trusted, sem assets)
{"ok":false,"error":{"code":"restore_required",
 "message":"tests.discover needs current restore assets and OpenRidr never restores implicitly; run the explicit restore first.",
 "details":{"consumer":"tests.discover","action":{"method":"restore.start","params":{}}}}}
```

Logs: [`16-trust-gate.json`](logs/16-trust-gate.json), [`16-restore-required.json`](logs/16-restore-required.json).

### Descoberta real da fixture (4 identidades, exit 0)

```text
discoveryId: discovery:5:1 · state: completed · exit: 0 · partial: false
project: OrderDesk.Tests (tests/OrderDesk.Tests/OrderDesk.Tests.csproj)
targetFramework: net8.0
command: …/sdks/8.0.408/dotnet test …/Order Desk discovery/tests/OrderDesk.Tests/OrderDesk.Tests.csproj
         -c Debug --no-restore --list-tests --nologo -v:minimal -f net8.0
cwd: …/Order Desk discovery · sdk: 8.0.408 · /…/sdks/8.0.408
items (4):
  OrderDesk.Tests.OrderTests.Id_is_preserved                          id=test:b2493325b27f86ba
  OrderDesk.Tests.OrderTests.Total_is_preserved                       id=test:60c033142f203871
  OrderDesk.Tests.OrderTests.Deliberate_failure_for_runner_evidence   id=test:b86a5b14b8c8f93a
  OrderDesk.Tests.OrderTests.Skipped_until_fixture_changes            id=test:0a3490f86e8e57b4
--- raw log ---
  OrderDesk.Core -> …/src/OrderDesk.Core/bin/Debug/net8.0/OrderDesk.Core.dll
  OrderDesk.Tests -> …/tests/OrderDesk.Tests/bin/Debug/net8.0/OrderDesk.Tests.dll
Test run for …/OrderDesk.Tests.dll (.NETCoreApp,Version=v8.0)
The following Tests are available:
    OrderDesk.Tests.OrderTests.Id_is_preserved
    OrderDesk.Tests.OrderTests.Total_is_preserved
    OrderDesk.Tests.OrderTests.Deliberate_failure_for_runner_evidence
    OrderDesk.Tests.OrderTests.Skipped_until_fixture_changes
```

O log mostra que `--list-tests` **compila** o projeto antes de listar. Duas descobertas seguidas mantêm os mesmos ids e trocam `revision`/`discoveryId` (teste boundary `real_vstest_discovery…`). Logs: [`16-discovery.json`](logs/16-discovery.json), [`16-discovery.log`](logs/16-discovery.log).

### MTP: mensagem exata, nenhum comando VSTest

Projeto temporário com `<UseMicrosoftTestingPlatformRunner>true</UseMicrosoftTestingPlatformRunner>`:

```text
state: mtp_unsupported · command: (none) · exit: None
mtpMessage: Microsoft.Testing.Platform is not supported in this version. Use the terminal.
--- raw log ---
Microsoft.Testing.Platform is not supported in this version. Use the terminal.
processes (kind tests:*): []
```

Nenhum `Test run for` aparece no log e não há processo `tests:*`. Logs: [`16-mtp.json`](logs/16-mtp.json), [`16-mtp.log`](logs/16-mtp.log).

### Adapter ausente é distinto de vazio e de falha

```text
state: adapter_missing · command: (none) · exit: None
error: No VSTest adapter is configured for this project: Microsoft.NET.Test.Sdk and a VSTest
       adapter (xunit.runner.visualstudio, NUnit3TestAdapter or MSTest.TestAdapter).
       OpenRidr does not add or install packages; configure the adapter in the project,
       restore explicitly and discover again.
```

Projeto com adapter mas sem testes:

```text
state: empty · exit: 0 · items: 0
error: The VSTest adapter ran and listed no tests. This is not a discovery failure: …
raw log: … The following Tests are available:
No test is available in …/Empty.dll. Make sure that test discoverer & executors are registered…
```

Mesmo projeto com erro de compilação real:

```text
state: failed · exit: 1
error: dotnet test exited 1 before discovery completed; the raw log is preserved.
raw log: …/Code.cs(1,38): error CS0246: The type or namespace name 'MissingType' could not be found …
```

Logs: [`16-adapter-missing.log`](logs/16-adapter-missing.log), [`16-empty.log`](logs/16-empty.log), [`16-failed.log`](logs/16-failed.log).

### Cancelamento e revogação reais

Descoberta presa num target `Exec sleep 120` após o build; cancelamento durante o build:

```text
state: cancelled · partial: true · items: 0 · exit: 143
termination: {"requested":"SIGTERM","observed":"exited","exit":143,"killedAfterGrace":false}
raw log (preservado): "  Cancel -> …/Cancel.dll"
```

Revogação de trust durante outra descoberta:

```text
state: cancelled · sealed: true · stale: true · partial: true · exit: 143
termination: SIGTERM → observed exited · exit 143
0,7 s depois: mesmo state/finishedAt/log; trust.granted false
```

O caso com **itens parciais preservados** (2 nomes já listados, exit `null`) é coberto pelo teste unitário `cancel_preserves_partial_items_and_an_unknown_exit_stays_unknown`; a execução real acima terminou durante o build, antes do marcador, então tinha 0 itens. Logs: [`16-cancel.json`](logs/16-cancel.json), [`16-revoke.json`](logs/16-revoke.json), [`16-revoke-late.json`](logs/16-revoke-late.json).

## UI real (Playwright/Chrome 155, Vite + host-http)

1. **Descoberta real**: abre a solução, trust, avalia, restaura, recarrega, seleciona a aba Tests, projeto `OrderDesk.Tests`, TFM `net8.0`, Discover → `Discovered 4 tests · exit 0`; confere `discovery:<workspace>:1`, revisão 1, comando com `--no-restore --list-tests --nologo -v:minimal -f net8.0`, cwd/SDK/exit, as 4 identidades e o display name, e marca 1 checkbox (`4 tests · 1 selected`). Em seguida edita `OrderTests.cs` sem salvar: aparece **stale, not failed**, a mensagem “Discover again” e os 4 checkboxes ficam desabilitados; Discover again (lendo o disco salvo) volta a `completed` revisão 2 e libera os checkboxes, enquanto o buffer continua sujo para o ticket 17.
2. **MTP**: projeto temporário, restore explícito e Discover → mensagem exata, `No command was run.`, `exit unknown`, 0 itens; o snapshot confirma `command: []` e nenhum processo `tests:*` do workspace.
3. **Adapter ausente**: projeto temporário sem packages → `No VSTest adapter`, orientação com `Microsoft.NET.Test.Sdk`/`xunit.runner.visualstudio`, `No command was run.`.
4. **Trust**: chamada direta `tests.discover`/`tests.run` antes de trust → `trust_required`; `tests.discover_cancel` sem operação → `invalid_state`; revisão 0 e nenhum processo.

Capturas: [`screenshots/16-tests-discovery.png`](screenshots/16-tests-discovery.png), [`16-tests-stale.png`](screenshots/16-tests-stale.png), [`16-tests-mtp.png`](screenshots/16-tests-mtp.png), [`16-tests-adapter-missing.png`](screenshots/16-tests-adapter-missing.png).

## Testes automatizados

```text
$ cargo test -p openridr-host
running 106 tests  (unit; inclui MTP/adapter, identidades/duplicatas/nomes aninhados/espaços,
                    vazio vs falha, cancel parcial, selo/linha tardia, registro antigo)
test result: ok. 106 passed; 0 failed; 0 ignored
running 9 tests    (debugger)
test result: ok. 9 passed; 0 failed; 0 ignored
running 53 tests   (host boundary dos tickets 01–15)
test result: ok. 53 passed; 0 failed; 0 ignored
running 9 tests    (test_discovery: gate, escopo/TFM, multi-TFM explícito, fixture real, MTP,
                    adapter, vazio/CS0246, cancelamento e revogação reais)
test result: ok. 9 passed; 0 failed; 0 ignored
```

Log: [`16-cargo-test.log`](logs/16-cargo-test.log).

```text
$ bun run test:e2e
  45 passed (4.4m)
```

Log: [`16-playwright.log`](logs/16-playwright.log) (inclui os 41 testes anteriores verdes e os 4 novos).

```text
$ bun run build
✓ built in 816ms

$ bun run tauri build --debug --no-bundle
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.25s
Built application at: …/src-tauri/target/debug/openridr
```

Logs: [`16-build.log`](logs/16-build.log), [`16-tauri-build.log`](logs/16-tauri-build.log).

## Cobertura do ticket e dos gates

| Item do ticket / AC / gate | Resultado | Evidência |
|---|---|---|
| Descoberta real xUnit pelo adapter VSTest, projeto/TFM/comando/contexto/revisão/identidade explícitos | 4 identidades estáveis, comando/cwd/SDK/TFM/revisão no snapshot; `displayName` não é a identidade | `16-discovery.*`, boundary `real_vstest_discovery…`, E2E 1 |
| idle/discovering/completed/empty/failed/cancelled, log/retry, operação identificada, cancelamento parcial, resposta tardia descartada | Estados distintos; `discoveryId`/revisão; `partial`; `sealed`/`Arc` novo descarta resultado antigo; Retry na UI | unit `cancel_…`/`sealed_…`/`a_late_old_record…`, `16-cancel.*`, `16-revoke*`, E2E 1 |
| Nenhum teste vs adapter ausente vs falha com orientação própria; sem package/TRX; ausência de resultado ≠ sucesso | `empty`/`adapter_missing`/`failed` com textos próprios; nenhum package editado; nenhum TRX lido | `16-empty.*`, `16-adapter-missing.*`, `16-failed.*`, boundary, E2E 2/3 |
| MTP detectado antes de VSTest com mensagem exata, sem comando/conversão | `mtp_unsupported`, `command: []`, exit `null`, mensagem exata, nenhum processo | `16-mtp.*`, boundary `mtp_is_refused…`, E2E 2 |
| Trust/contexto, sem restore implícito; revogação/cancelamento observam árvore e preservam log/exit unknown | `trust_required`; `restore_required` com ação; TERM→KILL; requested/observed/exit separados; log preservado; `null` = unknown | `16-trust-gate.json`, `16-restore-required.json`, `16-cancel.*`, `16-revoke*`, unit, boundary |
| G1 — trust/fronteira | Nenhum processo antes de trust; cancelamento é ação de segurança | boundary, E2E 4 |
| G2 — contexto/integridade | workspace/contexto/revisão no registro; avaliação/TFM atuais exigidos; selo descarta resultado tardio; exit observado | boundary, `16-revoke*` |
| G3 — consentimento/instaladores | Nenhum download/instalação/edição de package nesta fatia | logs/ambiente |
| G4 — UI/privacidade | UI inglesa, selects/checkbox/botões nativos rotulados, foco/teclado, log/lista roláveis, motivo textual de bloqueio; sem telemetria | E2E 1–3, capturas |
| G5 — validação/proveniência | OS/arch/versões, SDK, comandos, exit/termination, revisões, logs e limitações | esta página + logs |

ACs tocados: **AC-03** (restore explícito), **AC-27 parcial** (descoberta real/escopo/identidade; execução em 17), **AC-29 parcial** (cancelamento e falha de infraestrutura), **AC-30** (vazio/adapter/falha e contagens de descoberta), **AC-31 parcial** (MTP e ausência de TRX), **AC-01/07** (gate e revogação), **AC-32/33** (superfície acessível e evidência).

## Limitações e pendências honestas

- **Windows x64, macOS Intel e CRLF não foram executados.** `dotnet.exe`, paths com `\`, término de árvore no Windows e saída CRLF não têm evidência; o ticket 18 deve repetir descoberta/MTP/adapter/cancelamento nesses ambientes.
- **`--list-tests` compila o projeto** (observado: linhas `OrderDesk.Core -> …`/`OrderDesk.Tests -> …` antes do marcador). Descoberta exige restore atual e um build bem-sucedido; um erro de compilação é `failed`, não `empty`. Não há `--no-build`.
- **Identidade é o único nome impresso pelo VSTest.** `--list-tests` não emite FQN e display name separados; `fullyQualifiedName` é o texto trim e `displayName` é o mesmo texto, usado só para exibição. Teorias/dados não foram enumerados nesta fixture; nenhum TRX ou mock foi usado como prova.
- **Detecção de adapter é conservadora**: exige `Microsoft.NET.Test.Sdk` e um dos três adapters conhecidos nas packageRefs avaliadas. Adapters customizados/outros frameworks aparecem como `adapter_missing` com a lista de adapters suportados; nada é instalado. Central package management continua funcionando porque as versões avaliadas vêm do MSBuild.
- **MTP é gate por presença**: a propriedade `UseMicrosoftTestingPlatformRunner` bloqueia mesmo com valor `false` e qualquer menção a `Microsoft.Testing.Platform` no texto/packageRefs bloqueia. É deliberadamente conservador; não há migração nem comando alternativo.
- **Cancelamento real** desta execução terminou durante o build, então preservou 0 itens com `partial:true`; a preservação de 2 itens já listados e exit `null` foi provada no teste unitário correspondente. O exit observado no cancelamento real foi 143; `null` é o caso “exit unknown”.
- **Seleção não executa testes**: escolher itens é só UI; `tests.run` continua stub (ticket 17). Nada é prometido sobre filtros/execução.
- **Uma descoberta por vez**: uma segunda chamada enquanto `discovering` é `invalid_state`; não há fila.
- **Log limitado** (400 linhas/60k, truncamento declarado). Saída de build do dotnet é preservada no log bruto.
- **UI E2E usa Chrome real** contra Vite + host-http (mesmo `dispatch`), não a janela Tauri nativa; captura TCC já registrada nos tickets anteriores.
- **Acessibilidade**: controles nativos com nomes/`title`, lista rolável e foco visível; não houve leitor de tela nem auditoria WCAG.
- **Fixtures**: todas as execuções usaram cópias temporárias; a fixture versionada só recebeu os dois testes novos (falha deliberada e skip), nada foi restaurado/compilado nela.
