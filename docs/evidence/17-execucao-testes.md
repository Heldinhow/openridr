# Evidência — Ticket 17: executar todos ou os selecionados e preservar resultados

Implementação local em 28/10/2026, base `0257d82` (ticket 16). Fonte normativa: [ticket 17](../tickets/17-execucao-testes.md), [SPEC](../SPEC.md) AC-03/27/28/29/30/31/01/07/32/33, P0.7 e gates G1–G5 do [índice de tickets](../tickets/README.md). Decisões: [decisions.md](../decisions.md) (Ticket 17) e [implementation-notes.md](../implementation-notes.md) (Execução de testes).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| Chrome / Playwright | Google Chrome 155.0.8059.85 / 1.64.1 |
| SDK do host / do projeto | 10.0.100 em `/usr/local/share/dotnet` / 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` |
| Fixture | `fixtures/OrderDesk`, copiada com espaço no caminho; `bin/obj` nunca copiados |
| Testes da fixture | 2 passed, 1 falha deliberada, 1 `[Fact(Skip)]` (4 identidades) |
| UI | Chrome real contra Vite + host-http; a janela Tauri não foi dirigida (TCC) |

Logs: `17-environment.log` *(evidência removida do repositório público)*, `17-duplicate-display.log` *(evidência removida do repositório público)*, `17-drive.log` *(evidência removida do repositório público)*, `17-trust-gate.json` *(evidência removida do repositório público)*, `17-restore-required.json` *(evidência removida do repositório público)*, `17-run-all.json` *(evidência removida do repositório público)* + `17-run-all.log` *(evidência removida do repositório público)* + `17-run-all.trx` *(evidência removida do repositório público)* + `17-run-all-raw.log` *(evidência removida do repositório público)*, `17-run-selected.json` *(evidência removida do repositório público)* + `17-run-selected.log` *(evidência removida do repositório público)*, `17-empty-selection.json` *(evidência removida do repositório público)*, `17-invalid-selection.json` *(evidência removida do repositório público)*, `17-infrastructure.json` *(evidência removida do repositório público)* + `17-infrastructure.log` *(evidência removida do repositório público)*, `17-stale.json` *(evidência removida do repositório público)*, `17-mtp.json` *(evidência removida do repositório público)*, `17-cancel.json` *(evidência removida do repositório público)* + `17-cancel.log` *(evidência removida do repositório público)* + `17-cancel-raw.log` *(evidência removida do repositório público)*, `17-revoke.json` *(evidência removida do repositório público)* + `17-revoke-late.json` *(evidência removida do repositório público)* + `17-revoke.log` *(evidência removida do repositório público)*, `17-cargo-test.log` *(evidência removida do repositório público)*, `17-playwright.log` *(evidência removida do repositório público)*, `17-build.log` *(evidência removida do repositório público)*, `17-tauri-build.log` *(evidência removida do repositório público)*. Nada em `fixtures/` foi restaurado/alterado pelas execuções (cópias temporárias); nenhum package/SDK foi instalado ou editado.

## O que foi construído

- **Host** (`crates/host/src/test_run.rs` + `lib.rs`): `tests.run {project, targetFramework?, selection:"all"|"selected", testIds?}` e `tests.run_cancel`, com `tests.run` no snapshot (`state`, projeto/configuração/TFM, `selection`/ids/`requestedCount`, `command`, `cwd`, `resultsDirectory`, `trxPath`, `logPath`, `trxState`, `log`/`logTruncated`, `exit`, `termination`, `processId`, `awaitingDecision`, `revision`, `runId`, `discoveryId`/`discoveryRevision`, workspace/contexto, tempos, `results`, `counts`, `infrastructureFailure`, `error`, `sealed`/`stale`).
- **Gate**: o mesmo `resolve_test_scope` da descoberta (trust → `restore::require_restored` → SDK do projeto → avaliação `ok|partial` não stale → projeto `kind:"test"` explícito → TFM avaliado). Além disso exige descoberta **atual** (`completed`, não stale/sealed, mesmo projeto/TFM/workspace/contexto) e recheca MTP antes de qualquer argv. `selected` valida ids contra os itens correntes; vazio/desconhecido/duplicado devolvem `empty_selection`/`invalid_selection`/`duplicate_selection` e nada roda — nunca cai para `all`. Nenhum restore/build implícito; nenhum package/logger adicional.
- **Comando** (argv separado): `[<sdk-root>/dotnet, "test", <csproj absoluto>, "-c", <Configuration>, "--no-build", "--no-restore", "--nologo", "-v:minimal"]` + `["-f",<tfm>]` + `["--filter","FullyQualifiedName=A|FullyQualifiedName=B"]` só em selecionado + `["--logger","trx;LogFileName=openridr.trx","--results-directory",<appHome>/test-results/<ws>-<seq>-<ms>]`. cwd na raiz do workspace, `DOTNET_ROOT` do SDK do projeto, `DOTNET_CLI_UI_LANGUAGE=en` e grupo próprio no `Registry` (`kind:"tests:run:<Project>"`). Um run por vez, nunca junto de uma descoberta.
- **TRX**: parser estreito sem dependência XML. `<UnitTestResult>` fornece outcome/duration/Output; `<TestDefinitions>`/`<TestMethod className name>` mapeia FQN → identidade descoberta (não pelo label). `Passed/Failed/NotExecuted` ficam como a fonte diz; `NotExecuted` só vira `skipped` com a linha explícita `Test 'FQN' was skipped in the test run.` do `ResultSummary` (xUnit); senão `notExecuted`; outcome desconhecido fica `unknown`. TRX parcial entrega os resultados completos + `partial` e, fora de cancelamento, é `failed` de infraestrutura; ausente/corrompido idem, com caminho bruto. `completed` inclui asserção (exit 1 explicado por failed/notExecuted); exit ≠ 0 sem teste que explique, exit `null` ou TRX sem resultado é `failed` + `infrastructureFailure:true`. Resultados são limitados a 2.000 com `resultsTruncated`.
- **Cancelamento/fronteiras**: `tests.run_cancel` é ação de segurança sem trust, TERM→KILL no grupo, `termination` requested/observed, exit só observado (pode ser `null`). Resultados parciais/TRX/log ficam, estado `cancelled`, nenhum teste vira `cancelled`. Revogar/fechar/trocar termina+sela (`sealed`/`staleReason`); avaliação/restore/redetect marcam stale preservando resultados. Cada run troca o `Arc<TestRun>`, então linha/exit tardio do run antigo não substitui o atual.
- **Posição**: só de `in <file>:line <n>` no stack trace quando o arquivo canonicaliza dentro do workspace; path relativo e linha 0-based. Sem posição, sem botão de navegação e o log/TRX fica disponível. Nunca inferida do nome.
- **UI** (`src/TestsPanel.tsx`): Run all/Run selected/Cancel/Retry com motivo textual, estado/`runId`/`discoveryId`/revisões/contexto, seleção/comando/exit/TRX/log, contagens por outcome, lista com `<details>` (message/stack/stdout/stderr), “Open source position” só com posição real e aviso + log quando ausente, badge stale/sealed. Controles nativos e focáveis.

## Comandos e saídas reais

Driver real pela ponte HTTP de desenvolvimento, cópias temporárias, dotnet real, `OPENRIDR_HOME` temporário. Caminhos encurtados só na prosa; logs guardam os completos.

### Antes de trust e sem restore

```text
$ tests.run (Restricted)
{"ok":false,"error":{"code":"trust_required",
 "message":"tests.run requires workspace trust; no process was started."}}

