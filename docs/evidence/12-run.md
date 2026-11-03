# Evidência — Ticket 12: executar a saída avaliada e cancelar processos

Implementação local em 28/10/2026, base `5e8e637` (ticket 11). Fonte normativa: [ticket 12](../tickets/12-run.md), [SPEC](../SPEC.md) AC-03/10/15/17/18/19/20/01/07/23/24/32/33 e gates G1–G5 do [índice de tickets](../tickets/README.md). Decisões: [decisions.md](../decisions.md) (Ticket 12) e [implementation-notes.md](../implementation-notes.md) (Run).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| Chrome / Playwright | Google Chrome 155.0.8059.85 (canal `chrome`) / 1.64.1 |
| SDK do host | 10.0.100 em `/usr/local/share/dotnet` |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408`, `global.json` com `rollForward: disable` |
| Fixture | `fixtures/OrderDesk`, sempre copiada para diretório temporário; `Program.cs` alterado só nas cópias |
| App home do driver | temporário (`OPENRIDR_HOME=/tmp/openridr-12-home-*`); o E2E usa `$TMPDIR/openridr-e2e-home` |

Logs: `logs/12-environment.log` *(evidência removida do repositório público)*, `logs/12-run-ok.log` *(evidência removida do repositório público)* + `12-run-ok.json` *(evidência removida do repositório público)*, `logs/12-run-source-changed.json` *(evidência removida do repositório público)*, `logs/12-run-failed-build.json` *(evidência removida do repositório público)*, `logs/12-run-library.json` *(evidência removida do repositório público)*, `logs/12-run-missing-target.json` *(evidência removida do repositório público)*, `logs/12-run-cancel.log` *(evidência removida do repositório público)* + `12-run-cancel.json` *(evidência removida do repositório público)*, `logs/12-run-revoke.log` *(evidência removida do repositório público)* + `12-run-revoke.json` *(evidência removida do repositório público)* + `12-run-revoke-late.json` *(evidência removida do repositório público)*, `logs/12-cargo-test.log` *(evidência removida do repositório público)*, `logs/12-playwright.log` *(evidência removida do repositório público)*, `logs/12-build.log` *(evidência removida do repositório público)*, `logs/12-tauri-build.log` *(evidência removida do repositório público)*. Nenhum SDK/ferramenta foi instalado; nenhum arquivo versionado de `fixtures/` foi restaurado/alterado.

## O que foi construído

- **Host** (`crates/host/src/run.rs` + `lib.rs`): `run.start {project?, configuration?, targetFramework?}` e `run.cancel`, com `run` no snapshot (`state`, `project{name,path}`, `targetPath`, `buildId`, configuração/TFM, `command`, `cwd`, `log`/`logTruncated`, `exit`, `termination`, `processId`, `awaitingDecision`, `revision`, `runId`, `workspaceId`/`contextId`, tempos, `sealed`, `stale`, `staleReason`).
- **Gate**: trust → avaliação atual (`ok|partial`, não stale, mesmo workspace/contexto) → build atual do mesmo workspace/contexto, `state:"ok"`, `exit:0`, não stale/sealed, mesmo projeto/configuração/TFM, mesma revisão de avaliação e mesmo `TargetPath`. `library`/`test`/`unknown`, build falho, build/evaluation antigos, configuração/TFM não avaliados e `TargetPath` ausente são recusados com motivo, `details.reason`, `action` e `buildLog` quando existe. Nenhum restore/build é disparado por Run e nenhum path é substituído.
- **Fonte salva**: `build.start` captura o hash FNV-1a dos fontes salvos do startup e das referências transitivas; `run.start` relê e devolve `build_stale`/`reason:"source_changed"` com `changedSources`. Buffer sujo não entra na verificação; a UI avisa e oferece decisão explícita.
- **Processo**: argv separado `[<sdk-root>/dotnet, <TargetPath>]` para `.dll` gerenciado (executável nativo roda direto e é recusado sem bit de execução), cwd no diretório do `.csproj`, `DOTNET_ROOT` do projeto, env explícito e grupo próprio no `Registry` compartilhado (`kind:"run:<Project>"`). Duas threads drenam stdout/stderr para o `LogBuffer` (stderr `[stderr]`, 400 linhas/60k, truncamento declarado); o log sobrevive a exit/falha/cancelamento.
- **Exit e cancelamento**: `exited` é neutro e mostra o código observado; `exit:null` é “exit unknown”, nunca sucesso. `run.cancel` é ação de segurança sem trust, pede TERM→KILL ao grupo e reporta `requested`/`observed`/`exit`; um run por vez.
- **Fronteiras**: revogar/fechar/trocar termina o grupo e sela o registro; linhas/exit tardios não mudam o run do contexto novo. Revogar mantém o registro selado visível; fechar/trocar o descartam da aba, mas o processo continua no painel de processos.
- **UI**: a barra de startup (Startup | Configuration | TFM | Build | Run) ganhou `Run (Ctrl+R)`/`Stop`, `run-state`, identidade (run/build/targetPath), comando/cwd, exit, término, log de Run separado e preservado, motivo textual de bloqueio e o diálogo `run-dirty-dialog` (“Run saved build” / “Save all and rebuild” / “Cancel”). O aviso `run-dirty-warning` diz “Running the saved build; N unsaved buffer(s) are not included”. Cmd+R (macOS) e Shift+F10 (Windows) usam o mesmo gate; biblioteca nunca aparece como startup.

## Comandos e saídas reais

Driver real via ponte HTTP de desenvolvimento, cópia temporária com espaços no caminho e SDK 8.0.408. Caminhos encurtados só na prosa; os logs guardam os completos.

### Run ok (exit 0, stdout real)

```text
$ run.start → workspace.state
state: exited
exit: 0
runId: run:1:1
buildId: build:1:1
targetPath: /…/Order Desk run evidence/src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll
command: /…/OpenRidr/sdks/8.0.408/dotnet /…/src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll
cwd: /…/Order Desk run evidence/src/OrderDesk.Cli
configuration: Debug · targetFramework: None
--- raw run log ---
Order A-104: R$ 42,00
```

Log: `logs/12-run-ok.log` *(evidência removida do repositório público)*.

### Fonte salva depois do build → `build_stale` com arquivo e log do build

```text
$ run.start (após salvar Program.cs)
{"code":"build_stale",
 "message":"Run was not started: 1 saved source changed after this build (src/OrderDesk.Cli/Program.cs). Rebuild before running.",
 "details":{"reason":"source_changed",
            "buildId":"build:1:1",
            "changedSources":["src/OrderDesk.Cli/Program.cs"],
            "action":{"method":"build.start","params":{...}},
            "buildLog":"… Build succeeded. …"}}
