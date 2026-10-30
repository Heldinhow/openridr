# Referências visuais do JetBrains Rider (dark, padrão)

Capturadas em 2026-10-29 no Rider (cask brew, licença **Trial**, sem login), macOS ARM64, tema escuro padrão.
Projeto: cópia de `fixtures/OrderDesk` em `/tmp/OrderDesk` (git init local, 1 linha `// edit` a mais para gerar diff). Coordenadas abaixo são relativas à janela do Rider.

Observações gerais do layout:
- Barra superior (header): botões de janela, voltar/avançar, seletor de projeto "OrderDesk ▾", seletor de branch Git "main ▾" (no lugar de "Version Control" quando há repo), e **à direita**: Build (martelo), seletor de run config "OrderDesk.Cli ▾", Run (▶ verde), Debug (bug verde), menu ⋮, depois ícones de IA, Search Everywhere (lupa) e Settings (engrenagem).
- Durante Run/Debug, ▶ vira "Rerun", Debug vira ícone de reattach/rerun e aparece Stop (quadrado vermelho) logo à direita da run config.
- Barras laterais de tool windows: esquerda (Solution/Explorer, Commit, Structure, "…" no topo; Build, Tests, Services/Run, Debug, NuGet, Terminal, Problems, Git no rodapé) e direita (Notifications, AI, Database, Monitoring). Painel inferior (Run/Debug/Terminal/etc.) ocupa a largura toda sob o editor.
- Keymap padrão neste Rider: "Visual Studio (macOS)".

