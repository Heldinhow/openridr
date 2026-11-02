# 01 — Frame do debug reusa a aba (d11)

**Fonte:** `docs/SPEC-MEDIUM-DEBUG.md` §1. **Bloqueado por:** nenhum. **Status:** ready-for-agent.

## Objetivo

Clicar num frame do call stack não pode abrir uma segunda aba do mesmo arquivo quando o
adaptador manda caminho absoluto e a aba já existe com caminho relativo ao workspace.

## Pontos de código

- `src/solutionNodes.ts`: novo `workspaceRelative(path, workspaceDir)` — troca `\` por `/`,
  remove `workspaceDir + "/"` de caminho absoluto; caminho já relativo passa igual.
- `src/WorkspaceView.tsx`: `openFile` normaliza no topo (antes do `tabs.some(...)`) e usa o
  caminho normalizado em `file.read`, `setActivePath` e no tab; `navigateTo` também normaliza.
  `debugCurrentLine` pode usar o helper no lugar do `endsWith`.
- Manter `tab-${path}` relativo (outros specs dependem disso, ex. `tab-BrokenImport.csproj`).

## Teste primeiro

1. `tests/e2e/medium-debug-paths.spec.ts` (puro, sem host): importa `workspaceRelative` e cobre
   absoluto dentro do workspace, absoluto com `\`, relativo e absoluto fora do workspace.
2. Em `tests/e2e/debugger.spec.ts`, no teste real já existente que para num breakpoint: clicar
   duas vezes no mesmo `debug-frame-${id}` e afirmar `page.getByTestId("tab-src/OrderDesk.Cli/Program.cs")`
   com count 1 (ou o caminho relativo do frame). Requer netcoredbg real; skip documentado.

Rodar os dois antes da correção e ver falhar pelo motivo certo.

## Aceite

- [ ] Um frame clicado duas vezes mantém uma aba só.
- [ ] Nenhum caminho absoluto vira `tab-` no DOM.
- [ ] Teste puro verde; debugger real verde quando o adaptador existe.
- [ ] `bun run check` verde. Commit `fix(debug): reuse tab for stack frames`.
