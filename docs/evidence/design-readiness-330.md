# Estado de prontidão do workspace (#330)

Data: 2026-11-09. Superfície: app em modo web (host-http 5717 + Vite 2720, `OPENRIDR_HOME`
isolado), Chromium via Playwright, 1440×900.

## O que mudou

- **Faixa de avaliação** (`src/WorkspaceView.tsx`): três estados distintos.
  - em andamento: uma frase só, `Finishing restore and evaluating projects…`
    (`data-testid="eval-running"`), sem botão;
  - stale acionável: `Evaluation stale.` + motivo + botão `Re-evaluate` habilitado;
  - stale bloqueado: o botão sai e o motivo aparece no lugar (`data-testid="eval-stale-blocked"`).
- **Status bar** (`src/shell/StatusBar.tsx`): a ação de recuperação só é um botão quando pode agir.
  Bloqueada, ela vira o motivo em texto (`eval-blocked-reason`, `status-analysis-blocked`), com o
  texto completo no `title`. Nunca há botão desabilitado sem motivo visível.
- **Problems** (`src/ProblemsPanel.tsx`): a ressalva de cobertura parcial ganhou a ação
  `Restart analysis` ao lado (`data-testid="problems-restart-analysis"`); quando o host bloqueia a
  análise, o motivo aparece no lugar (`problems-restart-blocked`). O texto da ressalva não mudou.

## Verificado no app

| Estado | Como foi produzido | Resultado |
|---|---|---|
| Em andamento | `eval-run` clicado em OrderDesk | `eval-running` visível com a frase única; `eval-stale-run` ausente |
| Stale acionável | OrderDesk avaliado | `eval-state` = `Evaluated · Debug · TFM net8.0 · revision 2`; `eval-run` presente e habilitado |
| Bloqueado | `fixtures/MissingSdk/MissingSdk.csproj` (global.json pede 8.0.999) | `eval-blocked-reason` = "Trust the workspace and resolve the project SDK before evaluating."; `eval-run` ausente |
| Problems parcial + bloqueado | mesmo workspace, painel Problems | `problems-partial` presente; `problems-restart-blocked` = "Resolve the project SDK before analysis."; botão ausente |

`data-testid` preservados: `eval-state`, `eval-run`, `eval-stale`, `eval-stale-run`,
`eval-disabled-reason`, `problems-partial`, `problems-coverage`, `status-analysis`.

## Specs

- `tests/e2e/evaluation.spec.ts` — **6 passed / 1 failed**: `saving a project file in a temp copy
  marks the model stale and re-evaluates it` falha com `app-log` ausente depois de salvar. O mesmo
  teste falha **igual em `origin/main`** (medido com o CSS da main), então é ambiente (atalho Meta+S
  no Linux), não regressão.
- `tests/e2e/disabled-states.spec.ts` — verde no PR do #329 (mesma árvore de estados).

## Não verificado

- **Problems parcial + ação habilitada**: não reproduzido. OrderDesk tem cobertura `full` e o
  MissingSdk não tem SDK resolvido, então só o ramo bloqueado foi exercitado no app. O ramo
  habilitado é o mesmo `restartAnalysisReason === null` verificado no status bar.
- App Tauri real (macOS ARM64).

## Capturas

`docs/evidence/design-readiness-330/running-1440.webp` (faixa em andamento) e
`problems-1440.webp` (Problems com cobertura completa).
