# 04 — Hover de variável no debug (d12)

**Fonte:** `docs/SPEC-MEDIUM-DEBUG.md` §4. **Bloqueado por:** nenhum. **Status:** ready-for-agent.

## Objetivo

Com a sessão pausada, passar o mouse numa variável do editor mostra o valor, avaliado pelo
adaptador, sem sujar o campo Evaluate do painel.

## Pontos de código

- `crates/host/src/dap.rs`: `Dap::hover(&mut self, expression) -> Result<EvaluationView>` —
  `paused_context("Hover")` + `evaluate_in(expression, frame_id, "repl")`, **sem** gravar em
  `data.evaluation`.
- `crates/host/src/lib.rs`: dispatch `debug.hover` (trust como `debug.evaluate`, payload
  `{expression}`), devolvendo a avaliação (não o snapshot).
- `src/host/client.ts`: `call<DapEvaluation>("debug.hover", …)` helper se necessário.
- `src/CodeEditor.tsx`: prop opcional `onHoverEvaluate?: (expression: string) => Promise<{ value, typeName, error } | null>`;
  `hoverTooltip` de `@codemirror/view` registrado uma vez com ref atualizada; palavra sob o
  cursor por regex (`[\w.]+`), `hoverTime: 300`; tooltip mostra `typeName value` ou o erro.
- `src/WorkspaceView.tsx`: passa o handler só quando `dap.state === "paused"`; ele chama
  `debug.hover` e devolve `null` em falha de transporte.

## Teste primeiro

`tests/e2e/debugger.spec.ts` (teste real, skip sem adaptador): parar num breakpoint onde
`person` existe; `page.getByTestId("editor").hover()` sobre a palavra `person` (locator por
texto dentro de `.cm-line`); esperar `.cm-tooltip-hover` com o tipo/valor; garantir que
`debug-evaluate-result` não mudou. Falhar antes (hoje não existe tooltip).

## Aceite

- [ ] `debug.hover` exige pausa e não altera `dap.evaluation`.
- [ ] Hover só aparece pausado; erro do adaptador aparece no tooltip.
- [ ] Nenhuma dependência nova.
- [ ] E2E real verde quando o adaptador existe; senão skip registrado.
- [ ] `bun run check` verde. Commit `feat(debug): hover variable values while paused`.
