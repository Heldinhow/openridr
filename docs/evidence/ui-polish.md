# Evidência: diagnóstico visual (ícones e tipografia)

Data: 2026-11-04. App real `/Applications/OpenRidr.app` (0.3.0-alpha.6, macOS ARM64, tema escuro), aberto com `open -a OpenRidr`, navegado com `agent-desktop` e capturado com `screencapture -x`. Projeto aberto: Gabagool.sln (recente, Trusted). Sem browser/web mode. App fechado ao final.

Registro de verificação: o clique em "Open Settings" foi entregue (`delivered_unverified`) mas a captura seguinte não mostrou o diálogo; **a tela de Settings não foi inspecionada** (não verificado). Também não foram capturados: sessão de debug ativa (passos Step Over/Into/Out), terminal, painel de Tests, menus de contexto.

## Capturas (em `docs/evidence/ui-polish/`)

| Arquivo | O que mostra |
|---|---|
| `app-screencapture.png`, `tela-1.png`, `tela-2.png` | Welcome (Projects/Toolchain): ícone `</>` da marca, lupa, `…`, chevrons, `✓` como texto na Toolchain |
| `ws-1-aberto.png` | Workspace completo: toolbar, árvore, aba, editor, painel Debug, rail esquerdo e status bar |
| `ws-2-crop-toolbar.png` | Toolbar em resolução nativa: martelo (Build), sliders (run config), Play, Debug (bug), `⋮` |
| `ws-3-crop-tree.png` | Árvore: selo "C#" nos projetos e arquivos, pastas, ícones de ação do cabeçalho |
| `ws-4-crop-editor.png` | Aba com selo C#, breadcrumb, primeira linha do editor |
| `ws-5-settings.png` | Mesmo estado do workspace (o diálogo não abriu na captura; ver acima) |

## O que as imagens mostram

1. **Selo C# (`ws-3`, `ws-4`)**: é uma caixinha verde de ~17×13 px com o texto "C#" em 8 px (`.cs-badge`, `src/styles.css:2768-2776`). Não é ícone, é texto CSS com `::before`. Em 8 px o "#" borra; o verde saturado é a única cor da árvore e compete com o código (anti-referência do PRODUCT.md). Projeto e arquivo usam o mesmo selo, só 1 px maior no projeto, então não se distinguem.
2. **Play e Debug (`ws-2`)**: triângulo e inseto em traço fino (1,6/24 → ~1,2 px a 18 px) e verde. O triângulo tem cantos vivos, enquanto o resto do conjunto usa pontas arredondadas; o inseto tem 8 apêndices em 18 px e vira ruído. Ambos parecem "de outro conjunto" frente ao martelo.
3. **Settings (`ws-1`, canto superior direito)**: engrenagem de 16–18 px com 8 dentes e traço fino; sem peso visual de botão.
4. **Rail esquerdo e rodapé (`ws-1`)**: ícones de 16 px com traço ~1,07 px (1,6 × 16/24) sobre preto: leem como hairline. Pesos visualmente diferentes entre o rail (finos) e o botão de debug do painel (inseto verde isolado).
5. **Toolbar de árvore (`ws-3`)**: `eye`, lupa, mira, `⌄⌄`, `⌃⌃`, `⋮`, `—` com 22×22 px de alvo, abaixo do mínimo de 32×32 do PRODUCT.md (Accessibility). `⋮` e `…` são pontos de comprimento zero (`M12 5v.01`), cujo tamanho depende do `linecap`.
6. **Tipografia**: UI em sans de sistema e código em mono de sistema (ver abaixo). Tamanhos fracionários (0,78 rem ≈ 12,5 px) e selos de 8–9 px aparecem como texto "mole" ao lado do código.

## Inventário de ícones

Fonte única: `src/Icon.tsx` (85 linhas): um `Record<IconName,string>` de `d` de path, `viewBox 24`, `width/height 16`, `strokeWidth 1.6`, `round`. 38 nomes, ~42 usos de `<Icon name="…"/>`, mais 1 uso dinâmico (`name={…}`). Tamanhos efetivos: 16 (padrão), 18 (`.toolbar .icon`, `src/styles.css:2600`), 32 (`.empty-editor > .icon`, `:984`). Nenhum teste e2e referencia classes de ícone (`grep` em `tests/`: vazio), então a troca não toca `data-testid`.

Origem: os paths são cópias do **Lucide** (ex.: `debug` = `bug`, `build` = `hammer`, `settings`, `search`, `git`=`git-branch`), de versões antigas e com retoques à mão (`run`, `stop`, `shield`, `solution`, `solutionFolder`, `dependencies`, `more`, `pause`, `resume`, `stepOver/Into/Out`). O Lucide é ISC e **não consta em `THIRD-PARTY-NOTICES.md`** — débito de licença independente da troca.