```

### Build falho → Run recusado com exit e log

```text
$ run.start (último build failed)
{"code":"invalid_state",
 "message":"Run was not started: the last build did not succeed (failed · exit 1). Fix the build before running.",
 "details":{"reason":"build_failed","buildId":"build:1:2","exit":1,
            "buildLog":"… error CS1513: } expected … Build FAILED. …"}}
```

### Biblioteca, TargetPath ausente

```text
$ run.start project=src/OrderDesk.Core/OrderDesk.Core.csproj
{"code":"invalid_state",
 "message":"OrderDesk.Core is a library project; only a console executable project can be the run startup project."}

$ run.start (com o .dll do build removido do disco)
{"code":"invalid_state",
 "message":"Run was not started: the built TargetPath …/OrderDesk.Cli.dll is missing (No such file or directory (os error 2)). Rebuild before running.",
 "details":{"reason":"target_missing","action":{"method":"build.start","params":{}}}}
```

### Cancelamento real de programa com filho

Projeto temporário real que imprime `parent <pid> child <pid>`, inicia `/bin/sleep 60` e dorme 60 s. `run.cancel` no grupo:

```text
state: cancelled
exit: 143
runId: run:2:2
buildId: build:2:4
termination: {"exit":143,"killedAfterGrace":false,"observed":"exited","requested":"SIGTERM"}
--- raw run log (preserved) ---
parent 58236 child 58237
long-running
```

Log: `logs/12-run-cancel.log` *(evidência removida do repositório público)*. O teste Rust `cancelling_a_real_run_observes_termination_of_the_group_including_its_child` também lê os dois pids do log e confirma com `/bin/ps` que pai e filho desapareceram, compara o exit do run com o do registro e recusa um segundo `run.start` concorrente.

### Revogação sela o run

```text
trust.revoke while the program was sleeping
state: cancelled · sealed True · stale True
staleReason: Trust was revoked while this program was running; it was stopped and its result is not current.
termination: {"exit":143,"killedAfterGrace":false,"observed":"exited","requested":"SIGTERM"}
0.7 s later: state cancelled · finishedAt unchanged True · log has late lines: False
```

Logs: `logs/12-run-revoke.json` *(evidência removida do repositório público)* e `logs/12-run-revoke-late.json` *(evidência removida do repositório público)*.

## UI real (Playwright/Chrome 155, Vite + host-http)

Dois testes novos, cópias temporárias e dotnet real:

1. **“a real run shows the saved build output and refuses a source saved after the build”** — restore/avalia/build reais; a lista de startup só oferece `OrderDesk.Cli` (a biblioteca não aparece); Run mostra `Order A-104:`/`exit 0`, buildId, comando, cwd real e log preservado. Buffer sujo → aviso “Running the saved build; 1 unsaved buffer is not included” e diálogo com Run saved build/Save all and rebuild; “Run saved build” executa e a saída continua `Order A-104:` (não `B-777`). Salvar → Run desabilitado com “Sources were saved after this build”, Cmd+R responde o mesmo motivo; rebuild → Run mostra `Order B-777:` e Cmd+R repete a execução com outra identidade.
2. **“stopping a real long-running program reports requested and observed termination”** — projeto console temporário real (`Thread.Sleep(60_000)`), restore/avalia/build/run reais; Stop mostra `Run cancelled`, `Termination requested: SIGTERM`, `Observed: exited`, exit observado (143) e log preservado.

Capturas: `screenshots/12-run-output.png` *(evidência removida do repositório público)*, `12-run-dirty.png` *(evidência removida do repositório público)*, `12-run-saved-stale.png` *(evidência removida do repositório público)*, `12-run-rebuilt.png` *(evidência removida do repositório público)*, `12-run-cancelled.png` *(evidência removida do repositório público)*.

## Testes automatizados

```text
$ cargo test -p openridr-host
running 74 tests  (unit: run exit/null/unknown, cancel selado, target ausente/não executável,
                   muxer para .dll, hash de fontes do startup+referências, detecção de edição)
