# Evidência — Ticket 14: DAP real com breakpoints, frames e locals

Implementação local em 28/10/2026, revisão sobre `a925693` (ticket 13). Fonte normativa: [ticket 14](../tickets/14-debug-dap.md), [SPEC](../SPEC.md) P0.5/AC-10/18/21/22/23/24 e gates G1–G5 do [índice](../tickets/README.md). Nesta máquina o **netcoredbg 3.2.0-1092 real** abriu sessão DAP, parou em breakpoint, expôs frames/locals, fez step over/step into, continuou até o exit e foi parado com término observado. O smoke async + multi-thread também foi executado. Nada de Windows x64 foi executado; a flag ARM64 continua existindo, **OFF por padrão**.

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | rustc 1.99.0, cargo 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| Debugger | netcoredbg `3.2.0-1 (9744e1f, Release)` em `~/Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092/netcoredbg/netcoredbg` |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` |
| SDKs do sistema | 8.0.416 e 10.0.100 em `/usr/local/share/dotnet` |
| Base | `a925693` + árvore desta revisão |
| Dependências novas | nenhuma (o cliente DAP usa `serde_json`, stdio e o `Registry` existentes) |

Log: [`logs/14-environment.log`](logs/14-environment.log).

## Sessão real na fixture (launch, breakpoint, step, frames, locals, continue)

Fluxo pelo host real (`openridr-host-http`, app home padrão, porta 4318) com `workspace.open` na fixture, `trust.grant`, `debug.flag {enabled:true, confirm:true}`, avaliação, restore, build Debug e `debug.start`. Comando DAP exato: `netcoredbg --interpreter=vscode`; assembly avaliado: `src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll`; breakpoint em `src/OrderDesk.Cli/Program.cs:3`.

```text
restore ok 0
build ok 0 build:1:1
debug.start preparing build:1:1 …/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll
PAUSED {"stopped":{"allThreadsStopped":true,"reason":"breakpoint","threadId":12567785},
        "threads":[{"id":12567785,"name":"Main Thread"}],
        "frames":[{"id":0,"line":3,"name":"Program.<Main>$()","path":"…/Program.cs"}],
        "locals":{"scopes":[{"name":"Locals","variables":[
          {"name":"args","type":"string[]","value":"{string[0]}"},
          {"name":"order","type":"OrderDesk.Core.Order","value":"null"}]}]},
        "breakpoints":[{"line":3,"verified":true}]}
STEP OVER {"stopped":{"reason":"step","threadId":12567785},
           "frames":[{"id":1,"line":4,"name":"Program.<Main>$()"}],
           "locals":{"scopes":[{"name":"Locals","variables":[
             {"name":"order","type":"OrderDesk.Core.Order","value":"{OrderDesk.Core.Order}","variablesReference":2}]}]}}
VARIABLES frame 1 → order variablesReference 4
ENDED {"state":"terminated","exit":0,"exitKnown":true,"adapterExit":0,
       "debuggeePid":88825,"output":"Order A-104: R$ 42,00",
       "processes":[{"kind":"debug:OrderDesk.Cli","pid":88824,"state":"exited","exit":0}]}
```

O breakpoint começou `pending` no `setBreakpoints` e virou `verified` com o evento `breakpoint` do adaptador; sem símbolos ele permaneceria pendente. `order` era `null` na linha 3 e virou `{OrderDesk.Core.Order}` depois do step over. O exit 0 é o evento `exited` do adaptador; `adapterExit: 0` foi observado no `Registry` depois do término. O debuggee `88825` não existia mais (`ps` exit 1).

Logs: [`14-real-dap-fixture.log`](logs/14-real-dap-fixture.log), [`14-real-dap-fixture-transcript.log`](logs/14-real-dap-fixture-transcript.log).

## Smoke async + multi-thread (prova para o gate da flag ARM64)

Projeto console temporário net8.0 fora do repo (`…/opencode/async-multithread`, sem pacotes externos), com `async Task`/`Task.Delay`/`Task.WhenAll` e um worker `Task.Run` + `Thread.Sleep`; restore/build/launch pelo host, breakpoints em `Program.cs:21` (método async) e `Program.cs:46` (worker).

```text
build ok exit 0 id build:3:2
debug.start preparing adapter pid 88923
async breakpoint  line 21  thread 12568355  (Main Thread)
  locals: count=5 sum=0 results=null i=1
step over        line 22  thread 12568586  (.NET TP Worker, continuation)
step into        Program.Bump() line 29, locals value=1, caller frame line 22
worker breakpoint line 46 thread 12568840 (.NET TP Worker)
  locals: threadId=8 sum=0 i=0
