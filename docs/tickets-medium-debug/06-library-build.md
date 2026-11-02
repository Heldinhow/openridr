# 06 — Build de biblioteca no menu de projeto (b12)

**Fonte:** `docs/SPEC-MEDIUM-DEBUG.md` §6. **Bloqueado por:** nenhum (03 é independente). **Status:** ready-for-agent.

## Objetivo

No menu de um projeto de biblioteca/testes, **Build** funciona; **Run** e **Debug** não
aparecem (em vez de aparecerem desabilitados).

## Pontos de código

- `crates/host/src/build.rs`: `select_startup` (consumidor `build`) aceita `project` explícito
  `console` **ou** `library` com `state == "ok"` e `target_path`; sem `project`, mantém só
  console. `select_startup_for` usado por run/debug não muda.
- `crates/host/src/lib.rs`: nada além de usar essa seleção; conferir que o comando
  `dotnet build <csproj>` já serve para biblioteca e que a checagem de TargetPath não exige
  assembly.
- `src/WorkspaceView.tsx` `projectGates`: `buildReason` só exige trust + projeto avaliado com
  `targetPath`; `runReason`/`debugReason` continuam exigindo console. `startBuild` procura a
  config pedida em `evaluatedProjects` (não em `consoleCandidates`).
- `src/solutionMenu.ts`: item `build` sempre presente para projeto avaliado; `run`/`debug`
  omitidos quando `kind !== "console"`. `unit-test` continua desabilitado.
- `src/WorkspaceView.tsx` `projectMenu`: passa `executable` ao menu.

## Teste primeiro

`tests/e2e/solution-explorer.spec.ts`: no teste "trusted menu" já existente, biblioteca mostra
`tree-menu-build` habilitado (sem title de console) e **não** tem `tree-menu-run`/`tree-menu-debug`;
clicar `tree-menu-build` → `build-state` "Build succeeded" e `build-project` com
`OrderDesk.Core`. Console continua com Run/Debug habilitados. Falhar antes (hoje o Build da
biblioteca está desabilitado e Run/Debug existem).

## Aceite

- [x] `build.start` com projeto explícito de biblioteca compila e publica problemas.
- [x] Menu da biblioteca: Build habilitado; Run/Debug ausentes.
- [x] Startup select da toolbar continua só com consoles; Run/Debug recusam biblioteca.
- [x] `bun run check` verde. Commit `feat(build): build library projects from the tree`.