$ tests.run (trusted, sem assets)
{"ok":false,"error":{"code":"restore_required",
 "message":"tests.run needs current restore assets and OpenRidr never restores implicitly; run the explicit restore first.",
 "details":{"consumer":"tests.run","action":{"method":"restore.start","params":{}}}}}
```

Logs: `17-trust-gate.json` *(evidência removida do repositório público)*, `17-restore-required.json` *(evidência removida do repositório público)*.

### Run all real da fixture (2 passed, 1 failed, 1 skipped, exit 1)

```text
runId: testrun:1:1 · discoveryId: discovery:1:1 · discovery revision 1 · run revision 1 · context 2
state: completed · exit: 1 · infrastructureFailure: False · trxState: ok
selection: all · requested: 4
command: …/8.0.408/dotnet test …/Order Desk run/tests/OrderDesk.Tests/OrderDesk.Tests.csproj
         -c Debug --no-build --no-restore --nologo -v:minimal -f net8.0
         --logger trx;LogFileName=openridr.trx --results-directory …/app-home/test-results/1-1-…
counts: {"failed":1,"matched":4,"notExecuted":0,"passed":2,"skipped":1,"total":4,"unknown":0}
results (4):
  [failed] OrderDesk.Tests.OrderTests.Deliberate_failure_for_runner_evidence · 7 ms · trx Failed
           · position tests/OrderDesk.Tests/OrderTests.cs:29:0
    message: Assert.Equal() Failure: Values differ
    stack: at …Deliberate_failure_for_runner_evidence() in …/OrderTests.cs:line 29
  [skipped] OrderDesk.Tests.OrderTests.Skipped_until_fixture_changes · 1 ms · trx NotExecuted
    message: Fixture evidence: skipped on purpose until the fixture changes.
  [passed] OrderDesk.Tests.OrderTests.Id_is_preserved · 0 ms · trx Passed
  [passed] OrderDesk.Tests.OrderTests.Total_is_preserved · 0 ms · trx Passed
