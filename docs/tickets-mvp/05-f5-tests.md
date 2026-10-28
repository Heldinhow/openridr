# 05 — F5: Tool window Tests

**What to build:** a tool window Tests com árvore agrupada, filtro, run/debug/rerun failed/stop, status por teste e quick list ⌥⇧U, reusando descoberta e execução VSTest existentes.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] A árvore agrupa por projeto > prefixo do FQN > classe > método, declara "Grouping derived from test names" e mostra o FQN completo em tooltip; nenhuma posição ou identidade é inventada.
- [ ] O filtro por texto reduz a árvore sem perder os grupos dos itens que casam nem alterar a seleção.
- [ ] Run all, Run selected, Debug selected, Rerun failed, Stop e Refresh chamam os fluxos existentes e refletem o estado real (discovering, completed, empty, failed, cancelled, stale, sealed, partial).
- [ ] Rerun failed usa os ids `failed`/`notExecuted` do último run completo e fica desabilitado quando o resultado está stale/sealed.
- [ ] Quick list ⌥⇧U abre com Run all, Run selected, Debug, Rerun failed e Stop, com atalhos visíveis e itens indisponíveis esmaecidos com motivo.
- [ ] Estados vazio, adapter ausente, MTP não suportado, descoberta falha e resultado parcial têm texto próprio; ausência nunca vira sucesso.
- [ ] A tool window preserva o conteúdo ao alternar de slot e mantém os `data-testid` do painel atual.
- [ ] `bunx tsc --noEmit` e a suíte Playwright completa passam; commit local `feat(F5): ...`.
