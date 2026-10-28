# Evidência — Ticket 15: PTY real, entrada/saída e fechamento com decisão

Implementação local em 28/10/2026, base `7afc56e` (ticket 14). Fonte normativa: [ticket 15](../tickets/15-terminal-pty.md), [SPEC](../SPEC.md) AC-25/26/31 (parcial), AC-01/07/32/33, P0.6 e gates G1–G5 do [índice de tickets](../tickets/README.md). Decisões: [decisions.md](../decisions.md) (Ticket 15) e [implementation-notes.md](../implementation-notes.md) (Terminal PTY).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Shell descoberto | `/bin/zsh` (`$SHELL=/bin/zsh`), com `/bin/sh` exercitado nos testes Rust |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| Chrome / Playwright | Google Chrome 155.0.8059.85 (canal `chrome`) / 1.64.1 |
| SDK do host / do projeto | 10.0.100 em `/usr/local/share/dotnet` / 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` |
| Fixture | `fixtures/OrderDesk`, sempre copiada para `/tmp`; `dotnet test` real dentro da cópia |
| App home do driver | o padrão (`~/Library/Application Support/OpenRidr`), somente leitura de SDK; nada foi instalado |
| Janela | Chrome real contra Vite + host-http (mesmo `dispatch`); a janela Tauri não foi dirigida (TCC) |

Logs: [`logs/15-environment.log`](logs/15-environment.log), [`logs/15-drive.log`](logs/15-drive.log), [`logs/15-cargo-test.log`](logs/15-cargo-test.log), [`logs/15-playwright.log`](logs/15-playwright.log), [`logs/15-build.log`](logs/15-build.log), [`logs/15-tauri-build.log`](logs/15-tauri-build.log), snapshots [`15-terminal-start.json`](logs/15-terminal-start.json), [`15-terminal-ctrlc.json`](logs/15-terminal-ctrlc.json), [`15-terminal-dotnet-test.json`](logs/15-terminal-dotnet-test.json), [`15-terminal-closed.json`](logs/15-terminal-closed.json), [`15-terminal-revoke.json`](logs/15-terminal-revoke.json), [`15-terminal-switch.json`](logs/15-terminal-switch.json).

## O que foi construído

- **Host** (`crates/host/src/terminal.rs` + `lib.rs`): `terminal.start {cols?,rows?}`, `terminal.write {data}`, `terminal.resize {cols,rows}` e `terminal.stop {confirm:true}`, com `terminal` no snapshot (`state idle|running|exited|stopped|failed`, `shell`, `command`, `cwd`, `cols`/`rows`, `log`, `logTruncated`, `outputSeq`, `exit`, `termination`, `processId`, `awaitingDecision`, `kept`, `revision`, `workspaceId`/`contextId`, tempos, `staleReason`, `sealed`). `terminal.state` é lido junto de `workspace.state`.
- **Shell descoberto**: `$SHELL` só é aceito absoluto e executável; senão `/bin/zsh`, `/bin/bash`, `/bin/sh`. Windows (implementado, não executado): `COMSPEC`, PowerShell, `cmd.exe`. Cwd = root do workspace; argv = `[shell]`; `TERM=xterm-256color` apenas quando ausente.
- **Gate**: `terminal.start`/`write`/`resize` passam por `require_trust` antes de qualquer efeito; sem trust → `trust_required`, nenhum shell. `terminal.stop` é ação de segurança sem trust. `terminal.start` concorrente → `invalid_state`; shell ausente → `shell_missing`; falha de spawn → `state:"failed"` com mensagem no log.
- **PTY real**: `portable-pty 0.9` (só no host). O shell vira session leader com controlling terminal; `0x03` chega como SIGINT ao processo em foreground. Entrada é byte cru; saída é drenada em thread própria.
- **Registry**: `register_pty(kind, workspaceId, pid, killer)`; a thread de `wait()` publica o exit observado no slot que o registry lê. Término usa TERM→KILL no grupo, com TERM prévio ao foreground pgroup (`tcgetpgrp`); Windows usa o `ChildKiller` (TerminateProcess), sem TERM gracioso. `exit` é o código observado; morto por sinal = `null` (exit unknown), nunca 0.
- **Buffer**: 4.000 linhas/120k caracteres, `outputSeq` por chunk, truncamento declarado, linha parcial preservada, ANSI/OSC removidos para exibição em texto. O log sobrevive a exit/stop/falha e não recebe linhas depois do selo.
- **Fronteiras**: revogar termina + sela e mantém o registro; abrir/fechar/trocar termina + sela e volta a `terminal.idle` (o processo continua no painel). `sdk.redetect` não derruba o shell.
- **UI** (`src/TerminalPanel.tsx`): aba Terminal com shell/cwd/tamanho/estado/processo, output rolável, input de linha (Enter → linha + `\r`), Ctrl+C por botão e pelo teclado no input, Start/Close, diálogo “Keep terminal”/“Terminate shell and close”, exit/termination depois do close, log preservado e motivo textual em Restricted mode. O tamanho do PTY é medido do painel e enviado por `terminal.resize`.

## Comandos e saídas reais

Driver real pela ponte HTTP (host + PTY + zsh + `dotnet` reais), cópia temporária com espaço no caminho.

### Antes de trust: nenhum shell

```text
$ POST /call {"method":"terminal.start","params":{}}
{"error":{"code":"trust_required","message":"terminal.start requires workspace trust; no process was started."},"ok":false}
state after refusal: "idle" · processes: []
```

### Sessão real: shell, cwd, tamanho e processo observados

```text
$ POST /call {"method":"terminal.start","params":{"cols":200,"rows":40}}  [selected]
"terminal": {"state":"running","shell":"/bin/zsh","command":["/bin/zsh"],
             "cwd":"/private/tmp/openridr-15-…/Order Desk terminal",
             "cols":200,"rows":40,"processId":1,"exit":null,"sealed":false}
