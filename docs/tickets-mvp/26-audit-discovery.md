# 26 — Auditoria visual: dicas de descoberta na busca

**What to build:** Dar feedback de contagem e orientar a busca de tipos na aba Classes sem mudar a separação contratual entre Classes e Symbols.

**Source:** `docs/UI-AUDIT.md` L4; F3 #21/#28 da SPEC e decisão F3.

**Blocked by:** nenhum.

**Status:** completed — `feat(search): clarify symbol result counts and discovery`

- [x] Resultados mostram contagem coerente com itens visíveis/limite e indicam truncamento quando aplicável; não apresentar total inventado.
- [x] Busca por tipo sem resultados em Symbols sugere Classes de modo contextual, sem fundir resultados nem alterar semântica das abas.
- [x] Estados analysis-not-ready, vazio e retry continuam distintos e honestos.
- [x] E2E cobre contagem, limite/truncamento e sugestão sem resultado; preservar testids.
- [x] Executar build e E2E aplicável; registrar cobertura não executada.

## Comments

- Capturas: `33-search-symbols.png`, `34-search-text.png`, `90-search-symbols-retry.png`, `91-search-classes-retry.png`.
- A separação Classes (tipos) / Symbols (membros) é intencional e não deve ser reaberta neste ticket.
- Evidência desta implementação: [26-search-discovery.md](../evidence/26-search-discovery.md). Símbolos foram controlados pelo teste para exercitar truncamento e sugestão sem depender de cobertura/latência do LSP real.