```

Os 4 resultados casam com as identidades da descoberta (`testId` preenchido, `matched:true`); só o failed tem stack/posição. TRX/log brutos copiados para `17-run-all.trx` *(evidência removida do repositório público)* e `17-run-all-raw.log` *(evidência removida do repositório público)*; o run original ficou no app home temporário. Logs: `17-run-all.json` *(evidência removida do repositório público)*, `17-run-all.log` *(evidência removida do repositório público)*.

### Run selected exato (1 passed, exit 0, filtro real)

```text
runId: testrun:1:2 · discoveryId: discovery:1:1 · run revision 2
selection: selected · requested: 1
command: … dotnet test … -c Debug --no-build --no-restore --nologo -v:minimal -f net8.0
         --filter FullyQualifiedName=OrderDesk.Tests.OrderTests.Id_is_preserved
         --logger trx;LogFileName=openridr.trx --results-directory …
counts: {"failed":0,"matched":1,"passed":1,"total":1,"unknown":0}
results (1): [passed] OrderDesk.Tests.OrderTests.Id_is_preserved · 1 ms
```

Logs: `17-run-selected.json` *(evidência removida do repositório público)*, `17-run-selected.log` *(evidência removida do repositório público)*.

Seleção vazia e id desconhecido bloqueiam sem processo e sem cair para todos:

```text
selection=selected testIds=[]       → invalid_state · empty_selection
selection=selected testIds=[test:not-current] → invalid_state · invalid_selection
```

Logs: `17-empty-selection.json` *(evidência removida do repositório público)*, `17-invalid-selection.json` *(evidência removida do repositório público)*.

### Nomes de exibição duplicados (ferramenta real)

Projeto xUnit temporário com dois testes `[Fact(DisplayName="Same display")]`; o VSTest imprime o mesmo nome duas vezes e não fornece FQN na lista:

```text
$ dotnet test Disp.csproj -c Debug --no-restore --list-tests --nologo -v:minimal
The following Tests are available:
    Same display
    Same display

$ dotnet test Disp.csproj … --filter "FullyQualifiedName=Same display"
No test matches the given testcase filter `FullyQualifiedName=Same display` …