"processes": [{"id":1,"kind":"terminal:zsh","pid":74222,"state":"running",
               "exit":null,"termination":null,"workspaceId":1}]
```

### Entrada/saída reais e Ctrl+C em `sleep 30`

```text
$ terminal.write "pwd; echo terminal-$((6*7))\r"
/private/tmp/openridr-15-…/Order Desk terminal
terminal-42

$ terminal.write "sleep 30 && echo done-$((6*7))\r"   (0,7 s)
$ terminal.write "\u0003"                             (Ctrl+C)
^C
$ terminal.write "echo prompt-back\r"
prompt-back
elapsed_after_ctrl_c=0.77s
interrupted command did not finish (done-42 absent)
```

Log: [`15-terminal-ctrlc.json`](logs/15-terminal-ctrlc.json). O mesmo caminho é coberto pelo teste Rust com `/bin/sh`, que também verifica com `/bin/ps` que o filho em foreground morre no close.

### `dotnet test` manual e real através do shell

```text
$ SDK="$HOME/Library/Application Support/OpenRidr/sdks/8.0.408"; "$SDK/dotnet" --version
8.0.408

$ SDK="$HOME/Library/Application Support/OpenRidr/sdks/8.0.408"; "$SDK/dotnet" test OrderDesk.sln -nologo -v:minimal
  OrderDesk.Core -> …/src/OrderDesk.Core/bin/Debug/net8.0/OrderDesk.Core.dll
  OrderDesk.Tests -> …/tests/OrderDesk.Tests/bin/Debug/net8.0/OrderDesk.Tests.dll
  Versão do VSTest 17.11.1 (arm64)
  Aprovado!  – Com falha: 0, Aprovado: 2, Ignorado: 0, Total: 2, Duração: 1 ms - OrderDesk.Tests.dll (net8.0)
```

Log: [`15-terminal-dotnet-test.json`](logs/15-terminal-dotnet-test.json). O restore/build/test foram executados pelo comando digitado no PTY; o app não instala SDK nem pacote e a saída em português é o locale real da máquina (a UI continua em inglês).

### Fechar exige decisão; término e exit observados

```text
$ terminal.stop {}                      → confirmation_required
$ terminal.stop {"confirm":true}
"state":"stopped"
"exit":null
"termination":{"requested":"SIGTERM then SIGKILL","observed":"exited","exit":null,"killedAfterGrace":true}
process 1: {"state":"exited","exit":null,"termination":{…}}
log preservado: "Aprovado! … Total: 2" + prompt do zsh
```

Log: [`15-terminal-closed.json`](logs/15-terminal-closed.json). O zsh interativo ignora SIGTERM; o host observou a necessidade do SIGKILL e reportou `killedAfterGrace:true` e `exit unknown` em vez de inventar um código. `dotnet test` e o filho em foreground do teste Rust foram verificados como inexistentes depois do close (`ps`).

### Revogação sela; troca de workspace protege o novo contexto

```text
trust.revoke com o shell em execução:
"state":"stopped" · "sealed":true
"staleReason":"Trust was revoked while the shell was running; it was stopped and its output is not current."
log: "sealed-marker"
0,5 s depois: finishedAt unchanged = yes
novo terminal.start → trust_required

