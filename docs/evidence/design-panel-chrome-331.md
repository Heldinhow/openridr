# Chrome dos tool windows devolve área ao conteúdo (#331)

Data: 2026-11-09. Superfície: app em modo web (host-http 5717 + Vite 2720, `OPENRIDR_HOME`
isolado), Chromium via Playwright, 1100×760 (a largura estreita do critique).

## O que mudou

- **Tests** (`src/TestsPanel.tsx`): Expand all / Collapse all / Export saíram da linha do filtro e
  ficaram atrás de um `<details>` rotulado **More** (`data-testid="tests-overflow"`). O caminho
  exportado deixou de ocupar a linha inteira (`flex: 1 1 auto; min-width: 0`), então também não
  empurra a barra para uma segunda linha.
- **NuGet** (`src/NuGetPanel.tsx`): Scope, Source e Prerelease ficaram atrás de um `<details>`
  **Filters** (`data-testid="nuget-filters"`). A linha primária fica com Project, Refresh e as abas
  de modo.
- **Build** (`src/styles.css`): o botão carrega o nome do projeto de startup; ficou limitado a
  210 px com elipse, e os selects ganharam `min-width: 0`. A linha de comando cabe em uma linha.
- **Lista** (`src/styles.css`): `.tests-content` ganhou `min-height: 0` para poder rolar dentro do
  painel, e `.tests-side` passou a encolher (`flex: 0 1 340px; min-width: 220px`) — era ele que
  criava 103 px de rolagem horizontal no painel a 1100 px.

O disclosure é nativo (`<details>`/`<summary>`): Tab alcança o summary, Enter/Espaço abre, e os
controles dentro mantêm `data-testid`, `title` e o foco de teclado.

## Medido (Playwright, com build real no painel Build)

Altura **visível** = interseção do elemento da lista/log com o `.panel-body` do painel.

| Painel | 1100×760 | 1440×900 | Rolagem horizontal interna |
|---|---|---|---|
| Tests (lista) | **148 px** | **149 px** | nenhuma (`scrollWidth == clientWidth` em 686/686 e 955/955) |
| NuGet (lista) | **234 px** (246 de caixa) | **235 px** | nenhuma (902/902, 1242/1242) |
| Build (log real) | **125 px** | **125 px** | nenhuma (984/984, 1324/1324) |

Controles: NuGet **72 → 32 px** (uma linha); Build **72 → 58 px** a 1100 (a tag de estado desce para
a segunda linha, o comando fica em uma) e 32 px a 1440. Tests: barra de 47 px em uma linha, com o
disclosure no lugar dos três botões.

O `.panel-body` dos três painéis também não rola na horizontal (`scrollWidth == clientWidth` em
1012/1012 e 1352/1352). Nenhum controle removido; todos os `data-testid` preservados; `role="tab"`
intacto em `tests-tab-*`, `nuget-tab-*` e `nuget-mode-*`.

## Specs

- `tests/e2e/acceptance-fixes.spec.ts` — **5 passed**, com o teste de export abrindo o disclosure
  antes de clicar em `tests-export`.
- `tests/e2e/panel-disclosures.spec.ts` — **1 passed**: Tab do filtro alcança o summary, Enter abre
  Tests, Espaço abre NuGet, os controles internos aparecem e o summary tem nome acessível.
- `tests/e2e/tests.spec.ts` — atualizado para abrir o disclosure antes de `tests-collapse-all`
  (não rodado nesta sessão: o arquivo tem falhas conhecidas registradas em `AGENTS.md`).

## Não verificado

- App Tauri real (macOS ARM64).
- A altura do log foi medida com o build que o painel executou neste ambiente (não necessariamente
  um build bem-sucedido do projeto: o pin de SDK da fixture pode não estar instalado).

## Capturas

`docs/evidence/design-panel-chrome-331/{before,after}-{tests,nuget,build}-1100.webp`.
