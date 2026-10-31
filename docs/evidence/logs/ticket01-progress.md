# Ticket 01 — progresso (fatos verificáveis)

Base de comparação: `b8bd154`. Fatia já commitada: `9a558c8` (Build/Run/Debug em `useExecutionFlow.ts`).

## Marco 1 — corte escolhido e interfaces (2026-10-31)

- Novo hook `src/useEvaluationFlow.ts` ("preparação": restore + avaliação). Fica nele: `evaluating`, as 4 refs de automação (`autoEvaluation`, `autoRestore`, `autoEvaluatedAfterRestore`, `autoReevaluated`), `evaluateContext`, `runEvaluation`, `startRestore`, `cancelRestore` e os 4 efeitos automáticos (avaliar após Trust, restaurar após Trust, reavaliar quando o `.csproj` muda em disco, avaliar após restore). Textos, ordem das chamadas ao host e mensagens: movidos sem edição.
- Entradas (callbacks mínimos, sem acesso ao DOM): `snapshot`, `trusted`, `restoreAvailable`, `evaluationAvailable`, `runConfigsLoaded`, `activeNamedConfig`, `runConfigStore`, `commitRunConfigs`, `onSnapshot`, `setMessage`. Saídas: `evaluating`, `evaluateContext`, `runEvaluation`, `startRestore`, `cancelRestore`.
- Composição na view: o hook é chamado logo depois de `activeNamedConfig` (suas entradas já existem); `evaluationBusy` passa a ser calculado depois dessa chamada (só é usado em JSX).
- Com `useExecutionFlow` (build/run/debug), o ticket cobre restore, avaliação, build, run e debug.
- Diff: ver marco 2.

## Marco 2 — código extraído (2026-10-31)

`git diff --stat` do corte (antes de commitar): `src/WorkspaceView.tsx` 6 inserções / 110 deleções; arquivo novo `src/useEvaluationFlow.ts` (139 linhas). Saíram do componente, sem edição de texto: `evaluating` e as 4 refs de automação, `evaluateContext`, `runEvaluation`, os wrappers `startRestore`/`cancelRestore` e os 4 efeitos automáticos. Imports mortos (`restoreCommands`, `executionCommands.evaluateContext`) removidos da view. Efeitos de avaliação/restore passam a rodar na posição da chamada do hook (logo após `activeNamedConfig`), não mais no meio dos efeitos de LSP/símbolos — a ordem relativa entre eles não tem dependência (verificada pelos specs abaixo).

Estado acumulado do ticket (fatias `9a558c8` + esta): estado de restore/avaliação/build/run e os fluxos start* de Build/Run/Debug estão em `useEvaluationFlow.ts` + `useExecutionFlow.ts` + `executionCommands.ts` + `executionGates.ts` + `restoreCommands.ts`. Restam no componente apenas: fluxo de testes (`startTestDiscovery`/`startTestRun`, fora do escopo do ticket), `commitRunConfigs`/`selectRunConfig`/`commitRunConfigDialog` (configurações de execução, estado do diálogo) e o resto do app.

## Marco 3 — comparação com a base `b8bd154` (2026-10-31)

- Base: `bunx playwright test restore evaluation build run run-configs workspace debugger gates zz-acceptance` → 50 passed + 1 failed (`zz-acceptance` jornada, falha de ambiente do passo `dotnet --version`, corrigida em `b74b63d`); após `b74b63d`: zz 2 passed.
- Depois (HEAD + este corte): `bunx playwright test restore evaluation build run run-configs workspace debugger gates trust-sdk zz-acceptance` (TERM=xterm-256color, portas livres, processo único) → **exit 0, 54 passed, 0 failed, 0 skipped** (os 51 testes da base + `trust-sdk` 3).
- `bun run check` → exit 0. `bunx tsc --noEmit` limpo.
- Contagem do componente: `WorkspaceView.tsx` 4740 linhas (base 5126); ocorrências de hooks no componente 113 (base) → 98; `useEvaluationFlow.ts` 10, `useExecutionFlow.ts` 7.
