# Evidência — Debug: explicar a sessão que termina sem parar

Issue [#46](https://github.com/Heldinhow/openridr/issues/46). Fatias #76 (host) e #78 (painel).

## O que foi executado

### 1. Testes de host (o contrato do snapshot)

```sh
cargo test -p openridr-host --lib dap::tests
# 19 passed; 0 failed
```

11 testes novos em `crates/host/src/dap.rs`, dirigidos por mensagens DAP reais (sem netcoredbg, sem
adapter script): sessão que nunca parou sem breakpoint (frase com exit), exit nunca reportado mas
reportado o exit do debuggee (frase sem parêntese), parou-e-continuou (sem frase), breakpoint setado e
não atingido (sem frase), mute-all (frase de mute), filtro de exceções lançadas (sem frase),
run-to-cursor pendente (sem frase), parada pedida pelo usuário via `shutdown` (`cancelled`, sem
frase), `failed` pelo mesmo `fail` que a negociação com o adaptador usa (sem frase), adaptador que
morre sem reportar o exit do debuggee (`observe_adapter_exit`: sem frase) e último breakpoint removido
(lista vazia daquela fonte: frase de nenhum breakpoint). Os dois últimos vieram da revisão de código.

**Falha-antes provada:** trocando só a atribuição `self.no_stop_explanation =
self.no_stop_explanation_for(state)` por `= None` (e restaurando em seguida, sem `git stash`), os três
testes do caminho positivo falham:

```
test dap::tests::an_exit_that_was_never_reported_is_not_invented_in_the_explanation ... FAILED
test dap::tests::a_session_that_never_stopped_without_breakpoints_explains_the_completion ... FAILED
test dap::tests::muted_breakpoints_explain_the_end_as_muted ... FAILED
test result: FAILED. 14 passed; 3 failed
```

### 2. App real (Tauri, macOS arm64), com `agent-desktop`

Fixture `fixtures/OrderDesk` copiada para `/tmp/orderdesk-46`; app em `bun run tauri dev` com os
overrides de bundle de dev (`OPENRIDR_BUNDLED_NETCOREDBG`, `OPENRIDR_BUNDLED_CSHARP_LS`,
`OPENRIDR_BUNDLED_DOTNET_ROOT`), workspace aberto por encaminhamento de argv, `Trust project` na
primeira abertura. Uma instância só; app fechado ao final.

| Rodada | Ação | Observado |
|---|---|---|
| 1 | Debug com nenhum breakpoint (rodada de novo com o código final, depois dos ajustes da revisão: breakpoint removido com Cmd+F8, lista vazia daquela fonte) | `Terminated`, “No breakpoints were set; the program ran to completion (exit 0). Its output is in the Console tab.” em cor de informação, acima de Threads/Call frames/Locals; lista de breakpoints vazia; nenhuma nota atenuada de fim; Console com o output real do programa (“Order A-104: R$ 42,00”). |
| 2 | Breakpoint na linha 1 + **Mute all**, Debug | `Terminated`, “Breakpoints are muted; the program ran to completion (exit 0). Its output is in the Console tab.”; o breakpoint continua listado. |
| 3 | **Unmute all**, Debug | `Paused · breakpoint · 1 frame(s) · 1 thread(s)`, threads/frames/locals reais e **nenhuma** explicação de “sem parada”. |

As capturas ficaram fora do repositório (caminhos pessoais em `/tmp`), como as demais evidências
brutas; o resultado acima é o registro.

## O que não foi verificado

- **Playwright (`tests/e2e/debugger.spec.ts`)**: o spec inteiro falha neste HEAD antes de qualquer
  asserção do ticket, por um motivo pré-existente que não vem deste diff. `prepareDebugCopy` chama
  `debug.plan` + `debug.install`, e o host de E2E roda com `OPENRIDR_BUNDLED_NETCOREDBG` apontando para
  o bundle falso semeado por `seedVerifiedTools`; a resposta é
  `{"code":"invalid_state","message":"netcoredbg is bundled with OpenRidr; there is nothing to download or install."}`.
  Prova de que é pré-existente: o mesmo teste do `HEAD` (`debugs a real build with breakpoints…`,
  linha 192, não tocada por este trabalho) falha na mesma chamada com a mesma mensagem. Os asserts
  novos (positivo, condição que nunca vale, mute all e o caso “parou-e-continuou”) ficam no spec e
  passam a valer quando o harness souber conviver com o adaptador embutido — isso é a frente de
  estabilização de E2E (#43), não deste ticket.
- Selagem por troca de workspace mantendo a frase visível: regra derivada dos fatos (a frase é gravada
  no fim e a selagem não a limpa), **sem prova de app real**.
- Windows e macOS Intel: não executados, como no resto do repo.
