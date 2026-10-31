# 31 — M7: previews docados, sem cobrir o editor

- [x] Rename preview, code actions e Find Usages deixam de flutuar sobre o editor; docam abaixo dele (CSS `.editor-popup`).
- [x] Remover o cálculo de posição (`editorPopupStyle`, `data-anchor-*`).
- [x] D-12: click no texto e Esc continuam cancelando; testid/foco/Apply guards intactos. Geometria afirmada em `csharp-ls.spec.ts` (sem sobreposição; editor ≥100 px a 1100×700).
- [x] Capturas: `docs/evidence/screenshots/09-rename-preview.png` (1440×900), `m7-rename-preview-1100x700.png`, `m7-code-actions-1100x700.png`.
- [x] `bun run check`, `bun run build`, `csharp-ls.spec.ts -g "code actions are listed|rename|D-12"`.

Não verificado: Tauri nativo, Windows, Intel; suíte completa. Rename inline com destaque do range segue fora.
