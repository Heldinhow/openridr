# 01 — Tirar o chrome de avaliação da árvore

Status: implemented — verificação visual/E2E pendente de liberação da porta
Spec: [D1, D2, D3](../SPEC-SOLUTION-EXPLORER-VISUAL.md)

## Problema

A Solution Explorer mostra `MSBuild evaluated`, `Evaluate project model`, `Evaluated · revision N`,
`Project details` e dois rodapés de diagnóstico. Rider não mostra nada disso na árvore.

## Mudança

- Remover do `<nav className="side">` a tag do cabeçalho, a div `.eval-controls` e os dois
  `p.tree-footnote`.
- `eval-run` → botão-ícone de refresh na barra de status (`StatusBar`), antes do segmento de avaliação,
  com o mesmo gate (`evaluationAvailable && !evaluationBusy`) e o mesmo testid.
- `eval-state` → passa a ser o filho do `span[data-testid="status-evaluation"]`, mantendo o texto completo
  (`Evaluated · Configuration … · TFM … · revision N`), para `revisionOf`/`run.spec` continuarem lendo.
- `Project details` → só no menu ⋮ (`tree-more-details`); remover o botão do sidebar.
- Manter `eval-disabled-reason`, `eval-stale`, `eval-project-error-*` e os avisos de restore (D3).
- Remover CSS morto (`.side .eval-controls`, `.side .panel-title .tag` se ficar sem uso).

## Testes

- Atualizar specs que dependiam do texto na árvore: `evaluation.spec.ts` (MSBuild evaluated), `workspace.spec.ts`
  (Text outline), e os que clicam "Project details" (`evaluation.spec.ts`, `ui-layout.spec.ts`) para abrir o ⋮.
- Nenhum testid renomeado/removido.

## Aceite

`bun run check` passa. `eval-run` e `eval-state` continuam clicáveis/legíveis; a árvore não tem os textos
listados; detalhes do projeto abrem pelo ⋮.
