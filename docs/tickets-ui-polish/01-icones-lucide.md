# 01: Lucide e glifo C# no lugar dos paths à mão

**What to build:** `src/Icon.tsx` passa a renderizar componentes do `lucide-react` (import por ícone) mantendo a API `<Icon name="…"/>` e os 38 `IconName`. Traço renderizado de 1,5 px (`absoluteStrokeWidth`) em 16 e 18 px. O glifo C# do MDI (`language-csharp`, Apache-2.0) entra como 1 path em `src/icons/` com sua licença ao lado. Mapeamento: tabela do spec; os nomes marcados † são conferidos no pacote instalado e os que não existirem são trocados por equivalente do mesmo conjunto, registrado na nota.

**Blocked by:** aprovação explícita de `lucide-react` (AGENTS.md: sem dependência nova sem decisão registrada). Alternativa aprovável: vendorizar os SVGs de `lucide-static`.

**Status:** não iniciado.

- [ ] Decisão D7 registrada em `docs/decisions.md`.
- [ ] `src/Icon.tsx` sem nenhum path `d` escrito à mão (exceto o glifo C# vendorizado).
- [ ] Todos os 38 nomes mapeados; nenhum nome † inexistente no pacote.
- [ ] Traço renderizado de 1,5 px medido em crop de 16 e 18 px.
- [ ] Alvos de ícone ≥ 32×32 (inclui a toolbar da árvore, hoje 22×22).
- [ ] Bundle antes/depois medido (`bun run build`); custo por ícone registrado.
- [ ] `bun run check` e `bun run build` passam; `data-testid` e `role="tab"` intactos.
- [ ] Capturas antes/depois em `docs/evidence/ui-polish/`.
