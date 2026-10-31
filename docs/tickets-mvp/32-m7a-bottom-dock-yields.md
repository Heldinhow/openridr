# 32 — M7a residual: o painel inferior cede ao preview docado

Origem: prova no `.app` real (`docs/evidence/logs/native-r6-2026-10-31.md`): a 1100×700 com Problems aberto a 1ª ação e a 3ª linha do rename ficavam cortadas. Continua o ticket 31; M7b segue aberto.

- [x] Teste vermelho: a 1100×700 com `bottom-tab-problems` ativo, o rename preview passava 17 px da dock.
- [x] CSS: `.workspace-view:not(.bottom-closed):has(.editor-popup)` limita a linha inferior a `min(--output-height, 96px)`; `.editor-popup` `max-height: min(50%, 300px)`.
- [x] E2E (a 1100×700): 2 primeiras ações + Close, 3ª linha + Apply/Cancel visíveis no painel, painel acima da dock, editor ≥100 px.
- [x] `bun run check`, `bun run build`, `csharp-ls.spec.ts -g "code action|rename|D-12"` 12/12, `editor-overflow` + `find-usages` 3/3.
- [x] Capturas regeneradas (`m7-*`, `09-*`, `10-code-actions-*`; recorte do `.editor-pane`).

Não verificado: reprova no `.app` nativo (a cargo do native-proof-4), Windows, Intel.
