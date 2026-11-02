# QA bug hunt — 2026-11-02

Branch `qa/bug-hunt` (a partir de `origin/main`, versão 0.3.0-alpha.3). Nenhum código do produto foi alterado.

## Resumo

- Fluxo principal funciona de ponta a ponta no navegador contra o host real: abrir, confiar, avaliar, build, run, debug com breakpoint, testes, terminal, Find Usages e Go to Definition (depois de iniciar a análise).
- O maior atrito para um usuário novo: a análise (csharp-ls) não inicia sozinha, nem em sessões seguintes; o caminho está escondido no ícone de escudo "Workspace tools".
- Quatro itens altos: análise manual, erros sem sublinhado no editor, flag "modified" que não some depois de undo (bloqueia Build/Run com diálogo) e projeto com avaliação falha sem arquivos na árvore.
- A busca tem duplicatas, ranking fraco e `Cmd+Shift+F` (Find in Files) não faz nada.
- Não foi possível testar o app Tauri real (sem permissão de acessibilidade para o `agent-desktop`); tudo abaixo foi visto no navegador via `host:http` + Vite.

## Como foi testado

- `bun run host:http` (porta 4317, `OPENRIDR_HOME=.scratch/host-home`) e `bun run dev` (1420), dirigidos com Playwright (Chromium, 1440×900).
- Fixtures: `OrderDesk` (principal), `BrokenImport`, `MissingSdk`. Solução real: `~/Projetos/gabagool-dotnet/Gabagool.sln`, aberta **só em Restricted mode** para não gerar `obj/`/`bin/` fora do worktree.
- A única edição feita nas fixtures foi em `OrderDesk/.../Program.cs` (para provocar erros de build), revertida com `git checkout`. O csharp-ls 0.28.0 foi instalado dentro de `.scratch/host-home` (ignorado pelo git).
- Screenshots em `.scratch/qa-shots/` (só as citadas abaixo foram mantidas).

## Achados (por severidade)

### Alto

**1. A análise não inicia sozinha; completion, Go to Definition, Find Usages e Rename ficam indisponíveis até um caminho escondido** — experiência
- Passos: abrir OrderDesk, confiar. `Cmd+B` em `Order`.
- Esperado: navegação funcionando, ou um botão claro "Iniciar análise" ao lado do aviso.
- Obtido: barra de status diz "Definition unavailable: Analysis is not running. Start analysis to request editor features." sem ação. O botão está em escudo (canto direito) → "Workspace tools" → card "Analysis · csharp-ls" → "Start analysis". Antes, o csharp-ls aparece "Not installed" e exige "Review install plan" → "Confirm and install". Reabrindo um projeto já confiado a análise volta a "Not running" (Ctrl+Space: "Completion unavailable: Analysis is not running"). Depois de iniciar, tudo funcionou (Go to Definition levou ao construtor, Find Usages achou 4 referências, completion listou 6 itens).
- Os botões de editor "Rename symbol…" e "Code actions…" têm só ícone e o tooltip diz "Start analysis before…".
- Screenshots: `09-gotodef-trusted.png`, `10-workspace-tools.png`, `13-installed.png`.

**2. Erros de compilação não aparecem no editor (sem sublinhado nem marcador na margem)** — bug/experiência
- Passos: em `Program.cs` digitar `int x = "a"; Foo();`, salvar, aguardar.
- Esperado: sublinhado vermelho e marcador na margem.
- Obtido: o painel Problems lista os dois erros (CS0029 5:9, CS0103 5:14), mas o editor não tem nenhum elemento `.cm-lintRange`/`.cm-lint-marker` (contagem 0). A barra de status também não mostra contagem de erros. O clique no item do Problems navega corretamente.
- Screenshot: `33-problems.png` (o editor aparece acima do painel, sem marcas).

