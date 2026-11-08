# Evidência — o build explícito nunca corre com a automação do app (#275)

Issue: [#275](https://github.com/Heldinhow/openridr/issues/275), filha da
[#251](https://github.com/Heldinhow/openridr/issues/251) (origem medida:
[e2e-smoke-solo.md](e2e-smoke-solo.md)). Spec: [#275](https://github.com/Heldinhow/openridr/issues/275).
Decisão: [decisions.md](../decisions.md) (2026-11-07, issue #275). Tickets: #284 (host), #285 (UI),
#286 (análise).

## O que ficou provado

| Prova | Onde | Resultado |
|---|---|---|
| O build recusa enquanto a descoberta constrói | `crates/host/tests/test_discovery.rs` › `build_is_refused_while_a_discovery_is_building_the_test_project` | `build.start` e `build.solution` → `invalid_state` com o motivo e nenhum processo `build:`; depois da descoberta concluir, o mesmo `build.start` roda de verdade (`ok`, exit 0) |
| A descoberta recusa enquanto um build roda | `crates/host/tests/test_discovery.rs` › `discovery_is_refused_while_a_build_is_running` | `invalid_state` com o motivo, `revision` 0 e nenhum processo; depois do `build.cancel`, a mesma descoberta começa |
| A tabela de motivos e a ordem | `crates/host/src/build.rs` › `the_build_refusal_names_the_first_running_operation_and_nothing_when_idle` e `only_a_loading_analysis_blocks_the_build` | build > restore > descoberta > análise carregando; `None` com nada rodando |
| O build recusa enquanto a análise carrega a solução | `crates/host/tests/host.rs` › `build_is_refused_while_the_analysis_is_loading_the_solution_isolated` | `invalid_state` com o motivo durante `loading`, nenhum processo; com `ready`, o build roda (`ok`, exit 0) |
| A UI nomeia a descoberta antes do clique | `tests/e2e/build.spec.ts` › `the build gate names the automatic discovery before the click` | `toolbar-run`/`toolbar-debug` com o motivo no `title`, `build-disabled-reason` com o texto, `build-action`/`build-solution` desabilitados; depois de assentar, o Build conclui |
| O gate da UI e a ordem das recusas | `tests/unit/executionGates.test.ts` | motivo da descoberta e da análise, `null` com nada rodando, ordem (build e descoberta ganham da análise) e projeto sem gate |

## Falha-antes medida

- **#284 (host).** Com o teste final e o fix em stash:
  `build.start started while the discovery was building: "running"` e
  `the discovery started while a build was running: "discovering"` — os dois começavam.
- **#285 (UI).** Com `executionGates.ts` antigo: `Expected: "Test discovery is building the test
  project; wait for it to finish or cancel the discovery." Received: null` em
  `buildDisabledReason`/`runAvailabilityReason`; o caso "nada rodando" já passava.
- **#286 (host).** Com o código anterior: `build.start started while the analysis was loading:
  "running"` — o build começava. Na UI, o motivo era `null` em Build/Run/Debug e
  `"A program is already running."` ganhava da análise.

## A janela medida

- **Análise (solution).** `lsp.start` → `loading`: **0,8 s**; `loading` → `ready`: **3,5 s** (leaf do
  #286, OrderDesk, macOS arm64). No host manual, com o mesmo fixture: `ready` em ~6 s do
  `lsp.start`. É a janela em que o Build fica desabilitado depois de cada restore.
- **Descoberta.** O `settleDiscovery` da #251 já esperava a descoberta; o cenário novo do
  `build.spec.ts` observa o motivo dentro da janela (o spec inteiro leva ~11 s).

## O achado que restringiu o gate a solução (#299)

Num workspace de **projeto** (`.csproj` aberto direto), o csharp-ls **nunca sai de `loading`**:
medido no CrashApp, `loading` por mais de 3 minutos, processo a 0% de CPU, log parado em
`loading project "<caminho>"`; a mesma máquina carrega `OrderDesk.sln` em ~6 s. Com o gate valendo
para qualquer workspace, `run.spec.ts` (LongRunner, CrashApp) ficou com Build desabilitado até o
timeout de 3 min: **4 testes vermelhos**, medidos duas vezes.

Por isso o gate da análise é **solution-only** nos dois lados (`start_build_locked` no host,
`analysisLoading` em `executionGates.ts`), e o leaf do #286 cobre a exceção: com o projeto em
`loading`, `build.start` **precisa** funcionar. O travamento em si é o
[#299](https://github.com/Heldinhow/openridr/issues/299); quando ele for corrigido, o gate deve
voltar a valer para projeto.

## E2E (Playwright em modo web, `PLAYWRIGHT_CHANNEL=chromium`, par 1620/4517)

| Spec | Resultado |
|---|---|
| `tests/e2e/build.spec.ts` | 3 passed (48,8 s) — inclui o cenário novo |
| `tests/e2e/gates.spec.ts` | 10 passed (53,8 s) |
| `tests/e2e/run.spec.ts` | 5 passed (43,9 s) |

Antes do ajuste solution-only, `run.spec.ts` dava 2–4 vermelhos (LongRunner e CrashApp, 3 min cada,
`build-action` desabilitado com o motivo da análise). Depois: 5/5.

## Risco residual

- **Recarga da análise depois de carregada.** O csharp-ls recarrega o projeto sozinho quando um
  arquivo muda e o host não observa isso (o estado não volta a `loading`). Fora do escopo, como a
  spec registra.
- **Workspace de projeto** não tem gate de análise hoje, por causa do #299.
- **Build durante test run** (`tests.run`) é outra colisão, com outro dono; fora do escopo da spec.

## Não verificado

- **Run/Debug durante a descoberta** no host: sem teste dedicado. O build implícito dos dois passa
  pelo mesmo `start_build_locked` (caminho de código lido, não exercitado).
- **App Tauri real**: mudança de interface pura; a superfície pedida é o Playwright em modo web.
- **Smoke noturno completo**: só os três specs da área rodaram.
- Windows e macOS Intel.
