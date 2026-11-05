# 02 — Linhas da árvore: chevron, guia e seleção

Status: implementado
Spec: [D3, D4, D5](../SPEC-SOLUTION-FILE-ICONS.md)

## Mudança

- `src/TreeRow.tsx`: `icon` vira opcional; o slot de 16 px é sempre renderizado (vazio quando não há
  ícone), então nome de pasta e de arquivo alinham. Uma linha com `level ≥ 1` desenha `level` spans
  `.tree-guide`, posicionados na coluna do chevron de cada ancestral (`8 + i*18 + 7`), um por nível.
- `src/SolutionTree.tsx`: pastas de fonte, pastas do `.sln` e pastas do Show all files perdem o ícone
  (`Icon name="folder"` / `solutionFolder`); projetos e a raiz mantêm o glifo de tipo/solução, e
  Dependencies/pacotes/frameworks não mudam.
- `src/styles.css`: `.sol-row` com `position: relative`, `.tree-guide` (1 px, `--line`, sem
  `pointer-events`), e `.tree-row.selected`/`[aria-current="page"]` com
  `box-shadow: inset 0 0 0 1px var(--accent)` (altura não muda).

## Aceite

Nenhum `data-testid` renomeado; `role="tab"` do cabeçalho inferior intocado; E2E da árvore existente
continua passando.