$ dotnet test Disp.csproj … --logger trx (all)
Passed!  - Failed: 0, Passed: 2, Skipped: 0, Total: 2
TRX TestDefinitions: className="Disp.B" name="Two" · className="Disp.A" name="One"
```

O host mantém **uma** identidade com `occurrences:2`; `tests.run selected` recusa com `ambiguous_selection` (não dá para mirar uma cópia) e `Run all` sem filtro executa os dois testes reais (2 passed), mostrando os resultados como não casados (`matched:0`) porque a lista não tinha FQN. Boundary `duplicate_display_names_are_one_ambiguous_identity_never_a_selected_run`. Log: `17-duplicate-display.log` *(evidência removida do repositório público)*.

### Cancelamento real de um teste xUnit longo (`Task.Delay(60000)`)

```text
runId: testrun:3:3 · discoveryId: discovery:3:3
state: cancelled · exit: 143 · trxState: missing
termination: {"requested":"SIGTERM","observed":"exited","exit":143,"killedAfterGrace":false}
counts: {"passed":0,"failed":0,"skipped":0,"notExecuted":0,"unknown":0,"total":0}
error: Cancellation was requested; the run was terminated with exit 143. No test result was
       reported, so this run is not a success and no missing result is passed or skipped.
```

O VSTest foi morto antes de gravar TRX, então 0 resultados é o honesto; o log/`run.log` foi preservado. A preservação de um TRX **parcial** com resultados já escritos é provada no teste unitário `cancel_keeps_partial_results_and_never_marks_a_test_cancelled`. Logs: `17-cancel.json` *(evidência removida do repositório público)*, `17-cancel.log` *(evidência removida do repositório público)*.

### Falha de infraestrutura distinta de asserção (TRX ausente, exit 1)

Assembly de teste removido antes de `--no-build`:

```text
state: failed · exit: 1 · infrastructureFailure: True · trxState: missing
counts: {"passed":0,"failed":0,"skipped":0,"notExecuted":0,"unknown":0,"total":0}
error: The test host finished with exit 1 without writing a TRX file; no test result is known,
       so this is an infrastructure failure. The raw output is preserved.
```

Em contraste, o Run all acima tem `exit 1` explicado por um teste `Failed` e fica `completed`, não `failed` de infraestrutura. Logs: `17-infrastructure.json` *(evidência removida do repositório público)*, `17-infrastructure.log` *(evidência removida do repositório público)*.

### Stale após reavaliação (resultados preservados)

```text
run.state: completed · stale: true · results: 1
staleReason: The project model was re-evaluated; these test results are not current.
```

Log: `17-stale.json` *(evidência removida do repositório público)*.

### MTP continua bloqueado no run direto

```text
{"ok":false,"error":{"code":"invalid_state",
 "message":"Microsoft.Testing.Platform is not supported in this version. Use the terminal.",
 "details":{"reason":"mtp_unsupported"}}}
