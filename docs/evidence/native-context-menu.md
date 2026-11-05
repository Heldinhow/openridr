# Evidência — menu de contexto nativo (alpha.7 → fix)

Data: 2026-11-05. macOS ARM64 (Apple Silicon), tela desbloqueada, agent-desktop 2.5 com
`accessibility: granted` e `screen_recording: granted`.
Driver: `agent-desktop` no app Tauri real (WKWebView), nunca no navegador.

- **Antes**: `/Applications/OpenRidr.app` 0.3.0-alpha.7 (release notarizada; é o mesmo tree da
  `origin/main`, `23f3802`).
- **Depois**: bundle desta branch, `bun run tauri build --debug --bundles app` →
  `src-tauri/target/debug/bundle/macos/OpenRidr.app` (frontend de **produção** — `import.meta.env.PROD`
  verdadeiro, que é o que liga o guard; só o lado Rust é debug, e por isso o menu de campo de texto
  ainda lista "Inspect Element").
- Estado isolado: `.agents/skills/verify-openridr/scripts/desktop.sh up <bundle>` (cópia da fixture em
  `<tmp>/verify-openridr-<uid>-desktop/work`, `OPENRIDR_HOME` em `.../home`). Uma instância por vez;
  `down` no fim.

## O que o `right-click` do agent-desktop faz

Sem `--headed` ele usa a ação de acessibilidade **AXShowMenu** (o caminho do relato); com `--headed`
é clique físico. Os dois foram exercitados.

## Antes (0.3.0-alpha.7 instalada)

| Linha / alvo | Ação | Resultado | Captura |
|---|---|---|---|
| raiz `OrderDesk · 3 projects` | AXShowMenu | menu do app (`Solution context menu`) | — |
| projeto `OrderDesk.Cli` | AXShowMenu | menu do app (`Project context menu`) | — |
| arquivo `Program.cs` | AXShowMenu | menu do app (`Open`/`Copy Path`/…/`Delete`) | `03-before-file-app-menu.png` |
| **pasta de solução `src · 2 projects`** | AXShowMenu | **menu do WebKit**: `Reload`, `Share…`, `Services` | `01-before-folder-native-menu.png` |
| **`Dependencies` (OrderDesk.Cli)** | AXShowMenu | **menu do WebKit**: `Look Up`, `Translate`, `Copy`, `Share…`, `Services` | `02-before-dependencies-native-menu.png` |
| **toolbar `Build`** | AXShowMenu | **menu do WebKit**: `Reload` | `04-before-toolbar-native-menu.png` |
| arquivo `Program.cs` focado | `Shift+F10` físico | nada (nenhuma superfície de menu) | — |

Ou seja: o relato de "arquivo" não se reproduziu como descrito — a linha de arquivo já abria o menu do
app. O que estava quebrado era a **pasta de solução** (a linha `src · 2 projects`, que o relato chamou
de pasta), `Dependencies` e derivados, e **todo o teclado**.

## Depois (branch `fix/native-context-menu`)

Todas as capturas em `docs/evidence/native-context-menu/`.

| Linha / alvo | Ação | Resultado | Captura |
|---|---|---|---|
| pasta de solução `src · 2 projects` | AXShowMenu | `Solution folder context menu` — Build Solution, Restore, Reload, Find in Files (sem `Copy Path`/`Show in Finder`/`Add Project…`) | `05-after-folder-app-menu.png` |
| `Dependencies` (Cli) | AXShowMenu | `Project context menu` | `06-after-dependencies-project-menu.png` |
| grupo `Frameworks` (Cli) | AXShowMenu | `Project context menu` | `07-after-frameworks-group-project-menu.png` |
| folha `Microsoft.NETCore.App (net8.0)` | AXShowMenu | `Project context menu` | `08-after-framework-leaf-project-menu.png` |
| pacote `xunit 2.9.2` | AXShowMenu | `Project context menu` | `09-after-package-project-menu.png` |
| referência (`Projects`) | AXShowMenu | `Project context menu` (com `Run Unit Test` quando o dono é projeto de teste) | `10-after-project-ref-project-menu.png` |
| projeto `OrderDesk.Tests` | AXShowMenu | `Project context menu` | `11-after-project-app-menu.png` |
| raiz | AXShowMenu | `Solution context menu` | `12-after-root-solution-menu.png` |
| arquivo `Program.cs` | AXShowMenu | `File context menu` | `13-after-file-app-menu.png` |
| arquivo `Program.cs` focado | `Shift+F10` físico | `File context menu` | `14-after-shift-f10-file-menu.png` |
| **toolbar `Build`** | AXShowMenu | **nenhum menu** (`surfaces: []`) | `15-after-toolbar-no-native-menu.png` |
| editor (clique direito físico no código) | `--headed mouse-click` | `Editor context menu` do app (o guard não atrapalha) | `16-after-editor-app-menu.png` |
| campo `Search Everywhere` | AXShowMenu | menu de **edição** do sistema (Cut/Copy/Paste/Spelling…) — permitido | `17-after-textfield-native-edit-menu.png` |

