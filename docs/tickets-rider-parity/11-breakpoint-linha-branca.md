# 11 — Breakpoint em linha sem código (G19)

**Fonte:** `docs/SPEC-RIDER-PARITY.md`, seção "Breakpoint em linha sem código". **Bloqueado
por:** nenhum. **Status:** ready-for-agent.

## Objetivo

Um breakpoint em linha em branco ou só de comentário não pode parecer ativo e executável. Ele
é aceito, mas marcado como "sem código nesta linha" até o debugger reportar um bound
breakpoint.

## Pontos de código

- `src/CodeEditor.tsx`: gutter de breakpoint, `DebugBreakpoint.tsx`, tema da marca.
- `src/WorkspaceView.tsx`: `toggleBreakpoint`, lista de breakpoints salvos, `active.content`.
- `src/host/client.ts`: estado de breakpoint, se o host já reportar bound/unbound.
- `docs/decisions.md`: decisão da grill e pendência do comportamento do Rider.

## Comportamento

- Ao alternar breakpoint numa linha cujo texto é vazio ou só comentário, a marca é criada em
  estado "sem código" (visual atenuado, tooltip "No code on this line; the debugger will not
  stop here."). Não recusar e não mover para a linha seguinte.
- Numa linha de código, a marca continua normal.
- Quando o debugger reportar bound, a marca volta ao estado normal; se o adaptador não reportar,
  o aviso permanece e a decisão registra o limite.
- O comportamento do Rider para linha em branco continua **não verificado**; a pendência fica
  em `docs/decisions.md`.

## Teste e verificação

- Playwright: na linha em branco do fixture, alternar breakpoint e conferir a marca atenuada +
  tooltip; numa linha de código, conferir a marca normal; remover o breakpoint volta ao estado
  limpo.
- App real (agent-desktop): repetir no gutter, com hover para ler o tooltip, e iniciar um
  Debug para conferir que a marca não aparenta parada.

## Aceite

- [x] Breakpoint em linha em branco/comentário é aceito e marcado como sem código.
- [x] O tooltip explica que o debugger não vai parar ali.
- [x] Linha de código continua com a marca normal.
- [x] Não recusa, não move para a linha seguinte.
- [x] Pendência do comportamento do Rider registrada.
- [ ] Evidência no app real.
