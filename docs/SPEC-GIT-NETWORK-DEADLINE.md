# SPEC — Prazo em push e pull: rede não congela o host (issue #136)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#136](https://github.com/Heldinhow/openridr/issues/136) (*Related to #113*), dentro da frente #50 (aceitação de Git com remoto).

## Problema

`git.push` e `git.pull` rodam um comando de rede **sem prazo**. O `host_call` do shell serializa **todos** os dispatches do host atrás de um mutex global (`HOST_CALL`), então enquanto um push ou um pull espera um remoto lento ou inalcançável **nenhuma outra chamada do host roda**: `git.status`, tráfego de LSP, execução de testes, leitura de workspace. A UI continua pintada (as chamadas são assíncronas), então o app parece travado em vez de ocupado — e o painel Git, que iniciou a operação, é só a parte visível do problema.

A janela é limitada apenas pelo timeout TCP do próprio Git, que em algumas redes é de minutos, e não há como cancelar pelo app. O `fetch` da #113 já resolveu isso para si (prazo, kill do grupo de processos, mensagem em palavras do app); push e pull ficaram de fora de propósito naquela rodada.

## Solução

Toda operação de Git que fala com o remoto passa a ter prazo: **push 120 s** e **o fetch do pull 60 s** (o mesmo prazo do fetch da #113). No estouro, o host mata o **grupo de processos** — não só o `git` — e devolve a operação como `failed` com uma mensagem em palavras do app que diz o que a operação **não** tocou e o que conferir antes de tentar de novo. O resto do app volta a responder assim que a chamada retorna.

O limite honesto é **"congela no máximo 2 minutos"**, não "nunca congela": o lock global do shell continua no lugar, porque ele é o que garante a ordem de `lsp.didChange`/`session.write`, e tirá-lo está fora desta frente. O que muda é que a espera tem teto e nome.

O **merge do pull continua local e sem prazo**: ele não fala com a rede, e matá-lo no meio deixaria um merge pela metade — pior que o problema que o prazo resolve. Como o fetch vem antes e é ele quem tem prazo, um pull interrompido **nunca** deixa merge em andamento: ou o fetch termina e o merge roda, ou o fetch morre e nada foi mesclado.

## User stories

1. Como pessoa que usa o painel Git, quero que um **push** contra um remoto fora do ar **pare** em algum momento, para o app não ficar mudo por minutos.
2. Como pessoa que usa o painel Git, quero que um **Update** contra um remoto fora do ar **pare** em algum momento, pelo mesmo motivo.
3. Como pessoa que edita código com um push pendurado, quero que **o resto do app continue respondendo** enquanto a operação espera, para não perder a sessão por causa de uma ação do painel Git.
4. Como pessoa que usa o painel Git, quero que o push morto **não afirme sucesso**, para não acreditar que o commit foi publicado quando não sei.
5. Como pessoa que usa o painel Git, quero que o push morto diga que **pode ter chegado ao remoto**, para eu conferir a branch de lá antes de empurrar de novo.
6. Como pessoa que usa o painel Git, quero que o push morto diga que **nada local mudou**, para não ficar na dúvida se ele mexeu na minha branch, no index ou na árvore.
7. Como pessoa que usa o painel Git, quero que o push morto **não prometa que o upstream foi configurado**, porque um `--set-upstream` interrompido pode não ter valido.
8. Como pessoa que usa o painel Git, quero que o Update morto diga que **nada foi mesclado**, para não procurar um merge que não existe.
9. Como pessoa que usa o painel Git, quero que o Update morto diga que **refs já recebidas ficam**, para entender por que o contador pode ter mudado mesmo com a operação falhando.
10. Como pessoa que usa o painel Git, quero que o painel mostre o **estado real** depois de um push ou pull morto (ahead/behind, operação em andamento), para não ler um falso sucesso.
11. Como pessoa que usa o painel Git, quero que um push **legítimo e grande** não seja morto cedo demais, para o botão não virar inútil em repositório grande ou atrás de hook do servidor.
12. Como pessoa que mantém o OpenRidr, quero que o prazo **mate a árvore de processos** (`git-remote-http`, ssh, helper de credencial), para a chamada voltar de verdade em vez de ficar presa num pipe que um descendente segura.
13. Como pessoa que mantém o OpenRidr, quero testes de host com um **remoto que aceita a conexão e nunca responde**, para o prazo ser provado de forma determinística e sem rede real.
14. Como pessoa que mantém o OpenRidr, quero a **falha-antes medida**, para saber que o teste novo reprova sem a correção em vez de passar dos dois jeitos.
15. Como pessoa que valida a release, quero **prova no app Tauri real** de que o app responde durante a espera e mostra a mensagem do app no fim, para saber que a correção vale no binário.
16. Como pessoa que mantém o OpenRidr, quero que o que **não** foi verificado (cancelamento, hook de merge, Windows, Intel) apareça como não verificado, para não vender suporte que não existe.

## Implementation decisions

- **Prazo reusa a máquina da #113.** `run_with_deadline` já roda o filho em grupo de processos próprio, mata o grupo (`kill_group`), drena stdout/stderr em threads com canal + `recv_timeout` e devolve `(Output, timed_out)`. Push e o fetch do pull passam a usar essa função; nada de mecanismo novo, nada de dependência nova.
- **Dois números, com motivo.** `PUSH_DEADLINE` = 120 s e `FETCH_DEADLINE` = 60 s (o fetch do pull reusa este). Push sobe objetos (uplink costuma ser assimétrico) e passa por hook do servidor (`pre-receive`), então um prazo igual ao do download mataria push legítimo; o custo aceito é um congelamento de até 2 min em vez de 1. O comentário que hoje diz "the only Git operation with a deadline" passa a descrever o contrato real.
- **O pull ganha prazo só no fetch.** O pull já é dois comandos: `git fetch --quiet`, `git rev-parse @{upstream}` e `git merge --ff --no-edit <commit>`. O prazo entra no primeiro; o `rev-parse` e o merge continuam com `run` sem prazo, porque são locais. Com `--ff` explícito, um merge divergente falha sem criar `MERGE_HEAD`, então "nothing was merged" é verdadeiro no estouro do fetch.
- **Mensagem do push morto.** `Push stopped: <remote> did not answer within 120 seconds. The push may already have reached the remote: fetch and check the branch before pushing again. Your branch, index and working tree were not touched.` Ela não afirma sucesso, não afirma que o upstream foi configurado e não manda rodar comando de shell — quem confere o remoto é o **Fetch** do próprio app. `state` é `failed`.
- **Mensagem do pull morto.** `Update stopped: <remote> did not answer within 60 seconds. Nothing was merged; your branch, index and working tree were not touched, and refs already received stay.` Nenhum `git merge --abort` é sugerido porque nenhum merge começou; se já houvesse um merge em andamento, o `require_clean` recusa o Update **antes** de qualquer rede.
- **Estado devolvido é o local real.** As duas operações devolvem a mesma `Operation` de hoje, com o `status` relido: depois de um push morto o painel relê ahead/behind do local e mostra a verdade; depois de um pull morto `status.operation` continua `null` e a UI já recarrega status e branches no `finally`.
- **O lock global não muda.** Tirar o lock do caminho longo mexeria na ordenação de `lsp.didChange`/`session.write`, que é contrato, e o prazo já entrega o que a issue pede. O spec e o retro dizem isso em voz alta: o limite é a espera ter teto, não o app nunca congelar.
- **Sem UI, sem testid, sem dependência.** O diálogo de Push e o de Update já renderizam `operation.message` (`push-error`/`update-error`) e já mostram estado ocupado; nenhum `data-testid` novo ou renomeado, nenhuma mudança em `git.diff`/`git.fileDiff` nem no resto do contrato de Git.
- **Escopo de rede é exatamente push e pull.** O host não tem `clone`, `ls-remote` nem submodule; nenhum outro comando fala com o remoto.

## Testing decisions

- **Um teste bom aqui prova efeito e tempo, não texto na tela.** Cada caso confere que a chamada voltou dentro do teto, que a operação é `failed`, que o estado local está intacto e que nenhum descendente do `git` sobreviveu.
- **Seam: `push_with` e `pull_with`, com o remoto bare local de verdade.** Os testes reaproveitam `remote_repo`/`peer_clone`/`publish` (os mesmos do #112/#113), com um prazo curto injetado — o mesmo padrão de `fetch_with`.
- **O remoto que nunca responde é um listener TCP local** que aceita a conexão e fica calado (determinístico, sem rede real; TEST-NET-1 pode falhar rápido com "network unreachable" em sandbox). Prazo de 1 s.
- **Casos mínimos**: (1) push contra o listener volta `failed` em menos de 15 s, com a mensagem do app, e as refs do bare não ganharam o commit; (2) pull contra o listener volta `failed` em menos de 15 s, com "Nothing was merged", sem `MERGE_HEAD`, com `HEAD` e árvore intactos e `status.operation` nulo; (3) nos dois casos nenhum processo com a porta do listener sobrevive (`pgrep`, pulado onde não existe `pgrep`).
- **Falha-antes medida**: o mesmo cenário rodado contra o `run()` sem prazo não retorna (medido com `timeout`), que é o congelamento que a issue descreve. Teste que passa com e sem a correção não é guarda.
- **Não entra teste de "outra chamada de host responde durante a espera".** O congelamento mora no mutex `HOST_CALL` do shell, não no crate do host: um teste de host concorrente passaria antes e depois da correção, então não é guarda. Essa afirmação só se prova no app real.
- **Prova no app Tauri real com `agent-desktop`**: clone descartável com o remoto apontado para `192.0.2.1` (TEST-NET-1), Push e Update disparados pela UI, e a prova de que o app **responde durante a espera** (uma chamada de host como Problems/LSP) e de que o diálogo mostra a mensagem do app no fim. Uma instância por vez, `pgrep -x openridr` antes.
- **Nada de Playwright**: smoke noturno, não prova de sessão (decisão da #67).

## Out of scope

- **Cancelar um push/pull em andamento pela UI.** Vira issue filha (enhancement, *Related to #136*): o prazo já libera o host, e um botão de cancelar exige estado de operação em voo no host.
- **Tirar o lock global do dispatch** e qualquer mudança na ordenação de `lsp.didChange`/`session.write`.
- **Prazo no merge local** do pull e em hooks (`pre-merge-commit`), que continuam sem prazo como o `git commit` hoje.
- `clone`, `ls-remote`, submodules, tags, remoto autenticado/rede real.
- Mudança no Update (#112), concluir/abortar merge pela UI (#114), escolher remoto na UI.
- Windows e macOS Intel.

## Further notes

- Evidência em `docs/evidence/git-network-deadline.md`; formato de referência: `docs/evidence/git-remote-acceptance.md`.
- Decisão registrada em `docs/decisions.md` (2026-11-07); `docs/README.md` ganha a linha da frente; `GLOSSARY.md` não precisa de termo novo (Fetch, Update e Behind count já estão corretos).
- O fixture de remoto local (`scripts/git-remote-fixture.sh`) não muda.