Leitura das superfícies de menu pelo `agent-desktop` (`title`/itens), além das capturas:

```
antes,  pasta .sln : {title: null, items: Reload | Share… | Services}
depois, pasta .sln : {title: "Solution folder context menu", items: Build Solution | Restore Solution | Reload Solution | Find in Files… | Add Project…}
antes,  Dependencies: {title: null, items: Look Up “Dependencies” | Translate | Copy | Share… | Services}
depois, Dependencies: {title: "Project context menu", items: Manage NuGet Packages | Reload Project | …}
depois, toolbar Build: {surfaces: []}   (right-click devolve ACTION_FAILED porque nenhum menu abriu)
```

## Revisão (itens 1, 2, 4 e 5)

Depois de `dabe1fc`, três correções pedidas na revisão foram provadas no app real (mesmo bundle de
produção, `OPENRIDR_HOME` isolado, perfil de keymap trocado pela própria UI: Settings ▸ Keymap ▸
Profile ▸ IntelliJ ▸ Save settings, com `settings.json` do home confirmando `"profile": "intellij"`).

| Item | Prova | Captura |
|---|---|---|
| 1. terminal mantém o menu do sistema | clique direito físico no corpo do terminal (`/bin/zsh` rodando) abre o menu com `Cut`/`Copy`/`Paste`; `Copy` desabilitado sem seleção, `Paste` habilitado | `18-after-terminal-native-menu.png` |
| 1. colar funciona | `printf 'echo MENU_PASTE_OK' \| pbcopy`, clique direito → `Paste` → o texto entra no prompt e o `Return` executa: `$ echo MENU_PASTE_OK` / `MENU_PASTE_OK` | `19-after-terminal-paste.png` |
| 2. linha da árvore ganha de `Shift+F10` | perfil IntelliJ (onde `Shift+F10` = Run All Tests): foco na linha `Program.cs` + `Shift+F10` abre `File context menu` (não roda testes) | `20-after-intellij-shift-f10-row-menu.png` |
| 2. fora da árvore o keymap vale | no editor, `Shift+F10` com o mesmo perfil: nenhuma superfície de menu e a tool window **Tests** fica ativa com a execução concluída (`4 results · 2 passed · 1 failed · 1 skipped`) | `21-after-intellij-shift-f10-editor-runs-tests.png` |
| 4. foco volta para a linha clicada | foco em `Program.cs` → clique direito físico em `Order.cs` (WKWebView não move o foco no clique) → fecha o menu → `Order.cs` é a linha com `states: ["focused"]`; `Program.cs` sem `focused` | — (estado de acessibilidade) |
| 5. solution folder sem `Add Project…` | menu da linha `src · 2 projects` = Build Solution, Restore Solution, Reload Solution, Find in Files… | `05-after-folder-app-menu.png` |

## Re-check de foco (29ª rodada)

O fechamento do menu devolve o foco à linha **enquanto o menu o segura**; ação que foca o próprio alvo
mantém. Medido no E2E (Chromium, `tests/e2e/native-context-menu.spec.ts`, testid do elemento com foco
logo depois da ação):

