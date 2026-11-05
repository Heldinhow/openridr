# 03 — Tipo de símbolo C# por arquivo

Status: ready-for-agent
Spec: [D7](../SPEC-SOLUTION-EXPLORER-VISUAL.md)

## Mudança

- `crates/host/src/outline.rs`: parser conservador `symbol_kind(text, stem) -> Option<&'static str>`.
- `crates/host/src/lib.rs`: dispatch `workspace.symbolKinds` + `workspace_symbol_kinds` (read-only, sem trust,
  confinado ao workspace, 256 arquivos, 256 KB por arquivo).
- `src/SolutionTree.tsx`: busca uma vez por conjunto de fontes, cacheia no estado, passa `symbolKind` ao
  `FileIcon`; erro/limite cai no glifo C#.
- `src/host/client.ts`: tipos do método (não é snapshot).

## Testes

- Rust unitário: classe, interface, enum, record, struct, `record class`, comentário, stem preferido,
  arquivo sem declaração; contenção/limite no dispatch.
- `tests/unit/solutionVisuals.test.ts` cobre o mapeamento tipo→badge.

## Aceite

`OrderDesk.Core/Order.cs` (classe) mostra `C`; `OrderDesk.Tests` mostra o que as declarações reais disserem;
um `.cs` sem declaração reconhecida mantém o glifo C#. `cargo test -p openridr-host` verde nos testes novos.