| Grupo | Nomes | Uso | Estilo / problema | Sev. |
|---|---|---|---|---|
| Linguagem/arquivo | `cs-badge` (C#), `json-badge` (`{}`), `file` | árvore, abas, `FileIcon.tsx` | texto CSS 8–12 px, cor própria (verde/roxo); `.csproj`, `.sln`, `.props`, `.md`, `.xml` caem no `file` genérico | Alta |
| Ações de execução | `run`, `debug`, `stop`, `build`, `restart`, `resume`, `pause`, `stepOver/Into/Out` | toolbar, painéis Debug/Terminal | Play com cantos vivos; `stop` preenchido via CSS (`fill: currentColor`) e o resto em traço; `run` reutilizado como "Start shell" no Terminal (`TerminalPanel.tsx:226`, semântica trocada); step* desenhados à mão | Alta |
| Configurações | `settings`, `runconfig` | toolbar | engrenagem de 8 dentes fina; `runconfig` (sliders) parece outro "configurações" ao lado | Média |
| Navegação/árvore | `back`, `forward`, `chevron`, `expandAll`, `collapseAll`, `locate`, `eye`, `more`, `minimize`, `plus`, `close` | árvore, abas, painéis | traço 1,07 px; alvos de 22 px; `more` por ponto de comprimento zero; `chevron` da árvore é texto/CSS em partes (`.tree-chevron`, 10 px) | Média |
| Estrutura .NET | `solution`, `solutionFolder`, `project`, `dependencies`, `package`, `folder`, `shield` | árvore, rail | `solution` (monitor) e `solutionFolder` (pasta com triângulo) são ad hoc e ambíguos; `project` (caixa 3D) e `package` quase idênticos | Média |
| Painéis | `problems`, `terminal`, `tests`, `git`, `search`, `save`, `code`, `rename` | rail, abas, editor | traço consistente entre si, mas fino; `tests` (balão) pouco legível a 16 px | Baixa |
| Marca | `code` (`</>`) em Welcome e toolbar | cabeçalho | ok funcionalmente; ícone do app (`src-tauri/icons/`) fora do escopo | Baixa |
| Glifos de texto | `✓`, `!`, `…`, `⌘`, `↑ ↓` | Toolchain, status, dicas | dependem da fonte do sistema; `✓` e `!` não alinham com os ícones | Baixa |

Consistência de grade: todos em 24, mas o peso renderizado varia com o tamanho (1,07 px a 16, 1,2 px a 18, 2,1 px a 32) porque `strokeWidth` escala com o `viewBox`.

## Inventário de tipografia

| Item | Valor atual | Onde | Problema |
|---|---|---|---|
| UI | `--sans: "Inter Variable", Inter, -apple-system, …` | `src/styles.css:40` | **Inter não é embutida** (sem `@font-face`, sem pacote em `package.json`, `src/` sem woff/ttf) e `fc-list` desta máquina não lista Inter. Em runtime a UI é SF Pro; o DESIGN.md descreve Inter. A aparência depende da máquina |
| Mono | `--mono: "SF Mono", ui-monospace, SFMono-Regular, Menlo, …` | `:41` | mesma dependência do sistema |
| Editor | `fontFamily` default `ui-monospace`, 13 px | `src/actions/keymap.ts:55-57`, `CodeEditor.tsx:140-146` | opção "JetBrains Mono" existe em `fontFamilyOptions` mas a fonte não é embutida: quem escolhe e não a tem instalada cai para o fallback sem aviso |
| Terminal | `Menlo, Monaco, 'Courier New'`, 12 px | `TerminalPanel.tsx:67-68` | família diferente do editor e do `--mono` |
| Corpo | `14px/1.5`, `-webkit-font-smoothing: antialiased` | `:51-54` | antialiased afina texto claro sobre preto; DESIGN.md pede compensar peso/tracking no escuro e não há compensação |
| Escala | 12 px (59×), 0,75 rem (33×), 0,78 rem (21×), 13 px (20×), 11 px (12×), 14 px (8×), 0,8125 rem (3×), 0,7 rem (3×), 10 px (2×), 9 px (1×), 8 px (1×) | `src/*.css` | ~15 valores para 4 papéis do DESIGN.md (14/13/12/11); 0,78 rem = 12,48 px e 0,7 rem = 11,2 px são fracionários; 8/9/10 px violam o piso de 11 px |
| Comentários | `fontStyle: "italic"` no tema de sintaxe | `CodeEditor.tsx:69` | se a família do editor não tem itálico real, o WebKit sintetiza oblíquo (não verificado qual caso ocorre com `ui-monospace`); o screenshot mostra comentários inclinados |
| Números | sem `tabular-nums` | status bar, números de linha | colunas de números dançam; no mono não importa, na status bar sim |
| Ligaduras | não configuradas | — | com JetBrains Mono, as ligaduras ficam ligadas por padrão no navegador; precisa de decisão |

O que causa a aparência de baixa qualidade, em ordem: (1) selos de 8–9 px em texto CSS ao lado do código, (2) traço de 1,07 px não alinhado a pixel, (3) escala tipográfica com valores fracionários e fonte dependente do sistema, (4) conjuntos de ícones misturados e retocados à mão.
