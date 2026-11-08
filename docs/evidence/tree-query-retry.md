# Evidência — consulta de árvore que falha é repetida com limite (#264, ticket #281)

Issue: [#264](https://github.com/Heldinhow/openridr/issues/264), ticket
[#281](https://github.com/Heldinhow/openridr/issues/281). Ambiente: macOS ARM64, worktree isolada,
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
  anteriores provaram fica pintado.
- A limpeza na troca de identidade do pedido (workspace, revisão do Git, caminhos visíveis) ficou
  como estava: resposta de identidade antiga nunca pinta a árvore.
- Nada mudou no host, nos `data-testid` ou nas dependências.

## Por que o helper é um módulo, e não local ao componente

A spec pedia um helper local e a prova só no E2E, porque o repo não tem teste de componente React.
**A medição desmentiu a premissa**: o componente re-consulta sozinho. A cada 400 ms o
`workspace.state` substitui o snapshot, `views` é recalculado, `visibleFilePaths` ganha identidade
nova, o memo do pedido devolve um objeto novo e o efeito pede de novo — medido com log no efeito:
4 consultas `git.ignored` por segundo no spec, com o pedido de identidade idêntica. O mesmo acontece
com `workspace.symlinks`; `workspace.symbolKinds` não entra no laço porque sua dependência é a
assinatura em texto, não o objeto.

Consequência para o teste: um E2E que só falhe a primeira chamada **não** distingue "repetiu" de
"pediu de novo no próximo render" — com o conserto revertido, o marcador volta assim mesmo. Por isso
o helper virou módulo e a política ganhou teste unitário (`tests/unit/treeQueryRetry.test.ts`), que é
a falha-antes determinística. O laço de re-consulta em si é anterior a esta frente e ficou fora de
escopo: registrado na
[#300](https://github.com/Heldinhow/openridr/issues/300) com a medição.

## Testes

- `tests/unit/treeQueryRetry.test.ts` (novo, 6 casos): resposta que chega não repete; falha repetida
  devolve o sucesso; lote que nunca responde é dado por perdido em 3 tentativas; pedido já
  substituído não repete; pedido substituído **durante** a espera não repete; e a política é
  `[200, 400]`.
- `tests/e2e/solution-git-status.spec.ts`: três testes novos, um por consulta, cada um perdendo a
  **primeira** resposta do método com `page.route("**/call")` e afirmando o marcador na linha
  (`data-git-status="ignored"` num fonte coberto pelo `.gitignore`, `data-symlink="true"` num
  `Linked.cs`, `data-symbol-kind="class"` num fonte novo). O helper do spec para o poll
  `workspace.state` e o refresh `git.status` durante o teste e espera 1 s de silêncio antes de mudar a
  identidade: sem isso, um render alheio pede de novo e o teste mede o poll, não a consulta.
- A identidade é trocada pelo caminho real: recolher/expandir o projeto (`.gitignore`), alternar
  "Show all files" (symlinks) e "Re-evaluate project model" (tipo C#).

## Falha-antes medida

- **Unitário (política).** Com `askTreeQuery` reduzido ao comportamento antigo (pergunta uma vez e
  devolve `null`), `bun test tests/unit/treeQueryRetry.test.ts` → **2 failed** (`a failed answer is
  asked again…` e `a batch that never answers is given up after three attempts`), 4 passed. Com o
  helper: 6 passed.
- **E2E `.gitignore` e symlink.** Com `src/SolutionTree.tsx` de `c35f60b` (sem o helper) e o spec
  novo: **2 failed** — `data-git-status` recebe `""` e `data-symlink` recebe `""` depois de 10 s. Com
  o conserto: 2 passed.
- **E2E tipo C#.** Com o conserto revertido ele **passa**: o badge de um fonte novo é pedido duas
  vezes (a reavaliação manual e a automática do modelo que "changed on disk"), então a segunda
  varredura pinta o badge sem repetição. Fica registrado como limite do teste: ele afirma o contrato
  visível (o badge não fica faltando), não a repetição; a repetição dessa consulta é provada pelo
  unitário, que é o mesmo helper das três.

## Bateria

- `bun test tests/unit` → 145 passed, 0 fail (19 arquivos), incluindo os 6 novos.
- `bunx tsc --noEmit` limpo.
- `bunx playwright test tests/e2e/solution-git-status.spec.ts` → 6 passed (1,6 min).
- `bunx playwright test tests/e2e/solution-git-status.spec.ts --repeat-each=8` → **48 passed**
  (12,9 min), sem o sintoma da #245 (1 falha em 42 execuções para cada um).

## Não verificado

- App Tauri real: a mudança é UI pura (nenhum menu nativo, atalho do sistema, updater, seletor
  nativo, árvore de acessibilidade do WebKit, single-instance ou DMG).
- A **causa** da resposta que não chegou continua não provada (transporte da ponte HTTP/`invoke` ou
  erro do host sob o lock único). O conserto muda o comportamento, não a causa — como a spec diz.
- A taxa do sintoma em produção: a #245 mediu 1 em 42 execuções no E2E, não em uso real.
- Windows e macOS Intel (não suportados sem evidência).
- O laço de re-consulta a cada poll (acima) não foi corrigido nesta frente; a
  [#300](https://github.com/Heldinhow/openridr/issues/300) leva a medição.
