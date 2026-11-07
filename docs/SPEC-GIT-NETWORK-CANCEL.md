# SPEC — Cancelar um push, update ou fetch em andamento (issue #150)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#150](https://github.com/Heldinhow/openridr/issues/150) (*Related to #136*), dentro da frente #50 (aceitação de Git com remoto).

## Problema

Desde a #136 toda operação de rede do Git tem prazo: push 120 s, fetch e o fetch do Update 60 s. O prazo resolve o congelamento **sem fim**, mas não a espera: até o teto, o `host_call` do shell segura o mutex global `HOST_CALL` e **nenhuma outra chamada do host roda** — `git.status`, tráfego de LSP, testes, leitura de workspace. O painel Git mostra `Pushing…` e a única saída é esperar o prazo inteiro.

Um remoto lento ou um servidor que aceita e não responde custa 2 minutos de app mudo por clique. Quem percebeu o engano (remoto errado, VPN caída, servidor travado) não tem como desistir.

O que falta é pequeno e já tem quase tudo pronto: `run_with_deadline` (`crates/host/src/git.rs`) já roda o `git` em grupo de processos próprio e `kill_tree` mata a árvore; falta o host **guardar qual operação está em voo** e expor um `git.cancel` que o laço de espera observe. O outro lado é o shell: o cancelamento precisa ser despachado **sem** passar pelo mutex que a operação em voo segura — senão ele fica na fila atrás dela e nunca chega.

## Solução

O host passa a ter **um slot por instância** com a operação de rede em voo. `run_with_deadline` marca o slot ao começar, limpa ao terminar e, a cada volta do laço de espera (20 ms), verifica se foi cancelado — se foi, mata o grupo de processos exatamente como o prazo faz e devolve a operação como `failed` com uma mensagem em palavras do app que mantém as promessas honestas da #136.

`git.cancel` é o único método que o shell despacha **fora** do `HOST_CALL`: ele não toca estado do host (nem o `Mutex<Inner>`, nem disco), só levanta o flag do slot. A UI ganha **Cancel** nos diálogos de Push e Update (e no Fetch, que não tem diálogo), com Escape fazendo o mesmo que o botão.

O limite honesto continua o da #136 e agora é explícito: **o merge local do Update não é cancelável**. O slot só existe enquanto o `git` de rede roda; um cancel que chega depois disso não pega nada e a UI diz isso (`Nothing to cancel: …`) em vez de fingir que parou.

## User stories

1. Como pessoa que usa o painel Git, quero **cancelar um push em andamento**, para não esperar 2 minutos por um remoto que eu já sei que está errado.
2. Como pessoa que usa o painel Git, quero **cancelar um Update em andamento**, pelo mesmo motivo.
3. Como pessoa que usa o painel Git, quero **cancelar um Fetch em andamento**, porque ele é a mesma espera de 60 s sem saída.
4. Como pessoa que usa o painel Git, quero que o cancelamento **devolva o controle em segundos**, não no prazo.
5. Como pessoa que usa o painel Git, quero que o push cancelado diga que **pode ter chegado ao remoto**, para conferir a branch de lá antes de empurrar de novo.
6. Como pessoa que usa o painel Git, quero que o Update cancelado diga que **nada foi mesclado**, para não procurar um merge que não existe.
7. Como pessoa que usa o painel Git, quero que o cancelamento **não deixe processo órfão** (`git-remote-http`, ssh, helper de credencial), para o próximo push não brigar com um fantasma.
8. Como pessoa que usa o painel Git, quero que **Escape** no diálogo em voo cancele, para não precisar de mouse.
9. Como pessoa que usa o painel Git, quero **não fechar o diálogo** com a operação em voo, para não perder a mensagem final.
10. Como pessoa que usa o painel Git, quero que, quando não havia nada para cancelar, o app **diga isso**, em vez de silêncio.
11. Como pessoa que mantém o OpenRidr, quero que **só** `git.cancel` fure o lock do shell, com teste, para ninguém alargar a exceção de carona.
12. Como pessoa que mantém o OpenRidr, quero que a **ponte de dev** (`host-http`) tenha a mesma semântica do shell, para `bun run dev` não divergir do app.
13. Como pessoa que mantém o OpenRidr, quero a **falha-antes medida**, para saber que o teste novo reprova sem a correção.
14. Como pessoa que valida a release, quero **prova no app Tauri real** de que o botão cancela e a mensagem aparece, para saber que vale no binário.
15. Como pessoa que mantém o OpenRidr, quero que o que **não** foi verificado (Windows, Intel, merge local) apareça como não verificado.

