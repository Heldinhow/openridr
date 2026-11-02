# 05 — Search Everywhere: duplicatas, ranking, abas e consulta (b6, b7)

**Fonte:** `docs/SPEC-MEDIUM-NAV-SEARCH.md` §8. **Achados:** QA-BUGHUNT b6, b7.
**Bloqueado por:** nenhum. **Status:** ready-for-agent.

## Objetivo

Uma linha por resultado em Text/All, ranking que coloca nome exato no topo, Symbols
coerente com All e reabertura em All com a consulta anterior selecionada.

## Pontos de código

- `src/actions/FindAction.tsx`:
  - **b6:** montar `textHits` agrupando `file.matches` por `line` (chave
    `text:${path}:${line}`, primeira coluna da linha). `total` conta linhas agrupadas.
  - **b7 ranking:** criar funções de score (nome de arquivo exato < símbolo exato < prefixo
    de arquivo < prefixo de símbolo < ação < substring de arquivo < substring de símbolo <
    texto < arquivo que casa só pelo diretório). Em All, concatenar nessa ordem e limitar o
    texto a 50 entradas (`ALL_TEXT_LIMIT`) para não inundar; `ALL_LIMIT` continua 300.
    Dentro do mesmo score, ordenar por título e path.
  - **b7 abas:** a aba Symbols passa a listar tipos **e** membros (remover o filtro
    `!TYPE_KINDS.has`); Classes continua só tipos. Remover `matchingTypes` e o botão
    `search-suggest-classes` (o caso que ele cobria deixa de existir).
  - **b7 rótulo:** `search-kind` mostra `class` para `TYPE_KINDS`, `symbol` para membro,
    `file`, `action`, `text`.
  - **b7 estado:** abrir sempre em **All** quando `initialTab === "last"`; manter a consulta
    salva, mas selecionar o texto do input no mount (`inputRef.current?.select()`), para
    digitar substituir. Remover a persistência da aba (só a consulta fica).
- `src/WorkspaceView.tsx`: `openSearch("last")` → `openSearch("all")` nos dois call sites
  (ação e duplo Shift), se o ticket 03 ainda não tiver feito.
- Host não muda.

## Teste primeiro

Criar `tests/e2e/medium-nav-search.spec.ts` (host real, `fixtures/OrderDesk`):

1. `Meta+t`, aba Text, buscar `Order`: nenhum par `path:line` repetido nos detalhes dos itens.
2. Aba All, buscar `Order`: o primeiro item é `Order.cs` (nome exato) e vem antes de
   `OrderDesk.sln`; resultados de texto não ocupam o topo.
3. Aba Symbols, buscar `Order`: a classe `Order` aparece (sem `search-empty`/
   `search-suggest-classes`); em Classes ela também aparece.
4. Rótulos: um item de tipo mostra `class`; um membro mostra `symbol`.
5. Reabrir: buscar `Order`, Esc, `Meta+t`; input com `Order` **selecionado** (selectionStart 0,
   selectionEnd 5) e aba All; digitar `x` deixa o valor `x` (não `Orderx`).

Atualizar `tests/e2e/search.spec.ts`:
- o teste "last tab and text persist" passa a afirmar All + consulta selecionada;
- o teste de símbolos deixa de esperar `search-suggest-classes` e afirma que `OrderDeskType`
  aparece na própria aba Symbols.

Rodar os dois specs e ver falhar antes de implementar.

## Aceite

- [ ] Sem duplicatas por linha em Text/All.
- [ ] Ranking estável com nome exato no topo; texto limitado em All.
- [ ] Symbols inclui tipos e membros; Classes só tipos; rótulos corretos.
- [ ] Reabre em All com a consulta selecionada.
- [ ] Testes alvo verdes; `bun run check` verde.
- [ ] Commit local `fix(search): dedupe results and rank exact names` (só caminhos deste ticket).
