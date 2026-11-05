# 04 — Dependencies agrupadas (Frameworks / Packages / Projects)

Status: implemented — verificação visual/E2E pendente de liberação da porta
Spec: [D5](../SPEC-SOLUTION-EXPLORER-VISUAL.md)

## Problema

`Dependencies` listava referências e pacotes em linhas planas com sufixos `· project` / `· package`.
Rider agrupa em `Frameworks`, `Packages` e `Projects`, e o Helder pediu explicitamente essa diferenciação.

## Mudança

- `src/solutionNodes.ts`: `DependencyGroup` e `depGroupKey(project, group)`.
- `src/SolutionTree.tsx`: `Dependencies` passa a ter três grupos expansíveis; só grupos com filhos aparecem;
  Dependencies e grupos começam fechados; `expandableKeys(..., deps: true)` abre os três; os sufixos
  `· project`/`· package` saem. Frameworks continua derivado da TFM, Packages mostra nome+versão, Projects
  usa o ícone do projeto referenciado.
- `src/solutionVisuals.ts`: sem mudança de regra; `frameworkRows` já alimenta o grupo.
- Testids: `tree-deps-group-<projeto>-<grupo>` (novo), preservados `tree-deps-<projeto>`,
  `tree-framework-*`, `tree-package-*`, `tree-ref-*`.

## Testes

- `tests/e2e/solution-explorer.spec.ts` (Restricted/outline): grupos Frameworks e Projects visíveis, Packages
  ausente no Cli, framework `Microsoft.NETCore.App (net8.0)` e ref `OrderDesk.Core` abrem.
- `tests/e2e/evaluation.spec.ts` (Trusted/avaliado): pacote `Microsoft.NET.Test.Sdk 17.11.1` dentro de
  Packages, sem o sufixo antigo; ref dentro de Projects.
- `bun run check`/`bun run build`/`bun test` continuam valendo; E2E não rodado (porta ocupada).