## Implementation decisions

- **Um slot por instância do host, não um `static`.** `git::Net` (dois `AtomicBool`: `in_flight`, `cancelled`) vive em `Host`, ao lado do `inner`, **fora** do `Mutex<Inner>`. Um `static` global deixaria o cancel de um teste matar a operação de outro (os testes de host rodam em paralelo no mesmo processo) e faria a instância do E2E e a do usuário se enxergarem.
- **O kill é do próprio laço, não de quem cancela.** `git.cancel` só levanta o flag; quem chama `kill_tree` é a thread que já tem o `Child` — a cada `FETCH_POLL` (20 ms). Assim não há sinal cruzado por pid, e o comentário que hoje garante "o líder não foi colhido, então o pid não pode ser reusado" continua verdadeiro: ninguém sinaliza um pid que não é dono. Latência aceita: até 20 ms.
- **`run_with_deadline` devolve o motivo, não um booleano.** `(Output, Stop)` com `Stop::{No, Deadline, Cancelled}`; os três chamadores escolhem a mensagem pelo motivo. `timed_out: bool` não distingue prazo de cancelamento e a mensagem do prazo ("did not answer within N seconds") seria mentira num cancelamento.
- **O slot é marcado depois do spawn e limpo antes de colher.** `begin()` (limpa `cancelled`, marca `in_flight`) só depois que o `Command::spawn` deu certo; `end()` antes do `wait`/dreno final. Um cancel atrasado, que chega depois do `end()`, encontra o slot vazio e devolve `cancelled: false` em vez de matar a operação seguinte.
- **`git.cancel` não pede trust e não toca `Inner`.** Ele não muda estado, não abre repositório e só para um processo que o host já começou (e que só começou porque o trust foi dado). Pedir trust exigiria o `Mutex<Inner>` — pequeno, mas é exatamente o tipo de dependência que o cancelamento não deve ter.
- **`git.cancel` é o único método fora do `HOST_CALL`.** O mutex existe para preservar a ordem de chamadas que a UI dispara sem `await` (`lsp.didChange`, `session.write`); um cancelamento precisa **furar** a fila por definição. A exceção é uma lista nomeada no crate do host (`openridr_host::bypasses_host_lock`), usada pelos dois shells, com teste que fixa a lista em exatamente `["git.cancel"]` — é o teste pedido de que "outra chamada não fura o lock".
- **A ponte de dev ganha uma thread por requisição e um gate que espelha o shell.** `host-http` hoje é serial (`for request in server.incoming_requests()`), então um cancel ficaria enfileirado atrás do push. Passa a atender cada requisição numa thread, com um `Mutex` que faz o papel do `HOST_CALL` e a mesma isenção para `git.cancel`. Ordem preservada, `bun run dev` com a mesma semântica do app.
- **Mensagens: mesmas promessas da #136, motivo diferente.** `Push cancelled: the push may already have reached the remote. Check the branch there before pushing again.` / `Update cancelled: nothing was merged; your branch, index and working tree were not touched, and refs already received stay.` / `Fetch cancelled: your branch, index and working tree were not touched; refs already received stay.` `state` continua `"failed"` — nenhum estado novo no contrato de `Operation`, nenhum consumidor novo.
- **O merge local do Update não é cancelável, e a UI diz isso.** O slot morre quando o fetch termina; um cancel nessa janela devolve `cancelled: false` e o diálogo mostra `Nothing to cancel: the update is already merging.` (push: `…the push already finished.`, fetch: `…the fetch already finished.`). Matar o merge deixaria um merge pela metade — a decisão da #136 continua valendo.
- **O diálogo em voo não fecha mais.** `push-cancel`/`update-cancel` (dispensar) passam a ficar desabilitados durante a operação e Escape cancela em vez de fechar. Hoje dá para fechar o diálogo com o push em voo e o erro final se perde; com um botão de cancelar, essa saída deixa de fazer sentido. O diálogo de Abort merge não muda: abortar é local e não é cancelável.
- **`data-testid` novos, nenhum renomeado.** `update-stop`, `push-stop`, `branch-fetch-stop` (o `branch-fetch` continua existindo e mostrando `Fetching…`). As mensagens continuam em `update-error`/`push-error`/`branch-fetch-error`.
- **Sem dependência nova, sem CSS novo.** Os botões usam as classes que os diálogos já têm.

