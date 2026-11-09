# Chrome dos tool windows devolve área ao conteúdo (#331)

Data: 2026-11-09. Superfície: app em modo web (host-http 5717 + Vite 2720, `OPENRIDR_HOME`
isolado), Chromium via Playwright, 1100×760 (a largura estreita do critique).

## O que mudou

- **Tests** (`src/TestsPanel.tsx`): Expand all / Collapse all / Export saíram da linha do filtro e
  ficaram atrás de um `<details>` rotulado **More** (`data-testid="tests-overflow"`). O caminho
  exportado, quando aparece, não empurra mais a barra para uma segunda linha.
- **NuGet** (`src/NuGetPanel.tsx`): Scope, Source e Prerelease ficaram atrás de um `<details>`
  **Filters** (`data-testid="nuget-filters"`). A linha primária fica com Project, Refresh e as abas
  de modo.
- **Build** (`src/styles.css`): o botão carrega o nome do projeto de startup; ficou limitado a
  210 px com elipse, e os selects ganharam `min-width: 0`. A linha de comando cabe em uma linha.
- **Lista** (`src/styles.css`): `.tests-content` ganhou `min-height: 0` para poder rolar dentro do
  painel.

O disclosure é nativo (`<details>`/`<summary>`): Tab alcança o summary, Enter/Espaço abre, e os
controles dentro mantêm `data-testid`, `title` e o foco de teclado.

## Medido a 1100×760 (antes → depois)

| Painel | Controles | Lista/conteúdo | Painel |
|---|---|---|---|
| Tests | 32 px (1 linha) | ~151 px visíveis (283 − 33 abas − 47 barra − 32 controles − 20 padding) | 641 |
| NuGet | **72 → 32 px** | **286 → 246 px** | 325 → 285 |
| Build | **72 → 58 px** | reservado para o log: 283 − 33 − 58 = **192 px** | 72 → 58 |

`document.documentElement.scrollWidth > innerWidth` = `false` nos três painéis (sem rolagem
horizontal). Nenhum controle removido; todos os `data-testid` preservados; `role="tab"` intacto em
`tests-tab-*`, `nuget-tab-*` e `nuget-mode-*`.

## Specs

- `tests/e2e/acceptance-fixes.spec.ts` — **5 passed**, com o teste de export abrindo o disclosure
  antes de clicar em `tests-export`.
- `tests/e2e/tests.spec.ts` — atualizado para abrir o disclosure antes de `tests-collapse-all`
  (não rodado nesta sessão: o arquivo tem falhas conhecidas registradas em `AGENTS.md`).

## Não verificado

- App Tauri real (macOS ARM64).
- Log de Build com build de verdade: o ambiente não rodou build, então o `build-log` não existia;
  a altura reservada foi calculada pela caixa do painel.
- 1440×900: a mudança só afeta a linha que quebrava; as capturas ficaram em 1100×760.

## Capturas

`docs/evidence/design-panel-chrome-331/{before,after}-{tests,nuget,build}-1100.webp`.
