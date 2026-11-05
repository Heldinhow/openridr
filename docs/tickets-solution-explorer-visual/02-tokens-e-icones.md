# 02 — Tokens e ícones por tipo

Status: ready-for-agent
Spec: [D4–D6, D8, D9](../SPEC-SOLUTION-EXPLORER-VISUAL.md)

## Mudança

- `src/solutionVisuals.ts` (puro): `fileKind(path)`, `projectKindOf(kind, textKind)`, `frameworkRows(tfms, web)`,
  `packageParts(label)`, `isGeneratedPath(path)`.
- `src/icons/SymbolIcon.tsx` (badge C/I/E/R/S), `src/icons/ProjectIcon.tsx` (glifo por tipo).
- `src/FileIcon.tsx`: extensão → glifo/tint; `symbolKind` opcional; `projectKind` opcional.
- `src/Icon.tsx`: novos nomes `book`, `web`, `layers`, `refresh` (para D2 do ticket 01 se necessário).
- `src/SolutionTree.tsx`: `View.textKind` vindo do outline; projeto com `ProjectIcon`; sem chips de kind/TFM;
  atributos `data-project-kind`/`data-project-tfms`/`data-file-kind`; Dependencies com framework por TFM e
  referência usando o ícone do projeto referenciado; `no-index` esmaece.
- `src/styles.css`: bloco `--icon-*` com fallback + `light-dark()` e regras de badge/tint/no-index.
- `crates/host/src/outline.rs`: `project_kind` reconhece `Microsoft.NET.Sdk.Web` (e `UsingMicrosoftNETSdkWeb`),
  atrás de test/antes de Exe.

## Testes

- `tests/unit/solutionVisuals.test.ts` com `bun test` (script `test:unit`).
- Ajustar specs que liam `console`/`net8.0`/`library`/`test` na linha para os atributos.
- Rust: teste de `project_kind` web.

## Aceite

Cada tipo do mapa da spec aparece com o ícone/cor certo; sem dependência nova; `bun run check` e
`bun test tests/unit` passam.
