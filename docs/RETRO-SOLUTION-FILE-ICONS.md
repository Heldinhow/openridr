# Retro — ícones da Solution Explorer (referência T3 Code)

Spec: [SPEC-SOLUTION-FILE-ICONS](SPEC-SOLUTION-FILE-ICONS.md). Tickets:
[tickets-solution-file-icons/](tickets-solution-file-icons/README.md) (01–03).

## O que funcionou

- A referência visual tinha procedência clara (árvore do T3 Code, MIT) e o orquestrador apontou o código:
  ler `pierre-icons.ts`/`PierreEntryIcon.tsx` pela web deu o mapeamento e a paleta exatos, em vez de tentar
  adivinhar hex a partir de um PNG. Amostrar o PNG (Pillow) confirmou que a captura é o **tema escuro** do
  T3 Code, então os valores escuros não foram invenção.
- Vendorizar só os sete símbolos do `@pierre/trees` (Apache-2.0) + o `C#` do Simple Icons (CC0) manteve a
  regra de "sem dependência nova" e a de "nada de Rider/Zed".
- Separar `fileKind` (tipo) de `fileGlyph` (glifo) deixou o mapeamento testável sem DOM e permitiu que
  `.sln`/`.csproj`/`.DotSettings` compartilhassem um glifo sem perder o `data-file-kind` que já existia.
- O E2E novo com cópia temporária da fixture (README/Dockerfile/json criados na cópia) prova o resultado
  real da árvore sem sujar a fixture versionada, nas portas 1490/4387.

## O que deu errado / o que aprender

- Primeira versão do spec de E2E assumia `tree-dir-src`: no Show all files o `SolutionTree` **esconde** os
  diretórios que são ancestrais (`ancestors`/`projectDirs`) e as pastas de projeto aparecem como conteúdo do
  projeto, não como linha. As linhas de pasta reais no Show all são `bin`/`obj`/`.idea`.
- `expect(row.locator("svg")).toHaveCount(0)` não serve para "pasta sem ícone": o **chevron** também é um
  `<svg>`. O contrato certo é `.tree-icon` vazio (ou ausência de `[data-icon]`).
- Guia de indentação é `level`, não `level - 1`: o filho da raiz (projeto) já tem uma guia, porque a raiz é
  um nó de verdade na nossa árvore (o T3 Code não tem raiz).
- O `.csproj` do próprio projeto e o `.sln` da raiz nunca aparecem como linha de arquivo no Show all
  (`skip`), então o glifo de documento só tem prova unitária — anotado como não verificado no app.
- O `bundled/netcoredbg` semeado no app home **compartilhado** do E2E por uma sessão anterior faz
  `workspace.spec.ts:94` falhar esperando o texto do caminho não embutido. Não é o diff, mas custou tempo
  até ficar claro; portas próprias não isolam o app home.

## Dívida e limites

- O selo de tipo em 9 px é pequeno por desenho; quem precisa do tipo lê `title`/`data-symbol-kind`. Se o
  Helder quiser o tipo mais visível, é uma mudança de CSS de uma linha (tamanho do selo), não de estrutura.
- Projetos mantêm ícone de tipo, diferente da referência (lá são pastas). Decisão registrada em
  `decisions.md`; reverter é remover `icon={<ProjectIcon/>}` de `projectRows`.
- Tema claro segue não distribuído e não verificado; os hex claros do T3 Code ficam em 3–4:1 (mínimo de
  ícone), abaixo dos 4,5:1 dos antigos tokens claros.
- Captura no app real (agent-desktop) pendente de liberação; E2E prova a árvore no Chrome, não o WebKit do
  Tauri.

## Resultado

- `bun test tests/unit`: 28/28; `tsc --noEmit` limpo.
- `tests/e2e/solution-file-icons.spec.ts`: 3/3 (portas 1490/4387).
- Regressão: `solution-explorer`, `native-context-menu`, `ui-layout` 21/21; `medium-nav-layout` +
  `workspace` 20/21 (a falha é o app home compartilhado, acima).
- Evidência: [evidence/solution-file-icons.md](evidence/solution-file-icons.md) e
  `docs/evidence/solution-file-icons/*.png`.
