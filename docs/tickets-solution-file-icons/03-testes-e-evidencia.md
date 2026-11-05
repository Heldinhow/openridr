# 03 — Testes e evidência

Status: unitário e E2E feitos; captura no app real aguarda liberação do orquestrador
Spec: [aceite 4–8](../SPEC-SOLUTION-FILE-ICONS.md)

## Mudança

- `tests/e2e/solution-file-icons.spec.ts` (novo): cópia temporária da fixture `OrderDesk` com
  `README.md`, `appsettings.json`, `Dockerfile`, `docker-compose.yml` e `.sln.DotSettings.user`
  acrescentados na cópia (a fixture versionada não muda); abre em Restricted, liga `tree-show-all` e
  verifica glifo por `[data-icon]`, pasta sem `<svg>`, contagem de `.tree-guide` por nível e contorno da
  linha atual.
- `tests/unit/solutionVisuals.test.ts`: mapeamento nome/extensão → kind → glifo.

## Portas

E2E sempre nas portas próprias desta sessão (`OPENRIDR_E2E_UI_PORT=1490`,
`OPENRIDR_E2E_HOST_PORT=4387`) para não disputar 1420/4317 com outra execução.

## Evidência

`docs/evidence/solution-file-icons/` (antes/depois). O app real (agent-desktop) fica pendente até o
orquestrador liberar a sessão que está usando o app instalado.
