# Evidência — consulta de árvore que falha é repetida com limite (#264, ticket #281)

Issue: [#264](https://github.com/Heldinhow/openridr/issues/264), ticket
[#281](https://github.com/Heldinhow/openridr/issues/281), irmã
[#300](https://github.com/Heldinhow/openridr/issues/300). Ambiente: macOS ARM64, worktree isolada,
portas 1920/4917 (home do E2E derivada da porta da UI), Chromium. Data: 2026-11-08.

## O conserto

As três consultas de marcador da Solution Explorer (`git.ignored`, `workspace.symlinks`,
`workspace.symbolKinds`) tratavam uma resposta que **não chegou** como resposta **vazia**: o
`catch { break }` abandonava o lote e o resto da lista, e as duas consultas que limpam o próprio
estado antes de pedir (`.gitignore` e symlinks) deixavam a linha sem marcador até algo alheio mudar a
identidade do pedido.

- `src/treeQueryRetry.ts` (novo): `askTreeQuery(ask, superseded, delays)` repete o lote que falhou
  até três tentativas no total (`TREE_QUERY_RETRY_DELAYS_MS = [200, 400]`), aborta sem repetir quando
  o pedido já foi substituído — antes e depois da espera — e devolve `null` quando desiste.
- `src/SolutionTree.tsx`: as três consultas chamam o helper; depois de um lote dado por perdido o laço
  **continua** (`continue`) em vez de `break`, então os lotes seguintes ainda rodam e o que os
  anteriores provaram fica pintado. A resposta malformada de um lote (o array que não é array, ou um
  elemento que não é objeto/string) é tratada como lote dado por perdido, não como exceção não
  observada.
- **O efeito das consultas segue o id do pedido, não o objeto.** Era o achado alto da revisão: o poll
  de `workspace.state` (400 ms) substitui o snapshot, `views` é recalculado, o memo do pedido devolve
  um objeto novo com o **mesmo** `treeQueryId` e o efeito re-rodava — cancelando a repetição que
  esperava e descartando os lotes que já tinham respondido. Com o efeito preso ao id, a consulta
  roda uma vez por identidade (workspace + revisão do Git + caminhos visíveis), que é o contrato que
  os comentários do arquivo já prometiam. Isso também fecha a
  [#300](https://github.com/Heldinhow/openridr/issues/300) (a árvore re-consultava a cada poll).
- A limpeza na troca de identidade do pedido ficou como estava: resposta de identidade antiga nunca
  pinta a árvore.
- Nada mudou no host, nos `data-testid` ou nas dependências.

## Por que o helper é um módulo, e não local ao componente

A spec pedia um helper local e a prova só no E2E, porque o repo não tem teste de componente React.
Com o efeito agora preso ao id, o E2E distingue "repetiu" de "pediu de novo" — mas o unitário continua
sendo a prova determinística da política (três tentativas, as esperas, o aborto por substituição), e o
helper é compartilhado pelas três consultas. O teste unitário custa 6 casos e não precisa de host,
dotnet nem navegador; o E2E fica com o contrato visível.

## Testes

- `tests/unit/treeQueryRetry.test.ts` (novo, 6 casos): resposta que chega não repete; falha repetida
  devolve o sucesso; lote que nunca responde é dado por perdido em 3 tentativas; pedido já
  substituído não repete; pedido substituído **durante** a espera não repete; e a política é
  `[200, 400]`.
- `tests/e2e/solution-git-status.spec.ts`: três testes novos, um por consulta, cada um perdendo a
  **primeira** resposta do método com `page.route("**/call")` e afirmando o marcador na linha
  (`data-git-status="ignored"` num fonte coberto pelo `.gitignore`, `data-symlink="true"` num
  `Linked.cs`, `data-symbol-kind="class"` num fonte novo). A identidade é trocada pelo caminho real:
  recolher/expandir o projeto (`.gitignore`), alternar "Show all files" (symlinks) e tirar/pôr o
  fonte de volta no projeto (tipo C#, cuja varredura só acontece numa reavaliação). Cada teste
  também conta as chamadas que carregam o caminho e exige **exatamente duas** — a resposta perdida e
  a repetição —, então uma varredura posterior não passa por repetição.
- O helper do spec recusa a primeira resposta e, no caso do tipo C#, também as varreduras seguintes
  (só a repetição, que chega em 200/400 ms, passa): sem isso uma segunda varredura pintaria o que a
  resposta perdida não pintou e o teste mediria outra coisa.
- **Teste da #300** (mesmo spec): com a árvore parada, três polls de `workspace.state` com a mesma
  identidade não podem pedir `git.ignored`/`workspace.symlinks` de novo; um `focus` (nova revisão do
  Git) pede os dois outra vez.

## Falha-antes medida

- **Unitário (política).** Com `askTreeQuery` reduzido ao comportamento antigo (pergunta uma vez e
  devolve `null`), `bun test tests/unit/treeQueryRetry.test.ts` → **2 failed** (`a failed answer is
  asked again…` e `a batch that never answers is given up after three attempts`), 4 passed. Com o
  helper: 6 passed.
- **E2E, com o efeito preso ao id e só a repetição revertida** (o "antes" da política): os **três**
  testes falham — `data-git-status` e `data-symlink` recebem `""` e `data-symbol-kind` recebe `""`
  depois de 10 s. Com a repetição: os três passam.
- **E2E, com o conserto inteiro revertido** (`SolutionTree.tsx` de `c35f60b`): dos quatro testes
  novos, **três falham** — symlink (8 chamadas em vez de 2), tipo C# (badge vazio) e #300 (5 chamadas
  em vez de 0) — e o de `.gitignore` passa, porque a re-consulta do poll pinta o marcador em ~400 ms
  e, no nível do marcador, ela é indistinguível de uma repetição; o teste da #300 é quem pega esse
  caminho. É por isso que o efeito preso ao id faz parte desta frente: sem ele a repetição é
  cancelada pelo render seguinte e o sintoma é mascarado, que é o critério "os marcadores já provados
  ficam pintados" do ticket.

## Bateria

- `bun test tests/unit` → 145 passed, 0 fail (19 arquivos), incluindo os 6 novos.
- `bunx tsc --noEmit` limpo.
- `bunx playwright test tests/e2e/solution-git-status.spec.ts` → 6 passed (47 s).
- `bunx playwright test tests/e2e/solution-git-status.spec.ts --repeat-each=8` → **48 passed**
  (sem o sintoma da #245, 1 falha em 42 execuções para cada marcador).

## Não verificado

- App Tauri real: a mudança é UI pura (nenhum menu nativo, atalho do sistema, updater, seletor
  nativo, árvore de acessibilidade do WebKit, single-instance ou DMG).
- A **causa** da resposta que não chegou continua não provada (transporte da ponte HTTP/`invoke` ou
  erro do host sob o lock único). O conserto muda o comportamento, não a causa — como a spec diz.
- A taxa do sintoma em produção: a #245 mediu 1 em 42 execuções no E2E, não em uso real.
- Windows e macOS Intel (não suportados sem evidência).
