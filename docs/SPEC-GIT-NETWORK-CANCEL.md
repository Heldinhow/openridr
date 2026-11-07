# SPEC — Cancelar um push, update ou fetch em andamento (issue #150)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#150](https://github.com/Heldinhow/openridr/issues/150) (*Related to #136*), dentro da frente #50 (aceitação de Git com remoto).

## Problema

Desde a #136 toda operação de rede do Git tem prazo: push 120 s, fetch e o fetch do Update 60 s. O prazo resolve o congelamento **sem fim**, mas não a espera: até o teto, o `host_call` do shell segura o mutex global `HOST_CALL` e **nenhuma outra chamada do host roda** — `git.status`, tráfego de LSP, testes, leitura de workspace. O painel Git mostra `Pushing…` e a única saída é esperar o prazo inteiro.

Um remoto lento ou um servidor que aceita e não responde custa 2 minutos de app mudo por clique. Quem percebeu o engano (remoto errado, VPN caída, servidor travado) não tem como desistir.

A tentativa anterior desta frente manteve o Git de rede **síncrono** e abriu duas exceções para o cancelamento caber: um slot com o `opId` que o **cliente** inventa e uma lista de métodos que furam o lock do shell. Quatro rodadas de review acharam a mesma classe de bug, sempre com a mesma raiz: **a identidade da operação nasce no cliente** e o host não distingue "ainda não começou" de "já terminou" — então o cancel é aceito sem efeito, ou atinge a operação errada, e cada correção mudou a corrida de lugar em vez de eliminá-la.

A saída é a que o próprio app já usa para trabalho longo: **Build, Run e Tests**. O `build.start` devolve na hora um id do **host**, o trabalho roda numa thread com estado consultável (`running`/`ok`/`failed`/`cancelled` + mensagem), a UI lê esse estado no snapshot que já consulta a cada 400 ms, e o `build.cancel` nomeia uma operação que o host conhece por construção.

## Solução

`git.push`, `git.pull` e `git.fetch` passam a ser **start**: validam o que é barato (trust, workspace, `confirm`), registram a operação no host, devolvem **na hora** o id dela e rodam o Git de rede numa thread. O shell não segura mais o lock durante a rede, então nada fica enfileirado atrás do push e o `git.cancel` não precisa de exceção nenhuma — ele é uma chamada comum que nomeia o id que o host emitiu.

O estado da operação aparece no snapshot (`git`), com a mesma forma do Build: `idle`/`running`/`ok`/`failed`/`cancelled` e a mensagem em palavras do app. A UI já consulta esse snapshot a cada 400 ms, então o diálogo em voo mostra `Pushing…`/`Updating…`/`Fetching…` e troca para a mensagem final quando o estado vira terminal, sem `await` longo e sem canal novo.

`git.cancel { id }` cancela **só** a operação que o host emitiu com aquele id, e só enquanto o passo de rede dela está em voo. Cancelar depois que o `git` de rede terminou devolve `cancelled: false` e a UI diz isso (`Nothing to cancel: …`) em vez de fingir que parou.

O limite honesto continua o da #136 e agora é explícito: **o merge local do Update não é cancelável**. O passo de rede acaba quando o `git fetch` termina; a partir dali o cancel devolve `false` e o Update segue até o merge.

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
11. Como pessoa que usa o painel Git, quero que **o app continue respondendo** enquanto um push espera o remoto — status, arquivos, editor —, porque a operação longa não pode ser a vez de todo mundo.
12. Como pessoa que usa o painel Git, quero que o botão de Push **volte a funcionar sozinho** quando a operação termina, para não precisar adivinhar quando ela acabou.
13. Como pessoa que mantém o OpenRidr, quero que a **identidade da operação nasça no host**, para que nenhum cancel possa ser aceito sem efeito ou cair na operação seguinte.
14. Como pessoa que mantém o OpenRidr, quero **um mecanismo só** para trabalho longo (o do Build/Run/Tests) em vez de um segundo, para não manter duas semânticas de cancelamento.
15. Como pessoa que mantém o OpenRidr, quero que a **exceção do lock do shell deixe de existir**, para não haver lista de métodos que furam a fila.
16. Como pessoa que mantém o OpenRidr, quero que a **ponte de dev** (`host-http`) tenha a mesma semântica do shell, para `bun run dev` não divergir do app.
17. Como pessoa que mantém o OpenRidr, quero a **falha-antes medida**, para saber que o teste novo reprova sem a correção.
18. Como pessoa que valida a release, quero **prova no app Tauri real** de que o botão cancela e a mensagem aparece, para saber que vale no binário.
19. Como pessoa que mantém o OpenRidr, quero que o que **não** foi verificado (Windows, Intel, merge local) apareça como não verificado.

