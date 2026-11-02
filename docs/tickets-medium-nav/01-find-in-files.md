# 01 — Find in Files (b5 / d17)

**Fonte:** `docs/SPEC-MEDIUM-NAV-SEARCH.md` §1. **Achados:** QA-BUGHUNT b5; QA-DEVFLOW d17.
**Bloqueado por:** nenhum. **Status:** ready-for-agent.

## Objetivo

`⇧⌘F` e o item **Find in Files…** dos menus de contexto abrem uma busca no projeto que
realmente busca, respeita o escopo clicado e não reusa a consulta anterior do Search
Everywhere.

## Pontos de código

- `src/actions/keymap.ts`: adicionar `search.findInFiles` com `⇧⌘F` no perfil Rider e no
  IntelliJ (nos dois é o atalho padrão de Find in Files).
- `src/WorkspaceView.tsx`:
  - estado `findInFiles: { scope: string; nonce: number } | null`;
  - ação `search.findInFiles` (grupo Search) com `execute: () => setFindInFiles({ scope: "", nonce: ... })`;
  - `treeMenuItems`: trocar `onFind: () => openSearch("text")` por abrir Find in Files com
    `path` do alvo (`""` para raiz; o path do arquivo/pasta/projeto nos demais);
  - renderizar `<FindInFiles … onClose onNavigate={(path, line, character) => void navigateTo(...)} />`.
- Novo `src/FindInFiles.tsx` (um componente por arquivo). Reusar `call<TextSearch>("search.text", { query, caseSensitive })`
  com debounce ~180 ms e mínimo de 2 caracteres. Não duplicar lógica de host.
  - Agrupar `files[].matches` por `line` (uma entrada por linha; guardar a primeira coluna).
  - Filtrar por escopo no cliente: `path === scope || path.startsWith(scope + "/")`.
  - `data-testid`: `find-in-files`, `find-in-files-input`, `find-in-files-case`,
    `find-in-files-scope`, `find-in-files-results`, `find-in-files-item-N`,
    `find-in-files-empty`, `find-in-files-status`, `find-in-files-close`.
  - Esc fecha; `autoFocus` no input; clique/Enter navega.

## Teste primeiro

Criar `tests/e2e/medium-nav-find-in-files.spec.ts` (host real, `fixtures/OrderDesk`):

1. abrir workspace confiado, focar o editor, `Meta+Shift+F`: diálogo visível e input focado;
   digitar `A-104`; clicar no primeiro resultado; `Program.cs` ativo e cursor na linha do
   `A-104`.
2. clique direito em `tree-dir-src/OrderDesk.Core` → `tree-menu-find`: `find-in-files-scope`
   contém `src/OrderDesk.Core`; buscar `Order`; toda linha de resultado começa com esse prefixo.
3. buscar `ORDER` com Match case desligado (acha) e ligado (não acha), provando o parâmetro.
4. Esc fecha.

Rodar `bunx playwright test tests/e2e/medium-nav-find-in-files.spec.ts` e ver falhar pelo
motivo certo antes de implementar.

## Aceite

- [ ] `⇧⌘F` funciona com foco no editor e fora dele.
- [ ] Menu de contexto de arquivo/pasta/projeto/raiz abre com o escopo certo.
- [ ] Uma entrada por linha; clique navega para linha/coluna.
- [ ] Match case chega ao host (`caseSensitive`).
- [ ] Esc fecha; sem dependência nova; `data-testid` existentes intactos.
- [ ] Teste alvo verde; `bun run check` verde.
- [ ] Commit local `feat(search): find in files with scope` (só caminhos deste ticket).
