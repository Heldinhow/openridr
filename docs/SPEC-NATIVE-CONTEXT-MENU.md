# SPEC — menu de contexto em toda a linha da árvore (e sem o menu do WebKit)

Status: implementado nesta branch (`fix/native-context-menu`), pendente de release.
Origem: aceitação da 0.3.0-alpha.7, ticket 05 — `docs/evidence/alpha7-acceptance.md` (worktree
`test/alpha7-acceptance`). Relato: "o `AXShowMenu` na linha abre o menu nativo do WebKit
(`Reload`/`Share…`/`Services`) em vez do menu do app" em pasta e arquivo; raiz e projeto abriam o
menu certo.

## O que ficou provado (diagnóstico)

O relato misturava duas causas distintas; a reprodução no app instalado mostrou:

1. **Linhas sem `onContextMenu`.** A linha de arquivo e as de pasta de código **já abriam** o menu do
   app por `AXShowMenu` (o WebKit despacha `contextmenu` de verdade para uma ação de acessibilidade
   "Show menu"). Quem não tinha handler: a linha de **solution folder** (`.sln`, ex.: `src · 2
   projects`), `Dependencies`, os grupos `Frameworks`/`Packages`/`Projects` e as folhas
   `tree-framework-*`, `tree-package-*`, `tree-ref-*`. Sem handler não há `preventDefault`, então o
   WebKit mostra o próprio menu — em texto selecionável ele vem como "Look Up / Translate / Copy /
   Share… / Services"; em elemento neutro, como "Reload / Share… / Services".
2. **Teclado não existia.** `Shift+F10` (e a tecla de menu) não é traduzido em `contextmenu` pelo
   WebKit: no app real, com o foco na linha `Program.cs`, nada abria. Também não havia handler de
   teclado nas linhas.

Consequência: qualquer área da UI sem handler (toolbar, abas, status bar) mostra o menu de navegador
do WKWebView — "Reload" num app de produção é sempre defeito.

## Contrato

1. Toda linha da árvore abre o menu do app por **clique direito**, por **Shift+F10** e pela **tecla
   de menu** (`ContextMenu`), além da ação de acessibilidade (que chega como `contextmenu`).
2. `TreeRow` é o único lugar que decide sobre o menu: ele cancela o `contextmenu` em **toda** linha,
   mesmo numa linha sem menu (defesa em profundidade), e entrega a posição de abertura (mouse ou retângulo
   da linha) **e a própria linha** para quem a desenha. O menu devolve o foco à linha que o abriu — no
   mouse também, porque o WKWebView não move o foco no clique direito.
3. **Precedência de teclado.** Com o foco numa `.tree-row`, `Shift+F10` e a tecla de menu abrem o menu
   da linha e não o atalho do keymap (no perfil IntelliJ, macOS, `Shift+F10` é Run All Tests; no Rider
   Windows, Run). Fora da árvore o keymap vale normalmente. O handler global (`WorkspaceView`, capture)
   devolve o evento para a linha antes de casar atalho ou acorde.
4. Em build de produção o menu nativo do WebKit não aparece em nenhum lugar que não seja **campo de
   texto** (`input`, `textarea`, `[contenteditable]` — exceto `contenteditable="false"`) ou o
   **terminal** (`.xterm`), onde o menu do sistema é a funcionalidade: é dele que saem Copiar/Colar do
   shell. Em desenvolvimento ele fica em toda parte: é o caminho para "Inspect Element" no webview.
5. Nenhuma dependência nova; nenhum `data-testid` renomeado ou removido.

## Menus por tipo de linha

| Linha | Menu | Nome acessível |
|---|---|---|
| raiz da solução | `solutionMenuItems` | `Solution context menu` (default) |
| solution folder (`.sln`) | `solutionFolderMenuItems` — comandos da solução sem `Copy Path`/`Show in Finder` (a pasta pode não existir no disco) nem `Add Project…` (ele adiciona na raiz, não na pasta) | `Solution folder context menu` |
| projeto | `projectMenuItems` | `Project context menu` |
| pasta de código / disco | `fileMenuItems` (com `New File…`, `Rename…`, `Delete`) | `Folder context menu` |
| arquivo | `fileMenuItems` | `File context menu` |
| `Dependencies`, grupos e folhas de framework/pacote/referência | menu do **projeto dono** | `Project context menu` |

A escolha por "menu do dono" é a mesma do Rider: `Dependencies` e o que está dentro dela pertencem ao
projeto, e as ações úteis (`Reload Project`, `Build`, `Manage NuGet Packages`, `Run Unit Test`) são as
do projeto. `Edit`/ficha de propriedades continuam vindo do projeto.

## Fora de escopo

- Menu próprio por pacote (ex.: "abrir no nuget.org") e por framework: não há ação de host para isso.
- Menu de contexto do editor: já existia e não muda (é do app). O do terminal é o do sistema, por
  desenho (Copiar/Colar do shell).
- Windows/Linux: o guard é JS e vale para qualquer webview; só o macOS ARM64 foi executado.

## Prova

- `tests/e2e/native-context-menu.spec.ts` — falha antes (a linha `tree-folder-src` não cancela o
  evento), passa depois; cobre mouse, `Shift+F10` e tecla de menu, e o nome acessível de cada menu.
- `tests/unit/nativeMenu.test.ts` — o guard cancela fora de campo de texto e não cancela dentro.
- App real (Tauri/WKWebView) em `docs/evidence/native-context-menu.md`, antes e depois.