thread ids observed: [12568355, 12568586, 12568840]
stop: state=cancelled termination={"requested":"DAP disconnect (terminateDebuggee)",
      "observed":"exited","exit":0,"killedAfterGrace":false}
      exit=0 exitKnown=true adapterExit=0 debuggeePid=88924
after stop: dap=cancelled adapterExit=0; debug:AsyncProbe exited (registry)
```

O step over atravessou o `await` e a continuação rodou em **outra thread gerenciada** (12568586), o step into entrou em `Bump`, e o breakpoint do worker parou na thread 12568840 — três thread ids distintos, com locals reais em cada pausa. O `debug.stop` pediu `disconnect` com `terminateDebuggee`, observou adapter e debuggee saírem (`exit 0`) e manteve logs/output.

Uma segunda sessão foi parada por **revogação de trust** no meio de uma pausa no worker:

```text
revoke: state=cancelled sealed=true termination={"requested":"DAP disconnect (terminateDebuggee)",
        "observed":"exited","exit":0,"killedAfterGrace":false} adapterExit=0
processes: debug:AsyncProbe pid 89212 state=exited
```

Logs/JSON: [`14-real-dap-async.log`](logs/14-real-dap-async.log), [`14-real-dap-async-transcript.log`](logs/14-real-dap-async-transcript.log), [`14-real-dap-async.json`](logs/14-real-dap-async.json), [`14-real-dap-async-summary.json`](logs/14-real-dap-async-summary.json). PIDs 88825, 88924 e 89213 conferidos como inexistentes e nenhum `netcoredbg`/`AsyncProbe`/`OrderDesk.Cli` restante: [`14-process-tree.log`](logs/14-process-tree.log).

## UI real (Playwright + Chrome, mesmo build)

O E2E isola `OPENRIDR_HOME`, copia o adaptador já verificado do app home (sem download) e instala pelo fluxo de reuso do ticket 13. O teste novo exercita a UI real:

- gutter separado: `.cm-lineNumbers` e `.cm-debug-gutter`, marcador `●` + linha atual `▶`;
- breakpoint por Cmd+F8 e por clique no marcador (remove) + atalho (readiciona);
- launch por **Ctrl+D** no Mac, com estado `preparing → paused`, sessão/build/assembly/adaptador visíveis;
- frames `Program.<Main>$()`, locals (`order`), thread `Main Thread`, log DAP e output;
- **F8** step over para a linha 4, locals atualizados;
- continue até `terminated`, `exit 0`, output `Order A-104: R$ 42,00`;
- segunda sessão e **Stop** com `Termination requested`/`Observed: exited`;
- processos `debug:OrderDesk.Cli` observados como `exited` (poll de até 5 s).

```text
$ bun run test:e2e
  39 passed (3.2m)