```

Nenhum processo `tests:*` nasce. Log: `17-mtp.json` *(evidência removida do repositório público)*.

### Revogação durante run longo

```text
state: cancelled · sealed: true · stale: true · exit: 143
termination: SIGTERM → observed exited · exit 143
termination/exit/log preservados; trust.granted: false
0,7 s depois: mesmo state/finishedAt/log (resultado tardio não troca o run)
```

Logs: `17-revoke.json` *(evidência removida do repositório público)*, `17-revoke-late.json` *(evidência removida do repositório público)*, `17-revoke.log` *(evidência removida do repositório público)*.

## UI real (Playwright/Chrome 155, Vite + host-http)

1. **Run all e detalhes**: fixture real, discover 4, Run all → `Run completed`, `exit 1`, `4 results · 2 passed · 1 failed · 1 skipped · 0 not executed · 0 unknown`; comando mostra `--no-build --no-restore … --logger trx;LogFileName=openridr.trx --results-directory`; TRX `ok`; a linha do failed mostra mensagem `Assert.Equal() Failure`/`Expected: 43`/`Actual: 42` e stack `OrderTests.cs:line 29`; “Open source position” abre `tests/OrderDesk.Tests/OrderTests.cs` no cursor `28:0`. Run selected (1 checkbox) mostra `1 result · 1 passed`, filtro exato e `testrun:N:2`. Captura: `screenshots/17-tests-run-all.png` *(evidência removida do repositório público)*.
2. **Falha de infraestrutura**: com o assembly removido, Run all → `Run failed`, mensagem de infraestrutura, TRX `missing`. Captura: `screenshots/17-tests-run-infrastructure.png` *(evidência removida do repositório público)*.
3. **Cancelamento**: projeto temporário com `Task.Delay(60000)`, Run selected → Running → Cancel run → `Run cancelled`, `Observed: exited`, `0 passed`, API `state:"cancelled"`, `counts.passed:0`, termination observada; log preservado. Captura: `screenshots/17-tests-run-cancel.png` *(evidência removida do repositório público)*.
4. **Revogação**: segundo run longo, revoke durante Running → selo visível (`not current`), `Run cancelled`, `sealed:true`, `stale:true`, mesmo `finishedAt`/log 0,7 s depois. Captura: `screenshots/17-tests-run-revoke.png` *(evidência removida do repositório público)*.
5. **Stale por edição**: após Run selected, editar `OrderTests.cs` sem salvar → badge “preserved but not current” com contagens ainda visíveis; os checkboxes de descoberta ficam bloqueados (ticket 16).

## Testes automatizados

```text
$ cargo test -p openridr-host
test result: ok. 116 passed; 0 failed  (unit; inclui test_run::tests)
test result: ok. 9 passed            (debugger)
test result: ok. 53 passed           (host boundary 01–15)
test result: ok. 9 passed            (test_discovery)
test result: ok. 5 passed            (test_run: gates, MTP, fixture real all/selected, cancel+revoke, DisplayName duplicado)
```

Log: `17-cargo-test.log` *(evidência removida do repositório público)*.

```text
$ bun run test:e2e
  47 passed (4.0m)
```

Log: `17-playwright.log` *(evidência removida do repositório público)* (inclui os 45 anteriores verdes e os 2 novos).

```text
$ bun run build
✓ built in 870ms

