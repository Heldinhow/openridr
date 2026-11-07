# RETRO — Cancelar push, update e fetch (issue #150)

Rodada de 2026-11-07. Spec: [SPEC-GIT-NETWORK-CANCEL](SPEC-GIT-NETWORK-CANCEL.md). Evidência: [evidence/git-network-cancel.md](evidence/git-network-cancel.md). Decisão: `docs/decisions.md`, entrada de 2026-11-07 da #150.

## O que se sustentou

- **O padrão do Build pagou de novo, e desta vez sem inventar nada.** Start devolve id do host, estado no snapshot que a UI já consulta a cada 400 ms, cancel nomeia o id. Nenhum canal novo, nenhuma infraestrutura de teste nova: os casos reaproveitaram `silent_remote`, `remote_repo` e o seam `Host::dispatch`.
- **A identidade nasce no host, e isso eliminou a classe de bug.** Não existe mais cancel "pendente", nem `opId` do cliente, nem a pergunta "ainda não começou ou já terminou?": o cancel só pode nomear uma operação que o host emitiu, e o próprio registro diz se ela ainda pode ser cancelada. As quatro rodadas da tentativa síncrona brigavam com uma corrida que o desenho novo não tem.
- **A remoção foi parte da entrega.** `UNSERIALIZED_METHODS`/`bypasses_host_lock`, o ramo do lock no shell e a fila/overtake da ponte de dev saíram junto: com o start voltando na hora, nada segura o lock durante a rede, e a fila só acrescentava thread e canal. Menos código do que entrou.
- **A prova de UI pegou dois defeitos que o teste de host não pegaria.** O botão de cancelar aparecia antes de o host emitir o id (clique sem efeito) e o Escape do diálogo usava uma closure anterior ao `pending`. Ambos são de fluxo, não de contrato: só aparecem dirigindo a tela.
- **O cenário determinístico da nota "nothing to cancel".** Um hook `post-merge` que segura o merge por 3 s torna previsível a janela em que o passo de rede já acabou e a operação continua rodando — sem isso, o caso seria uma corrida.

## O que quebrou / custou

- **Quatro rodadas de review no desenho síncrono, e a decisão de refazer.** O caminho errado foi manter o Git de rede síncrono e abrir exceções (slot com `opId` do cliente, lista de métodos que furam o lock). Cada correção mudou a corrida de lugar. O custo está registrado na decisão: a lição é que "o host não distingue ainda-não-começou de já-terminou" não se conserta com mais um campo, e sim tirando a identidade do cliente.
- **Dois subagentes em paralelo conflitaram no mesmo arquivo.** #207 e #208 tocam o mesmo `Kind`, o mesmo `git_pull`/`git_fetch` e o fim do mesmo `host.rs`. Resolver os conflitos à mão custou mais que rodar em série; a pré-fiação que fiz na UI (efeito e `cancelNetwork` genéricos antes de delegar) reduziu o estrago, mas não o eliminou.
- **Depurar o E2E foi a maior parte do tempo.** Três armadilhas seguidas: `data-testid="branch-menu"` está no **wrapper** (sempre visível), então "o menu está aberto?" não podia ser respondido por ele; o popover fecha em qualquer `pointerdown` fora dele, inclusive dentro do diálogo, então cada operação precisa reabrir o menu; e um botão focado que fica `disabled` move o foco para o `body`, então o `keydown` do Escape nunca chegava ao diálogo (o handler passou para a `window`).
- **Um teste flaky por `index.lock`.** O cenário da nota "nothing to cancel" falhou uma vez com `Unable to create '.git/index.lock'` — um `git status` concorrente com o `git merge`, possível agora que a operação de rede não segura mais o lock do shell. Não reproduziu nas rodadas seguintes e não foi investigado; fica como risco conhecido (um `git status` pode colidir com um comando local do próprio host).
- **O `git status` do teste de "o app responde durante" estourou um limite de 2 s** porque a ponte atende uma requisição por vez e estava atrás de `git.ignored`/`symlinks` da árvore. O limite virou 5 s: o que se prova é que o push não segura o host, não que a resposta é instantânea.

## Revisão independente (rodada 1 do desenho novo)

Dois eixos em paralelo (`gpt-6.1-terra`), fixed point `9097444`; correções em `3dd0961` (código) e `78c0f6a`/`5460783` (docs).

