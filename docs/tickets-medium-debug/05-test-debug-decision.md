# 05 — Debug de teste: decisão e mensagem (d10)

**Fonte:** `docs/SPEC-MEDIUM-DEBUG.md` §5. **Bloqueado por:** nenhum. **Status:** ready-for-agent.

## Objetivo

Não implementar debug de teste nesta rodada, mas registrar por que e deixar a mensagem
honesta em todos os pontos onde o usuário encontra o botão desabilitado.

## Evidência (explore notes)

- Testes rodam num único `dotnet test` (VSTest cria o `testhost`; o host não vê o pid).
- `debug.start` só lança o assembly console avaliado (projeto de teste é biblioteca).
- O cliente DAP só envia `launch`; não há `attach`, `processId` nem filtro de teste.
- Alternativa "lançar vstest.console sob o netcoredbg" debuga o runner, não o teste.

## Mudanças

- `docs/decisions.md`: entrada nova (data 2026-11-02) com a decisão e as duas alternativas
  descartadas; citar que substitui/estende a decisão de 2026-10-31 sobre "debug de testes fora
  do P0" (não reabrir sem attach/testhost).
- `docs/acceptance-matrix.md`: nota na linha 24c apontando a decisão.
- `src/TestsPanel.tsx`: `testDebugUnavailable = "Debugging tests needs a test-host attach the debugger does not support yet. Use Run selected."`
- `src/TestsQuickList.tsx`: usar a mesma constante (sem texto paralelo).
- `src/WorkspaceView.tsx`: as ações `tests.debugSelected` já usam a constante; mensagem de
  `execute` idem.
- `tests/e2e/testsuite.spec.ts`: atualizar as duas asserções de texto (continua desabilitado).

## Aceite

- [ ] Mesma mensagem no painel, quick list, menu e ação.
- [ ] Decisão e matriz atualizadas.
- [ ] Suíte de testes de UI verde com o novo texto.
- [ ] `bun run check` verde. Commit `docs(tests): record why test debugging is unavailable`.