test result: ok. 74 passed; 0 failed; 0 ignored
running 49 tests  (boundary: gate antes de trust, build falho/stale, fonte salva alterada,
                   biblioteca, TargetPath ausente, execução real com saída/exit, cancelamento real
                   de pai+filho, revogação selada/late, um run por vez)
test result: ok. 49 passed; 0 failed; 0 ignored
```

Log: `logs/12-cargo-test.log` *(evidência removida do repositório público)*. Numa execução intermediária, o teste LSP `initialize_failure…` falhou transitoriamente com `InvalidOperationException` do csharp-ls e os testes seguintes falharam por poison do `ENV_LOCK`; a execução completa seguinte (a do log) passou 74+49. Não houve mudança de código entre as duas.

```text
$ bun run test:e2e
  ✓ run.spec.ts:79 a real run shows the saved build output and refuses a source saved after the build
  ✓ run.spec.ts:172 stopping a real long-running program reports requested and observed termination
  … 34 testes dos tickets 01–11 continuam verdes
  36 passed (3.1m)
```

Log: `logs/12-playwright.log` *(evidência removida do repositório público)*. Portas 4317/1420 foram encerradas antes e ficaram livres depois (`lsof` sem saída).

```text
$ bun run build
✓ built in 796ms

$ bun run tauri build --debug --no-bundle
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.23s
Built application at: ~/Projetos/openridr/src-tauri/target/debug/openridr
```

Logs: `logs/12-build.log` *(evidência removida do repositório público)* e `logs/12-tauri-build.log` *(evidência removida do repositório público)*.

## Cobertura do ticket e dos gates

| Item do ticket / AC / gate | Resultado | Evidência |
|---|---|---|
| Run exige startup/configuração/TFM e build válido do contexto; falho/inválido/biblioteca bloqueiam com motivo e contexto | Gate completo no host; `library`/`test`/`unknown`, `build_failed`, `build_stale`, `target_missing`/`target_not_executable`, ambiguidade e config/TFM não avaliados têm `details.reason` + `action`/`buildLog` | `12-run-library.json`, `12-run-failed-build.json`, `12-run-missing-target.json`, `12-run-source-changed.json`, `cargo-test` |
| Lançar TargetPath da avaliação atual, nunca presumido; comando/cwd/contexto/revisão/logs; sem restore implícito | Comando e cwd reais no snapshot; `run.start` não chama restore/build; TargetPath do build comparado ao avaliado antes de lançar | `12-run-ok.log`, `12-run-ok.json`, E2E 1, `cargo-test` |
| Aviso de buffer/salvo alterado e decisão explícita; nunca sobrescrever edição | Aviso e diálogo com “Run saved build”/“Save all and rebuild”; fonte salva vira `build_stale`/`source_changed`; UI bloqueia e Cmd+R explica | E2E 1, `12-run-dirty.png`, `12-run-saved-stale.png`, `12-run-source-changed.json` |
| Console real, saída/exit observados, log preservado; exit ausente = unknown | stdout real, `exit 0`/`143`; `exited` neutro; `exit:null` só existe como unknown (teste unitário); log após término/cancelamento | `12-run-ok.log`, `12-run-cancel.log`, E2E 1/2, `cargo-test` |
| Cancelar árvore real com filhos; solicitação ≠ observado; revogação bloqueia novas execuções | TERM→KILL no grupo; pai+filho verificados com `ps`; `requested`/`observed`/`exit` separados; revogação sela e mantém registro | `12-run-cancel.log`, `12-run-revoke*.json`, teste boundary |
| G1 — trust/fronteira | `run.start` nega antes de trust; `run.cancel` é segurança sem trust; gate direto testado | `cargo-test`, E2E |
| G2 — contexto/integridade | workspace/contexto/buildId/config/TFM/evaluation revision comparados; argv separado/cwd/env explícitos; selo descarta linha/exit tardios | `cargo-test`, `12-run-revoke*` |
| G3 — consentimento/instaladores | Nenhum download/instalação/elevação nesta fatia | ambiente |
| G4 — UI/privacidade | UI inglesa, controles nativos rotulados, motivo textual, Ctrl+R/Shift+F10, sem telemetria/upload | E2E 1/2, capturas |
| G5 — validação/proveniência | OS/arch/versões, SDK host/projeto, comandos, exit, buildId/revisão, logs e limitações por ambiente | esta página + logs |

ACs tocados: **AC-03/10** (TargetPath e política sem restore), **AC-15** (snapshot do build anterior), **AC-17/18** (startup/run e console), **AC-19** (identidade build/fonte), **AC-20** (run/árvore/exit/cancelamento), **AC-01/07** (trust e revogação), **AC-23/24 parcial** (Run independente de debugger/flag), **AC-32/33** (superfície acessível e evidência).

## Limitações e pendências honestas

- **Windows x64 e macOS Intel não foram executados.** `dotnet.exe`, `TerminateProcess` no lugar de TERM/KILL, paths com `\`, CRLF e o atalho Shift+F10 não têm evidência aqui; o ticket 18 deve repetir run ok/falha/cancelamento nesses ambientes. O código escolhe Shift+F10 pelo `app.platform` e o grupo de processos no Windows continua sendo a limitação já registrada nos tickets 02/04/11.
- **Alteração de fonte é por hash do conjunto capturado no build**: arquivos `.cs` novos/removidos só entram numa nova avaliação; o hash cobre startup + referências transitivas, não o repositório inteiro. Editar um arquivo que o build não compilou não o torna stale.
- **TargetPath gerenciado** roda via `dotnet <TargetPath>`; o apphost nativo ao lado do `.dll` não é usado nem presumido. Executável nativo sem bit de execução é recusado (Unix; Windows não validado).
- **Sem stdin/PTY**: o processo recebe stdin nulo; não há console interativo nem debugger nesta fatia (tickets 14/15). stdout/stderr são lidos por linha e o log é limitado a 400 linhas/60k com aviso de truncamento.
- **`savedAt` é da UI**: o bloqueio imediato por “salvo depois do build” usa o horário local do save; o gate real do host é o hash do arquivo, e o motivo exibido pelo host é `build_stale`/`source_changed`.
- **Um run por vez**; trocar configuração/TFM exige reavaliação e rebuild, como no build. Não há execução de múltiplos consoles simultâneos.
- **Playwright usa Chrome real contra Vite + host-http**, não a janela Tauri nativa; o mesmo `dispatch` é chamado. A captura do desktop continua limitada pelo TCC desta máquina.
- **Acessibilidade**: controles nativos, `aria-label`/título e motivo textual; não houve teste com leitor de tela nem auditoria WCAG.
- **`fixtures/`**: todas as execuções reais usaram cópias temporárias; nada foi restaurado/alterado na fixture versionada.