- **Spec, alto — cancel aceito podia terminar como `failed`.** Um cancel que chegava durante o pré-flight e uma falha logo depois (árvore suja, sem remoto) publicavam `failed`: o `git.cancel` respondia `true` e a operação não se reportava cancelada. `Net::end` passou a publicar `cancelled` quando o cancel foi aceito, mantendo a mensagem da própria operação — o motivo real continua visível —, e o teste da atomicidade foi atualizado para essa expectativa.
- **Spec, alto — um segundo start podia trocar o id em `pending`.** Se o host já tivesse terminado a primeira operação e a UI ainda não tivesse visto o snapshot, um Fetch aceito substituía o id e a mensagem do Push nunca aparecia (o diálogo ficava em `Pushing…` para sempre). A UI ganhou um `startNetwork` único, com guard síncrono e `pending !== null`.
- **Spec, médio — Escape durante o start era descartado.** Agora a intenção fica em `cancelOnStart` e o cancel é disparado com o id assim que o host responde.
- **Spec, médio — depois de prazo ou kill, `finish_network` não era chamado**, então um cancel tardio era aceito para um filho já morto. Passou a ser chamado em todo caminho, antes do dreno.
- **Standards, violação documentada — a spec e a evidência afirmavam que Playwright substitui o app real, contra o `AGENTS.md`.** O `AGENTS.md` mudou na `main` (#214, decisão da #210) enquanto esta frente rodava: a entrada de decisão que eu tinha escrito virou duplicata e saiu, e os três docs agora citam a decisão da #210.
- **Standards, smells de duplicação.** Os três starts do host viraram `spawn_net`; Update e Push na UI viraram `startNetwork`.
- **Rodada 2 (re-check do eixo Spec, escopo só das correções): sem achados.**

## Lições

- **Cancelamento é identidade, não flag.** Um flag observado por um laço resolve o kill; o que faltava era o host ser o dono da identidade da operação. Enquanto o cliente inventa o id, toda correção é uma corrida nova.
- **Se o trabalho é longo, ele não pode ser a vez de todo mundo.** A regra que resolveu a #150 não é sobre Git: é a mesma do Build/Run/Tests, e a exceção no lock do shell era o sintoma de que o Git de rede estava no lugar errado.
- **`data-testid` em wrapper não responde "está aberto?".** Para saber se um popover está montado, o teste precisa de um elemento de dentro dele.
- **Foco em botão que fica `disabled` vai para o `body`.** Atalho de teclado dentro de diálogo com ação em voo tem de ouvir a `window`, não o `div` do diálogo.

## Melhorias de ambiente (skill `retro`)

Candidatos para a próxima sessão, em ordem de severidade:

- **Padrão de código (juízo) que faltou ao revisor: atalho de teclado em diálogo com ação em voo ouve a `window`.** Um botão focado que fica `disabled` move o foco para o `body`, e o `keydown` nunca chega ao `div` do diálogo — foi o que fez o Escape não cancelar. É regra de julgamento (não há lint que pegue), então o lugar é a lista de padrões que o revisor aplica; hoje ela só existe espalhada em `AGENTS.md`.
- **Armadilha de navegação nos specs: `data-testid` no wrapper não responde "está aberto?".** `branch-menu` está no contêiner sempre montado, então `isVisible()` é sempre verdadeiro e o teste que "abre o menu" nunca abre. Duas saídas, ambas baratas: um `branch-menu-popover` no popover (testid novo, contrato permite) ou uma linha em `docs/MAPA.md` dizendo qual testid responde "aberto". Custou a maior parte do tempo de depuração do E2E.
- **Verificação: o `AGENTS.md` ficou para trás da decisão.** A regra de prova de UI mudou em `docs/decisions.md` (#210) enquanto esta frente rodava, e a spec desta frente chegou a contradizer o `AGENTS.md` por horas. A convenção de marcar a entrada antiga com "Substituída em parte" já existe; o que falta é o `AGENTS.md` citar essa entrada em vez de repetir a regra — assim a mudança tem um lugar só.
- **Economia de ferramenta: dois implementers em paralelo nos mesmos arquivos custam mais que rodar em série.** #207 e #208 tocam o mesmo `Kind`, o mesmo `git_pull`/`git_fetch` e o fim do mesmo `host.rs`; os conflitos custaram mais que a espera. Regra prática: fatiar por arquivo, não por ticket, quando os tickets são irmãos do mesmo mecanismo.

## Follow-ups

- **`index.lock` entre `git status` e um comando local do host** (visto uma vez no E2E): se voltar, serializar os comandos locais do Git no host, sem tocar no passo de rede.
- **Cancelar o merge local do Update** continua fora (decisão da #136).
- **Progresso da transferência** (bytes, porcentagem) não existe; o cancelamento é a única saída durante a espera.
- **`git.checkout`/`git.commit`/diff/log** continuam síncronos: são locais e não seguram a rede.
- Não verificado: Windows, macOS Intel e o app Tauri real com `agent-desktop` (opcional pela regra nova de verificação de UI).