| Arquivo | Tela | Layout |
|---|---|---|
| 01-main-window.png | Janela principal, projeto carregado, sem arquivo aberto | Header completo; árvore "Solution" à esquerda; run config "OrderDesk.Cli" + Run + Debug à direita do header; dicas de atalhos no editor vazio; status bar com badge "Trial" no canto direito. |
| 02-editor-tree.png | Árvore de solução + editor com Program.cs | Árvore: OrderDesk › src (2 projetos), tests (1 projeto), Scratches and Consoles. Aba "Program.cs" com ícone C#; gutter com números de linha; barra verde de linha alterada ao lado do número; breadcrumb do caminho na status bar (esquerda) e posição/LF/UTF-8/indent à direita. |
| 03-breakpoint-gutter.png | Editor com breakpoint | Bolinha vermelha no gutter (esquerda do número da linha, ao lado da lâmpada de ações); linha inteira com fundo vermelho translúcido. |
| 04-debug-paused.png | Debug parado no breakpoint (Threads & Variables) | Painel "Debug" embaixo, abas por sessão ("OrderDesk.Cli"). Toolbar vertical-esquerda: Rerun, Stop, Resume, Pause, **Step Over, Step Into, Step Out**, View/Mute Breakpoints. Sub-abas: Threads & Variables, Console, Debug Output, Parallel Stacks, Memory, Modules, Tasks. Três colunas: threads | frames (void Program.Main…) | variáveis (args, order) com campo "Evaluate expression" embaixo. Linha atual com seta amarela e inlay hints de parâmetros; painel Monitoring à direita. |
| 05-debug-after-step-over.png | Após Step Over | Seta na linha 4, `order: Order` como inlay no fim da linha 3, "≤ 38 ms elapsed" na linha atual; variável `order` expansível. |
| 06-debug-console.png | Aba Console do Debug | Mostra o comando executado (caminho do binário) e a saída do programa. Obs.: contém "^[r" por um atalho que enviei por engano ao console. |
| 07-run-output.png | Tool window Run com saída | Aba "Run" + aba "OrderDesk.Cli"; toolbar vertical (Rerun, Stop, scroll, limpar); linha de comando, saída `Order A-104: R$ 42,00`, "Process finished with exit code 0."; toast "Build succeeded" no canto inferior direito. |
| 08-terminal.png | Tool window Terminal | Aba "Local" + "+" e "AI Agents ▾" à direita; prompt do shell no diretório do projeto. |
| 09-problems.png | Tool window Problems | Abas: File, All Solution Files, Toolset/Environment, Qodana; mensagem "No problems in Program.cs". |
| 10-git-log.png | Tool window Git > Log (com painel Commit à esquerda) | Painel Commit (esquerda): Changes / Unversioned Files, Amend, caixa de mensagem, botões Commit e Commit and Push…. Embaixo: filtros Branch/User/Date/Paths, lista de commits, painel de detalhes à direita. |
| 11-commit-diff.png | Diff de Commit (Program.cs) | Aba de editor "Commit: Program.cs"; toolbar com setas de diferença, "1 difference, 0 included", toggle side-by-side/unified; versão HEAD (hash `e0033f4d`) à esquerda, "Current version" à direita, linha adicionada em verde com checkbox. |
| 12-run-config-dropdown.png | Dropdown do seletor de run config (recorte do popup) | Linha da config com ▶, Debug e ⋮ ao lado; item "Edit Configurations…" abaixo. |
| 13-edit-run-configurations.png | Diálogo Run/Debug Configurations | Lista à esquerda (".NET Project › OrderDesk.Cli", toolbar +/−/copiar/pasta/ordenar); à direita Name, "Allow multiple instances", "Store as project file"; abas Configuration/Logs; campos Project, Target framework, Exe path, Program arguments, Working directory, Env vars, .NET runtime arguments, Runtime, Terminal mode, etc. Rodapé: Run ▾, Cancel, Apply, OK. |
| 14-settings-keymap.png | Settings > Keymap | Busca no topo da lista; dropdown de keymap ("Visual Studio (macOS)"), árvore de ações (Editor Actions, Main Menu, Tool Windows…), campo de busca de atalhos, aviso de conflito com atalhos do macOS, Cancel/Save. |
| 15-editor-context-menu.png | Menu de contexto do editor | Show Context Actions, Paste, Copy/Paste Special, Column Selection Mode, Go To, Folding, Run/Debug/Cover/Profile Unit Test, Inspect, Refactor, Start Debugging and Run to Here, Open In, Local History, Git, Quick Definition, Compare with Clipboard — com atalhos alinhados à direita. |
| 16-search-everywhere.png | Search Everywhere | Popup centralizado sobre o editor: abas All/Classes/Files/Symbols/Actions/Text, "Include non-solution items", campo de busca, resultados com tipo/projeto à direita e **preview do arquivo** abaixo da lista. |
| 17-rename-inline.png | Rename inline | Identificador selecionado com caixa, todas as ocorrências destacadas, popup pequeno abaixo com o novo nome (`order1`) e menu ⋮ de opções. |
| 18-build-tool-window.png | Tool window Build | Aba "Build Output"; toolbar com martelo, setas, busca, contadores de erros (0) e avisos (0); "Build finished successfully" à esquerda e log MSBuild à direita. |
| 19-nuget.png | Tool window NuGet | Abas Packages/Sources/Folders/Log; busca, escopo "Solution", fonte "nuget.org", "Prerelease"; Installed/Discover; seções "Packages to Update" e "Installed Packages"; detalhes à direita. |
| 20-unit-tests.png | Tool window Tests (Explorer) | Árvore tests › OrderDesk.Tests › OrderTests (4 tests); toolbar de run/debug/cover à esquerda e filtro no topo. |
| 20-unit-tests-run.png | Tests após Run All | Aba "All tests from Solution"; contadores 4 / ✓2 / ✖1 / skip 1 no topo; árvore com falha (1 teste falhou, existente na fixture) e painel de detalhes à direita. |
| 21-solution-explorer.png | Painel Solution + Tests | Painel Solution com toolbar (sincronizar, busca, locate, expandir/recolher, ⋮, ocultar). |

Não capturado: diálogo Search Everywhere com ações; sessão de Debug com variáveis expandidas; Terminal com comando; Build com erro; Edit Configurations com outro tipo de config.
