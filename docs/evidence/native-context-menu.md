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
| pasta de solução `src · 2 projects` | AXShowMenu | `Solution folder context menu` — Build Solution, Restore, Reload, Find in Files, Add Project (disabled) | `05-after-folder-app-menu.png` |
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

## Testes automatizados

Portas próprias (1460 UI / 4357 host), fixture em cópia temporária, `dotnet` 8.0.408 no PATH.

| Passo | Comando | Antes (HEAD) | Depois |
|---|---|---|---|
| E2E novo | `OPENRIDR_E2E_UI_PORT=1460 OPENRIDR_E2E_HOST_PORT=4357 bunx playwright test tests/e2e/native-context-menu.spec.ts` | **1 failed** — `tree-folder-src` não cancela o `contextmenu` (`expect(cancelled).toBe(true)` recebeu `false`) | **1 passed** (4,0 s) |
| Regressão árvore/busca | idem, `solution-explorer.spec.ts medium-nav-find-in-files.spec.ts` | — | **16 passed** (45,7 s) |
| Run final (tree commitada) | idem, os três specs juntos | — | **17 passed** (1,1 min) |
| Unit | `bun test tests/unit` | — | **25 passed** (4 novos) |
| Tipos | `bunx tsc --noEmit` | — | sem erros |

O "antes" do E2E foi rodado com `git stash push -- src` (só o código de produto voltou ao HEAD; o spec
novo ficou) e depois `git stash pop`. A primeira asserção do spec — a que falha antes — não mudou
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
