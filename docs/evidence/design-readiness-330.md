# Estado de prontidão do workspace (#330)

Data: 2026-11-09. Superfície: app em modo web (host-http 5717 + Vite 2720, `OPENRIDR_HOME`
isolado), Chromium via Playwright, 1440×900.

## O que mudou

- **Faixa de avaliação** (`src/WorkspaceView.tsx`): três estados distintos.
  - em andamento: uma frase só, `Finishing restore and evaluating projects…`
    (`data-testid="eval-running"`), sem botão;
  - stale acionável: `Evaluation stale.` + motivo + botão `Re-evaluate` habilitado;
  - stale bloqueado: o botão fica desabilitado e o motivo completo aparece ao lado
    (`data-testid="eval-stale-blocked"`).
- **Status bar** (`src/shell/StatusBar.tsx`): a ação de recuperação continua com o mesmo
  `data-testid` em todos os estados; bloqueada, ela fica desabilitada **e** o motivo aparece em
  texto ao lado (`eval-blocked-reason`, `status-analysis-blocked`). Nunca há botão desabilitado sem
  motivo visível. Enquanto a avaliação roda, a barra esconde parcial e análise e o motivo transitório
  fica só na `title`: a frase única da faixa é o que se lê.
  O motivo da barra é curto ("Resolve the project SDK first."); o completo fica na faixa.
- **Problems** (`src/ProblemsPanel.tsx`): a ressalva de cobertura parcial ganhou a ação
  `Restart analysis` ao lado (`data-testid="problems-restart-analysis"`); quando o host bloqueia a
  análise, o botão fica desabilitado e o motivo aparece ao lado (`problems-restart-blocked`). O
  texto da ressalva não mudou.

## Verificado no app

| Estado | Como foi produzido | Resultado |
|---|---|---|
| Em andamento | `eval-run` clicado em OrderDesk | `eval-running` visível com a frase única; `eval-stale-run` ausente |
| Ação habilitada | OrderDesk avaliado (sem stale) | `eval-state` = `Evaluated · Debug · TFM net8.0 · revision 2`; `eval-run` presente e habilitado |
| Bloqueado | `fixtures/MissingSdk/MissingSdk.csproj` (global.json pede 8.0.999) | `eval-blocked-reason` = "Resolve the project SDK first."; `eval-run` presente e desabilitado |
| Problems parcial + bloqueado | mesmo workspace, painel Problems | `problems-partial` presente; `problems-restart-blocked` = "Resolve the project SDK before analysis."; botão desabilitado |
| Problems parcial + habilitado | `fixtures/BrokenImport/BrokenImport.csproj` | `coverage partial`; `problems-restart-analysis` habilitado; sem `problems-restart-blocked` |

`data-testid` preservados: `eval-state`, `eval-run`, `eval-stale`, `eval-stale-run`,
`eval-disabled-reason`, `problems-partial`, `problems-coverage`, `status-analysis`.

## Specs

- `tests/e2e/evaluation.spec.ts` — **6 passed / 1 failed**: `saving a project file in a temp copy
  marks the model stale and re-evaluates it` falha com `app-log` ausente depois de salvar. O mesmo
  teste falha **igual em `origin/main`** (medido com o CSS da main), então é ambiente (atalho Meta+S
  no Linux), não regressão.
- `tests/e2e/workspace-readiness.spec.ts` — **3 passed**: em andamento sem ação, bloqueado com
  motivo ao lado do botão desabilitado, e parcial com `Restart analysis` habilitado.
- `tests/e2e/analysis-gate.spec.ts` + `tests/e2e/disabled-states.spec.ts` — **4 passed**.
- `tests/e2e/disabled-states.spec.ts` — verde no PR do #329 (mesma árvore de estados).

## Decisão de contrato: botão bloqueado continua no DOM

O ticket pede que "Re-evaluate" só apareça quando pode agir. O `AGENTS.md` trata `data-testid`
como contrato e proíbe removê-lo. A resolução foi manter o botão com o mesmo `data-testid`,
desabilitado, **e** o motivo em texto ao lado — que é o que o critério de aceite cobra ("nenhum
estado mostra ação de recuperação desabilitada sem motivo visível"). O botão sai do DOM em nenhum
estado.

## Não verificado

- App Tauri real (macOS ARM64).

## Capturas

`docs/evidence/design-readiness-330/running-1440.webp` (faixa em andamento) e
`problems-1440.webp` (Problems com cobertura completa).
