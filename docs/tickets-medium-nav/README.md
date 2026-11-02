# Tickets — QA médios de navegação e busca (2026-11-02)

Fonte normativa: [SPEC-MEDIUM-NAV-SEARCH](../SPEC-MEDIUM-NAV-SEARCH.md). Um ticket por área,
um commit por achado. Sem push/PR/release.

| Ticket | Achados | Entrega |
|---|---|---|
| [01 — Find in Files](01-find-in-files.md) | b5, d17 | `⇧⌘F` e menu de contexto buscam no projeto com escopo e Match case |
| [02 — Back/Forward e Go to Line](02-navigation-history.md) | d15, d16, b19 | Histórico de navegação e Go to Line (`⌘G`/`⌘L`) |
| [03 — Abas, Recent Files e perfil Rider](03-tabs-keymap.md) | d17, d18, b16, b19 | `⌘W`, troca de abas, `⌘E`, `⇧⌘O`, `⌘D`/`⌘⌫`, duplo Shift |
| [04 — Diálogos Push/Update](04-git-dialogs.md) | d14 | Foco no confirmar e Esc |
| [05 — Search Everywhere](05-search-ranking.md) | b6, b7 | Sem duplicatas, ranking, Symbols coerente, consulta selecionada |
| [06 — Layout](06-layout.md) | b9, b8 | Find Usages não sobrepõe, banner não empurra, drawer sem overflow |

Ordem de execução: 01 → 02 → 03 → 04 → 05 → 06. Os tickets 01–06 são independentes entre si,
exceto 05, que pode aproveitar a troca de `openSearch("last")` do 03.
