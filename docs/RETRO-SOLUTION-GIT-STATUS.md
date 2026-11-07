# Retro — Solution Explorer: status Git e marcador de symlink (#49)

Sessão filha do orquestrador, worktree `feat/issue-49-solution-git-status`. Escopo entregue:
spec `docs/SPEC-SOLUTION-GIT-STATUS.md`, issues #107 (symlink), #108 (status por arquivo),
#109 (ignorado), #110 (verificação/evidência). Duas rodadas de revisão independente (r1 com
1 alto + 6 médios + 3 baixos; r2 com 2 médios; o alto foi fechado no re-check).

## O que funcionou e deve continuar

- **Grill antes de escrever a spec, com recomendação por item.** As onze perguntas (escopo de
  linha, ignorado real × `no index`, contrato de symlink, tag × sticker, raiz do repo × do
  workspace, trust, refresh) voltaram com amarrações que viraram decisões — e nenhuma delas foi
  reaberta na revisão. O caso mais caro, "solução em subpasta do repositório", já entrou na
  função pura com teste.
- **Fatia vertical por ticket, não por camada.** Cada ticket entregou host + UI + teste e ficou
  verificável sozinho (#107, #108, #109); o fatiamento por camada que eu tinha proposto no grill
  teria deixado três tickets sem prova até o fim.
- **Função pura para tudo que decide estado.** `solutionGitStatus` (mapa repo→workspace,
  precedência, merge de ignorados), `treeQueryId` (identidade de consulta em JSON) e
  `solutionListing` (cache por epoch) rodam em `bun test` sem renderer. As duas rodadas de revisão
  encontraram problemas de *invalidação*, e cada correção virou caso de unidade no helper que a UI
  usa — não em um teste de render inexistente.
- **`gitRefresh` como máquina de estados testável.** Extrair o "uma chamada em voo, um rerun,
  contexto relido após await" para um módulo com `fetch` controlado permitiu provar a ordem
  exata (clear antes da promessa resolver) que a revisão pediu, sem React.
- **Prova no app real com fixture pública e PID.** Bundle de debug do próprio HEAD,
  `OPENRIDR_HOME` isolado, fixture `OrderDesk` transformada em repo descartável em `/tmp`,
  `desktop.sh down` fechando pelo PID gravado. O par antes/depois do link (`02` × `04`) é o que
  preserva a correção do alto; as transições commit→foco e checkout→foco ficaram como relato
  porque não há artefato contínuo — declarado, não vendido como passe.
- **Falha-antes barata por reversão de trecho.** As correções de host (literal pathspec, NUL,
  exit code) foram provadas com nomes de arquivo que o Git interpretaria como magic/glob; nenhum
  `stash`, nenhum worktree novo.

## Achados de ambiente (ordenados por custo)

1. **`cargo test` com dois filtros: a documentação do repo e o comportamento divergem.** O
   `AGENTS.md` diz "para vários: `cargo test -p openridr-host -- filtro_a filtro_b`"; nesta
   máquina isso rodou 2 testes, mas a revisão tratou a forma como não reproduzível. O custo foi
   reescrever a evidência e reexecutar. *Melhoria*: fixar a forma segura no `AGENTS.md` ("uma
   invocação por filtro") ou validar a forma com a versão de cargo do repo, para a próxima sessão
   não escolher por conta própria.
2. **A suíte Rust completa é sensível a carga de outras sessões.** Duas falhas de LSP
   (`lsp_loads…`, `lsp_completion…`) apareceram em uma rodada com a máquina ocupada e passaram
   isoladas; a rodada seguinte, livre, foi 100/100. Já está em
   `docs/evidence/test-stability.md`, mas o custo de diagnosticar de novo é real. *Melhoria*: o
   harness poderia detectar "outro `openridr`/cargo rodando" e avisar antes do run completo, ou a
   evidência registrar a carga esperada junto da medição.
3. **`agent-desktop screenshot` usa argumento posicional, não `--path`.** O primeiro comando
   falhou com `INVALID_ARGS`; o `SKILL.md` do verify-openridr mostra `screenshot` sem exemplo de
   arquivo. *Melhoria*: uma linha no `test-app-agent-desktop` com o comando exato de captura para
   arquivo.
4. **Rebuild do bundle de debug reexecuta o setup gate.** Depois de `tauri build --debug`, o app
   abriu em "Setting up…" por alguns segundos antes da árvore; esperar o snapshot pronto (em vez
   de assumir que a janela já está no workspace) evitou um falso "não abriu". *Melhoria*: a
   receita do `desktop.sh`/skill pode dizer que o primeiro snapshot depois do rebuild pode cair no
   SetupGate.
5. **Nenhum guardrail pega "assinatura por join/split de newline".** O achado médio 5 (nome com
   newline virando separador) passou por `tsc` e pelos testes porque o teste original não tinha um
   nome com newline. Não há lint que pegue isso; o que existe é a convenção de identidade em JSON.
   *Melhoria*: um teste-guard no repo (como o `env_writes_are_confined_to_isolated_leaves`) que
   procure `join("\n")`/`split("\n")` em assinaturas de consulta seria barato — mas hoje não há
   categoria no `AGENTS.md` para isso; fica como sugestão, não como decisão.

## As rodadas de revisão

| Rodada | Achados | Correção/prova | Limite que ficou |
|---|---|---|---|
| r1 (`a1a9f48`) | 1 alto: `list_dir` chamava `is_dir()` e seguia o alvo do link, inclusive fora e em Restricted. 6 médios: ignored não renovava; rerun pendente perdido no refresh; pathspec/NUL/exit do `git.ignored`; newline em assinatura; symlink de outro contexto; comandos de teste não reproduzíveis. 3 baixos: `check-ignore` documentado errado; smoke/visual | `DirEntry::file_type` e link como objeto (sem stat do alvo); `gitRevision` + `treeQueryId` nas consultas com cache limpo; `gitRefresh` com contexto relido; `--literal-pathspecs` + só registros terminados em NUL + qualquer exit ≠ 0 = `failed`; arrays em vez de join/split; workspace na consulta de symlink; comandos separados; justificativa do `check-ignore` corrigida; spec no smoke e capturas declaradas estáticas | Suíte completa não repetida depois das correções; E2E não rodado; integração React com resposta atrasada provada só no helper |
| r2 (`23db593`) | 2 médios: `clear` de trust esperava o fetch pendente; listing do Show All Files nunca invalidado por revisão/contexto | `run()` limpa antes do early-return e mantém o rerun enfileirado; `solutionListing` com epoch, reload só dos níveis abertos, resposta velha descartada | Render/React não medido; GUI não reaberta (sem mudança visual) |
| re-check | alta fechada (`file_type`/`symlink_metadata` sem stat do alvo) | — | — |

## O que ficou fora e por quê

- **Nó `Imports`** citado no problema da issue: o aceite só pede status e symlink; registrado como
  não-objetivo na spec, sem follow-up aberto.
- **Agregação de status em pasta/projeto**: precisaria decidir precedência de filhos e custo em
  pasta fechada; ficou fora da rodada.
- **Gutter/`git.diff` com a mesma limitação de raiz do repo**: o mapeamento novo é só da árvore;
  o gutter continua comparando caminho direto. Limite declarado.
- **Watcher de arquivo**: mudança externa aparece no foco ou no Refresh; sem polling, como
  aprovado.
- **Pasta symlink não navegável**: consequência de nunca seguir o alvo; os fontes avaliados abaixo
  dela não aparecem na árvore.
- **`ignored` durante filtro da árvore**: a tag some até o filtro ser limpo (o status Git fica).
- **Prova de render**: não há renderer de teste no repo; o descarte de respostas atrasadas é
  provado nos helpers, não na montagem React.
- **E2E**: `tests/e2e/solution-git-status.spec.ts` entrou em `scripts/e2e-smoke.txt`, mas não foi
  executado nesta sessão (decisão #67).
- **Windows e macOS Intel**: não executados.