**3. Flag "modified" não volta ao limpo depois de undo; Build e Run passam a pedir confirmação a cada vez** — bug
- Passos: abrir `Order.cs`, digitar `x`, `Cmd+Z`.
- Esperado: conteúdo igual ao do disco, aba sem `*`.
- Obtido: aba continua `Order.cs*`, status mostra "1 modified buffer". Consequências observadas: Build abre "Build with unsaved buffers?"; Run abre "Run the saved build with unsaved buffers?"; fechar a aba abre "Close modified file?"; recarregar a página dispara `beforeunload`.
- Causa provável: o estado "dirty" é um booleano ligado ao evento de edição, não comparado ao conteúdo salvo/versão limpa do histórico.
- Screenshots: `50-dirty-after-undo.png`, `30-run.png`, `36-modal-after-save.png`.

**4. Projeto com avaliação falha (BrokenImport) fica sem arquivos na árvore e sem erro no Problems** — bug
- Passos: abrir `fixtures/BrokenImport`, confiar.
- Esperado: pelo menos `Program.cs` e `BrokenImport.csproj` navegáveis; erro MSB4019 também em Problems.
- Obtido: o nó do projeto só mostra a mensagem vermelha. Com "Show All Files" aparecem apenas `obj` e `.gitignore`; `Program.cs`, `global.json` e o `.csproj` **não** aparecem (a busca por "Program" na aba Files os encontra). O `.csproj` só abre por "Edit ›" → "Edit 'BrokenImport.csproj'" no menu de contexto. O painel Problems diz "0 errors" (o MSB4019 só existe na árvore). O botão Build fica desabilitado com tooltip "Evaluate the project model before building", embora a avaliação tenha rodado e falhado.
- Screenshots: `53-brokenimport.png`, `57-broken-problems.png`.

### Médio

**5. `Cmd+Shift+F` (Find in Files) não faz nada; "Find in Path" é outra coisa** — bug/experiência
- `Cmd+Shift+F` no editor: nenhuma resposta; o texto digitado em seguida cai no buffer (virou `TotalOrder(...)` no teste). O atalho de "Find in Path" é `⌥⇧⌘E` (Actions → "find"), que só abre o Search Everywhere na aba Text, sem escopo, máscara, caso ou regex.
- O menu de contexto "Find in Files…" (arquivo, solução) também só abre o Search Everywhere global, **ignora o escopo** (pasta/arquivo clicado) e reabre com a consulta anterior ("files").
- Screenshots: `20-find-in-path.png`, `24-find-in-files-ctx.png`, `19-actions-find.png`.

**6. Resultados duplicados na aba Text / All** — bug
- Passos: `Cmd+T`, buscar `Order` em OrderDesk (43 resultados) ou `Gamma` em Gabagool.
- Obtido: a mesma linha aparece 2–3 vezes (`OrderDesk.sln:8` ×3, `Program.cs:3` ×2, `DashboardMonitor.cs:19`, `:31`, `:42` ×2).
- Causa provável: um resultado por ocorrência do termo na linha (a linha do .sln tem "Order" 3 vezes; `IGammaClient _gamma` tem "Gamma" 2 vezes), sem agrupar por linha.
- Screenshot: `17-search-everywhere.png`.

**7. Search Everywhere: ranking, abas e estado da consulta** — experiência
- Busca `Order` em All: `OrderDesk.sln`, `OrderDesk.Cli.csproj` e `Program.cs` (casa só pelo diretório) vêm antes de `Order.cs` (nome exato). Resultados TEXT inundam a lista.
- Aba Symbols dá "No results — Try Classes" para `Order`, enquanto a aba All mostra a mesma classe com o rótulo "SYMBOL".
- Ao reabrir, a consulta anterior não fica selecionada: digitar anexa (`Orderfind`). A aba também fica na última usada (Text), não em All.

**8. Painel "Workspace tools" estoura horizontalmente** — bug
- Abrir o escudo → "Review install plan". O `aside.tools-drawer` mede 639 px com `scrollWidth` 2457 px; linhas longas (comando, caminhos, texto do debug adapter) não quebram e ficam cortadas ("Scope: Solution · OrderDes…", "The opt-in … neither inst…"). Ao abrir o plano o painel rola sozinho para a direita (`scrollLeft` 1806) e o usuário vê só o fim das linhas.
- Screenshots: `10-workspace-tools.png`, `12-install-plan-left.png`.