## Implementation decisions

- **O padrão é o do Build, não um novo.** Uma operação de rede é um registro com id do host, `kind`, `state` e `message`, guardado no host e publicado no **snapshot** (`git`), exatamente como `build`/`run`/`tests`. A UI não ganha canal novo: o `workspace.state` que ela já consulta a cada 400 ms carrega o estado. Alternativa recusada: manter o síncrono e ampliar as exceções do lock — foi o que as quatro rodadas de review já reprovaram.
- **A identidade nasce no host.** `git.push`/`git.pull`/`git.fetch` devolvem `{ "id": "git:<n>" }` em milissegundos e o `git.cancel` recebe esse id. Não existe mais `opId` vindo do cliente, nem cancel "pendente", nem a distinção entre "ainda não começou" e "já terminou": o cancel só pode nomear uma operação que o host emitiu, e o estado diz se ela ainda pode ser cancelada.
- **O start valida só o que é barato.** Trust, workspace aberto e `confirm: true` no push continuam síncronos (são o contrato de segurança e de API, e falham sem tocar a rede). Tudo o que depende do repositório — abrir o repo, `git status`, achar o remoto, árvore suja — roda na thread e aparece como `failed` com a mensagem do app. Motivo: o `git status` de um repositório grande é justamente um dos passos que hoje trava o app, e ele não pode segurar o lock.
- **Uma operação de rede por host.** Um segundo start enquanto outro roda é recusado (`invalid_state`, nomeando a operação em voo), como o `build.start` faz. A UI já desabilita os botões; a recusa é a rede de segurança do contrato.
- **O passo de rede e o cancelamento são uma transição só.** O registro da operação nasce no start (antes de qualquer trabalho) e o flag de cancelado é lido e limpo sob o **mesmo mutex** que o `cancel` usa, no instante em que o filho do `git` termina. Assim o cancel ou chega antes (e a operação se reporta `cancelled`, no Update antes de qualquer merge) ou depois (e o host responde `cancelled: false`, porque o passo de rede já não está em voo). Sem essa atomicidade sobrava a janela em que o cancel era aceito e ignorado — a raiz da classe de bug das rodadas anteriores.
- **O kill é do laço que já tem o `Child`.** O cancel só levanta o flag; quem chama o `kill_tree` que o prazo já usa é a thread que tem o `Child`, a cada volta do laço (20 ms). Ninguém sinaliza um pid que não é dono, e o comentário que garante "o líder não foi colhido, então o pid não pode ser reusado" continua verdadeiro.
- **`run_with_deadline` continua devolvendo o motivo, não um booleano.** `(Output, Stop)` com `Stop::{No, Deadline, Cancelled}`: a mensagem do prazo ("did not answer within N seconds") seria mentira num cancelamento, e o chamador escolhe a frase pelo motivo.
- **Um cancel aceito é o que a operação reporta.** Se o cancel chega antes de a operação tocar a rede e ela então falha por conta própria (árvore suja, sem remoto, `git` que não sobe), o estado publicado é `cancelled` com a mensagem **da própria operação** — o `git.cancel` que respondeu `true` nunca termina como `ok` nem como uma falha que o usuário leria como "rodou mesmo assim". O motivo real continua visível na mensagem.
- **Uma operação de rede por vez, também na UI.** Um segundo start com outro em voo é recusado no host (`invalid_state`) e a UI não tenta: `startNetwork` guarda `pending`/`starting` e ignora o pedido, porque trocar o id em `pending` faria a mensagem final da primeira operação nunca aparecer.
- **O estado terminal é do host, e a mensagem também.** A operação publica `ok`/`failed`/`cancelled` + a mensagem em palavras do app; a UI só renderiza. Nenhum estado novo em `Operation` (que continua sendo o retorno interno das três funções): o `cancelled` é do registro publicado.
- **O merge local do Update não é cancelável, e a UI diz isso.** O passo de rede acaba quando o `git fetch` termina; um cancel nessa janela devolve `cancelled: false` e o diálogo mostra `Nothing to cancel: the update is already merging.` (push: `…the push already finished.`, fetch: `…the fetch already finished.`). Matar o merge deixaria um merge pela metade — a decisão da #136 continua valendo.
- **A exceção do lock do shell e a fila da ponte de dev viram código morto, e saem.** Lido o código: `host_call` toma `HOST_CALL` durante o `dispatch` inteiro, e hoje o `git.push` passa até 120 s lá dentro — é por isso que tudo o mais faz fila e que um cancel precisava de um caminho fora do lock. Com o start, o `dispatch` do `git.push` dura o tempo de registrar a operação (milissegundos) e o trabalho de rede roda numa thread que ninguém enfileira; o `git.cancel` é uma chamada comum, sem nada para ultrapassar. A fila FIFO com um worker da ponte `host-http` existia pela mesma razão (fazer o cancel furar a fila sem perder a ordem de chegada de `lsp.didChange`/`session.write`); sem chamada longa, o laço de aceitação que atende uma requisição por vez preserva a ordem **por construção**, e a fila só acrescenta uma thread e um canal. Saem `UNSERIALIZED_METHODS`/`bypasses_host_lock` (e o teste que os fixava), o ramo do lock no shell Tauri e a fila/overtake da ponte. Menos código, e uma lista que ninguém pode alargar de carona porque não existe mais.
- **Mensagens: mesmas promessas da #136, motivo diferente.** `Push cancelled: the push may already have reached the remote. Check the branch there before pushing again.` / `Update cancelled: nothing was merged; your branch, index and working tree were not touched, and refs already received stay.` / `Fetch cancelled: your branch, index and working tree were not touched; refs already received stay.` Os prazos da #136 não mudam (push 120 s, fetch e o fetch do Update 60 s).
- **O diálogo em voo não fecha mais.** `push-cancel`/`update-cancel` (dispensar) ficam desabilitados enquanto a operação roda e Escape cancela em vez de fechar. Hoje dá para fechar o diálogo com o push em voo e o erro final se perde; com um botão de cancelar, essa saída deixa de fazer sentido. O diálogo de Abort merge não muda: abortar é local e não é cancelável.
- **`data-testid` novos, nenhum renomeado.** `update-stop`, `push-stop`, `branch-fetch-stop` (o `branch-fetch` continua existindo e mostrando `Fetching…`). As mensagens continuam em `update-error`/`push-error`/`branch-fetch-error`.
- **Sem dependência nova, sem CSS novo.** Os botões usam as classes que os diálogos já têm.

