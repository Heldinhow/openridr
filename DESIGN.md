# DESIGN.md — OpenRidr

> **Status: proposta.** Este documento não foi ratificado. Ele descreve o sistema visual que o
> código **já** implementa (fonte: `src/styles.css`, `src/rider-chrome.css`, `src/git-panel.css`) e
> propõe as correções que a crítica da [#328](docs/design/critique-328.md) recomenda. Cada regra
> marcada **proposto** ainda não está no código; cada uma marcada **hoje** é o que o app faz agora.
> Mudar este arquivo exige a mesma revisão de um PR de código.

## 1. Produto e modo

OpenRidr é um IDE C# para quem já trabalha com .NET e conhece o Rider. A superfície é **Operate**:
o usuário completa uma tarefa, então escaneabilidade, consistência e expectativa nativa valem mais
que expressão. A marca vive no detalhe preciso, não em decoração.

- **Tema:** escuro, único distribuído. Os tokens claros existem em `@supports (color: light-dark())`
  e são **forward-compatible**, não um tema suportado.
- **Plataforma verificada:** macOS ARM64. Windows e macOS Intel não são suportados sem evidência.
- **Densidade:** app de trabalho. 32 px é o mínimo de alvo para um controle; a árvore é a exceção declarada e usa **22 px por linha** com ícone de 16 px, a densidade do VS Code e do Rider. Itens de menu usam 28 px.

## 2. Princípios

1. **O código é o produto.** Chrome neutro, superfícies por alfa e linhas de 1 px existem para o
   editor ser o elemento mais claro da tela. Nada de cor decorativa competindo com sintaxe.
2. **O estado é dito em texto.** Cor reforça, nunca carrega sozinha um significado (sucesso, erro,
   parcial, stale). Todo estado tem uma frase e, quando ação existe, a ação ao lado.
3. **Uma ação primária por tela.** Se duas ações competem pelo mesmo peso, uma delas é secundária.
4. **Familiaridade ganha de novidade.** Vocabulário e atalhos do Rider, quando não custam nada.
   Onde o produto não alcança o Rider (cobertura de análise, debug), ele diz isso — não imita.
5. **Densidade não é licença para ilegibilidade.** Piso de 11 px, alvo de 32 px, contraste medido.

## 3. Tokens

Fonte única: `:root` em `src/styles.css`. **Nenhum componente inventa valor fora daqui.** A paleta escura inspirada no T3 Code vale para o app inteiro desde a #350: welcome, setup, trust, diálogos, menus, configurações e painéis leem os mesmos tokens do workspace.

### 3.1 Cor

| Papel | Token | Valor | Uso |
|---|---|---|---|
| Fundo da janela | `--bg` | `#090909` | canvas, editor |
| Painel | `--panel` | `#101010` | side, tool window |
| Superfície elevada | `--raised` | `#1b1b1b` | diálogo, popover, menu |
| Moldura | `--chrome` | `#050505` | toolbar, chrome, rail, status |
| Texto | `--fg` | `#f5f5f5` | corpo |
| Texto secundário | `--muted` | `#b3b3b3` | metadado, dica (4,5:1 também sobre hover e linha selecionada) |
| Texto desabilitado | `--disabled` | `#737373` | controle inativo |
| Ação | `--accent` | `#548af7` | foco, seleção, ação primária |
| Ação em hover | `--accent-hover` | `#6d9bf9` | hover da primária |
| **Sobre ação** | `--on-accent` | **hoje** `#101114` | rótulo sobre `--accent` (5,71:1; hover 6,93:1) |
| **Ação como texto** | `--accent-text` | **hoje** `#8ab2ff` | link, aba ativa, branch atual (6,51:1 sobre painel) |
| Superfície de hover | `--accent-surface` / `-strong` | branco 4% / 8% | hover de item neutro |
| Linha | `--line` / `--line-internal` / `--line-input` | branco 8% / 6% / 11% | separadores e borda de campo |
| Seleção | `--select` / `--select-row` / `--select-row-focus` | `#282828` / `#262626` / `#343434` | seleção de aba, linha da árvore, linha com foco |
| Erro / aviso / sucesso / info | `--error` `--warning` `--success` `--info` | `#f87171` `#fbbf24` `#34d399` `#60a5fa` | texto e ícone |
| Sólidos | `--error-solid` `--warning-solid` `--success-solid` `--info-solid` | `#ef4444` `#f59e0b` `#10b981` `#3b82f6` | preenchimento de selo |
| Diff | `--diff-add-surface` `--diff-del-surface` `--diff-mod-surface` `--diff-empty-surface` `--diff-intraline-surface` | `#2d4a35` `#5a2d33` `#263d61` `#27292e` `#5377ac` | linha adicionada/removida/modificada, linha vazia do split e trecho intraline |
| Conector do split | `--diff-connector-removed` `--diff-connector-added` `--diff-connector-modified` | `#7a3b44` `#2f6b44` `#3f5f8f` | conector entre as duas colunas do diff (#368) |
| Status de Git | `--git-modified` `--git-added` `--git-deleted` `--git-untracked` `--git-ignored` `--git-conflicted` | `#82c0f0` `#8ad07d` `#d27d7d` `#eda773` `#b3b3b3` `#f9a1a1` | nome e selo na árvore e no painel Commit |

A paleta é única desde a #350: não há tokens por superfície nem alias em `.workspace-view`. A aba ativa usa `--tab-active-surface` `#252525` com borda `#3a3a3a`. O welcome e o workspace são a mesma tela em estados diferentes e leem os mesmos valores.

Regras:

- **Superfícies sobem por alfa, não por hex novo.** Elevação = branco a 4%/8% sobre a superfície
  abaixo. Um hex de superfície novo é um bug de sistema.
- **Azul é ação, foco e seleção.** Não é decoração, não é estado de execução (run/debug são
  `--success`, stop é `--error`).
- **Cores de tipo e de git** (`--icon-*`, `--git-*`) são dados, não tema: carregam significado de
  linguagem/estado de arquivo e não se reaproveitam fora desse papel.
- **Proibido `var(--x, #hex)` para token que não existe.** Os dois que existiam (`--danger` e
  `--border`) viraram tokens reais; hex estrutural fora do `:root` é dívida, não estilo.

### 3.2 Tipografia

| Papel | Token | Valor | Uso |
|---|---|---|---|
| Título de página | `--fs-title` | 20 px | setup, estado vazio de tela cheia |
| UI | `--fs-ui` | 14 px | corpo de texto, editor de texto plano |
| Rótulo | `--fs-label` | 13 px | título de painel, cabeçalho de seção, código |
| Metadado | `--fs-meta` | 12 px | linha de árvore, tab, status, item de menu |
| Selo | `--fs-badge` | 11 px | badge, contador, tag |

- **Famílias:** `--sans` (Inter Variable, embutida) para UI e `--mono` (JetBrains Mono Variable,
  embutida) para código, terminal e números.
- **Piso:** 11 px. Nada abaixo disso, nem em selo. **Hoje:** o selo de símbolo saiu de 7 px, as
  duas rails de ação de 10 px viraram `--fs-badge`, e o selo `↗` da pasta `config` saiu de 9 px.
  Ficam abaixo do piso só `<text>` de SVG (arte de ícone) e o tamanho do editor, que é escolha do
  usuário.
- **Peso, não tamanho, separa níveis próximos.** Título de painel = 600 no `--fs-label`; não se cria
  um 15 px para "parecer título".
- **Números tabulares** em status bar, contadores e colunas numéricas (`font-variant-numeric:
  tabular-nums`). **Proposto.**

### 3.3 Espaçamento

**Hoje** — a escala é 4 px e vive em `:root`. Um valor que cai num passo usa o token; valores densos
fora da escala (1/2/3/6/10 px) ficam crus de propósito, para migrar a escala não mudar a densidade:

| Token | Valor | Uso |
|---|---|---|
| `--sp-1` | 4 px | dentro de um controle (ícone–texto) |
| `--sp-2` | 8 px | padding de item denso, gap entre ícones |
| `--sp-3` | 12 px | padding de painel, gap entre grupos |
| `--sp-4` | 16 px | separação de seções |
| `--sp-5` | 24 px | bloco de conteúdo (diálogo, estado vazio) |
| `--sp-6` | 32 px | margem de tela cheia |

### 3.4 Raio, altura, elevação, movimento

| Token | Valor | Uso |
|---|---|---|
| `--r-sm` | 4 px | selo, `kbd`, marca inline |
| `--r-md` | 6 px | botão de ícone, item de menu, linha de árvore |
| `--r-lg` | 8 px | botão, campo, ilha de painel |
| `--r-xl` | 12 px | diálogo, popover, Search Everywhere |
| `--r-pill` | 999 px | selo arredondado, contador |
| `--control-h` | 32 px | botão, campo, alvo de tool band (a linha da árvore é exceção: 22 px) |
| — | 28 px | item de menu, linha de log (**proposto** virar `--row-h-menu`) |
| — | 52 px | toolbar superior |
| — | 32 px | status bar (o alvo de 32 px vale para os controles dela; o separador é uma linha inset, não uma borda) |
| `--shadow-popover` | `0 16px 40px -18px rgba(0,0,0,.8)` | popover, menu |
| `--shadow-dialog` | `0 24px 72px -20px rgba(0,0,0,.9)` | diálogo |
| `--highlight` | `inset 0 1px rgba(255,255,255,.06)` | aresta superior de superfície elevada |

- **Movimento:** 120 ms para cor, 150 ms para transform. Só `background-color` e `transform` — nada
  que reflua layout. `prefers-reduced-motion` zera tudo (já implementado).
- **Sem animação de entrada em painel.** Tool window aparece; não desliza.

### 3.5 Foco

- Anel de **2 px** `--accent` com `outline-offset: 2px` no padrão.
- Em linha densa (árvore, menu, item de rail) o anel é **inset** (`outline-offset: -2px`) para não
  ser cortado pelo scroller. **Hoje** no tool band: o rail tem 34 px e rola, então o anel global
  caía fora; inset ele mede 5,38:1 sobre o rail e 4,34:1 sobre o botão ativo
  ([evidência](docs/evidence/design-focus-333.md)).
- Foco nunca é só preenchimento: quem usa teclado precisa ver a borda.

## 4. Regras de componente

### Botão

- **Primário** (`.primary`): uma por tela, `--accent` de fundo, rótulo `--on-accent` (**hoje**;
  5,71:1 no default e 6,93:1 no hover).
- **Secundário**: fundo `--accent-surface`, borda `--line`, texto `--fg`. É o padrão.
- **Destrutivo** (`.danger`): fundo neutro, texto `--error`. Nunca é o controle mais alto da tela.
- **Ícone**: 32×32, sem borda, hover por alfa, `aria-label` obrigatório.
- Altura mínima 32 px; `:active` escala 0,97; desabilitado perde fundo e ganha `--disabled`.

### Campo

- Altura 32 px, fundo `--bg`, borda `--line-input`, raio `--r-lg`.
- Foco: borda `--accent` + halo `0 0 0 3px rgba(84,138,247,.24)`.
- Rótulo sempre presente — visível ou `sr-only`; placeholder não substitui rótulo.
- Erro fica **ao lado do campo**, em uma linha, com o texto real do host e uma instrução de reparo
  (**hoje**: o caminho inválido diz para conferir o caminho e o arquivo).

### Abas e tool windows

- Cabeçalho inferior mantém `role="tab"` (contrato de teste e de acessibilidade).
- Aba ativa = fundo do editor, texto `--fg`; inativa = `--muted`; hover = `--accent-surface`.
- Rail e abas são **o mesmo destino**: o rail é atalho, a aba é o estado. Todo item de rail tem
  `aria-label` e `title`. Rótulo persistente não cabe nos 34 px do rail sem alargar a coluna do grid
  e roubar largura do editor; a alternativa (rótulo sob o ícone) dobraria a altura de um rail que já
  rola a 760 px. Fica o nome acessível, e o cabeçalho inferior — mesmo destino — carrega o rótulo.
- O chrome de cada painel é **uma** linha de comando primário; ações secundárias vão para um
  disclosure rotulado (`panel-disclosure`: **More** em Tests, **Filters** em NuGet).

### Árvore

- Linha de 22 px com ícone de 16 px, recuo de 18 px por nível, seleção por preenchimento (nunca preenchimento + negrito ao mesmo tempo).
- Tipo é carregado pelo desenho do **Material Icon Theme**, não por chip extra e não por cor de token:
  o SVG traz o próprio `fill` e nenhum estado da linha o tinge. Pasta também tem desenho, escolhido
  pelo nome dela, em duas variantes (fechada e aberta).
- Cabeçalho e faixa de avaliação ficam fixos; só a árvore rola.
- Metadado de projeto em segunda camada (nome em cima; `kind · TFM · restore` embaixo).

### Diálogo e menu

- Diálogo: `--raised`, raio `--r-xl`, `--shadow-dialog`, largura fixa por tipo (Settings não muda de
  tamanho entre páginas; a página rola dentro).
- Menu/popover: `--raised`, raio `--r-xl`, `--shadow-popover`, item de 28 px com raio `--r-md`.
- Item desabilitado explica o motivo em `title`/`aria-description`, **não** em parágrafo dentro do
  menu (**hoje**).
- Esc fecha; Enter aciona o botão focado; Tab circula dentro do modal.

### Estado vazio, carregando, erro

- **Vazio:** uma frase que diz o que fazer a seguir, não o que não existe. Nada de atalho de teclado
  para uma lista vazia.
- **Carregando:** uma frase no presente do indicativo ("Terminando restore e avaliando projetos…") e
  nenhuma ação habilitada que ainda não pode agir. Botão desabilitado mostra o motivo.
- **Parcial:** diz o que **não** foi coberto e oferece a ação de recuperação **ao lado** — nunca a
  instrução sem o controle.
- **Erro:** o texto real do host, sem tradução que perca informação, mais uma linha de reparo.
- Status e notices usam `role="status"` (educado) para progresso e `role="alert"` para falha.

### Selo, tag, dica

- Selo: raio `--r-sm`, `--fs-badge`, cor sólida só quando o estado é binário e importante.
- Status de git na árvore: a **palavra** (tag) é o canal não-cromático; a cor reforça.
- `kbd`: 11 px, borda `--line`, raio 3 px, só para atalho que existe de verdade.

## 5. Piso de acessibilidade

- **Contraste:** 4,5:1 para texto normal; 3:1 para texto grande (≥ 18,66 px ou ≥ 14 px em negrito) e
  para limites de componente/indicador de foco. Medir sobre a superfície **composta** (alfa incluso),
  não sobre o token cru.
- **Alvo:** 32×32 px para qualquer controle; a linha da árvore é a exceção declarada, com 22 px de altura e 16 px de ícone.
- **Teclado:** todo controle alcançável, ordem de Tab igual à ordem visual, foco sempre visível.
- **Nome acessível:** todo controle só de ícone tem `aria-label`; imagem decorativa tem `alt=""`.
- **Sem cor sozinha:** estado tem texto ou forma além da cor.
- **`prefers-reduced-motion`** respeitado globalmente.

## 6. Proibido

1. Hex novo fora do bloco de tokens (exceto arte de ícone e paleta de sintaxe).
2. `var(--token-inexistente, #fallback)`.
3. Texto abaixo de 11 px.
4. Card dentro de card com sombra; elevação é alfa, não sombra em painel.
5. Duas ações primárias na mesma tela.
6. Cor como único portador de estado.
7. Instrução de recuperação sem o controle que a executa.
8. Fonte nova sem decisão registrada em `docs/decisions.md`.

## 7. Dívida conhecida (medida na #328)

| Onde | O quê | Achado | Estado |
|---|---|---|---|
| `src/styles.css` | rótulo branco sobre `--accent` = 3,30:1 (hover 2,72:1) | P1 | corrigido (#329) |
| `src/styles.css` | `var(--danger, …)`, `var(--border, …)` para tokens inexistentes | P2 | corrigido (#334) |
| `src/styles.css` | `#f75464`, `#e5b94e`, `#43454a`, `#2e436e`, `#a9c7ff` fora do token | P2 | corrigido (#334) |
| `src/styles.css` | `font: 700 7px/1` no selo, rótulos de ação a 10 px | P3 | corrigido (#335) |
| `src/styles.css` | escala de espaçamento inexistente; raio hardcoded | P3 | corrigido (#335) |
| `src/shell/ToolWindowBand.tsx` | foco cortado nas laterais do rail | P2 | corrigido (#333) |
| `src/WorkspaceView.tsx`, `src/ProblemsPanel.tsx` | stale/parcial sem ação adjacente | P1 | corrigido (#330) |
| `src/TestsPanel.tsx`, `src/NuGetPanel.tsx`, `src/BuildPanel.tsx` | chrome consome a altura do painel | P1 | corrigido (#331) |
| `src/TrustPrompt.tsx` | "executable inspection" abstrato; permissivo é o botão dominante | P2 | **aberto (#332, pede decisão do Helder)** |
| Welcome, editor vazio, Settings, Git, menu | microcopy sem próximo passo | P3 | corrigido (#336) |

## 8. Governança

- Este documento e `docs/design/critique-328.md` são a referência para revisão de UI.
- Token novo ou regra nova entra por PR que atualiza este arquivo **e** o código no mesmo commit.
- `data-testid` e `role="tab"` são contrato: nenhuma regra visual os remove.
- Zero dependência nova de UI sem decisão em `docs/decisions.md` e aprovação do mantenedor.
- A crítica é reexecutada com a skill Impeccable (`critique`) depois de um ciclo de correções; o
  placar de 23/40 é a linha de base.
