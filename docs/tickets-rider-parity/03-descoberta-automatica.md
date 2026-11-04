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

- [ ] Solução confiável, com SDK resolvido e restaurada, lista os testes sem clique.
- [ ] A descoberta roda uma vez por fronteira e não entra em laço com edição/render.
- [ ] Falha de descoberta mostra a causa no painel.
- [ ] Trocar o projeto de teste dispara a descoberta do novo projeto.
- [ ] Evidência no app real.