```

Capturas do mesmo build: [`screenshots/14-dap-paused.png`](screenshots/14-dap-paused.png), [`screenshots/14-dap-stepped.png`](screenshots/14-dap-stepped.png), [`screenshots/14-dap-stopped.png`](screenshots/14-dap-stopped.png). Estado final da sessão E2E: [`logs/14-e2e-debug-session.json`](logs/14-e2e-debug-session.json). Log Playwright: [`logs/14-playwright.log`](logs/14-playwright.log).

## Testes de fronteira (Rust + adaptador DAP controlado)

```text
$ cargo test -p openridr-host
running 90 tests   (unit; inclui 6 do cliente DAP)
test result: ok. 90 passed
running 9 tests    (boundary do instalador do ticket 13)
test result: ok. 9 passed
running 50 tests   (boundary dos tickets 01–14)
test result: ok. 50 passed; 0 failed
```

Log: [`logs/14-cargo-test.log`](logs/14-cargo-test.log). Casos DAP:

- framing `Content-Length` em bytes com adaptador controlado em Python sobre stdio (initialize/launch/setBreakpoints/configurationDone/threads/stackTrace/scopes/variables/disconnect);
- ids/`seq` monotônicos, resposta só chega ao request dono, timeout remove o pendente e resposta tardia não ressuscita;
- reverse request `runInTerminal` com argv exato (espaços e `;` sem shell), cwd/env e pid observado no `Registry`;
- `stopped → frames/scopes/variables` reais e breakpoint não verificado permanece pendente;
- geração de stop: `continued` invalida frames/locals de uma resposta tardia;
- `exited` sem código vira `exit unknown` (`exitKnown:false`), nunca 0;
- disconnect com exit ausente mantém `None` e relata o observado;
- gates do host: sem restore → `restore_required`; flag OFF → `experimental_required`; sem build → `no_build`; fonte salva depois do build → `build_stale/source_changed`; build falho → `build_failed`; adaptador com versão errada → `tool_missing`; contexto re-detectado → `evaluation_stale`; nenhum processo do debugger é criado nesses casos;
- `ready_adapter` recusa macOS Intel (`unsupported_platform`) e flag OFF (`experimental_required`).

## Cobertura do ticket e dos gates

| Item | Resultado | Evidência |
|---|---|---|
| Build Debug/TargetPath atual, assembly avaliado, gates de trust/adaptador/plataforma/flag/contexto | `debug.start` recusa sem restore, sem build, build falho/stale, fonte alterada, adaptador de versão errada e flag OFF; usa `assemblyPath` avaliado (`TargetPath` ou `TargetDir`+`TargetFileName`), nunca `dotnet run` | `cargo test` boundary; `14-real-dap-*` |
| Sessão DAP real `--interpreter=vscode`, estados, breakpoint pendente/verificado, gutter | Sessão real pausada/encerrada; breakpoint pendente → verificado pelo adaptador; gutter separado e atalhos/clique | `14-real-dap-fixture.log`, `14-dap-*.png` |
| Pausa: continue, step over, step into, frames e locals do frame; resposta tardia não ressuscita pausa | Frames/locals reais em três pausas; F8/F7 e `debug.variables`; geração de stop descarta resposta antiga; ao continuar/encerrar locals/frames deixam de ser atuais | `14-real-dap-async.log`, `14-real-dap-async.json`, `cargo test` |
| Logs DAP/adaptador/host e output preservados; exit ausente = `exit unknown`; árvore verificada/visível; revogação | Logs sobrevivem a stop/falha; output `Order A-104…`; exit 0 observado; adapter/debuggee sem pid; revogação sela e termina | `14-real-dap-*`, `14-process-tree.log` |
| Windows x64 exige smoke próprio; Mac Intel sem asset; ARM64 experimental OFF por padrão | Windows x64 **não executado** (gap explícito); Intel segue `unsupported_platform` sem asset substituto; flag continua OFF por padrão | `cargo test`, esta página |
| Prova para retirar a flag ARM64 (launch/breakpoint/stepping/stop + async/multi-thread, versões/comandos/processos) | Provada em macOS arm64 com netcoredbg 3.2.0-1092, 3 thread ids, step over/into, stop e revogação; **a flag foi mantida** nesta fatia, como decidido | `14-real-dap-async*` |
| G1–G5, Ctrl+D/Cmd+F8/Shift+F9/Ctrl+F8/F8/F7, foco/accessible labels | Mac: Ctrl+D, Cmd+F8, F8/F7, clique no gutter, botões nativos com nome/motivo; Windows keys não executadas | E2E novo, screenshots |

## Limitações e pendências honestas

- **Windows x64 não executado.** O host e a UI têm o caminho (`Shift+F9`, `Ctrl+F8`, asset win64), mas launch/breakpoint/stepping/stop/processos em Windows exigem smoke próprio; nada aqui aprova Windows.
- **Mac Intel** continua sem asset `osx-x64` na release 3.2.0-1092; nenhuma versão/asset é substituído. **Linux** segue fora do P0.
- **#256** (risco upstream do netcoredbg) é risco não reproduzido; não foi observada falha universal nem compatibilidade declarada pelo upstream (asset osx-arm64 é *community supported*).
- **Flag ARM64 mantida** com opt-in explícito e OFF por padrão. A prova deste ticket serve ao gate de retirada futura; a retirada não faz parte deste ticket.
- **Variáveis aninhadas** não são expandidas: a UI mostra o valor e `variablesReference` do topo, sem árvore recursiva. Watches, attach e debug de testes estão fora do P0.
- **`runInTerminal`** não foi emitido pelo netcoredbg nos fluxos de launch usados; o host o implementa e testa com requisição controlada (argv exato, cwd/env, pid), mas não há prova de árvore externa com o adaptador real.
- **E2E** usa Chrome real contra Vite + host-http e um adaptador copiado do app home para a home isolada; não baixa o asset. A janela Tauri nativa não foi capturada (limitação de TCC já registrada); o build compila (`cargo check` em `src-tauri`).
- **Logs** são limitados (400 linhas/60k por buffer) e declaram truncamento; o log DAP registra tráfego resumido (método/id/evento), não o JSON completo de cada mensagem. O output do debuggee é preservado separadamente.
