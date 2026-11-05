# 04 — Correções da revisão: licença, acessibilidade e contraste

Status: implementado
Spec: [D6.1, D6.2 e aceite 3/6/7](../SPEC-SOLUTION-FILE-ICONS.md)

## Mudança

- `THIRD-PARTY-NOTICES.md`: copyright do T3 Code restaurado ao literal do LICENSE do repositório
  ("Copyright (c) 2026 T3 Tools Inc."). Seção do `@pierre/trees` ganhou a origem exata (npm
  `@pierre/trees@1.0.0-beta.6`, arquivo `dist/builtInIcons.js`, tabela símbolo → componente), a lista das
  modificações feitas nos SVGs (sprite removido, `bg`/`fg` → `glyph-page`, `opacity` inline → classe, cores
  fixas → `currentColor`) e o `NOTICE.md` do pacote reproduzido na íntegra, como o Apache-2.0 pede.
- `src/TreeRow.tsx`: prop nova `description`, renderizada num `.sr-only` **fora** do `<button>` e ligada por
  `aria-describedby` (`useId`). Fora do botão porque um nó interno também entra no nome acessível — o
  primeiro teste E2E falhou exatamente com "Order.cs C# class" como nome.
- `src/SolutionTree.tsx`: a linha de arquivo passa `description="C# <tipo>"` quando o símbolo é conhecido.
- `src/FileIcon.tsx`: o wrapper do hexágono é `aria-hidden="true"` (o `title` continua para o tooltip do
  mouse); o selo já era `aria-hidden`.
- `src/styles.css`: `.glyph-page` de `opacity: .5` para `.75` (5,31:1 no `--panel` contra 2,95:1 antes; pior
  caso 3,86:1 na linha atual focada) e `opacity: 1` em `prefers-color-scheme: light`, onde a transparência
  derrubaria para 2,34:1.
- `tests/e2e/solution-file-icons.spec.ts`: dois testes novos — descrição acessível ("C# class") com nome
  acessível intacto, e o piso de 3:1 medido nos tokens/opacidade do CSS real para `--panel`, `--raised` e
  `--select`.

## Aceite

`bun run check`, `bun test tests/unit` e os 5 specs de `solution-file-icons` verdes nas portas 1490/4387;
números de contraste registrados em [evidence/solution-file-icons.md](../evidence/solution-file-icons.md).