workspace.open (outra cópia):
"terminal": {"state":"idle","revision":0,"processId":null}
processos antigos: {"kind":"terminal:zsh","state":"exited","termination":{…}}
```

Logs: [`15-terminal-revoke.json`](logs/15-terminal-revoke.json) e [`15-terminal-switch.json`](logs/15-terminal-switch.json).

## UI real (Playwright/Chrome 155, Vite + host-http)

Dois testes novos em `tests/e2e/terminal.spec.ts`, com cópias temporárias e shell/dotnet reais:

1. **“a restricted workspace explains the terminal and starts no shell”** — workspace em Restricted mode: a aba Terminal mostra o motivo textual, Start/input desabilitados, estado `Not started` e shell `not started`; chamada direta `terminal.start` pela ponte devolve `trust_required` e o snapshot continua `terminal.idle`, sem processo `terminal:*` rodando.
2. **“a real shell runs in the workspace, interrupts a child with Ctrl+C and closes after confirmation”** — trust, aba Terminal, Start; shell `/bin/zsh`, cwd igual ao root real, tamanho reportado. `pwd` devolve o workspace; `echo terminal-e2e-$((6*7))` devolve `terminal-e2e-42`; `dotnet --version` pelo SDK fixado devolve `8.0.408`. `sleep 30 && echo done-$((6*7))` seguido de Ctrl+C no input: `^C`, prompt de volta e `done-42` ausente. Close → diálogo; “Keep terminal” mantém a sessão e um novo `echo` funciona; Close → “Terminate shell and close”: estado `Stopped`, `Exit: exit unknown`, `Termination requested: SIGTERM then SIGKILL · Observed: exited`, log anterior preservado, input desabilitado. O snapshot do host confirma que o exit exibido é o mesmo do processo no registry.

Capturas: [`15-terminal-running.png`](screenshots/15-terminal-running.png), [`15-terminal-ctrl-c.png`](screenshots/15-terminal-ctrl-c.png), [`15-terminal-close-dialog.png`](screenshots/15-terminal-close-dialog.png), [`15-terminal-closed.png`](screenshots/15-terminal-closed.png).

## Testes automatizados

```text
$ cargo test -p openridr-host
running 98 tests   (unit: descoberta/fallbacks de shell, buffer/seq/ANSI, resize, exit unknown, selo tardio)
running 9 tests    (debugger boundary, inalterado)
running 53 tests   (boundary: pré-trust sem shell, PTY real pwd/echo, Ctrl+C em sleep 30,
                    resize, close confirmado com filho em foreground verificado por ps,
                    revogação selada, troca de workspace, gate de todos os métodos executáveis)
test result: ok. 98 passed; 0 failed
test result: ok. 9 passed; 0 failed
test result: ok. 53 passed; 0 failed
```

Log: [`15-cargo-test.log`](logs/15-cargo-test.log). A suíte de LSP csharp-ls passou nesta execução; não foi preciso repetir.

```text
$ bun run test:e2e
  ✓ terminal.spec.ts:34 a restricted workspace explains the terminal and starts no shell
  ✓ terminal.spec.ts:73 a real shell runs in the workspace, interrupts a child with Ctrl+C and closes after confirmation
  … 39 testes dos tickets 01–14 continuam verdes
  41 passed (3.5m)
```

Log: [`15-playwright.log`](logs/15-playwright.log). Portas 4317/1420 foram encerradas antes e ficaram livres depois; `ps` não encontrou shell, `sleep 30` nem `dotnet test` remanescentes.

```text
$ bun run build
✓ built in 973ms

