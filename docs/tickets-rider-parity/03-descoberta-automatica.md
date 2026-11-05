# 03 — Descoberta automática de testes (G08)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Testes". **Bloqueado por:** nenhum.
**Status:** ready-for-agent.

## Objetivo

Abrir uma solução com projeto de teste e ver a árvore preenchida sem clicar em "Discover
tests". Falha de descoberta mostra a causa no painel.

## Pontos de código

- `src/WorkspaceView.tsx`: `testCandidates`, `testsDisabledReason`, `testProjectPath`,
  `startTestDiscovery`, efeito de auto-start da análise (padrão a seguir) e o snapshot.
- `src/TestsPanel.tsx`: estados `No discovery has run`, `discovering`, `completed`, `empty`,
  `failed` e a mensagem de erro.
- `crates/host/src/test_discovery.rs`: descoberta por projeto; não muda o contrato.
- `crates/host/src/lib.rs`: `tests.discover` e a fronteira de restore/avaliação.

## Comportamento

- A descoberta roda uma vez por fronteira `workspace.id:contextId:restore.revision` (mais o
  caminho do projeto de teste selecionado), quando trust, SDK, restore e avaliação estão
  prontos. Não roda a cada render, edição ou abertura de painel.
- Com vários projetos de teste, o selecionado é descoberto; trocar a seleção dispara a
  descoberta daquele projeto.
- Falha (SDK, restore, avaliação, adaptador ausente, MTP) aparece no painel com a mesma
  mensagem do host; não fica "No discovery has run" nem árvore vazia sem explicação.
- A descoberta automática é cancelável pelo botão existente e não dispara se já houver uma
  em andamento.

## Teste e verificação

- Playwright com `fixtures/OrderDesk`: abrir a solução, esperar a avaliação/restore e afirmar
  que `tests-items` tem itens sem clicar em `tests-discover`; provocar falha (cópia sem
  adaptador ou projeto inválido) e afirmar a mensagem; editar um arquivo e garantir que não
  houve nova descoberta.
- App real (agent-desktop): abrir OrderDesk, ir a Tests e ver a árvore; trocar o projeto de
  teste, se houver mais de um; conferir uma falha proposital.

## Aceite

- [x] Solução confiável, com SDK resolvido e restaurada, lista os testes sem clique.
- [x] A descoberta roda uma vez por fronteira e não entra em laço com edição/render.
- [x] Falha de descoberta mostra a causa no painel.
- [x] Trocar o projeto de teste dispara a descoberta do novo projeto.
- [ ] Evidência no app real.

## Feito (2026-11-04)

- `src/WorkspaceView.tsx`: a descoberta automática usa a fronteira
  `workspace.id:contextId:restore.revision:caminho do projeto` e um ref para rodar uma vez;
  troca de projeto, restore concluído ou novo contexto disparam uma nova tentativa. Restore em
  andamento bloqueia; descoberta manual marca a mesma fronteira. Falha ao iniciar vai para
  `tests-auto-error` no painel com a mensagem do host.
- `src/TestsPanel.tsx`: renderiza `tests-auto-error`; falhas de execução (`failed`,
  `adapter_missing`, `mtp_unsupported`, etc.) continuam com o texto do host.
- `tests/e2e/tests-auto-discovery.spec.ts`: árvore preenchida sem clique, edição e reabertura
  sem nova descoberta, adapter ausente automático e troca entre dois projetos de teste.
- `tests/e2e/automatic.ts`: `clickDiscover` espera a revisão nova do host terminar, porque uma
  descoberta automática já pode estar atual.
- `bun run check`; Playwright: `tests-auto-discovery` 3/3, `tests` 7/7, `testsuite` + `ui-fold`
  6/6, `qa-devflow-top4 -g "achado 2"` 1/1, `zy-gutter` 1/1.
- **Pendente no app real (agent-desktop):** abrir OrderDesk, ver a árvore em Tests sem clique,
  trocar o projeto, conferir uma falha proposital; a varredura final com `verify-openridr`
  fecha este aceite. `zz-acceptance` P0 não foi concluído: timeout ao clicar em
  `tree-file-src/OrderDesk.Cli/Program.cs` depois de o teste clicar no projeto (o clique
  recolhe o projeto; caminho não tocado por este ticket).
