# 03 — Tipo de símbolo C# por arquivo

Status: implemented — verificação visual/E2E pendente de liberação da porta
Spec: [D7](../SPEC-SOLUTION-EXPLORER-VISUAL.md)

## Mudança

- `crates/host/src/outline.rs`: parser conservador `symbol_kind(text, stem) -> Option<&'static str>`.
- `crates/host/src/lib.rs`: dispatch `workspace.symbolKinds` + `workspace_symbol_kinds` (read-only, sem trust,
  confinado ao workspace, 256 arquivos/256 KB por chamada, `known` por mtime e `truncated` explícito).
  O root é clonado e o lock é solto antes do I/O.
- `src/SolutionTree.tsx`: pede só fontes visíveis (projeto/pastas abertas), em lotes de 128, cacheia
  `kind`+mtime, mescla sem zerar e passa `symbolKind` ao `FileIcon`; erro/limite cai no glifo C#.
- `src/host/client.ts`: tipos do método (não é snapshot).

## Testes

- Rust unitário: classe, interface, enum, record, struct, `record class`, comentário, stem preferido,
  arquivo sem declaração; contenção/limite no dispatch.
- `tests/unit/solutionVisuals.test.ts` cobre o mapeamento tipo→badge.

## Aceite

`OrderDesk.Core/Order.cs` (classe) mostra `C`; `OrderDesk.Tests` mostra o que as declarações reais disserem;
um `.cs` sem declaração reconhecida mantém o glifo C#. `cargo test -p openridr-host` verde nos testes novos.