## Testing decisions

- **Um teste bom aqui prova efeito e tempo.** Cada caso confere que a chamada voltou **muito antes do prazo**, que a operação é `failed` com a mensagem do app, que o estado local está intacto e que nenhum descendente do `git` sobreviveu.
- **Seam: `push_with`/`pull_with`/`fetch_with` recebem o `&Net`**, como já recebem o prazo. Os testes reaproveitam `remote_repo`/`peer_clone`/`publish` e o listener `silent_remote()` da #136 — o remoto que aceita e nunca responde, determinístico e sem rede real.
- **Casos mínimos** (prazo longo, 120 s, para o cancelamento ser o único motivo possível do retorno): (1) push cancelado volta `failed` em menos de 5 s com `Push cancelled`, bare sem o commit, `HEAD` local intacto; (2) Update cancelado volta `failed` com `Update cancelled` e "nothing was merged", sem `MERGE_HEAD`, `HEAD` e árvore intactos; (3) Fetch cancelado volta `failed` com `Fetch cancelled` e refs de tracking inalteradas; (4) nos três, nenhum processo com a porta do listener sobrevive (`pgrep`, pulado onde não existe).
- **Cancelar sem nada em voo devolve `cancelled: false`** e não mata nada — o caso que a UI usa para a nota.
- **O teste da exceção do lock fixa a lista**: `bypasses_host_lock("git.cancel")` é `true` e `git.push`, `git.pull`, `git.fetch`, `lsp.didChange`, `session.write` e `git.status` são `false`. Um método novo na lista reprova o teste, que é o ponto.
- **Falha-antes medida**: com o laço sem olhar o flag, o mesmo cenário roda até o prazo (120 s) e o teste de 5 s reprova; registrado na evidência com o tempo medido.
- **Prova no app Tauri real com `agent-desktop`**: clone descartável com o `origin` apontado para um listener local que aceita e não responde (o mesmo cenário da #136), workspace aberto por argv, e os três caminhos pela UI — Push → Cancel, Update → Cancel, Fetch → Cancel — mostrando a mensagem do app em segundos e o app respondendo depois. **O próprio cancelamento é a prova de que ele fura o lock**: se ele passasse pelo `HOST_CALL`, nunca seria despachado e o diálogo voltaria só no prazo.
- **Nada de Playwright**: smoke noturno, não prova de sessão (decisão da #67). Nenhum spec novo, nenhum spec alterado.

## Out of scope

- **Cancelar o merge local do Update** (e qualquer passo local: `git merge`, `git commit`). Matá-lo deixa merge pela metade; a #136 já decidiu que fica sem prazo e sem cancelamento.
- **Cancelar `git.checkout`, `git.commit`, `git.branchCreate`, diff e log.** São locais e não seguram a rede; se um dia entrarem sob o mesmo contrato, entram pela mesma lista.
- **Tirar o `HOST_CALL` do caminho longo** (o push continua segurando o lock enquanto roda). A exceção é só para `git.cancel`.
- **Timeout de conexão do Git** e a redação contra remoto que descarta pacotes: o contrato da #136 não muda.
- **Progresso da transferência** (bytes, porcentagem) e cancelar por sinal do SO (Ctrl-C no terminal).
- **Windows e macOS Intel.**

## Further notes

- Evidência em `docs/evidence/git-network-cancel.md`; formato de referência: `docs/evidence/git-network-deadline.md`.
- Decisão nova em `docs/decisions.md`; `docs/README.md` ganha a linha da frente. `GLOSSARY.md` ganha **Cancel (Git network)** — é termo de usuário e a distinção com "Stop" do Build/Run importa.
- O fixture de remoto local (`scripts/git-remote-fixture.sh`) não muda; o listener que não responde é o mesmo `python3` da evidência da #136.
- Tickets filhos, nesta ordem de bloqueio: [#206](https://github.com/Heldinhow/openridr/issues/206) (host: slot + `git.cancel` + mensagens + testes), [#207](https://github.com/Heldinhow/openridr/issues/207) (shell: `host_call` + ponte de dev) e [#208](https://github.com/Heldinhow/openridr/issues/208) (UI: diálogos + Fetch + prova no app).