$ bun run tauri build --debug --no-bundle
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.00s
Built application at: /Users/helder/Projetos/openridr/src-tauri/target/debug/openridr
```

Logs: [`15-build.log`](logs/15-build.log) e [`15-tauri-build.log`](logs/15-tauri-build.log).

## Cobertura do ticket e dos gates

| Item do ticket / AC / gate | Resultado | Evidência |
|---|---|---|
| Shell realmente disponível, PTY só após trust, shell/cwd visíveis; chamada direta não cria shell | `$SHELL` validado; zsh real; `trust_required` + `terminal.idle` sem processo antes de trust | `15-drive.log`, E2E 1, `cargo-test` |
| Entrada/output/Ctrl+C reais; `dotnet test` manual sem instalar SDK/pacote nem fingir execução | `pwd`/`echo` reais; SIGINT em `sleep 30` em 0,77 s; `dotnet test` real com 2 testes aprovados | `15-terminal-ctrlc.json`, `15-terminal-dotnet-test.json`, E2E 2, `cargo-test` |
| Fechar shell ativo oferece manter ou encerrar; cancelar conserva; prompt ativo = shell rodando | Diálogo Keep/Terminate; Keep mantém sessão e input; `terminal.stop` sem confirm → `confirmation_required` | E2E 2, `15-terminal-close-dialog.png`, `cargo-test` |
| Saída/estado/exit do shell, não do último comando; falha/remanescentes/exit unknown explícitos; log preservado | `exit unknown` após SIGKILL; requested/observed/killedAfterGrace separados; log mantém `dotnet test` e prompt | `15-terminal-closed.json`, E2E 2, `15-terminal-closed.png` |
| Revogar bloqueia novas sessões e decide processos; troca/fechamento encerra ou relata sem alegar isolamento | Revoke sela e mantém registro; novo start → `trust_required`; switch volta idle e processos antigos `exited` | `15-terminal-revoke.json`, `15-terminal-switch.json`, E2E 1, `cargo-test` |
| G1 — trust/fronteira | `terminal.start/write/resize` negados antes de trust; `terminal.stop` é segurança sem trust; sem spawn pré-trust | `cargo-test`, `15-drive.log`, E2E 1 |
| G2 — contexto/integridade | shell/cwd/processo/exit no snapshot; exit ausente = null; selo descarta output/exit tardios; argv separado e cwd explícito | `cargo-test`, snapshots JSON |
| G3 — consentimento/instaladores | Nenhum download/instalação/elevação nesta fatia; `dotnet test` foi comando do operador | ambiente |
| G4 — UI/privacidade | UI inglesa, controles nativos nomeados, foco/teclado, motivo textual, resize/scroll sem esconder ações; sem telemetria/upload | E2E 1/2, capturas |
| G5 — validação/proveniência | OS/arch/versões, shell, SDK, comandos, exit/termination e logs por ambiente; Windows explicitamente não executado | esta página + logs |

ACs tocados: **AC-25/26** (PTY/input/Ctrl+C/fechamento/exit), **AC-31 parcial** (terminal complementar; mensagem MTP fica em 16), **AC-01/07** (trust/revogação/processos), **AC-32/33** (superfície acessível e evidência por ambiente).

## Limitações e pendências honestas

- **Windows x64 não foi executado.** ConPTY via `portable-pty`, descoberta por `COMSPEC`/PowerShell/`cmd.exe` e término por `TerminateProcess` existem no código, mas nenhum comportamento Windows é comprovado aqui; o ticket 18 deve repetir shell/input/Ctrl+C/close nesse ambiente.
- **Sem emulador de terminal.** A view mostra texto com ANSI/OSC removidos; redraws do zsh (prompt, bracketed paste, cursor) aparecem como repetições e preenchimento de linha. Não há cores, cursor ou tela alternativa (TUIs full-screen). `Ctrl+C` só é interceptado enquanto o input do terminal tem foco; no output focável o navegador copia normalmente.
- **Término é do grupo/foreground, não uma varredura da árvore inteira.** Jobs em outros process groups que o shell tenha criado antes do close não são enumerados; o painel de processos só acompanha o shell. Um job em foreground recebe TERM antes do grupo (verificado por `ps` no teste Rust) e o zsh interativo ignora SIGTERM, então o caminho normal observado é `SIGTERM then SIGKILL` com `killedAfterGrace:true` e `exit unknown`.
- **`exit` de shell morto por sinal é `null`.** É o comportamento correto do contrato (`portable-pty` reportaria 1); um shell que sai sozinho (`exit`) reporta o código real.
- **`sdk.redetect` não derruba o shell** (terminal não depende do modelo avaliado); revogar/abrir/fechar/trocar derrubam e selam.
- **Playwright usa Chrome real contra Vite + host-http**, não a janela Tauri; a captura do desktop continua limitada por TCC nesta máquina.
- **Acessibilidade**: botões/input nativos, `aria-label`, `role="status"`/`role="dialog"`, foco visível e motivo textual; não houve teste com leitor de tela nem auditoria WCAG.
- **Fixture e SDKs**: `dotnet test` rodou em cópia temporária; nada em `fixtures/` foi alterado e nenhum SDK/ferramenta foi instalado, atualizado ou removido. A configuração em `~/Library/Application Support/OpenRidr` não foi apagada.
