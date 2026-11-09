# Critique de design do front-end — #328

Data: 2026-11-09. Issue-mãe: [#328](https://github.com/Heldinhow/openridr/issues/328).
Método: **dual-agent** (Impeccable 4.8.2, comando `critique`): Assessment A (design review) e
Assessment B (detector + overlay no navegador) em subagentes isolados, sintetizados aqui.
Modo da superfície: **Operate** (app de trabalho: escaneabilidade, consistência e expectativa
nativa antes de expressão). Tema avaliado: **escuro** (o único distribuído).

## Como foi capturado

App real em modo web, instância isolada desta worktree:

| Item | Valor |
|---|---|
| Host | `cargo run -p openridr-host-http -- --port 5617` |
| UI | Vite + `vite.e2e.config.ts` em `127.0.0.1:2620` |
| App home | `OPENRIDR_HOME=/tmp/openridr-328` (descartável, fora do repo) |
| Workspace | cópia de `fixtures/OrderDesk` em `/tmp/openridr-328/work` |
| Navegador | Chromium via Playwright, 1440×900 CSS px, `deviceScaleFactor: 2` |
| Estado | trusted, restore `ok`, evaluation `ok` (3 projetos), build `ok` (exit 0), tests `completed`, análise `partial` |

Todas as capturas estão em [`critique-328/`](critique-328/) (webp, 1440 px de largura). O app
desktop (Tauri/macOS) **não** foi usado: a crítica é de interface, e a superfície web é a mesma
árvore de componentes. O que depende do shell nativo ficou fora (ver "Não verificado").

## Placar de saúde do design

| # | Heurística | Nota | Problema principal |
|---|---|---:|---|
| 1 | Visibilidade do estado do sistema | 2 | Carregamento mostra "Evaluation stale" e "Re-evaluate" desabilitado: não distingue esperar de agir |
| 2 | Correspondência com o mundo real | 3 | Vocabulário .NET e Rider acerta; "executable inspection" é opaco na decisão de trust |
| 3 | Controle e liberdade do usuário | 3 | Trust oferece modo restrito e cancelar; build tem cancelar |
| 4 | Consistência e padrões | 2 | Rail inferior duplica abas sem rótulo; barras internas de cada painel divergem |
| 5 | Prevenção de erro | 2 | A ação de maior risco (trust) é a mais destacada, e o rótulo dela tem contraste baixo |
| 6 | Reconhecer em vez de lembrar | 2 | Rail só de ícones e "M/A/D" na direita exigem decoreba |
| 7 | Flexibilidade e eficiência | 3 | Atalhos visíveis, categorias de busca e foco de teclado apoiam quem já sabe |
| 8 | Design estético e minimalista | 2 | Tests e NuGet gastam a altura do painel em chrome antes do conteúdo |
| 9 | Reconhecer, diagnosticar e recuperar de erro | 2 | Caminho inválido diz o que falhou e não como consertar; análise parcial pede restart sem ação ao lado |
| 10 | Ajuda e documentação | 2 | Editor vazio ensina atalhos; setup/trust exigem conhecimento prévio do produto |
| **Total** | | **23/40** | **Fraco** (faixa 16–23), perto de "utilizável com atrito" |

O déficit principal é **explicar e recuperar estado**, não ornamento. A base visual (chrome neutro,
código central, hierarquia da árvore) está acima da média de um app interno.

## Especificidade do design

**Escrito para um IDE .NET — mas só intermitentemente.** A árvore solução/projeto/dependências, o
TFM no breadcrumb, o projeto de startup e a configuração, as fontes NuGet e as ressalvas de
cobertura da análise codificam o trabalho de verdade. Já a welcome genérica, os painéis flutuantes
arredondados, as fileiras de botões indiferenciadas e o rail nu de ícones poderiam ser transplantados
para outro app desktop sem mudar nada.

A identidade deveria vir de **tornar o modelo .NET e sua prontidão legível** — não de mais logo, cor
ou mímica do Rider. É exatamente o que os achados P1 cobram.

## Scanner determinístico

`impeccable detect --json src` → **exit 2, 1 achado**: `overused-font` (Inter) em `src/styles.css:35`.
É heurística de distinção, não defeito de usabilidade: num IDE, uma face familiar e legível é a
escolha certa. **Ruído** para este critique.

O overlay no navegador (injeção de `detect.js` em 4 estados) reportou:

| Estado | Achados | Detalhe |
|---|---|---|
| Árvore do workspace | 0 | — |
| Editor | 5 | 4 `low-contrast` + 1 `nested-cards` |
| Settings | 4 | 2 `low-contrast` (um deles **branco sobre `--accent` = 3,3:1**), 1 `flat-type-hierarchy`, 1 `nested-cards` |
| Welcome | 0 | — |

O que o detector pegou e a revisão humana não tinha medido:

- **`#6f737a` sobre `#1e1f22` = 3,5:1** e sobre `#26282c` = 3,1:1 — números de linha/régua do editor.
  Texto de interface abaixo de 4,5:1; números de linha são isentos de WCAG (são decoração repetida do
  conteúdo), mas a 13 px eles somem no tema escuro.
- **`#5fb865` sobre `#2e436e` = 4,0:1** e `#9d6afb` sobre `#263552` = 3,4:1 — cores de sintaxe/git
  sobre a linha selecionada. Aqui a isenção não vale: é código, e o usuário precisa lê-lo.
- **`flat-type-hierarchy` no Settings** (corpo 14 px, h2 15 px, h3 16,4 px): o próprio detector ressalva
  que o peso separa os níveis. Fica como observação, não como defeito.

Falsos positivos declarados: `overused-font`; os dois `nested-cards` (painel dentro de painel é a
metáfora certa de um IDE, não um card decorativo).

## O que está funcionando

1. **O modelo de domínio é visível.** Solução, projetos, dependências e arquivos C# se agrupam numa
   hierarquia compreensível, e o `net8.0` no breadcrumb liga o código ao alvo de compilação
   ([07-editor](critique-328/07-editor.webp), [07c-breadcrumb](critique-328/07c-breadcrumb.webp)).
2. **Trust tem uma alternativa real.** Modo restrito, caminho explícito e escolha reversível — em vez
   de um binário "confie ou saia" ([04b-trust-dialog](critique-328/04b-trust-dialog.webp)).
3. **O trabalho continua central.** Chrome neutro, sintaxe legível e faixa de atalhos mantêm o editor
   como protagonista; sucesso e cancelamento de build são ditos em texto, não só em cor
   ([16-build-running](critique-328/16-build-running.webp), [17-build-done](critique-328/17-build-done.webp)).

## Achados priorizados

### [P1] A mensagem de estado transforma a prontidão inicial em beco sem saída

- **O que:** [05-workspace-loading](critique-328/05-workspace-loading.webp) mostra "Evaluation stale",
  "Re-evaluate" desabilitado, build/run desabilitados e, ao mesmo tempo, mensagens de "starting" e
  "partial". [09-panel-problems](critique-328/09-panel-problems.webp) manda "restart analysis" sem
  nenhuma ação de restart ao lado. É um retrato temporal — não prova que o processo trava.
- **Por que importa:** o usuário não distingue "espere" de "faça algo", e pode ler diagnóstico parcial
  como solução limpa ou como workspace quebrado.
- **Correção:** separar três estados — em andamento, stale acionável, parcial — em
  `src/WorkspaceView.tsx` (faixa de avaliação e status bar) e `src/ProblemsPanel.tsx`. Enquanto a
  operação roda, uma frase só: "Terminando restore e avaliando projetos…". "Re-evaluate" só aparece
  quando pode agir, com motivo quando desabilitado. Em Problems, ação "Restart analysis" adjacente à
  ressalva de cobertura, preservando o texto preciso e o comportamento do host.
- **Comando Impeccable:** `clarify`.

### [P1] Texto de ação primária reprova em contraste

- **O que:** `--accent: #548af7` com rótulo branco (`button.primary`, `src/styles.css:224-231`) mede
  **3,30:1**; no hover (`--accent-hover: #6d9bf9`) cai para **2,72:1**. Aparece em Open, Trust project
  e Save. Texto normal precisa de 4,5:1. Confirmado pelo overlay no Settings ("white on #548af7 3.3:1").
  Na mesma família: `--muted` sobre superfície de hover (painel + 8% branco) = 4,27:1 e `--accent` como
  texto sobre painel = 4,18:1.
- **Por que importa:** as ações mais importantes — inclusive a decisão de trust — são as mais difíceis
  de ler.
- **Correção:** token `--on-accent` (ex.: `#101114`) separado do `--accent`; manter o azul e verificar
  default e hover ≥ 4,5:1. Clarear `--muted` sobre hover e o azul quando usado como texto, sem mexer no
  azul de foco/seleção sem medir de novo.
- **Comando Impeccable:** `audit`.

### [P1] O chrome dos tool windows come a área de trabalho

- **O que:** a 1440×900, Tests ([10-panel-tests](critique-328/10-panel-tests.webp)) mostra controles de
  projeto/TFM, abas internas, filtros e rail de ações antes da lista — e a lista é cortada embaixo.
  NuGet ([15-panel-nuget](critique-328/15-panel-nuget.webp)) empilha abas, faixa vertical de ações,
  vários filtros, botões de modo e texto de SDK. A 1100×760
  ([23-narrow-1100](critique-328/23-narrow-1100.webp)) o Build quebra Run para a segunda linha e o log
  é cortado na borda.
- **Por que importa:** o trabalho do usuário é ler resultado de teste, pacote ou log — não operar a
  moldura. Em tela de laptop o conteúdo perde espaço exatamente quando é preciso.
- **Correção:** em `src/TestsPanel.tsx`, `src/NuGetPanel.tsx`, `src/BuildPanel.tsx` e no grid do
  workspace, uma linha primária de comando; Expand/Collapse/Export e fontes de pacote vão para um
  overflow rotulado ou disclosure. Reservar ≥ 120 px de lista visível a 1100×760 e testar overflow.
  Sem remover controles nem `role="tab"`.
- **Comando Impeccable:** `layout`.

### [P2] A decisão de trust privilegia permissão em vez de compreensão

- **O que:** "Trust authorizes executable inspection" é abstrato; "Trust project" é o botão azul
  dominante e o modo restrito é neutro
  ([04b-trust-dialog](critique-328/04b-trust-dialog.webp)). O foco inicial e a ativação por teclado não
  foram verificados.
- **Por que importa:** quem abre código desconhecido decide sobre segurança sem um modelo concreto do
  que o trust habilita, e o visual empurra para a opção mais permissiva.
- **Correção:** em `src/TrustPrompt.tsx`, trocar a frase abstrata por uma descrição curta e verificada
  no contrato do que pode executar código do projeto; manter caminho, escopo e reversibilidade; dar
  peso comparável às duas escolhas e rotular o modo restrito como o caminho seguro, sem prometer
  capacidade que ele não tem.
- **Comando Impeccable:** `clarify`.

### [P2] Foco e navegação são visualmente frágeis

- **O que:** o anel de foco em [22-focus-toolband](critique-328/22-focus-toolband.webp) aparece
  cortado nas laterais do rail; a 1100 px se repete. Há um outline global de 2 px (`src/styles.css:280`)
  e um override do rail em `:2823`. O rail é só ícone, e "M/A/D" do lado direito é o mais críptico.
- **Por que importa:** quem navega por teclado precisa de indicador completo e localizável; quem chega
  agora não deveria memorizar dois sistemas de navegação.
- **Correção:** em `src/shell/ToolWindowBand.tsx` e `src/styles.css`, foco **inset**
  (`outline-offset: -2px`) ou padding de rail que não corte; verificar cada item nas duas larguras;
  rótulos curtos persistentes onde couber e nome acessível em todo item só de ícone.
- **Comando Impeccable:** `harden`.

### [P2] Tokens fantasma e cores fora do token

- **O que:** `var(--danger, #e05555)` (`src/styles.css:1843`) e `var(--border, #3c3f41)` (`:1856`)
  apontam para custom properties que **não existem** em nenhum lugar: o fallback é sempre o valor
  efetivo. Fora do bloco de tokens há `#f75464`/`#e5b94e` (status de diagnóstico, `:1143-1144`),
  `#43454a` (linha selecionada), `#2e436e` (seleção com foco), `#a9c7ff` (outline de foco da árvore),
  `#2a1a1a`/`#16241a` (avisos), `#0b0b0b` (contraste do selo C#).
- **Por que importa:** trocar tema ou tom vira caça ao hex; e o par `--danger`/`--border` mente sobre o
  contrato — quem edita acha que existe.
- **Correção:** criar `--danger`/`--border` (ou apontar para `--error`/`--line`) e promover cada hex
  estrutural a token semântico; deixar hardcoded só o que é arte de ícone.
- **Comando Impeccable:** `polish`.

### [P3] Escala de tipografia e espaçamento fora do token

- **O que:** o grosso já usa `--fs-*` (118× `--fs-meta`, 32× `--fs-label`, 15× `--fs-badge`), mas sobram
  `font: 700 7px/1` (selo de símbolo, `src/styles.css:3501`), `10px` em dois botões de ação
  (`:2655`, `:3231`), e one-offs de 15/16/20/22/24 px. Espaçamento não tem escala: 25 combinações
  distintas de `padding`, e raio tem 21× `3px` e 11× `4px` hardcoded ao lado dos tokens.
- **Por que importa:** 7 px é ilegível em qualquer tela; one-offs impedem qualquer ajuste global de
  densidade.
- **Correção:** piso de 11 px (só selo), trocar os one-offs pelos quatro papéis, e uma escala de
  espaçamento `--sp-1..6` (4/8/12/16/24/32) com raio sempre por token.
- **Comando Impeccable:** `typeset`.

### [P3] Microcopy de erro e de estado vazio

- **O que:** caminho inválido repete o caminho e não sugere reparo
  ([03-welcome-invalid-path](critique-328/03-welcome-invalid-path.webp)); o editor vazio lista "Save
  file" antes de existir arquivo ([06-workspace](critique-328/06-workspace.webp)); a welcome vazia
  ensina `↑ ↓ / Enter` com a lista vazia ([01-welcome-projects](critique-328/01-welcome-projects.webp));
  Settings diz "approved dark token set" (linguagem de implementação); Git diz "Not a git
  repository" sem próximo passo ([11-panel-git](critique-328/11-panel-git.webp)); o menu de contexto
  do editor repete explicações longas de itens desabilitados
  ([21-editor-context-menu](critique-328/21-editor-context-menu.webp)).
- **Por que importa:** cada uma dessas frases é o momento em que o usuário decide se o produto está
  quebrado ou se ele fez algo errado.
- **Correção:** uma linha de reparo em cada erro (sem esconder o erro real do host), estado vazio que
  propõe a próxima ação válida, e explicação de item desabilitado em `title`/`aria-description` em vez
  de parágrafo no menu.
- **Comando Impeccable:** `clarify`.

## Alertas por persona

- **Jordan, primeira vez:** encontra dois caminhos de abertura quase idênticos
  ([01](critique-328/01-welcome-projects.webp)), lê "executable inspection" e escolhe entre Trust e
  Restricted ([04](critique-328/04-trust-prompt.webp)), e cai num workspace que parece stale com o
  botão de recuperação desabilitado ([05](critique-328/05-workspace-loading.webp)). A cada passagem, a
  próxima ação segura fica menos clara que a anterior.
- **Sam, depende de acessibilidade:** tenta usar Open/Trust/Save com rótulo branco sobre azul a
  3,30:1, e tabula até o Build e vê o anel de foco cortado. Travessia por teclado, zoom e leitor de
  tela **não** foram testados — não afirme falha neles.
- **Dev .NET vindo do Rider:** reconhece a linguagem, espera confiar no estado de diagnóstico e
  compilar o projeto de startup; "TFM Not selected" persiste ao lado de um build bem-sucedido
  ([17](critique-328/17-build-done.webp)), e "Analysis: Partial coverage" com o pedido de restart em
  Problems não oferece recuperação local ([09](critique-328/09-panel-problems.webp)).

## Observações menores

- A welcome vazia oferece busca e campo de caminho antes de existir qualquer projeto recente; a
  primeira ação deveria ser uma só.
- `Settings` reserva uma área em branco grande; não é hierarquia proposital.
- O sucesso de build é explícito (exit 0 + log), mas fica enterrado na densidade da toolbar: falta um
  resumo de fim de tarefa.
- O rail direito tem três botões desabilitados ("Monitoring", "AI Assistant", "Database") que só
  existem para dizer o que o produto não é.
- `src/styles.css` tem overrides sucessivos que anulam regras anteriores (ex.: `:774` vs `:2823`,
  `:1352` vs `:2885`): conferir a cascata antes de qualquer ajuste visual.

## Perguntas para considerar

1. Dá para o workspace ter **uma** frase autoritativa de prontidão, com detalhe sob demanda, em vez de
   exigir que o usuário reconcilie restore, evaluation e analysis?
2. O que faz "0 erros" significar documento limpo sem implicar solução limpa quando a cobertura é
   parcial?
3. Quais controles merecem espaço permanente depois que o usuário escolheu projeto de teste ou fonte
   de pacote uma vez?
4. A ação permissiva de trust é visualmente privilegiada porque é mais segura, ou só porque destrava o
   caminho feliz?
5. Se todo tooltip de ícone desaparecesse, um dev .NET novo ainda acharia a próxima ação sem chutar?

## Não verificado

- **App Tauri real (macOS ARM64)**: nenhuma captura do shell nativo (menu, atalhos do sistema, updater,
  seletor nativo, árvore de acessibilidade do WebKit). A crítica é da interface web.
- **Tema claro**: não é distribuído; os tokens claros existem só documentados e não foram avaliados.
- **Estados que exigiriam falha real**: build com erro, restore falhando, análise quebrada, conflito de
  disco, diálogo de saída com buffer sujo. Não foram capturados — a experiência de recuperação deles
  não foi pontuada.
- **Interação**: hover (só a toolbar), arrasto de splitter, foco por teclado em todos os painéis,
  leitor de tela, zoom de UI e reflow.
- **Windows e macOS Intel**: não executados, como no resto do repositório.

## Índice das capturas

| Arquivo | Tela |
|---|---|
| `01-welcome-projects` | Welcome / Projects, lista de recentes vazia |
| `02-welcome-toolchain` | Welcome / Toolchain |
| `03-welcome-invalid-path` | Erro de caminho no formulário de abertura |
| `04-trust-prompt`, `04b-trust-dialog` | Modal de decisão de trust |
| `05-workspace-loading` | Workspace recém-aberto (restore/eval em voo) |
| `06-workspace`, `06b-toolbar`, `06c-statusbar` | Workspace sem arquivo aberto, toolbar e status bar |
| `07-editor`, `07b-tabbar`, `07c-breadcrumb` | Editor com arquivo C#, abas e breadcrumb |
| `08-panel-build` | Tool window Build |
| `09-panel-problems` | Tool window Problems |
| `10-panel-tests`, `10b-panel-tests-crop` | Tool window Tests (recorte do cabeçalho) |
| `11-panel-git` | Tool window Git |
| `12-panel-terminal` | Tool window Terminal |
| `13-panel-debug` | Tool window Debug |
| `14-panel-run` | Tool window Run |
| `15-panel-nuget`, `15b-panel-nuget-crop` | Tool window NuGet (recorte do cabeçalho) |
| `16-build-running`, `17-build-done` | Build em andamento e concluído (exit 0) |
| `18-settings`, `18b-settings-dialog` | Diálogo de Settings |
| `19-recent-files` | Paleta de arquivos recentes |
| `20-find-in-files`, `20b-search-everywhere` | Find in Files e Search Everywhere |
| `21-editor-context-menu` | Menu de contexto do editor |
| `22-focus-toolband` | Anel de foco no tool band |
| `23-narrow-1100` | Janela a 1100×760 |
| `24-toolbar-hover` | Hover em botão da toolbar |

## Próximo passo

As correções saem em issues filhas, uma por achado — ver a lista na issue-mãe #328. Os padrões que
esta crítica recomenda estão em [`DESIGN.md`](../../DESIGN.md) (proposta).
