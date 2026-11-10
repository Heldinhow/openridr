# Crítica do app — rodada 2 (features e visual)

Data: 2026-11-10. Frente: `app-critique-2`. Rodada anterior: [#328](critique-328.md)
(só visual, já implementada). Esta rodada cobre **o app inteiro, de novo, nas duas frentes** e
responde o que melhorou e o que ainda falta.

> **⚠️ DEGRADED: single-context (modelo sem visão + sem subagentes).**
> **Avaliação visual por medição, sem inspeção de imagem pelo modelo; capturas para revisão humana.**
> O modelo desta sessão não recebe imagens e a cota de review/research estava esgotada, então o
> visual foi avaliado por medição (contraste composto, alvo, overflow, anel de foco), leitura de
> fonte e o detector determinístico do Impeccable — não por julgamento estético pixel a pixel. As
> notas abaixo são de **medição**, não de nota visual humana; onde a conclusão depende de estética,
> o relatório diz.

## Como foi executado

App real desta worktree, instância isolada, em Linux ARM64 (Hyprland):

| Item | Valor |
|---|---|
| Binário | `bunx tauri build --debug --no-bundle` → `src-tauri/target/debug/openridr` |
| App home | `OPENRIDR_HOME=/tmp/openridr-critique-2/home` (descartável) |
| Workspace | cópia de `fixtures/OrderDesk` em `/tmp/openridr-critique-2/work` |
| Janela | 1440×900, workspace 7, por PID; input/captura sob `flock /tmp/openridr-gui.lock` |
| Adaptador | `OPENRIDR_BUNDLED_NETCOREDBG=src-tauri/resources/netcoredbg` (o `netcoredbg` do build Linux) |
| Superfície web | Vite `127.0.0.1:3220` + `openridr-host-http` `127.0.0.1:6217`, mesmo fixture, home isolada |
| Navegador | Chromium (Playwright), 1440×900 e 1100×760, `deviceScaleFactor: 2` |

O app desktop foi aberto pelo caminho real (`openridr <sln>` em argv, o mesmo do "abrir com"), com
trust, restore, avaliação e análise reais. A superfície web foi usada para **dirigir e medir** os
mesmos fluxos (mesma árvore de componentes, mesmo host): teclado sintético no WebKitGTK parou de
chegar ao webview depois do diálogo de trust, e não há ferramenta de ponteiro nesta máquina.

Detector: `impeccable detect --json src` → **exit 2, 1 achado** (`overused-font`, Inter,
`src/styles.css:35`). É a mesma heurística de distinção da rodada anterior: num IDE, uma face
familiar é a escolha certa. **Ruído** para esta crítica.

## Placar

| Frente | Nota | Leitura |
|---|---:|---|
| **Visual** | **7/10** | Base sólida e consistente; sobram cortes de texto, alvos pequenos e um painel fora do token. |
| **Fluxo (rodar/depurar/executar)** | **7/10** | Abrir → rodar em ~3 s; debug real funciona; os atritos são de estado e de rótulo, não de capacidade. |

Heurísticas de Nielsen (a mesma escala da #328, para comparar):

| # | Heurística | Nota | Problema principal |
|---|---|---:|---|
| 1 | Visibilidade do estado | 3 | A barra de status corta a prontidão (`Evaluated · De…`) |
| 2 | Correspondência com o mundo real | 3 | "TFM Not selected" com um TFM só; "from unknown to" no NuGet |
| 3 | Controle e liberdade | 3 | Descoberta automática bloqueia Run/Debug e manda cancelar em outro painel |
| 4 | Consistência e padrões | 3 | `git-panel.css` fora do token; keymap "Rider (macOS)" no Linux |
| 5 | Prevenção de erro | 3 | Restore explícito, trust e fechar buffer sujo seguem bons |
| 6 | Reconhecer em vez de lembrar | 3 | Rail só de ícone, mas abas rotuladas e atalhos no editor vazio |
| 7 | Flexibilidade e eficiência | 3 | Atalhos, paleta e Run na barra; falta saída no gate da descoberta |
| 8 | Estética e minimalismo | 3 | Chrome denso em Tests/NuGet; três botões mortos no rail |
| 9 | Recuperação de erro | 3 | Mensagens reais do host + reparo; "Restore failed" quando nada restaurou |
| 10 | Ajuda e documentação | 1 | O editor vazio ensina o errado em modo Trusted |
| **Total** | | **28/40** | **Bom** (faixa 24–31); era 23/40 na #328 |

## O que melhorou desde a #328

1. **Contraste deixou de ser o problema.** Em todas as telas medidas, o único texto abaixo de 4,5:1
   é de **controle desabilitado** (`--disabled` sobre `--chrome` = 4,3:1; sobre `--panel` = 4,0:1),
   que é isento. O rótulo da ação primária agora usa `--on-accent` (5,71:1; hover 6,93:1) — o achado
   P1 da #328 está fechado.
2. **Tokens de verdade.** `src/styles.css` tem **53 hex, todos dentro do `:root`**; nenhum fora do
   bloco. `--danger`/`--border` viraram `--error`/`--line`, e `#a9c7ff` virou
   `--focus-ring-strong`. O detector não achou hex solto.
3. **Foco visível e inset.** Tabulação real: os dez botões do rail inferior, as linhas da árvore e a
   toolbar mostram anel de 2 px `--accent`, com `outline-offset: -2px` no rail e na árvore — o
   corte lateral da #328 não existe mais ([61-focus-rail](critique-app-2/61-focus-rail.webp)).
4. **Piso de 11 px respeitado.** Nenhum texto abaixo de 11 px em nenhuma tela medida (welcome,
   trust, workspace, 8 painéis, editor, diálogos) — o `font: 700 7px` e os rótulos de 10 px saíram.
5. **Toolbar não estoura a 1100 px.** Todos os botões continuam visíveis e clicáveis; o "Run" não
   quebra mais para a segunda linha.
6. **Ícones de uma família só.** Material Icon Theme, pasta aberta/fechada por nome, sem selo de
   símbolo sangrando na linha — a árvore ficou mais calma.
7. **Estados com ação ao lado.** `Evaluation stale` agora traz **Re-evaluate** junto e o motivo; o
   restore tem cancelar; o build tem stop. O beco sem saída da #328 não se repete.
8. **Debug real no Linux.** O `netcoredbg` embutido pausa, mostra frames/locals, dá step e continua
   ([23-debug-paused](critique-app-2/23-debug-paused.webp)) — antes desta frente o Linux nem era
   considerável.

## Achados priorizados

### [P1] A barra de status corta a informação que o dev mais olha

- **O que:** a 1440 px, `status-evaluation` mede **68 px** para um conteúdo de 244 px
  ("Evaluated · Debug · TFM net8.0 · revision 2" vira "Evaluated · De…"). Ao lado, `status-analysis`
  ocupa 170 px com estado + botão + motivo ("Analysis: not running Start analysis Restore is not
  current."). O CSS dá `flex: 0 1 240px` à avaliação e `flex: 0 0 auto` à análise: quem cede é a
  prontidão. A 1100 px o corte é o mesmo (68/244).
- **Por que importa:** é o lugar onde o dev confirma "posso rodar?" antes de apertar Run. O item que
  cede é justamente o resumo; o que empurra é um motivo longo que ninguém lê na barra.
- **Correção:** prioridade invertida em `src/shell/StatusBar.tsx` + `src/styles.css` — avaliação com
  `min-width` que caiba o resumo (ou `flex: 0 0 auto`), análise reduzida ao estado com o motivo em
  `title`/popover; `footer.status` com 28 px reais (hoje o conteúdo tem 30 px e 3 px são cortados).
- **Evidência:** [60-status-1440](critique-app-2/60-status-1440.webp); medição em
  `60-status-1440.json` (larguras, `scrollWidth`, truncamento por item).

### [P1] "TFM Not selected" num projeto que tem um TFM só

- **O que:** Build, Tests e NuGet mostram um seletor de TFM com valor **"Not selected"** e uma única
  opção útil (`net8.0`). O build e o run funcionam com o campo vazio; o NuGet mostra "TFM not
  selected" no plano de instalação ([33b-nuget-plan](critique-app-2/33b-nuget-plan.webp)).
- **Por que importa:** para quem vem do Rider, "Not selected" lê como projeto não configurado, e o
  dev procura o que faltou selecionar. É um controle que pede uma decisão que não existe.
- **Correção:** quando o projeto avaliado tem um TFM, pré-selecioná-lo (e mostrar o valor, não o
  placeholder); com vários, manter o seletor. O gate continua sendo o do host.
- **Evidência:** `62-build-tfm.json` (`value: ""`, opções `["Not selected", "net8.0"]`),
  [08-panel-build](critique-app-2/08-panel-build.webp), [08-panel-tests](critique-app-2/08-panel-tests.webp).

### [P2] A primeira abertura bloqueia Run/Debug e manda cancelar o que não está na tela

- **O que:** ao abrir a solução, a descoberta automática de testes constrói o projeto de teste e o
  build; até terminar, Run e Debug ficam desabilitados com "Test discovery is building the test
  project; wait for it to finish or cancel the discovery." O controle de cancelar está no painel
  Tests, não ao lado da mensagem. Nesta máquina quente o gate durou ~1 s; no app desktop frio a mensagem ainda
  estava lá 3 s depois de abrir e sumiu antes dos 15 s
  ([desktop-03-workspace-settled](critique-app-2/desktop-03-workspace-settled.webp)).
- **Por que importa:** o primeiro gesto do dev é apertar Run. Ele vê o botão apagado, uma instrução
  de cancelar e nenhum lugar para cancelar ali.
- **Correção:** duas saídas cabem, e a recomendação é a primeira — **(a)** levar a ação para a
  mensagem (um "Cancel discovery" no painel que mostra o motivo, chamando o mesmo `tests.cancel` do
  painel Tests); **(b)** não disparar a descoberta na abertura, deixando-a para o primeiro uso de
  Tests. (a) mantém a descoberta automática e só devolve a saída; (b) encurta a abertura mas muda o
  comportamento que os specs de Tests esperam. Preservar o gate do host e o `tests-cancel`.
- **Evidência:** OCR do app desktop; `run-disabled-reason` no `08-*-overflow`/`05-loading`.

### [P2] O painel Git está fora do sistema de tokens

- **O que:** `src/git-panel.css` tem **16 hex fora do `:root`** e um botão de commit com azul próprio
  (`#386fda`) e `color: white`, diferente do `--accent` do resto do app. As cores de diff
  (`#263d61`, `#264b35`, `#423a3c`, `#5377ac`…) e as de status (`#78a9d1`, `#80bd88`, `#d27d7d`,
  `#d99578`) também são hex locais.
- **Por que importa:** é o único arquivo que ainda viola a regra "hex novo fora do bloco de tokens"
  do `DESIGN.md`; o Commit parece um botão de outro produto ao lado de Run/Debug.
- **Correção:** promover as cores de diff/status a tokens semânticos (`--diff-*`, `--git-*` já
  existem) e o botão a `--accent`/`--on-accent`.
- **Evidência:** `grep -n '#[0-9a-fA-F]\{6\}' src/git-panel.css`;
  [34-git-commit-tool](critique-app-2/34-git-commit-tool.webp).

### [P2] Microcopy que ensina o errado

- **Editor vazio:** "Text editing is always available. Analysis and execution require trust."
  aparece **também em modo Trusted** ([desktop-03-workspace-settled](critique-app-2/desktop-03-workspace-settled.webp)).
  Em `src/WorkspaceView.tsx:5237` a frase é incondicional.
- **NuGet:** "Updated Humanizer from **unknown** to 3.0.10" — o estado anterior é "not referenced",
  não "unknown".
- **Keymap:** a barra mostra "Keymap: Rider (macOS)" rodando no Linux; o perfil é o mesmo, mas o
  rótulo nomeia outra plataforma.
- **Por que importa:** cada uma dessas frases é o momento em que o usuário decide se o produto está
  quebrado ou se ele fez algo errado.
- **Correção:** condicionar a frase ao trust; trocar "unknown" por "not referenced"; rotular o
  keymap pelo perfil sem a plataforma (ou pela plataforma real).

### [P2] Alvos abaixo do piso de 32 px

- **O que:** os seis botões do cabeçalho da árvore medem **24×24** (`tree-show-all`, `tree-search`,
  `tree-locate`, `tree-expand-all`, `tree-collapse-all`, `tree-more`, `tree-minimize`), o
  "Re-evaluate" da barra de status mede **18×27**, e "Workspace actions" **26×32**. O `DESIGN.md`
  declara 32 px como mínimo (a linha da árvore é a exceção, não o cabeçalho).
- **Por que importa:** são os controles mais usados com mouse na árvore; 24 px exige mira.
- **Correção:** manter o desenho de 16 px e aumentar a área clicável (padding ou `min-height`), sem
  mudar a densidade da linha.

### [P3] Três botões desabilitados no rail direito

- **O que:** "Monitoring", "AI Assistant" e "Database" ocupam o rail direito, desabilitados, a 11 px
  e 4,3:1 (isento por serem desabilitados, mas visíveis). Não fazem nada e não têm plano no
  repositório.
- **Por que importa:** o rail é o lugar de maior densidade de ícones da tela; três deles só dizem o
  que o produto não é.
- **Correção (decidida):** remover os três. Placeholder desabilitado sem função contradiz a direção
  de IDE simplificada; voltar a ter qualquer um deles é decisão do Helder, não de uma sessão.

### [P3] Chrome do painel Tests

- **O que:** com a janela a 1440×900 e o painel inferior no tamanho padrão (320 px), o corpo tem
  282 px e o conteúdo do Tests, 641 px: abas, projeto/TFM, contadores, filtro e ações vêm todos antes
  da primeira linha da árvore. A lista mostra 4 testes.
- **Por que importa:** o trabalho é ler resultado de teste; a moldura come quase metade do painel.
- **Correção:** uma linha primária (projeto + TFM + Run) e o resto em disclosure rotulado, como a
  #331 fez nos outros painéis.

### [P3] Abrir um arquivo `.cs` pelo CLI mostra erro

- **O que:** `openridr /caminho/Program.cs` com um workspace aberto responde "Could not open …: Open
  a .sln, a .slnx, a .csproj or a directory that contains one." em vez de abrir o arquivo no
  workspace atual ([desktop-10-editor](critique-app-2/desktop-10-editor.webp)).
- **Por que importa:** é um caminho de "abrir com" que existe na CLI e não tem saída; o usuário fica
  com um erro e nenhuma ação.
- **Correção:** se o caminho é um arquivo de código dentro do workspace aberto, abrir a aba; senão,
  manter o erro atual com a ação "Open the containing solution".

## Fluxos executados (rodar / depurar / executar)

| Fluxo | Resultado | Passo e evidência |
|---|---|---|
| Abrir solução + trust | **funciona** | argv → trust (854 ms) → árvore (1,1 s) → Run habilitado (2,2 s, máquina quente); restore/eval/análise reais |
| Build implícito + Run | **funciona** | clique em Run → build + execução → `Order A-104: R$ 42,00` e "Process finished with exit code 0." em **2,6 s** ([21-run-done](critique-app-2/21-run-done.webp)) |
| Debug com breakpoint | **funciona** | Ctrl+F8 na linha 3 → Debug → **Paused**, frame `Program.<Main>$()`, `order = null`; step over → `order = {OrderDesk.Core.Order}`; continue → **Terminated** ([23-debug-paused](critique-app-2/23-debug-paused.webp), [24-debug-stepped](critique-app-2/24-debug-stepped.webp)) |
| Tests | **funciona** | descoberta 4, "Run" → **4 results · 2 passed · 1 failed · 1 skipped**, mensagem e stack do xUnit, exit 1 ([31-tests-done](critique-app-2/31-tests-done.webp)) |
| NuGet | **funciona** | busca "Humanizer" → 30 pacotes; plano com o comando exato (`dotnet add … --no-restore`); Install → exit 0; oferece Restore ([33-nuget-search](critique-app-2/33-nuget-search.webp), [33b-nuget-plan](critique-app-2/33b-nuget-plan.webp)) |
| Git | **funciona** | Commit tool com 1 arquivo, diff split com hash do HEAD, log de 1 commit ([34-git-commit-tool](critique-app-2/34-git-commit-tool.webp), [35-git-diff](critique-app-2/35-git-diff.webp)) |
| Settings | **funciona** | 5 seções, tema, zoom, densidade, editor; Salvar/Cancelar |
| Find in Files / Search Everywhere | **funciona** | busca no workspace com arquivo:linha; paleta com FILE/SYMBOL/ACTION ([37-find-in-files](critique-app-2/37-find-in-files.webp), [50-palette-program](critique-app-2/50-palette-program.webp)) |
| Fechar buffer sujo | **funciona** | "Save / Keep editing / Discard and close" com o caminho ([41-dirty-close](critique-app-2/41-dirty-close.webp)) |
| Restricted mode | **funciona** | gates explicam por que Run/Build/Debug não rodam; edição continua |
| Abrir `.cs` pelo CLI | **falha** | erro do host, sem ação (P3 acima) |

**Funciona mal:** Run/Debug na primeira abertura (gate da descoberta), "TFM Not selected", barra de
status truncada.

**Falta (pequeno, bloqueia rodar/depurar):** nada essencial. O único item é a ação de cancelar a
descoberta junto da mensagem que manda cancelá-la.

## Fora do escopo (simplificação)

O produto é uma IDE simplificada: abrir, rodar, depurar, testar. Estas ausências são **registro**,
não ticket:

- completion/refactor/análise profunda: o csharp-ls cobre o básico; não competir com o Rider.
- multi-target e multi-startup: um TFM, um startup.
- NuGet: update em lote, CPM editável, fontes editáveis.
- Git: grafo, paginação, rebase interativo.
- Tests: continuous testing e coverage (os dois botões existem desabilitados).
- terminal: perfis, split.
- Monitoring / AI Assistant / Database: não existem; os botões deveriam sair (P3).

## Não verificado

- **Julgamento estético pixel a pixel:** o modelo desta sessão não recebe imagens. O que está escrito
  acima vem de medição, fonte e detector; as capturas estão no repositório para revisão humana.
- **App Tauri no macOS (alvo verificado do produto):** nada do shell nativo (menu, atalhos do
  sistema, updater, seletor nativo, árvore de acessibilidade do WebKit, DMG). O que foi executado é
  o build Linux desta worktree.
- **Interação por ponteiro no app desktop:** não há ferramenta de ponteiro nesta máquina; teclado
  sintético chegou ao webview para abrir a paleta, mas não para os atalhos de execução. Os fluxos
  foram dirigidos e medidos na superfície web (mesma árvore de componentes e mesmo host).
- **Tema claro:** não é distribuído.
- **Leitor de tela, zoom de UI e reflow:** não testados.
- **Windows e macOS Intel:** não executados, como no resto do repositório.

## Índice das capturas

`docs/design/critique-app-2/` (webp, 1440 px de largura).

| Arquivo | Tela |
|---|---|
| `01-welcome-projects`, `02-welcome-toolchain`, `03-welcome-invalid-path` | Welcome, toolchain e erro de caminho |
| `04-trust-prompt` | Diálogo de trust |
| `05-workspace-loading`, `06-workspace-ready` | Workspace abrindo e pronto |
| `07-editor` | Editor com `Program.cs` |
| `08-panel-{build,debug,git,nuget,tests}` | Tool windows |
| `10-layout-1100x760-closed`, `10-layout-1440x900-tests` | Layout estreito e Tests a 1440 |
| `21-run-done` | Run concluído (exit 0) |
| `23-debug-paused`, `24-debug-stepped` | Debug pausado e depois do step |
| `30-tests-panel`, `31-tests-done` | Tests antes e depois de rodar |
| `33-nuget-search`, `33b-nuget-plan` | Busca e plano de instalação |
| `34-git-commit-tool`, `35-git-diff` | Commit tool e diff |
| `36-settings`, `41-dirty-close`, `42-run-config`, `43-keymap`, `44-context-menu` | Diálogos |
| `37-find-in-files`, `50-palette-program` | Busca e paleta |
| `60-status-1440`, `61-focus-rail` | Barra de status e anel de foco |
| `desktop-01-trust`, `desktop-03-workspace-settled`, `desktop-10-editor` | App Tauri real (Linux) |

## Tickets

Issue-mãe: [#364](https://github.com/Heldinhow/openridr/issues/364). Uma filha por achado, sem
dependência nova e sem bloqueio entre elas:

| Achado | Ticket | Tipo | Risco |
|---|---|---|---|
| Barra de status corta a prontidão | [#365](https://github.com/Heldinhow/openridr/issues/365) | bug | medium |
| "TFM Not selected" com um TFM só | [#366](https://github.com/Heldinhow/openridr/issues/366) | bug | medium |
| Run/Debug na 1ª abertura (descoberta automática) | [#367](https://github.com/Heldinhow/openridr/issues/367) | enhancement | medium |
| Painel Git fora do token | [#368](https://github.com/Heldinhow/openridr/issues/368) | enhancement | low |
| Microcopy que ensina o errado | [#369](https://github.com/Heldinhow/openridr/issues/369) | bug | medium |
| Alvos de 32 px nos cabeçalhos | [#370](https://github.com/Heldinhow/openridr/issues/370) | bug | low |
| Remover os três botões M/A/D | [#371](https://github.com/Heldinhow/openridr/issues/371) | enhancement | medium |
| Densidade do painel Tests | [#372](https://github.com/Heldinhow/openridr/issues/372) | enhancement | medium |
| `.cs` pelo CLI no workspace atual | [#373](https://github.com/Heldinhow/openridr/issues/373) | bug | medium |

O [#332](https://github.com/Heldinhow/openridr/issues/332) (trust prompt) continua aberto e **fora
desta leva**. O `DESIGN.md` continua sendo a referência de padrões; onde esta crítica propõe mudar
uma decisão registrada, a issue diz qual.