## Testing decisions

- **Um teste bom aqui prova efeito, tempo e disponibilidade.** Cada caso confere que o start voltou em milissegundos, que outra chamada do host **responde durante** a operação, que o cancel devolve o controle muito antes do prazo, que o estado publicado é `cancelled` com a mensagem do app, que o estado local está intacto e que nenhum descendente do `git` sobreviveu.
- **O seam é o mais alto que existe: `Host::dispatch`.** O contrato que a UI usa é `git.push` → `{ id }`, `git.cancel { id }` → `{ cancelled }` e o snapshot com o estado; testar por ali cobre host, thread, registro e publicação de uma vez, e é o único lugar onde "o app continua respondendo durante a operação" pode ser provado. Os casos reaproveitam o `remote_repo`/`peer_clone`/`publish` e o listener `silent_remote()` da #136 — o remoto que aceita e nunca responde (determinístico, sem rede real).
- **Os testes unitários de `git.rs` continuam no mesmo seam de função** (`push_with`/`pull_with`/`fetch_with` recebem o `&Net` ao lado do prazo), agora pegando o id de `net.begin(kind)` em vez de inventá-lo. Nenhum seam novo, nenhuma infraestrutura nova de teste.
- **Casos mínimos** (prazo longo, 120 s, para o cancelamento ser o único motivo possível do retorno): (1) push cancelado publica `cancelled` em menos de 5 s com `Push cancelled`, bare sem o commit, `HEAD` local intacto; (2) Update cancelado publica `cancelled` com `Update cancelled` e "nothing was merged", sem `MERGE_HEAD`, `HEAD` e árvore intactos; (3) Fetch cancelado publica `cancelled` com `Fetch cancelled` e refs de tracking inalteradas; (4) nos três, nenhum processo com a porta do listener sobrevive (`pgrep`, pulado onde não existe); (5) nos três, um `git.status` despachado com a operação em voo responde antes do cancel.
- **Cancelar sem nada em voo (ou com um id que o host não conhece) devolve `cancelled: false`** e não mata nada — o caso que a UI usa para a nota.
- **Um segundo start com uma operação em voo é recusado** com `invalid_state`, sem tocar a rede.
- **Falha-antes medida**: com o laço sem olhar o flag, o mesmo cenário roda até o prazo (120 s) e o teste de 5 s reprova; registrado na evidência com o tempo medido.
- **A prova de UI é Playwright no modo web** (decisão do Helder de 2026-11-07, “A superfície de verificação de UI segue o que mudou”, em `docs/decisions.md`: interface pura se prova no Playwright em modo web, só os specs da área): um cenário novo em `tests/e2e/git.spec.ts` aponta o `origin` de um repositório descartável para um listener local que aceita e não responde (o mesmo cenário da #136) e percorre os três caminhos pela UI — Push → Cancel, Update → Cancel, Fetch → Cancel —, mais Escape cancelando em vez de fechar e a nota `Nothing to cancel`. Roda **só esse spec**, com o slot do E2E (portas 1420/4317) pedido ao orquestrador e aviso de "E2E livre" ao terminar. O spec existente que checa `git.push` é atualizado no contrato novo (o start devolve `{ id }`; a falha de "sem remoto" chega pelo estado publicado).
- **A checagem no app Tauri real com `agent-desktop` é opcional e curta**, só para o que é nativo: a janela, o menu nativo e o diálogo nativo do shell. O que o fluxo do host faz já é provado pelo Playwright, que roda o mesmo `host-http` e a mesma UI.

## Out of scope

- **Cancelar o merge local do Update** (e qualquer passo local: `git merge`, `git commit`). Matá-lo deixa merge pela metade; a #136 já decidiu que fica sem prazo e sem cancelamento.
- **Cancelar `git.checkout`, `git.commit`, `git.branchCreate`, diff e log.** São locais, não seguram a rede e continuam síncronos.
- **Tornar assíncronos os outros métodos do host.** O padrão existe (Build/Run/Tests); esta frente só traz o Git de rede para ele.
- **Timeout de conexão do Git** e a redação contra remoto que descarta pacotes: o contrato da #136 não muda.
- **Progresso da transferência** (bytes, porcentagem) e cancelar por sinal do SO (Ctrl-C no terminal).
- **Windows e macOS Intel.**

## Further notes

- Evidência em `docs/evidence/git-network-cancel.md`; formato de referência: `docs/evidence/git-network-deadline.md`.
- Decisão nova em `docs/decisions.md` (a de 2026-11-07 do slot/`opId` fica como histórico e é substituída por esta); `docs/README.md` ganha a linha da frente. `GLOSSARY.md` ganha **Cancel (Git network)** — é termo de usuário e a distinção com "Stop" do Build/Run importa.
- O fixture de remoto local (`scripts/git-remote-fixture.sh`) não muda; o listener que não responde é o mesmo `python3` da evidência da #136.
- Tickets filhos, em **fatias verticais** (cada uma atravessa host, shell e UI e é verificável sozinha), nesta ordem de bloqueio: [#206](https://github.com/Heldinhow/openridr/issues/206) **Cancel num Push** (carrega o mecanismo compartilhado: registro no snapshot, start assíncrono, `git.cancel`, fim das exceções do lock), [#207](https://github.com/Heldinhow/openridr/issues/207) **Cancel num Update** e [#208](https://github.com/Heldinhow/openridr/issues/208) **Cancel num Fetch**, as duas bloqueadas pela primeira.
- O código síncrono da tentativa anterior que o desenho novo torna morto é removido na mesma frente: `opId` do cliente, slot com `pending`/`finished`, `bypasses_host_lock`/`UNSERIALIZED_METHODS`, o ramo do shell e a fila/overtake da ponte de dev. O que serve (o `Stop`, as mensagens, o `kill_tree`, os testes de prazo, o `silent_remote`) fica.