$ bun run tauri build --debug --no-bundle
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.58s
Built application at: …/src-tauri/target/debug/openridr
```

Logs: `17-build.log` *(evidência removida do repositório público)*, `17-tauri-build.log` *(evidência removida do repositório público)*.

## Cobertura do ticket e dos gates

| Item do ticket / AC / gate | Resultado | Evidência |
|---|---|---|
| Run all/selected com identidades válidas, filtro real e bloqueio de vazio/desconhecido/duplicado | Fixture: 4 all / 1 selected; filtro `FullyQualifiedName=…`; `empty_selection`/`invalid_selection`/`duplicate_selection` sem processo nem fallback para all; DisplayName duplicado vira `ambiguous_selection` no selected e run all sem filtro executa os dois | `17-run-all.*`, `17-run-selected.*`, `17-empty-selection.json`, `17-invalid-selection.json`, `17-duplicate-display.log`, unit/boundary |
| Estados e resultados passed/failed/skipped/notExecuted/unknown ligados a run/discovery/revisão/contexto | `completed`+exit 1 de asserção vs `failed` infra vs `cancelled`; contagens/`runId`/`discoveryId`/revisões na UI e API; stale preserva | `17-run-all.*`, `17-infrastructure.*`, `17-cancel.*`, `17-stale.json`, E2E 1–3 |
| Detalhes message/stack/stdout/stderr/log bruto; navegação só com posição real | TRX message/stack/position reais; stdout/stderr unit; sem posição → aviso + log; caminho fora do workspace não navega | `17-run-all.log`, unit `a_real_trx…`/`a_stack_position…`, E2E 1 |
| Cancelamento parcial, infra vs asserção, logs preservados, retry explícito | cancel exit 143/0 results, `cancelled`; TRX parcial unit; infra `missing`+exit 1; Retry run na UI | `17-cancel.*`, `17-infrastructure.*`, unit `cancel_…`, E2E 1/3 |
| TRX estruturado, sem package/logger adicional, MTP/trust/sem restore implícito | logger embutido; `trust_required`/`restore_required`; MTP run recusado antes de argv; nenhum package/SDK instalado | `17-trust-gate.json`, `17-restore-required.json`, `17-mtp.json`, logs/ambiente |
| Revogação/contexto/resultado tardio; G1–G5; UI acessível | run selado, mesmo `finishedAt`/log depois; controles nativos com nome/motivo; listas/logs roláveis | `17-revoke*`, boundary, E2E 4 |
| G1 — trust/fronteira | Nenhum processo antes de trust; cancelar é ação de segurança | `17-trust-gate.json`, boundary |
| G2 — contexto/integridade | workspace/contexto/revisões no registro; descoberta atual exigida; selo descarta resultado tardio; exit observado/`null` | `17-stale.json`, `17-revoke-late.json`, unit/boundary |
| G3 — consentimento/instaladores | Nenhum download/instalação/edição de package; só TRX embutido | ambiente/logs |
| G4 — UI/privacidade | UI inglesa, botões/checkbox/details nativos, foco/teclado, motivos textuais, sem telemetria | E2E 1–4, capturas |
| G5 — validação/proveniência | OS/arch/versões, SDK, comandos/argv, TRX/log paths, exit/termination, revisões, limites declarados | esta página + logs |

ACs tocados: **AC-27/28** (execução all/selected, resultados/detalhes/navegação), **AC-29** (cancelamento parcial, infra vs asserção), **AC-30** (contagens ligadas a operação/revisão, sem package novo), **AC-31** (MTP/trust/restore no host), **AC-03** (restore explícito), **AC-01/07** (gate/revogação), **AC-32/33** (superfície acessível e evidência).

## Limitações e pendências honestas

- **Windows x64, macOS Intel e CRLF não foram executados.** `dotnet.exe`, paths `\`, término de árvore e saída CRLF não têm evidência; o ticket 18 deve repetir all/selected/cancel/infra nesses ambientes.
- **Cancelamento real desta execução preservou 0 resultados** porque o VSTest foi morto antes de gravar TRX. A leitura de TRX parcial com resultados completos e exit desconhecido/cancelado é provada no teste unitário, não em E2E; um TRX parcial fora de cancelamento é `failed` de infraestrutura com os resultados parciais preservados.
- **TRX corrompido** é provado em unidade; o E2E usa TRX ausente (assembly removido) como falha de infraestrutura real. Não há seam de arquivo para corromper o TRX de um run real.
- **`skipped` depende do xUnit adapter**: o TRX só tem `NotExecuted`; a linha explícita `Test 'FQN' was skipped in the test run.` do `ResultSummary` é usada. Outro adapter sem essa linha fica `notExecuted`, nunca inventado.
- **DisplayName duplicado** foi exercitado com ferramenta real: a lista do VSTest não tem FQN e imprime o mesmo nome duas vezes, então a seleção individual é recusada (`ambiguous_selection`) e Run all executa os dois sem filtro; os resultados ficam `matched:false` (visíveis e sem posição se o stack não existir). **Teorias/dados** continuam sem evidência nesta fixture; a identidade é o FQN da descoberta e o resultado só casa por TestDefinitions FQN.
- **Posição** exige `in <file>:line <n>` absoluto/relativo resolvido no workspace; stack sem linha, arquivo simbólico/ausente/fora não navega. A linha mostrada ao usuário é 1-based, o alvo do editor é 0-based.
- **Uma execução por vez** e sem fila; iniciar run durante descoberta (ou vice-versa) é `invalid_state`. O `--no-build` depende do build feito pela descoberta (`--list-tests` compila); assembly removido é falha de infraestrutura, não rebuild implícito.
- **Resultados limitados a 2.000** com `resultsTruncated`; o TRX bruto no app home continua completo.
- **E2E usa Chrome real** contra Vite + host-http (mesmo `dispatch`), não a janela Tauri nativa; captura TCC já registrada nos tickets anteriores.
- **Acessibilidade**: controles nativos com nomes/`title`, `<details>` e listas roláveis, foco visível; não houve leitor de tela nem auditoria WCAG.
