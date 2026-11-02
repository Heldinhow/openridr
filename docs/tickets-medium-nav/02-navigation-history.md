# 02 — Back/Forward e Go to Line (d15, d16 / b19)

**Fonte:** `docs/SPEC-MEDIUM-NAV-SEARCH.md` §2–3. **Achados:** QA-DEVFLOW d15, d16; QA-BUGHUNT b19.
**Bloqueado por:** nenhum. **Status:** ready-for-agent.

## Objetivo

Histórico de navegação real com botões/atalhos e Go to Line funcionando nos dois perfis.

## Pontos de código

- `src/WorkspaceView.tsx`:
  - `NavEntry = { path: string; line: number; character: number }`; refs
    `{ entries: NavEntry[]; index: number }` + estado `{ canBack, canForward }`.
  - `recordNavigation()` usa `activePathRef.current` e `cursorRef.current` antes de sair.
  - Chamar `recordNavigation()` em `navigateTo()` e nos opens de usuário: árvore
    (`SolutionTree onOpenFile`), `openSearchFile`, menu de contexto (`treeMenuItems`,
    `fileMenuItems`), `eval-open-project`, `openGlobalJson` e afins. O `initialFile` e o
    `openFile` interno do `navigateTo` não gravam duas vezes.
  - `travel(delta)`: abre `entries[next]`, seta `editorTarget` com `navigationSeq`, sem gravar.
    Nova navegação trunca `entries.slice(0, index + 1)`; limite 100.
  - Ações `navigate.back` / `navigate.forward` (grupo Navigation), `disabledReason` quando não
    há destino. Trocar os dois botões fixos da toolbar (linhas ~3499) por botões com
    `data-testid="nav-back"` / `"nav-forward"`, `disabled` pelo estado e `onClick`.
  - `search.line`: remover `editor: false`, o `execute: () => undefined` e o motivo stub.
    Estado `gotoLineRequest` + ação que incrementa; `disabledReason` sem aba ativa.
- `src/CodeEditor.tsx`: nova prop `gotoLineRequest?: number`; `useEffect` chama
  `gotoLine(view)` de `@codemirror/search` e `view.focus()` quando o número muda.
- `src/actions/keymap.ts` (só perfil Rider para os novos atalhos de histórico; IntelliJ
  mantém o que tem):
  - `navigate.back`: `⌘[` e `⌘⌥←`; `navigate.forward`: `⌘]` e `⌘⌥→`.
  - `search.line` Rider: `⌘G` e `⌘L`; IntelliJ: `⌘L` (já existente).
  - Ação `search.line` precisa de `editor: true` para o handler global consumir antes do
    CodeMirror.

## Teste primeiro

Criar `tests/e2e/medium-nav-navigation.spec.ts` (host real, `fixtures/OrderDesk`):

1. abrir `Program.cs` pela árvore, depois `Order.cs`; `nav-back`/`nav-forward` habilitados;
   `Meta+[` volta para `Program.cs`, `Meta+]` avança para `Order.cs`; repetir com
   `Meta+Alt+ArrowLeft/Right`.
2. `Meta+g` abre `.cm-goto-line`; preencher `3` e Enter; `status-cursor` na linha 3.
   `Meta+l` idem (perfil Rider).
3. Go to Definition (csharp-ls): abrir `Program.cs`, cursor em `Order`, `Meta+b`, esperar a aba
   `Order.cs`; `Meta+[` volta para `Program.cs` na posição anterior. `test.skip` quando
   `tools.plan` não tiver csharp-ls `present`.

Rodar o spec e ver falhar antes de implementar.

## Aceite

- [ ] Histórico grava abertura de arquivo e navegação; Back/Forward funcionam por botão e atalho.
- [ ] Nova navegação após voltar descarta o ramo à frente; inicial não grava.
- [ ] Go to Line funciona com foco no editor e fora dele, nos dois atalhos Rider.
- [ ] IntelliJ mantém seus atalhos (não ganha `⌘G` de Go to Line).
- [ ] Teste alvo verde; `bun run check` verde.
- [ ] Commit local `feat(navigation): history and go to line` (só caminhos deste ticket).
