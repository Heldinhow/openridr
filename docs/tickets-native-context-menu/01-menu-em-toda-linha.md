# 01 — Menu do app em toda linha, sem o menu do WebKit

Status: implemented e verificado no app real (macOS ARM64, 2026-11-05)
Spec: [SPEC-NATIVE-CONTEXT-MENU.md](../SPEC-NATIVE-CONTEXT-MENU.md)

## Problema

Na aceitação da alpha.7 (ticket 05) o `AXShowMenu` numa linha de pasta abriu o menu do WebKit
(`Reload`/`Share…`/`Services`). Reproduzido no app instalado: a linha de **solution folder**
(`src · 2 projects`), `Dependencies`, os grupos e as folhas de dependência não tinham handler; linha de
arquivo e de pasta de código já abriam o menu do app. `Shift+F10`/tecla de menu não abriam nada em
lugar nenhum, e toolbar/abas/status bar mostravam o menu de navegador do WKWebView.

## Mudança

- `src/TreeRow.tsx`: `onContextMenu?: (event: MouseEvent)` → `onMenu?: (position: MenuPosition)`. A
  linha sempre faz `preventDefault()` no `contextmenu` e trata `Shift+F10` e a tecla `ContextMenu` no
  `onKeyDown`, abrindo o menu no retângulo da linha. `MenuPosition` é exportado daqui.
- `src/SolutionTree.tsx`: `onMenu(position, target)` no contrato; todas as linhas passam um alvo.
  Novos alvos: `{ kind: "solution-folder", name }` para a pasta do `.sln`; as linhas de
  `Dependencies`, grupos, framework, pacote e referência usam `{ kind: "project" }` do projeto dono.
- `src/solutionMenu.ts`: `solutionFolderMenuItems` (a solução sem `Copy Path`/`Show in Finder`).
- `src/WorkspaceView.tsx`: `openTreeMenu(position, target)` (sem `event`), `solution-folder` em
  `treeMenuItems`, `treeMenuLabel` (nome acessível por tipo) e `closeTreeMenu`, que devolve o foco à
  linha quando o menu foi aberto por teclado.
- `src/nativeMenu.ts` (novo): `keepsNativeContextMenu` e `installNativeContextMenuGuard`; `main.tsx`
  instala com `import.meta.env.PROD` — produção cancela o `contextmenu` em tudo que não é campo de
  texto; desenvolvimento mantém (Inspect Element no webview).
- Nenhum `data-testid` novo, renomeado ou removido; nenhuma dependência nova.
- **Revisão:** o guard global também deixa passar o **terminal** (`.xterm`: é dele que saem
  Copiar/Colar) e ignora `contenteditable="false"`; com o foco numa `.tree-row`, `Shift+F10`/tecla de
  menu abrem o menu da linha antes do keymap (IntelliJ = Run All Tests); `onMenu(position, row)` dá o
  foco de volta à linha que abriu o menu, inclusive no clique direito; a solution folder perde
  `Add Project…`.

## Testes

- `tests/e2e/native-context-menu.spec.ts` (novo): a linha `tree-folder-src` cancela o evento; menu por
  clique direito na pasta de solução, raiz, `Dependencies`, grupo, framework, pacote e referência; menu
  por `Shift+F10` e pela tecla `ContextMenu`, com o foco de volta na linha no `Escape`; nome acessível
  de cada menu.
- `tests/unit/nativeMenu.test.ts` (novo): o guard cancela fora de campo de texto, não cancela em
  `input`/`textarea`/`[contenteditable]`, e não registra nada quando desligado.
- Rodados: E2E 1460/4357 (vermelho antes, verde depois), `solution-explorer.spec.ts` +
  `medium-nav-find-in-files.spec.ts` (16/16), `bun test tests/unit` (25/25), `bunx tsc --noEmit`.

## Aceite

- [x] Clique direito, `Shift+F10` e ação de acessibilidade abrem o menu do app em raiz, pasta de
      solução, projeto, pasta de código, arquivo, `Dependencies`, grupos e folhas de dependência.
- [x] Em produção nenhuma área fora de campo de texto mostra menu do WebKit (toolbar provada; guard
      cobre o resto).
- [x] Campo de texto e terminal mantêm o menu do sistema (no terminal, `Paste` cola no shell).
- [x] Com a linha focada, `Shift+F10`/tecla de menu abrem o menu dela mesmo quando o keymap mapeia a
      tecla (perfil IntelliJ); fora da árvore o atalho do keymap continua valendo.
- [x] Ao fechar, o foco volta para a linha que abriu o menu (teclado e mouse) **quando o menu ainda o
      tem**; ação que abre e foca diálogo/painel/editor mantém o foco do alvo.
- [x] Nenhuma dependência nova; `data-testid` preservados.
- [ ] Windows/macOS Intel: não executado (o guard é JS e não depende da plataforma).
