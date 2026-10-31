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
- Contagem do componente: `WorkspaceView.tsx` 4740 linhas em `0aa37b3` (base 5126); ocorrências de hooks 113 (base) → 98 (medidas intermediárias; finais abaixo); `useEvaluationFlow.ts` 10, `useExecutionFlow.ts` 7.

## Marco 2b — fatia 3: store de run configs e fluxos "avaliação segue a run config" (2026-10-31)

- Novo `src/useRunConfigs.ts` (47 linhas): `runConfigStore`, `runConfigsLoaded`, efeito de leitura por workspace e `commitRunConfigs`. `selectRunConfig` e `commitRunConfigDialog` (a avaliação seguindo a run config ativa) foram para `useEvaluationFlow.ts` (175 linhas), que já continha `evaluateContext`/`runEvaluation`/restore; `useEvaluationFlow` recebe `commitRunConfigs` do outro hook (sem ciclo).
- Diff (`git diff --stat`): `WorkspaceView.tsx` 2 inserções / 71 deleções líquidas nesta fatia (arquivo `useRunConfigs.ts` novo; `useEvaluationFlow.ts` +38).
- Comparação com a base `b8bd154`: mesmo comando do marco 3 → **exit 0, 54 passed, 0 failed, 0 skipped**; `bun run check` exit 0; `bunx tsc --noEmit` limpo.
- Componente agora:     4672 linhas; 95 ocorrências de hooks (base 5126 / 113).
- O que ainda mora no `WorkspaceView` relacionado ao critério: só derivações de render a partir de `snapshot.evaluation`/`snapshot.restore` (listas de projetos/TFM/configurações, gates `restoreIncomplete`, `consoleCandidates`), o estado do diálogo de run config (`runConfigOpen`/`runConfigInitialId`), `setStartupProject`/`invalidRunConfigReason`/`openConfigOnFieldError` (dependem de `activeRunConfig`, derivado) e o JSX. Fluxo de testes (`startTestDiscovery`/`startTestRun`) permanece fora do critério e não foi tocado.

## Notas de revisão (fatia 2, aplicadas junto da fatia 3)

- **Números:** a base única de comparação é `b8bd154`: `WorkspaceView.tsx` 5126 linhas / 113 ocorrências de hooks. (O 5119 que aparece em `docs/implementation-notes.md` era a medida de `0587ee3`, antes de outras mudanças; não é base.) Comando da contagem de hooks: `grep -c "use\(State\|Ref\|Effect\|Callback\|Memo\)" src/WorkspaceView.tsx` (conta linhas com a ocorrência, não chamadas distintas). Delta confiável da fatia 2: 5126 → 4844 linhas e 113 → 107.
- **Ordem de efeitos:** o efeito Run-após-rebuild (`pendingRunAfterBuild`) estava em ~:924 do `WorkspaceView` e passou a rodar dentro de `useExecutionFlow` (chamado em ~:2242, depois de ~12 outros `useEffect`). Mudança de ordem **sem dependência conhecida**: o efeito só lê `build`/`pendingRunAfterBuild` do mesmo render (deps `[build, pendingRunAfterBuild]`) e só chama `setPendingRunAfterBuild(null)` e `startRun` (chamada ao host), sem ler refs/estado que os outros efeitos escrevam no mesmo commit. Cobertura: `run-configs.spec.ts:116` "dialog Run saves and rebuilds before launching the requested configuration" (passou antes e depois, no 54/54 de `0aa37b3` e de `cdfa541`) e `:142` "dialog Run does not launch its requested configuration when rebuild fails". Não há assert extra "a ordem não importa": o motivo acima e esses dois testes são a prova.
- **Efeitos da fatia de avaliação:** mesma observação — os 4 efeitos automáticos de restore/avaliação passaram a ser registrados logo depois de `activeNamedConfig` (antes dos efeitos de LSP/símbolos/git); `restore.spec.ts:58`/`:191` e `evaluation.spec.ts:63`/`:183` cobrem o comportamento.

## Marco 4a — cobertura e escopo final (2026-10-31)

- Números finais (HEAD `cdfa541`): `WorkspaceView.tsx` 4672 linhas / 95 ocorrências de hooks (base `b8bd154`: 5126 / 113); comando: `grep -c "use\(State\|Ref\|Effect\|Callback\|Memo\)" src/WorkspaceView.tsx`.
- Novo `tests/e2e/zy-restore-interrupt.spec.ts`: (a) clicar `restore-cancel` durante o restore automático → `Restore cancelled`, sem novo restore automático, retry disponível; (b) revogar Trust durante o restore → `Restricted mode`, restore para, nada é avaliado depois. Determinismo: `Directory.Build.targets` na cópia temporária com `sleep 40` antes de `_GenerateRestoreGraphProjectEntry` (sem tocar o produto). `--repeat-each=3`: 6 passed, exit 0; sem `sleep` órfão após o término (o grupo é encerrado). Primeira tentativa sem o atraso corria contra o restore de ~1 s e falhava em repetições — descartada.
- Falha de rede real no restore: **não coberto** (depende de rede/feed real).
- Item 1 do ticket reescrito com o escopo real e o que fica fora do critério (ver `docs/implementation-notes.md`, "Ticket 01 — escopo final").
