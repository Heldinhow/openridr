# M7a — provas vermelhas (tickets 32 e 34)

- Ticket 32 (e2e a 1100×700 com Problems ativo, antes do CSS que faz a dock ceder): `expect(preview.bottom).toBeLessThanOrEqual(dock.top)` → Expected `<= 452`, Received `469.55` (preview 17 px abaixo da dock).
- Ticket 34 (implementação do ticket 32 sem as correções A/B/C): `expect(getByTestId("rename-preview-summary")).toBeVisible()` e `expect(getByTestId("code-actions-summary")).toBeVisible()` → Expected visible, Received hidden (resumo oculto por `@media (max-height:760px)`); os testes de status e de ordem DOM do popup após `.editor-body` ficam depois dessas asserções.
- Verde após a correção: `csharp-ls.spec.ts -g "code action|rename|D-12"`, `editor-overflow`, `find-usages` (exit 0; ver ticket 34).