| Ação do menu | Foco depois | Observação |
|---|---|---|
| `Open` (arquivo) | a linha da árvore | abrir pela árvore mantém o foco na árvore, como no clique simples (`focus-after-navigation.spec.ts`); o editor não é focado por esse caminho |
| `Rename…` | `tree-name-input` | diálogo foca o campo no commit e vence a linha |
| `New File…` (projeto) | `tree-name-input` | idem |
| `Delete` | `tree-delete-cancel` | idem |
| `Find in Files…` | `find-in-files-input` | painel foca o campo |
| `Esc` (sem ação) | a linha | também depois de abrir o menu pelo mouse |

No app real (mesmo bundle, `OPENRIDR_HOME` isolado):

| Caso | Prova | Captura |
|---|---|---|
| menu → `Rename…` | diálogo `Rename` com o campo `Name` em `states: ["focused"]` — a linha não roubou | `22-after-rename-keeps-dialog-focus.png` |
| menu → `Open` | linha `Program.cs` continua com `states: ["focused"]`; o editor não é tocado nesse caminho | — (estado de acessibilidade) |
| `Esc` fecha o menu | linha `Order.cs` (a que abriu) fica com `states: ["focused"]` | `23-after-escape-returns-focus-to-row.png` |

`Esc` é enviado no app real por `osascript -e 'tell application "System Events" to key code 53'`
(o `press escape` do agent-desktop vira `AXCancel` e não fecha o menu React).

## Testes automatizados

Portas próprias (1460 UI / 4357 host), fixture em cópia temporária, `dotnet` 8.0.408 no PATH.

| Passo | Comando | Antes (HEAD) | Depois |
|---|---|---|---|
| E2E novo | `OPENRIDR_E2E_UI_PORT=1460 OPENRIDR_E2E_HOST_PORT=4357 bunx playwright test tests/e2e/native-context-menu.spec.ts` | **1 failed** — `tree-folder-src` não cancela o `contextmenu` (`expect(cancelled).toBe(true)` recebeu `false`) | **3 passed** (9,1 s) |
| Regressão árvore/busca/atalhos/foco | idem, `solution-explorer`, `medium-nav-find-in-files`, `actions`, `focus-after-navigation` + o spec novo | — | **31 passed** (1,0 min) |
| Unit | `bun test tests/unit` | `.xterm` e `contenteditable="false"`: **3 failed** com o `nativeMenu.ts` do commit anterior | **26 passed** (5 do guard) |
| Tipos/build | `bunx tsc --noEmit`, `bun run check` | — | sem erros |

Os itens da revisão também têm E2E que falha antes: com o `WorkspaceView.tsx` anterior, o teste do
perfil IntelliJ falha (o `Shift+F10` na linha não abre menu, o keymap roda testes) e a asserção
`tree-menu-add-project` do menu da solution folder recebe `1` em vez de `0`. O teste do `.xterm` e o do
`contenteditable="false"` falham com o `nativeMenu.ts` anterior.

O "antes" do E2E foi rodado com `git stash push -- src` (só o código de produto voltou ao HEAD; o spec
novo ficou) e depois `git stash pop`. Na revisão isso passou a ser feito por cópia do arquivo para
`/tmp` e restauração, porque `git stash` é global ao repositório e outra worktree empurrou um stash no
meio (ver `RETRO-NATIVE-CONTEXT-MENU.md`). A primeira asserção do spec — a que falha antes — não mudou
depois; as asserções de nome acessível (`File`/`Solution folder` context menu) entraram junto com o
`treeMenuLabel` e foram verificadas no run final, já no commit.

## Não verificado

- **Windows e macOS Intel**: o guard é JS (`preventDefault` em `contextmenu`) e deve valer em qualquer
  webview, mas só macOS ARM64 foi executado. No Windows a tecla de menu (`ContextMenu`) foi exercitada
  apenas no Chromium via Playwright.
- **DMG notarizado desta correção**: não construído nem instalado; a prova usa
  `tauri build --debug --bundles app` (frontend de produção, Rust debug). O guard roda no frontend, mas
  a embalagem final não foi repetida.
- **Menu nativo em área de texto do terminal (xterm)**: segue permitido por desenho; não foi aberto.
- `AXShowMenu` sobre `statictext` (nós sem `ref_id`, que o agent-desktop não consegue mirar) não é
  exercitável por esta ferramenta; o guard cancela o evento no documento, que é o mesmo caminho.
