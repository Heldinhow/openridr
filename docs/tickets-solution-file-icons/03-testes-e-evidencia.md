# 03 — Testes e evidência

Status: unitário, E2E e app real feitos
Spec: [aceite 4–8](../SPEC-SOLUTION-FILE-ICONS.md)

## Mudança

- `tests/e2e/solution-file-icons.spec.ts` (novo): cópia temporária da fixture `OrderDesk` com
  `README.md`, `appsettings.json`, `Dockerfile`, `docker-compose.yml` e `.sln.DotSettings.user`
  acrescentados na cópia (a fixture versionada não muda); abre em Restricted, liga `tree-show-all` e
  verifica glifo por `[data-icon]`, pasta sem `<svg>`, contagem de `.tree-guide` por nível e contorno da
  linha atual.
- `tests/unit/solutionVisuals.test.ts`: mapeamento nome/extensão → kind → glifo.
- Ticket 04 acrescentou a descrição acessível do tipo de símbolo e a medição de contraste do corpo dos
  glifos de dois tons (WCAG 1.4.11), os dois com teste no mesmo spec.

## Portas

E2E sempre nas portas próprias desta sessão (`OPENRIDR_E2E_UI_PORT=1490`,
`OPENRIDR_E2E_HOST_PORT=4387`) para não disputar 1420/4317 com outra execução.

## Evidência

`docs/evidence/solution-file-icons/`: `depois-*.png` (E2E, antes = capturas de
`solution-explorer-visual/` no commit base) e `desktop-0*.png` (app real, build desta branch com
`OPENRIDR_HOME` isolado e cópia da fixture com Dockerfile/json/md/gitignore).
