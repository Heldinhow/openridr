# 04: Escala tipográfica em 4 tokens

**What to build:** tokens `--fs-ui:14px`, `--fs-label:13px`, `--fs-meta:12px`, `--fs-badge:11px` em `src/styles.css`; substituir os ~15 valores atuais (12 px, 0,75/0,78/0,8125/0,82/0,7 rem, 10/9/8 px, etc.) pelos tokens, sem mudar layout além do necessário. `font-variant-numeric: tabular-nums` na status bar e contadores. Avaliar `-webkit-font-smoothing` no texto da UI por captura antes/depois. Não reformatar o CSS inteiro; só trocar os valores de `font-size`.

**Blocked by:** nenhum.

**Status:** implementado (commit local; ver RETRO-UI-POLISH).

- [ ] `grep -nE "font-size: *(0\.[0-9]+rem|[0-9]px|10px)" src/*.css` não retorna nada abaixo de 11 px nem rem fracionário.
- [ ] Papéis (título, linha, meta, selo) reconhecíveis sem ler o texto (crop antes/depois).
- [ ] Nenhum truncamento novo na árvore, status bar ou abas (ws-1 comparada).
- [ ] `bun run build` passa; e2e sem falhas novas.