**9. Find Usages sobrepõe o tool window de baixo e o banner empurra a UI** — bug
- Com o painel Build aberto, `Shift+F12`: o painel Find Usages cobre o conteúdo do Build (a faixa inferior mede 270 px, mas só ~95 px ficam visíveis). O banner de status no topo do editor empurra as abas ~46 px para baixo a cada mensagem.
- Screenshot: `29-usages-with-bottom.png`.

**10. Aba Terminal não recebe foco** — experiência
- Abrir a aba e digitar não mostra nada; as teclas se perdem. Só depois de clicar dentro do terminal ele responde.
- Screenshot: `46-terminal.png`.

**11. NuGet: "Current" aparece como `unknown`; só lista e atualiza** — bug/experiência
- Em `OrderDesk.Tests` os 3 pacotes mostram Current `unknown` (Latest 18.11.0 / 2.9.3). `dotnet list package --format json` (rodado à parte) devolve `resolvedVersion` 17.11.1 / 2.9.2 / 2.8.2. Não há busca nem instalação de pacote novo (só "Update…"). Em `OrderDesk.Cli`: "No packages are referenced by this project".
- Screenshot: `42-nuget-tests.png`.

**12. Menu do projeto oferece Run/Debug/Build para bibliotecas, mas todos vêm desabilitados** — experiência
- Botão direito em `OrderDesk.Core`: "Build Selected Projects", "Run 'OrderDesk.Core'", "Debug 'OrderDesk.Core'" desabilitados com tooltip "Only evaluated console projects can be built, run or debugged." (não dá para compilar só uma biblioteca ou o projeto de testes pela árvore). "Run Unit Test" também desabilitado.
- O menu de arquivo só tem Open / Copy Path / Find in Files (sem New, Rename, Delete, Reveal in Finder).
- Screenshot: `22-ctx-project.png`.

**13. MissingSdk termina em beco sem saída** — experiência
- Abrir `fixtures/MissingSdk` e confiar: "Evaluate project model" fica desabilitado com tooltip "Trust the workspace and resolve the project SDK before evaluating." (já está Trusted). A mensagem do painel diz "Evaluation needs a resolved project SDK. Host SDK 10 is never substituted." e a barra de status mostra "Project SDK missing (8.0.999)". Não há link para Toolchain, nem texto dizendo qual SDK instalar nem que o `global.json` pina `8.0.999`.
- Screenshot: `58-missingsdk.png`.

**14. Buffer limpo não recarrega quando o arquivo muda no disco** — bug (visto no dev bridge)
- Com `Program.cs` salvo e aberto, `git checkout` do arquivo por fora. Após ~10 s e após trocar de aba o editor continuou com 6 linhas (o disco tem 4), sem aviso. Não verificado no Tauri (pode haver watcher/foco da janela que o navegador não dispara).

**15. Janela pequena: o editor some** — experiência
- Em 800×520 o editor fica com ~60 px de altura porque a faixa inferior mantém 270 px; em 560×420 a barra de abas do editor corta o nome do arquivo.
- Screenshot: `62-small-window.png`.

**16. Duplo Shift abre o Search Everywhere durante seleção com teclado** — bug
- `Shift+↑` seguido de `Shift+Home` em sequência rápida (soltando o Shift entre os dois) abriu o Search Everywhere e roubou o foco (o Backspace e o `Cmd+S` seguintes caíram no diálogo). Com 300 ms entre os dois não disparou. Teclas sintéticas do Playwright são mais rápidas que um humano; reteste com digitação real.
- Screenshot: `36-modal-after-save.png`.

### Baixo

**17. Mensagens com jargão interno e mensagens velhas**
- Restricted: "Definition unavailable: lsp.definition requires workspace trust; no process was started." (sem ação). A mensagem continuou na barra depois de confiar o projeto até a próxima ação.
- Problems: "coverage full", "Analyzer coverage: not verified", "Only this covered document/publication is claimed; the solution is not claimed clean", "revision 2 Current".
- Estado vazio do editor, já Trusted: "Analysis and execution require trust."
- Gabagool (aberto só em Restricted, nunca confiado) mostra no Problems "Trust was revoked; analysis is unavailable…".
- Show All Files: pastas `bin`/`obj` com sufixo "no index".
- Screenshots: `06-gotodef-restricted.png`, `57-broken-problems.png`, `47-show-all.png`.

