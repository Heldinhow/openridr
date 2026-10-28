# 05 — F5: Tool window Tests

**What to build:** a tool window Tests com árvore agrupada, filtro, run/debug/rerun failed/stop, status por teste e quick list ⌥⇧U, reusando descoberta e execução VSTest existentes.

**Blocked by:** 01

**Status:** done

- [x] A árvore agrupa por projeto > prefixo do FQN > classe > método, declara "Grouping derived from test names" e mostra o FQN completo em tooltip; nenhuma posição ou identidade é inventada.
- [x] O filtro por texto reduz a árvore sem perder os grupos dos itens que casam nem alterar a seleção.
- [x] Run all, Run selected, Debug selected, Rerun failed, Stop e Refresh chamam os fluxos existentes e refletem o estado real (discovering, completed, empty, failed, cancelled, stale, sealed, partial). Debug selected fica registrado, visível e desabilitado com motivo: debug de teste é P1 em `docs/README.md` e não existe fluxo VSTest de debug para reusar.
- [x] Rerun failed usa os ids `failed`/`notExecuted` do último run completo e fica desabilitado quando o resultado está stale/sealed.
- [x] Quick list ⌥⇧U abre com Run all, Run selected, Debug, Rerun failed e Stop, com atalhos visíveis e itens indisponíveis esmaecidos com motivo. Acordes ⌘U R/D/L/F no perfil Rider; o perfil IntelliJ usa ⇧F10, ⌃⇧F10, ⇧F9 e ⌥⇧R.
- [x] Estados vazio, adapter ausente, MTP não suportado, descoberta falha e resultado parcial têm texto próprio; ausência nunca vira sucesso.
- [x] A tool window preserva o conteúdo ao alternar de slot e mantém os `data-testid` do painel atual.
- [x] `bunx tsc --noEmit` e a suíte Playwright completa passam; commit local `feat(F5): ...`.

## Comments

- Evidência nova: `docs/evidence/screenshots/f5-tests-quick-list.png` (quick list real com acordes e motivos).
- Spec: `tests/e2e/testsuite.spec.ts` (nome ordena depois de `tests.spec.ts` porque os ids `discovery:`/`testrun:` usam contadores globais do host e a spec antiga fixa `:1`).
- Seleção e projeto de teste foram içados para a `WorkspaceView`, então a seleção sobrevive à troca de tool window e o quick list age com a janela fechada.
