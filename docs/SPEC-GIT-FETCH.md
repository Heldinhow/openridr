# SPEC — Fetch explícito no painel Git (issue #113)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#113](https://github.com/Heldinhow/openridr/issues/113) (frente #50, aceitação de Git com remoto).

## Problema

O painel Git mostra o **behind count** (`↓N`) e a lista de branches remotas a partir das refs de tracking locais, mas o app não tem nenhuma ação de fetch. Até alguém empurrar ou rodar **Update…**, um commit publicado pela contraparte não aparece no contador: o painel lê `0 ahead, 0 behind` enquanto o upstream já andou. O mesmo vale para uma branch remota recém-publicada, que simplesmente não existe na lista até algo buscar.

Reproduzido na aceitação de Git com remoto (#50): depois de a contraparte empurrar um commit, **Refresh** no painel Commit manteve a linha da branch em `main · upstream origin/main`; rodar `git fetch origin` no mesmo clone e atualizar mudou para `0 ahead, 1 behind`.

O Update (pull) faz fetch e por isso incorpora o commit mesmo com `↓ 0` na tela — a lacuna é o contador e a lista de branches remotas, não o pull. Mas confiar no Update como fetch esconde duas coisas: ele exige árvore limpa e upstream, e ele move a branch, quando o que a pessoa queria era só olhar o que chegou.

## Solução

O painel Git ganha uma ação **Fetch** própria: um botão no menu de branches e uma entrada `Fetch…` no Find Action. Ela roda `git fetch --prune` no remoto do upstream da branch (primeiro remoto configurado quando não há upstream) e, no fim, o painel mostra os contadores já relidos e a lista de branches remota atualizada — incluindo branches novas publicadas pela contraparte e branches apagadas, que o `--prune` remove da lista.

O **Refresh** que já existe continua sendo leitura local do status: leitura barata, chamada ao abrir o painel e no foco da janela, sem rede e sem surpresa. O diálogo nenhum: fetch move só refs, não é destrutivo. Quando o remoto não responde em 60 segundos, o host mata o processo e diz isso em palavras do app, porque um `git` pendurado trava o app inteiro (o shell serializa todos os dispatches do host num lock global).

## User stories

1. Como pessoa que usa o painel Git, quero uma ação **Fetch** explícita, para atualizar o estado do remoto sem sair do app nem abrir o terminal.
2. Como pessoa que usa o painel Git, quero que **Refresh continue sendo leitura local**, para abrir o menu de branches sem disparar rede nem travar atrás de um remoto lento.
3. Como pessoa que usa o painel Git, quero que o `↓` reflita o upstream **depois do fetch**, para saber que chegou trabalho antes de decidir se atualizo agora.
4. Como pessoa que usa o painel Git, quero ver uma **branch remota recém-publicada** na lista depois do fetch, para poder checar ela como branch local.
5. Como pessoa que usa o painel Git, quero que uma branch **apagada no remoto** saia da lista depois do fetch, para a lista não virar um cemitério de refs mortas.
6. Como pessoa que usa o painel Git, quero que o fetch use **o remoto do meu upstream**, para o contador que muda ser o do upstream que a linha da branch mostra.
7. Como pessoa que usa o painel Git, quero que um fetch **não toque na minha árvore de trabalho** nem na minha branch, para poder buscar com trabalho em andamento.
8. Como pessoa que usa o painel Git, quero que um fetch **funcione durante um merge em andamento**, para poder ver o que a contraparte fez enquanto eu resolvo.
9. Como pessoa que usa o painel Git, quero que o botão diga **o comando exato** que vai rodar (`git fetch --prune`), para não descobrir depois que ele apagou uma ref da lista.
10. Como pessoa que usa o painel Git, quero que sem remoto configurado o botão fique **desabilitado com o motivo**, para não clicar em algo que só vai falhar.
11. Como pessoa que usa o painel Git, quero que o erro de **credencial** apareça em palavras do app, para entender que o helper de credencial não respondeu em vez de ler "terminal prompts disabled" e ir para o terminal sem motivo.
12. Como pessoa que usa o painel Git, quero que outro erro qualquer mostre o texto do Git, para eu conseguir diagnosticar o que o app não previu.
13. Como pessoa que usa o painel Git, quero que um remoto **fora do ar** não congele o app, para continuar editando e rodando testes enquanto o fetch falha.
14. Como pessoa que usa o painel Git, quero que esse estouro de prazo diga que **nada foi alterado**, para não ficar na dúvida se o fetch pela metade mexeu em algo.
15. Como pessoa que usa o painel Git, quero que o fetch apareça no **Find Action**, para achar a ação sem caçar menu.
16. Como pessoa que usa o painel Git, quero que o botão mostre um estado **ocupado** enquanto busca, para saber que o clique foi aceito.
17. Como pessoa que usa o painel Git, quero uma mensagem de sucesso **factual** com o remoto e os contadores, para confirmar o efeito sem abrir log.
18. Como pessoa que valida a release, quero **prova no app Tauri real** de que o `↓` sai de 0 para 1 depois de a contraparte empurrar e eu clicar em Fetch, para saber que a correção vale no binário.
19. Como pessoa que mantém o OpenRidr, quero testes de host com **remoto real descartável**, para o comportamento ser provado por refs e não por texto na tela.
20. Como pessoa que mantém o OpenRidr, quero que o que não foi verificado (remoto autenticado, Windows, Intel) apareça como não verificado, para não vender suporte que não existe.

## Implementation decisions

- **Ação nova `git.fetch`, sem parâmetros; Refresh intacto.** O host ganha um método trust-gated `git.fetch`, que devolve a mesma `Operation` das outras ações de Git (incluindo o status relido), então o contador e a lista de branches já voltam frescos de graça. O Refresh do menu de branches e do painel Commit continua chamando só `git.status`: ele é disparado ao abrir o menu e no foco da janela, e rede ali seria tráfego escondido.
- **Remoto do upstream, com fallback.** O fetch usa o remoto do upstream da branch (o prefixo de `origin/main`, por exemplo); sem upstream, o primeiro remoto configurado; sem nenhum, o botão fica desabilitado com o motivo já usado pelo Push. Nunca `--all`.
- **`--prune` é parte do contrato.** `git fetch --quiet --prune <remote>`: a lista de branches remotas é metade do problema da issue, e só o prune remove refs de branches apagadas na contraparte (só `refs/remotes/*`). O `title` do botão declara o comando.
- **Nada bloqueia o fetch além de repositório e remoto.** Árvore suja, merge/rebase em andamento e HEAD destacado não impedem: o fetch move só refs, não toca index, árvore, branch local nem `MERGE_HEAD`. É o oposto do Update, que exige árvore limpa porque escreve.
- **Prazo de 60 s, e só no fetch.** O `host_call` do shell serializa todos os dispatches do host num lock global: um `git` pendurado congela o app inteiro. O fetch roda com prazo, e no estouro o host mata o filho e devolve `state: "failed"` com mensagem do app dizendo que nada foi alterado (o fetch já concluído é idempotente e só mexe em refs). Push e pull ficam fora desta frente — o mesmo risco neles é a issue #136.
- **Erro de credencial vira palavra do app; o resto continua cru.** O host já roda `git` com `GIT_TERMINAL_PROMPT=0` e `stdin` nulo. A classe "o Git pediu credencial e não havia como perguntar" (falha de autenticação, prompt desabilitado, URL com credencial mascarada) é traduzida para uma mensagem que diz que o helper de credencial tem que responder; qualquer outra falha mantém a cauda do stderr, com o texto completo em `Operation.output`, como hoje.
- **Sem diálogo, sem atalho, sem fetch automático.** Ação direta com botão ocupado (fetch não é destrutivo), mensagem factual no sucesso nomeando o remoto e os contadores relidos, e nada de rede no foco da janela, em timer ou em polling. Atalho de teclado fica fora desta rodada: `git.update` acabou de ser desamarrado do menu nativo (#111/#130) e o Rider também não tem default para Fetch.
- **O contador continua sendo o de refs de tracking locais.** O `↓` vale desde o último fetch ou push; o app não passa a afirmar que ele é fresco. A mensagem de sucesso diz o que o fetch viu, e nada no painel inventa selo de "stale".
- **Testids novos, nenhum renomeado.** `branch-fetch` (botão), `branch-fetch-error` (erro da ação no menu) e `branch-fetch-reason` (motivo do bloqueio, no padrão `branch-update-reason`/`branch-push-reason`). `branch-refresh`, `commit-refresh` e `git-refresh` não mudam de nome nem de comportamento.
- **Sem dependência nova.** Tudo é Git de linha de comando pelo host; nada de libgit2 nem de cliente HTTP no React.

## Testing decisions

- **Um teste bom aqui prova efeito no repositório, não texto na tela.** Cada caso confere refs (`rev-parse`/`for-each-ref` dos `refs/remotes/*`), contadores do `git.status` depois da operação e — quando o caso é de recusa — que nada mudou.
- **Seam: a função de fetch do host, com o remoto bare local real.** Os testes reaproveitam os helpers que o #112 já usa (`remote_repo` + `peer_clone`: bare `origin.git`, clone `user` que o app abriria e contraparte `peer`), sem mock de Git e sem rede.
- **Casos mínimos**: (1) contraparte publica um commit → fetch → `behind` vai de 0 para 1, sem tocar `HEAD`, index nem árvore; (2) contraparte publica uma branch → fetch → ela aparece na lista de branches remotas; (3) contraparte apaga a branch no remoto → fetch com prune → a ref de tracking some da lista; (4) árvore suja e merge em andamento não impedem o fetch, e `MERGE_HEAD`/arquivo sujo continuam intactos depois; (5) sem remoto o método é recusado com o motivo do app; (6) a URL do remoto com credencial não aparece na mensagem nem em `output` (o mascaramento que já existe continua valendo).
- **O prazo é provado por um remoto que não responde.** Um caso aponta o clone para um endereço não-roteável (TEST-NET-1, `192.0.2.1`) com prazo curto e confere que o `git` foi morto, que a operação volta `failed` com a mensagem do app e que nenhuma ref mudou — sem depender de rede real.
- **UI no app Tauri real com `agent-desktop`** (`desktop.sh up` com `VERIFY_OPENRIDR_DESKTOP_SOLUTION` apontando para o clone `user` de `scripts/git-remote-fixture.sh`), como na aceitação da #50 e no #112. Playwright não é prova de sessão (decisão da #67); o spec de diálogo existente não ganha caso novo aqui.
- **O fixture de remoto local não muda**: `scripts/git-remote-fixture.sh` já cria `origin.git`, `user` e `peer`; o commit da contraparte é produzido por shell.

## Out of scope

- **Prazo em push e pull** e o lock global do dispatch: #136 (*Related to #113*).
- **Fetch automático** no foco, em timer ou polling; e qualquer selo de "contador possivelmente velho".
- **Atalho de teclado** para o Fetch, e mudança no keymap.
- **Escolher o remoto na UI** ou `--all`.
- **Tags** (`--tags`), submodules, remoto autenticado/rede real.
- Mudar o comportamento de Update (#112), concluir/abortar merge pela UI (#114).
- Windows e macOS Intel.

## Further notes

- Evidência em `docs/evidence/git-fetch.md`, capturas em `docs/evidence/git-fetch/`. Formato de referência: `docs/evidence/git-remote-acceptance.md`.
- Decisão registrada em `docs/decisions.md` (2026-11-06); `GLOSSARY.md` ganha **Fetch**; **Behind count** e **Update** já estavam corretos.
- `docs/README.md` ganha a linha da frente; `docs/implementation-notes.md` ganha o contrato da operação e os testids novos.