**18. Dois fluxos diferentes para confiar** — diálogo da abertura: um clique em "Trust project"; status bar "Restricted mode": caixa "I trust this repository to execute code" + "Trust and inspect project".

**19. Atalhos Rider ausentes** — `Cmd+E` (recentes), `Cmd+Shift+O` (Go to file) e `Cmd+L` (Go to line) não fazem nada. Com `Keymap: IntelliJ / Rider` e `Rider (macOS)` na tela.

**20. Tela inicial**
- `↑ ↓` / `Enter` só funcionam depois de focar a busca (`/`); com o foco no body, `ArrowDown`+`Enter` não abre nada, apesar da dica na tela.
- O botão "Open" chama `osascript choose file` filtrando `sln/slnx/csproj`: não aceita pasta, embora a tela diga "Open a .sln, .csproj or folder" e "⌘O open folder". No dev bridge o host é de uma thread só e fica travado (health sem resposta) enquanto o diálogo nativo está aberto; é preciso fechar o diálogo.
- Screenshots: `64-welcome-recents.png`, `03-open-orderdesk.png`.

**20b. Debug** — botões de passo só com ícone; `F10` não passa a linha (a keymap IntelliJ usa `F8`; confirmar se o hint diz isso); o painel Debug precisa de rolagem na altura padrão (Watches fora da tela); em build falho a aba Debug abre e manda ver a aba Build. Screenshot: `40-debug-step.png`.

**21. Textos repetidos** — o plano do csharp-ls mostra "Executable and version verified…" duas vezes; no Tests, cada teste repete "display name: …" igual ao nome. O botão "Rebuild (Cmd+F9)" convive com o botão de toolbar "Build" (mesmo atalho).

**22. Painel Solution na largura padrão (280 px)** — toolbar do painel corta os últimos ícones e as linhas de projeto mostram "co…", "n…" (tipo e TFM). O redimensionamento funciona (280 → 420 px), então é só o padrão.

**23. Menores** — `favicon.ico` 404 no Vite; `PRODUCT.md` e `DESIGN.md`, citados no roteiro da sessão, não existem no repositório.

## O que funcionou (sem achado)

- Welcome → abrir por caminho (a pasta resolve para o `.sln`) → diálogo de confiança, `Esc` cancela, Restricted mode lê a estrutura como texto.
- Árvore com solution folders `src`/`tests`; Evaluate/Re-evaluate; Restore automático após confiar.
- Build ok e com erros (CS0029/CS0103 com caminho e linha), Run (saída `Order A-104: R$ 42,00`), Debug com breakpoint na margem, pausa, Locals, Stop; descoberta e execução de testes (2 passed, 1 failed, 1 skipped).
- Go to Definition (vai para o construtor), Find Usages (4 resultados corretos, clique navega), completion (6 itens), `Cmd+F` no editor, redimensionar a Solution.

## Não foi possível testar

- **App Tauri (`/Applications/OpenRidr.app`)**: o `agent-desktop` retornou `PERM_DENIED` (sem permissão de acessibilidade para o terminal). Seletor nativo de arquivos, atalhos do sistema, watcher de arquivos externo e janela real não foram verificados.
- Windows e macOS Intel (fora do alvo).
- Rename (`Shift+F6`) e Code actions: só vi o estado "Start analysis" desabilitado; não executei por causa do risco de reescrever as fixtures.
- Git panel e Commit tool window, Settings além de abrir o diálogo (Keymap/Editor/Build/Version Control não explorados), Light theme (indisponível por design).
- Solução real com análise/build: só aberta em Restricted para não criar `obj/`/`bin/` em `~/Projetos`. `OpenCode/BackendBaseProject.sln` e `golang/Dota2API/Dota2API.sln` não foram abertas.
- Show All Files com `.sln` na raiz: não aparece na árvore (observado), mas não verifiquei se há outra forma de abri-lo além da busca.
- Textos pt/en misturados: a UI inteira está em inglês nas telas visitadas; não encontrei texto em português.
- Desempenho em solução grande e foco por teclado em todos os diálogos (só testei Esc/Enter nos principais).
