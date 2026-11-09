# Design tokens: contraste, cores semânticas e escala (#329, #334, #335)

Data: 2026-11-09. Frente: `feat/issue-328-design-polish`. Superfície: app em modo web
(host-http 5717 + Vite 2720, `OPENRIDR_HOME=/tmp/openridr-328-polish`), Chromium via Playwright,
1440×900.

## O que mudou

### #329 — contraste do texto sobre ação primária e superfícies de estado

| Par | Antes | Depois |
|---|---:|---:|
| rótulo sobre `--accent` (`#548af7`) | branco **3,30:1** | `--on-accent` `#101114` **5,71:1** |
| rótulo sobre `--accent-hover` (`#6d9bf9`) | branco **2,72:1** | `--on-accent` **6,93:1** |
| `--muted` sobre painel + 8% branco (hover) | **4,25:1** | `#b3b3b3` **5,12:1** |
| `--muted` sobre linha selecionada (`#43454a`) | **4,18:1** | **4,58:1** |
| `--accent` como texto sobre painel | **4,18:1** | `--accent-text` `#8ab2ff` **6,51:1** |

Razões calculadas por luminância relativa WCAG sobre a superfície composta (alfa incluso).
`--accent` continua sendo o azul de marca de fundo/foco/seleção; só o texto ganhou token próprio.
`button.primary` e `:hover` usam `--on-accent`; `link-button`, abas ativas, branch atual, título de
seção NuGet e o marcador de linha atual do debug usam `--accent-text`.

### #334 — tokens fantasma e cores fora do token

- `var(--danger, #e05555)` e `var(--border, #3c3f41)` viraram `--danger`/`--border` reais.
- Hexes estruturais promovidos a token semântico **com o mesmo valor** (diff visual nulo):
  `--status-error`, `--status-warning`, `--select-row`, `--select-row-focus`, `--focus-ring-strong`,
  `--warning-surface`, `--success-surface`, `--on-icon`, `--mark-surface`, `--surface-active`,
  `--debug-current`, `--debug-current-halo`, `--debug-breakpoint-line`, `--debug-current-line`,
  `--diff-del-surface`, `--diff-add-surface`, `--change-add`, `--change-mod`, `--tab-active-surface`,
  `--tab-active-border`, `--commit-group-surface`, `--commit-group-text`, `--shadow-menu`,
  `--shadow-drawer`, `--shadow-popover-strong`, `--shadow-dialog-strong`.
- Ficam hardcoded, por serem dados e não tema: paleta de sintaxe, cores de git (`--git-*` e
  `commit-name.*`), tintas de ícone (`solution-icons.css`) e fundos de linha de diff.

### #335 — escala de tipografia e espaçamento

- `--sp-1..6` (4/8/12/16/24/32). 223 declarações de `padding`/`gap`/`margin` com valor exato da
  escala passaram a usar o token (mesmo px, diff visual nulo). Valores densos fora da escala
  (1/2/3/6/10/14/18/20 px) ficaram: encaixá-los mudaria a densidade que o ticket manda preservar.
- Raio sempre por token: 33 `border-radius` numéricos viraram `--r-sm`/`--r-md`/`--r-pill`
  (3→4 px, 2→4 px, 5→6 px, 9 px→pill; 10 elementos medidos, todos em controles densos).
- Piso de 11 px: `font: 700 7px/1` do selo de símbolo virou 11 px em caixa de 12 px; os dois
  rótulos de 10 px das rails de ação (Tests/NuGet) viraram `--fs-badge`; o selo `↗` da pasta
  `config` (`src/solution-icons.css`) saiu de `bold 9px/8px` para `--fs-badge`/1.
- One-offs de UI: 15 px → `--fs-label`/`--fs-ui`, 16 px → `--fs-ui`, títulos de diálogo
  `h2` 16 px/700 → `--fs-ui`/600, `h1` 24 px → `--fs-title` 20 px.

## Verificação medida

Diff de estilos computados (720 elementos visíveis, antes × depois, mesma navegação):

| Propriedade | Mudança | Ocorrências |
|---|---|---:|
| `color` | `#a3a3a3` → `#b3b3b3` (`--muted`) | 332 |
| `color` | branco → `#101114` (`--on-accent`) | 1 |
| `font-size` | 15 → 14 px | 8 |
| `font-size` | 10 → 11 px | 8 |
| `border-radius` | 3 → 4 px | 10 |
| `font-weight` | 700 → 600 (h2) | 2 |

Nenhuma outra propriedade mudou **nos 720 elementos e estados amostrados** (welcome, trust,
workspace, Problems, Tests, NuGet, Build, Git, Settings). A amostra não cobre todo estado do app:
Run configurations, debug em execução e a pasta `config` da árvore não estavam visíveis.

Piso de 11 px: nenhum `font-size` abaixo disso no CSS ou em estilo inline de componente. Ficam
abaixo de 11 px só `<text>` de SVG (glifos de ícone, arte) e o tamanho do editor, que é configuração
do usuário.

## Specs

- `tests/e2e/disabled-states.spec.ts` — **9 passed** (critério de aceite do #329).
- `tests/e2e/setup-gate-layout.spec.ts` — **passed**; `tests/e2e/medium-nav-layout.spec.ts` —
  5 passed / 2 failed. Os dois `docked Find Usages` falham **igual em `origin/main`** (medido: mesma
  asserção `data-cursor` "2:17" × "4:0" com o CSS da main), então são dívida anterior, não regressão.
- `tests/e2e/workspace.spec.ts` — 7 passed / 7 failed **neste ambiente**: o primeiro teste escolhe
  modo restrito e o trust store do host de E2E persiste a decisão para os testes seguintes, que então
  não veem o prompt e falham ao salvar. As asserções falham em Restricted mode, não por CSS; a
  mensagem é sempre `app-log` ausente com Build/Run/Git desabilitados. Não reproduzido em macOS.

## Correções da revisão de código

- `run-config-add` ainda usava `--accent` como texto (4,18:1) → `--accent-text`.
- `border-radius` cru restante em `rider-chrome.css` (6 px) e `git-panel.css` (4 px) → tokens.
- Selo `↗` da pasta `config` a 9 px → `--fs-badge`.
- `.cm-line-current` e o override de `rider-chrome.css` tinham valores diferentes; agora são dois
  tokens (`--debug-current-line`, `--debug-current-line-strong`) com os valores originais, sem troca
  de aparência.
- Comentário da escala de espaçamento corrigido (a escala vale para valores que caem num passo; os
  densos ficam crus de propósito).

## Não verificado

- App Tauri real (macOS ARM64) e tema claro: não distribuído/fora desta máquina.
- Layout a 1440 px além das capturas: `ui-layout.spec.ts` está quarentenado.
- Hover real do `button.primary` (medido por token, não por captura).

## Capturas

`docs/evidence/design-tokens-328/{before,after}-*.webp` — welcome, trust, workspace, Problems, Tests,
NuGet, Settings, 1100×760.
