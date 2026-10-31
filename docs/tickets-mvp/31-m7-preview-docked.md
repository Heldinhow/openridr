# 31 — M7a: previews docados, sem cobrir o editor

Escopo: **M7a (overlap/docking) resolvido. M7b (rename inline sobre o identificador com range) aberto** — ver `decisions.md` “M7: previews docados”. M7 não está totalmente fechado.

- [x] Rename preview, code actions e Find Usages deixam de flutuar sobre o editor; docam abaixo dele (CSS `.editor-popup`: `min-height:100px`; teto 40% **superseded** → 50% no ticket 32; `order:2` removido no ticket 34).
- [x] ~~Janelas baixas: ocultar resumo e dicas do LSP~~ — **superseded no ticket 34** (nada informativo é ocultado).
- [x] Remover o cálculo de posição (`editorPopupStyle`, `data-anchor-*`).
- [x] D-12: click no texto e Esc continuam cancelando; testid/foco/Apply guards intactos. `csharp-ls.spec.ts` afirma não-sobreposição (tolerância 0,5 px, sub-pixel), editor ≥100 px a 1100×700, 1ª ação + Close (code actions) e Apply/Cancel (rename) dentro do painel a 1100×700. `editor-overflow.spec.ts` ajustado ao layout docado. Helper renomeado `editorPointOutsidePopup`.
- [x] Capturas agora recortam `.editor-pane` (sem painel de ferramentas, sem caminho do usuário): `09-rename-{prompt,preview,preview-files,stale,applied}.png`, `10-code-actions-{list,preview,applied,empty,stale,diagnostic}.png`, `m7-rename-preview-1100x700.png`, `m7-code-actions-1100x700.png`. As versões antigas (full-page, uma com `~/.dotnet/...` (caminho do usuário)) ficam no histórico git local não publicado (`ee30055` e anteriores); sem rewrite nem push.
- [x] `bun run check`, `bun run build`, `csharp-ls.spec.ts -g "code action|rename|D-12"` 12/12, `editor-overflow`/`find-usages` verdes.

Não verificado: Tauri nativo, Windows, Intel; suíte completa. A 1100×700 com ferramentas+painel inferior abertos o editor fica com ~4 linhas e o Close fixo cobre parte da 2ª ação (rolagem do painel).
