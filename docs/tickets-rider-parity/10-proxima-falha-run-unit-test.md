# 10 — Próxima falha e Run Unit Test por tipo (G12, G17)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Testes". **Bloqueado por:** nenhum.
**Status:** ready-for-agent.

## Objetivo

Navegar entre falhas de teste sem caçar na lista e não mostrar "Run Unit Test" num projeto que
não tem testes; num projeto de teste, o item roda todos os testes dele.

## Pontos de código

- `src/TestsPanel.tsx`: cabeçalho dos resultados, `run.results`, `outcome`, `<details>` e
  `onNavigate`.
- `src/WorkspaceView.tsx`: `testCandidates`, `testsDisabledReason`, `startTestRun`,
  `treeMenuItems`/`projectMenu`.
- `src/solutionMenu.ts`: `ProjectMenu` (novo flag de projeto de teste e callback de teste).
- `src/SolutionTree.tsx`: `view.kind` já distingue `test`.

## Comportamento

- **G12:** botão "Next failure" no cabeçalho dos resultados. Cicla apenas resultados com
  outcome de falha, expande o detalhe, rola até ele e navega para `result.position` quando
  existe. Sem resultados falhos, fica desabilitado com motivo. Sem atalho novo.
- **G17:** em projeto que não é de teste, "Run Unit Test" não aparece. Em projeto de teste,
  aparece habilitado e chama `tests.run` com `selection: "all"` para aquele projeto (muda para
  a aba Tests). Se os gates de restore/SDK/avaliação falharem, fica desabilitado com o mesmo
  motivo do painel; se o host recusar, a mensagem aparece.
- Nada de rodar testes sem projeto de teste explícito.

## Teste e verificação

- Playwright: com uma execução que tem 2+ falhas, clicar em "Next failure" e conferir o
  resultado expandido/posição; sem falhas, conferir o motivo. No menu de `OrderDesk.Core`
  (biblioteca), afirmar que "Run Unit Test" não existe; no de `OrderDesk.Tests`, afirmar que
  existe e dispara `tests.run` com todos os testes.
- App real (agent-desktop): repetir os dois fluxos e conferir a aba Tests após o Run Unit Test.

## Aceite

- [x] "Next failure" cicla as falhas, expande e navega para a origem quando existe.
- [x] Sem falhas, o botão está desabilitado com motivo.
- [x] "Run Unit Test" não aparece em projeto que não é de teste.
- [x] Em projeto de teste, "Run Unit Test" roda todos os testes do projeto.
- [x] Gates e erros do host aparecem com motivo.
- [ ] Evidência no app real.
