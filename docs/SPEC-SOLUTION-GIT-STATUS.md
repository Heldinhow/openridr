# SPEC — Solution Explorer: status Git nas linhas e marcador de symlink

Issue-mãe: [#49](https://github.com/Heldinhow/openridr/issues/49) — "Solution Explorer: Git status colors and symlink markers".

## Problem Statement

A Solution Explorer mostra a solução como uma árvore de texto e ícones, sem nenhuma leitura do Git. Quem está com três arquivos modificados, um novo e um ignorado não vê nada disso na árvore: precisa abrir o painel Commit e comparar caminho por caminho para descobrir o que mudou. O Rider pinta o nome do arquivo pelo status do VCS, e é essa a informação que falta aqui.

Symlink é o outro buraco: um arquivo ou pasta que é link aparece igual a um arquivo ou pasta comum. Pior, uma pasta que é link é listada como pasta e pode ser expandida — o que segue o alvo sem o usuário saber, podendo sair do workspace ou entrar em ciclo.

## Solution

Cada **linha de arquivo** da árvore passa a mostrar o status Git do caminho: `modified`, `added`, `untracked`, `ignored` ou `conflicted`. O status vem do snapshot de `git.status` que o app já mantém, com o caminho mapeado da raiz do repositório para a raiz do workspace; o `ignored` de verdade vem de uma consulta nova ao `git check-ignore` para os caminhos visíveis. A cor entra no nome do arquivo e uma tag curta com a palavra do status carrega a mesma informação sem depender de cor; a linha também expõe o status em atributo e em descrição para leitor de tela. O status se atualiza depois de commit, checkout, criação de branch, pull e push (que já atualizam o snapshot) e quando a janela volta ao foco.

Um arquivo ou pasta que é **symlink** ganha um marcador no glifo, e uma pasta symlink deixa de ser expansível: a árvore mostra que é link e não segue o alvo. Isso vale para o modo normal (fontes avaliadas) e para o modo "Show all files", e o host confina o diretório real que contém o link, nunca resolvendo o alvo para descobrir que é link.

## User Stories

1. Como desenvolvedor, quero ver na árvore quais arquivos estão modificados, para saber o que mudou sem abrir o painel Commit.
2. Como desenvolvedor, quero distinguir um arquivo novo (added) de um modificado, para revisar antes de commitar.
3. Como desenvolvedor, quero ver arquivos untracked na árvore, para não esquecer de adicioná-los.
4. Como desenvolvedor, quero ver arquivos ignorados pelo `.gitignore` marcados, para entender por que eles não aparecem no painel Commit.
5. Como desenvolvedor resolvendo conflito, quero ver o arquivo marcado como `conflicted`, para achar o que precisa de atenção primeiro.
6. Como desenvolvedor, quero que o status continue correto depois de commitar, para não olhar uma árvore desatualizada.
7. Como desenvolvedor, quero que o status continue correto depois de trocar de branch ou dar pull, para confiar na árvore ao voltar para ela.
8. Como desenvolvedor que faz commit/checkout no terminal, quero que a árvore se atualize quando volto para a janela do app, para não precisar clicar em Refresh.
9. Como desenvolvedor com uma solução que fica numa subpasta do repositório, quero o status dos arquivos da solução mesmo assim, para não perder a leitura por causa do layout do repo.
10. Como desenvolvedor com mudanças em outros projetos do mesmo repositório, quero que a árvore mostre só o que está dentro do workspace, para não poluir a solução aberta.
11. Como usuário de leitor de tela, quero ouvir o status do arquivo na descrição da linha, para ter a informação sem depender de cor.
12. Como usuário com baixa visão, quero uma tag textual além da cor, para ler o status mesmo sem distinguir as cores.
13. Como usuário de teclado, quero que a linha continue sendo um único alvo focável com o status anunciado, para navegar sem paradas extras.
14. Como desenvolvedor, quero ver que um arquivo é symlink, para não confundir o link com o arquivo apontado.
15. Como desenvolvedor, quero ver que uma pasta é symlink e não conseguir expandi-la, para a árvore não seguir alvo desconhecido.
16. Como desenvolvedor, quero que um symlink para fora do workspace seja apenas marcado, e nunca lido pela árvore, para não abrir caminho para fora do projeto.
17. Como desenvolvedor, quero que um symlink pendente ou em ciclo seja marcado sem derrubar a árvore, para continuar navegando.
18. Como desenvolvedor em modo restrito, quero ver o marcador de symlink (que é só metadado), sem status Git (que exige confiança), para saber o que estou olhando sem executar nada.
19. Como mantenedor, quero o mapeamento repositório→workspace como função pura testada, para provar subpasta, caminho fora e estados combinados.
20. Como mantenedor, quero o `check-ignore` em lote e limitado, para não spawnar um Git por linha nem crescer sem teto.
21. Como mantenedor, quero que a árvore descarte resposta velha de refresh, para uma troca de workspace ou de trust não pintar status de outro projeto.
22. Como testador, quero o caminho real (repo descartável + app Tauri) coberto por spec noturno e por prova no app, para não aprovar por unidade só.

## Implementation Decisions

- **Escopo de linha: só arquivos.** Linhas de arquivo — fontes avaliadas no modo normal e arquivos listados no modo "Show all files" — recebem status. Pasta e projeto **não** agregam status de filhos nesta rodada; `no index` continua sendo a heurística existente ("não é fonte avaliada"/dotfile) e não muda de significado.
- **Fonte única do status.** O snapshot `git.status` que o `WorkspaceView` já carrega é a fonte; a árvore recebe o snapshot por prop e não faz uma chamada por arquivo. `git.status` continua trust-gated e não muda de contrato.
- **Ignorado é `.gitignore` de verdade.** Método novo `git.ignored { paths }` (trust-gated, como todo `git.*`): recebe caminhos relativos ao workspace **visíveis** na árvore, em lotes de no máximo 256, e devolve `{ state, message, ignored }` — `ignored` com os caminhos pedidos que o Git ignora, `state`/`message` com o mesmo vocabulário de dados do `git.status` (`not_a_repository`, `git_missing`, `failed`). A fonte é `git status --porcelain=v1 -z --ignored=matching -- <paths>` com argv fixo (sem shell, sem stdin), teto de entrada (256 caminhos, 64 KiB de pathnames) e de saída (256 KiB); um diretório ignorado (`build/`) responde pelos arquivos abaixo dele. `check-ignore` foi descartado porque `-z` exige `--stdin` e ele reporta arquivo rastreado como ignorado (não consulta o index): excluir isso exigiria um segundo spawn (`ls-files`). `no index` (heurística) e `ignored` (Git) são coisas diferentes e podem coexistir na mesma linha.
- **Estados e precedência.** O status da linha é derivado do par `index`/`worktree` que o `git.status` já devolve, com precedência `conflicted` > `untracked` > `added` > `modified`; `ignored` só se aplica a caminho que o `git.status` não reporta (um arquivo ignorado não aparece no porcelain) e que o `check-ignore` marca. Tabela normativa:

  | `index`/`worktree` | status da árvore |
  |---|---|
  | par de conflito (`UU`, `AA`, `DD`, `AU`, `UA`, `DU`, `UD`) | `conflicted` |
  | `??` | `untracked` |
  | `index = A` (fora de conflito) | `added` |
  | `M`, `T`, `R`, `C` (index ou worktree) | `modified` |
  | caminho fora do `git.status` e marcado pelo `check-ignore` | `ignored` |

  O rótulo granular que o `git.status` já calcula (`Staged`, `Staged + modified`, `Renamed`, `Type changed`…) não vira cor nova: ele fica no `title` e na descrição da linha. Arquivo deletado não aparece na árvore (não existe no disco), então `deleted` não é um estado da árvore.
- **Mapeamento repositório→workspace.** Função pura: caminho do `git.status` é relativo à raiz do repositório (`root` do snapshot); o caminho da árvore é relativo à raiz do workspace. O mapeamento monta o absoluto pela raiz do repo e o torna relativo ao workspace; caminho que cai fora do workspace é descartado. Quando as duas raízes coincidem (caso da fixture) o resultado é o próprio caminho. O gutter do editor e `git.diff` **não** mudam nesta rodada; a limitação equivalente que existe lá fica registrada como limite conhecido.
- **Marcador sem depender de cor.** A linha de arquivo ganha uma tag curta com a palavra do status (`modified`, `added`, `untracked`, `ignored`, `conflicted`), o atributo `data-git-status` (contrato de teste) e uma descrição AX (`Git status: Modified`) separada do nome acessível, no mesmo padrão de `aria-describedby` que o `TreeRow` já usa. A cor é um token novo aplicado **só ao nome** (e à tag); o fundo âmbar de `no index`, a seleção e a densidade atual da linha não mudam. Nenhum redesign da árvore.
- **Contraste medido.** Os tokens de status são escolhidos e medidos sobre `--panel`, hover, linha selecionada/focada e a linha `no index`, com alvo mínimo de 4.5:1 para texto; os valores e as medições vão para a evidência, como já foi feito para os ícones. Se um tom do Rider não passar, o token é ajustado — a paridade visual não vence a acessibilidade.
- **Refresh sem watcher.** Os gatilhos existentes (commit, checkout, criação de branch, pull, push já chamam o refresh do snapshot) continuam valendo. Entra também refresh no evento de foco da janela, com debounce/dedupe (uma chamada em voo por vez) e guarda de identidade: a resposta só é aplicada se o workspace e o estado de trust ainda forem os do início da chamada; resposta velha é descartada. Sem watcher de arquivo, sem polling, sem dependência nova.
- **Symlink no host, sem seguir o alvo.** `workspace.tree` passa a devolver `symlink: bool` por entrada, lido de `symlink_metadata` do próprio caminho (nunca de um resolvedor que siga o alvo). Entra um método novo, read-only e **sem trust** (como `workspace.tree` e `workspace.symbolKinds`), `workspace.symlinks { paths }`, em lotes de no máximo 256, que devolve `{ entries: [{ path, symlink }] }` para os caminhos que existem; caminho inválido ou fora do workspace é omitido, sem oráculo de existência. O confinamento é feito sobre o **diretório real que contém** o link (o pai), nunca resolvendo o link; link pendente, link em ciclo e link para fora do workspace são respondidos como `symlink: true` e nada mais é lido.
- **Symlink na UI: marcado e não seguido.** Linha de arquivo ou pasta com `symlink` ganha um marcador no glifo (badge pequeno, no padrão do sticker de símbolo), `data-symlink="true"` e descrição AX ("Symbolic link — not followed"). Pasta symlink **não tem chevron e não expande**: a listagem de um nível continua vindo do host, mas a UI nunca pede o nível de uma entrada `symlink`, então traversal e ciclo não acontecem pela árvore. O marcador aparece nos dois modos (fontes avaliadas e "Show all files").
- **Trust.** Status Git, tags e `git.ignored` só em modo Trusted (não nasce um segundo fluxo de confiança). Marcador de symlink é metadado read-only e aparece também em Restricted, com o mesmo confinamento.
- **Sem dependência nova.** Cores em tokens no `:root` com o fallback escuro atual; glifo de link vem do conjunto Lucide já instalado; nenhum pacote novo de UI ou runtime.
- **Contrato de atributos.** `data-testid` existentes não são renomeados nem removidos. Novos contratos: `data-git-status` (`modified|added|untracked|ignored|conflicted`) e `data-symlink="true"` nas linhas; nenhum `data-testid` novo é necessário para isso.

## Testing Decisions

- **O que é um bom teste aqui:** ele observa o que o usuário vê — a tag/atributo na linha, o marcador de link, a pasta que não expande, o status que muda depois da operação — ou o mapeamento puro. Nada de afirmar estrutura interna que só existe para derivar a decisão.
- **Unidade (`bun test`, `tests/unit/`):** função pura de status com (a) raiz do repo igual à do workspace; (b) workspace em subpasta do repo; (c) caminho do status fora do workspace → descartado; (d) `conflicted`, `untracked`, `added`, `modified`, `staged+modified`; (e) ignorado que não está no status; (f) precedência quando o mesmo caminho casa mais de uma regra; (g) rótulo granular preservado para o `title`; (h) caminho com separador Windows normalizado. Segue o padrão de `solutionNodes.test.ts`/`commitDiff.test.ts`.
- **Host (`cargo test -p openridr-host`):** `workspace.symlinks` e o `symlink` de `workspace.tree` com link para arquivo, link para pasta, link pendente, link em ciclo (pasta que aponta para ancestral), link para fora do workspace e caminho comum; `workspace.symlinks` sem trust, confinado, com lote limitado e caminho fora omitido. `git.ignored` com arquivo ignorado por padrão, arquivo rastreado (não ignorado), caminho com espaço e com newline (prova do `-z`), fora do repositório e repositório ausente (estado de dado, não pânico). Testes de host seguem o harness hermético atual.
- **E2E (`tests/e2e/solution-git-status.spec.ts`, spec novo, roda só no smoke noturno):** repo descartável criado pelo teste, como em `git.spec.ts`: modificar/commitar e trocar de branch → a tag do arquivo muda sem recarregar; arquivo novo → `untracked`; arquivo coberto pelo `.gitignore` → `ignored`; symlink de arquivo e de pasta criados no repo → marcador presente e pasta sem chevron/expansão. Não é rodado nesta sessão (decisão #67: Playwright só smoke noturno); entra na lista de verificação como "escrito, não executado".
- **App real:** `agent-desktop` no app Tauri sobre cópia em `/tmp` da fixture, uma instância, depois da liberação da instância desktop pelo orquestrador (#50 tem prioridade). Prova: status na árvore, tag presente, atualização depois de commit/checkout e marcador de symlink. O que o ambiente não permitir fica como "não verificado", com o motivo.
- **Seams.** Host (`workspace.symlinks`, `git.ignored`, `workspace.tree.symlink`) para os dados; função pura para o mapeamento; DOM da árvore (`data-git-status`/`data-symlink`) para o efeito. Nenhum seam novo de UI além do `SolutionTree`/`TreeRow` que já existem.

## Out of Scope

- Nó **Imports** e qualquer árvore de `using`/namespaces — citado no problema da issue, fora do aceite; não abre follow-up nesta rodada.
- Agregar status em pasta/projeto, contagem de mudanças no nó, ou estado de pasta fechada.
- Mudar o gutter do editor, `git.diff`/`git.fileDiff` ou o mapeamento de caminho deles.
- Status em abas do editor, Recent Files, Find in Files ou Quick List.
- Watcher de arquivo, polling, ou atualização automática fora do foco da janela.
- Mudar a heurística `no index`, `IGNORED_DIRS` ou o que a árvore lista.
- Redesign da árvore, mudança de densidade, ícones novos de tipo de arquivo.
- Windows e macOS Intel: não executados; não são declarados suportados.

## Further Notes

- O par `git.status` + `git.ignored` cobre o aceite sem inventar estado: `conflicted`/`untracked`/`added`/`modified` vêm do porcelain; `ignored` vem do `check-ignore`, porque o porcelain não lista ignorados.
- A tag textual é a decisão de acessibilidade: a palavra é a mesma do estado, então não há legenda a aprender, e a cor é reforço. O `title` guarda o rótulo granular do commit tool (`Staged + modified` etc.).
- Symlink é tratado como objeto link em todo caminho: o host faz `symlink_metadata` do próprio caminho e confina o pai; nada no fluxo resolve o alvo. A janela TOCTOU entre o `stat` e uma eventual leitura por outro método continua existindo e fica declarada, como já foi registrada para o diff.
- Limites conhecidos que entram na retro: status só de arquivos; mudanças externas só aparecem no foco (ou no Refresh do painel Git); pasta symlink não pode ser navegada pela árvore; repositório aninhado dentro do workspace usa a raiz do repositório que o `git.status` abriu a partir do workspace.
