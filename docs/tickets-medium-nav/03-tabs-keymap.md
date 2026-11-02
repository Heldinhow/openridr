# 03 — Abas, Recent Files, Go to File, perfil Rider e duplo Shift (d17, d18, b16)

**Fonte:** `docs/SPEC-MEDIUM-NAV-SEARCH.md` §4–6. **Achados:** QA-DEVFLOW d17, d18; QA-BUGHUNT b16, b19.
**Bloqueado por:** nenhum (independente do 02). **Status:** ready-for-agent.

## Objetivo

Fechar/trocar abas, Recent Files, Go to File no estilo Rider, corrigir as divergências do
perfil Rider e impedir que o duplo Shift dispare durante seleção com teclado.

## Pontos de código

- `src/actions/keymap.ts`:
  - `editor.closeTab` `⌘W`, `editor.nextTab` `⇧⌘]` + `⌃Tab`, `editor.previousTab` `⇧⌘[` +
    `⌃⇧Tab`, `search.recentFiles` `⌘E` — iguais nos dois perfis.
  - `search.file` Rider: `⇧⌘O` (sai `⇧⌘T`). `search.everywhere` Rider: **sem atalho** (sai
    `⌘T`); IntelliJ inalterado (`search.file` = `⌘O`, `search.everywhere` sem atalho).
- `src/WorkspaceView.tsx`:
  - ações `editor.closeTab` (`requestCloseTab(activePath)`), `editor.nextTab`/`previousTab`
    (wrap em `tabs`), `search.recentFiles`; motivos de desabilitado quando não há aba.
  - `recentFiles: string[]` (máx. 20, mais recente primeiro), atualizado em `openFile` e ao
    ativar aba; abrir pela lista chama `openFile` e fecha o diálogo.
  - Novo `src/RecentFilesDialog.tsx`: `role="dialog"`, Esc, `autoFocus`, lista
    `data-testid="recent-files"`, itens `recent-file-N`, vazio `recent-files-empty`.
  - `openSearch("last")` → `openSearch("all")` nos dois call sites (ação e duplo Shift).
- `src/CodeEditor.tsx`: nova prop `profile` (Rider/IntelliJ). Só no Rider, adicionar com
  `Prec.high(keymap.of([...]))`:
  - `Mod-d` → `copyLineDown` (`@codemirror/commands`);
  - `Mod-Backspace` → `deleteLine`.
  O perfil IntelliJ continua com o keymap padrão do CodeMirror (Mod-d seleciona ocorrência).
- Duplo Shift em `WorkspaceView.tsx`: disparar apenas em **duas batidas limpas** — keydown de
  Shift arma; qualquer keydown de outra tecla enquanto Shift está pressionado invalida;
  keyup de Shift conclui a batida; duas conclusões em ≤400 ms abrem. Inputs/contenteditable
  continuam isentos.

## Limitação a registrar

Acrescentar em `docs/decisions.md` (seção nova, data 2026-11-02):
- Format/Organize Usings não entram porque o host não expõe `textDocument/formatting` nem
  `source.organizeImports`; a UI não finge suporte.
- Recent Files é lista da sessão (não persistida) e Find in Files não tem regex/máscara
  (limite do `search.text`).

## Teste primeiro

Criar `tests/e2e/medium-nav-tabs-keymap.spec.ts` (cópia temporária de `fixtures/OrderDesk`
para os testes que editam o buffer):

1. abrir `Program.cs` e `Order.cs`; `Meta+w` fecha a ativa; `Meta+Shift+]`/`Control+Tab`
   alternam; `Meta+Shift+[`/`Control+Shift+Tab` voltam.
2. `Meta+e` abre `recent-files` com os dois arquivos; clicar no segundo abre.
3. `Meta+Shift+O` abre Search Everywhere na aba Files; `Meta+t` **não** abre nada.
4. Rider: `Meta+d` duplica a linha (contagem +1 e texto repetido); `Meta+Backspace` apaga a
   linha (contagem −1). IntelliJ (settings.json): `Meta+d` não duplica.
5. `Shift+ArrowUp` seguido de `Shift+Home` em sequência rápida não abre Search Everywhere;
   duas batidas de Shift abrem.

Atualizar `tests/e2e/actions.spec.ts` para os novos defaults (`search.file` ⇧⌘O,
`search.everywhere` "No shortcut"; IntelliJ `search.file` ⌘O). Rodar os dois specs e ver
falhar antes de implementar.

## Aceite

- [ ] Atalhos e ações novos funcionam; `⌘T` não abre mais Search Everywhere no Rider.
- [ ] `⇧⇧` continua abrindo; seleção com Shift não dispara.
- [ ] Recent Files abre/fecha/navega; Go to File abre na aba Files.
- [ ] Perfil IntelliJ não muda comportamento de edição.
- [ ] Limitações registradas em `docs/decisions.md`.
- [ ] Testes alvo verdes; `bun run check` verde.
- [ ] Commit local `feat(editor): tabs, recent files and rider keymap` (só caminhos deste ticket).
