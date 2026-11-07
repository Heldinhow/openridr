# RETRO — Fetch explícito no painel Git (issue #113)

Rodada de 2026-11-06, `65c8bec` + `049398a` + `64ff24e` + `44949e1` + `cc79d75`, macOS arm64. Spec: [SPEC-GIT-FETCH](SPEC-GIT-FETCH.md). Evidência: [evidence/git-fetch.md](evidence/git-fetch.md).

## O que se sustentou

- **O sintoma da issue fechou de verdade.** Com a contraparte já tendo empurrado um commit, o painel lia `0 ahead, 0 behind`; depois do clique em Fetch, `0 ahead, 1 behind` e a branch publicada apareceu na lista. Provado no app real e fora dele (`git status -sb` → `[behind 1]`).
- **Refresh continuou barato.** A decisão de não pôr rede no Refresh manteve o menu abrindo instantaneamente e o foco da janela sem tráfego; o Fetch é o único caminho que fala com o remoto, e é explícito.
- **`--prune` pagou o custo.** A branch apagada na contraparte saiu da lista no mesmo clique, sem tocar em branch local nem em commit — o teste de host cobre inclusive a branch local homônima.
- **O prazo é real, e mata o grupo.** Contra `192.0.2.1` o app voltou com mensagem do app em 74 s e nenhum descendente vivo; no host, um listener que aceita e não responde prova o mesmo em 1 s.
- **Teste com remoto real de novo.** Bare + `user` + `peer` provaram contador, prune, árvore suja/merge em andamento, sem remoto, credencial e prazo sem mock de Git.

## O que quebrou / custou

- **O primeiro recorte do prazo estava errado no ponto que mais importa.** `child.kill()` mata só o `git`; `git remote-http`/`git-remote-http` herdam os pipes e continuam vivos. Medido: os dois processos sobrevivem ao kill do pai. Sem o kill de grupo, a leitura dos pipes ficaria bloqueada — o congelamento que o prazo existe para evitar. Só apareceu na revisão independente.
- **O refspec explícito não bastava.** Achado alto da rodada 2: o Git ainda aplica `remote.<nome>.fetch` como refmap adicional, então `refs/mirror/main` nascia mesmo com o refspec nomeado. Foi preciso `--refmap=`; a falha-antes foi medida duas vezes (prune apagando `refs/mirror/doomed`, refmap criando `refs/mirror/main`).
- **A primeira mensagem de prazo prometia demais.** "Nothing was changed" não é garantível: um kill pode cair depois de o Git atualizar refs. Virou "your branch, index and working tree were not touched; refs already received stay", e o ticket #137 foi corrigido junto.
- **O teste de credencial provava pouco.** A asserção era "o segredo não aparece", e o próprio Git já omite o userinfo na falha de conexão — passaria sem mascaramento nenhum. Agora o mascaramento é provado onde o `output` é montado, com um stderr que carrega a credencial.
- **O teste de prazo com `Duration::ZERO` não discriminava.** Ele passava com e sem o kill de grupo. O listener local + `pgrep` pelos descendentes é o que falha sem a correção.
- **Duas rodadas de app para uma mudança de texto.** A mensagem de prazo mudou depois da revisão, e a captura S4 precisou ser refeita com bundle novo (~2 min de build + 70 s de espera), disputando o slot do app com outra sessão.

## Lições

- **Prazo em comando de rede só vale se matar a árvore de processos.** `kill()` no líder não fecha os pipes que os filhos herdaram; em Git isso é sempre verdade (`remote-http`, ssh, askpass). Grupo de processos + espera limitada na leitura é o par mínimo.
- **`--prune` e refspec configurado são uma armadilha de contrato.** O alcance do prune é o que o repositório manda, não o que o botão promete. Nomear o refspec **e** zerar o refmap (`--refmap=`) é o que torna a promessa verdadeira; um sem o outro deixa um buraco (o primeiro achado da rodada 2 foi exatamente esse).
- **Mensagem de erro é contrato.** "Nada mudou" é uma promessa forte e quase sempre falsa para operação interrompida; descrever o que a operação nunca toca é sempre verdadeiro.
- **Teste que passa com e sem a correção não é guarda.** Vale medir a falha-antes de cada achado de revisão: dois dos testes novos só viraram prova depois de rodar sem a correção.
- **Um teste que mede o próprio Git descobre o que ele já faz.** O Git já mascara o userinfo na falha de conexão; sem medir, o teste de credencial seria teatro.
- **Fixture de remoto local continua pagando.** Terceira frente seguida usando `git-remote-fixture.sh` sem mudança nenhuma.

## Follow-ups

- **#136** (*Related to #113*): push e pull continuam sem prazo, e o lock global do `dispatch` faz um `git` pendurado congelar o app inteiro. O fetch desta frente é a prova de que o padrão funciona; falta aplicar (ou tirar o lock do caminho longo).
- **`Fetch…` no Find Action pelo teclado** não foi exercitado no app real (a prova foi pelo menu de branches); o caminho existe e está wired.
- **Botão desabilitado sem remoto** provado só por teste de host e leitura do gate, não num clone sem remoto aberto no app.
- **Tags, submodules e remoto autenticado** continuam fora de escopo; o fetch não busca tags explicitamente (elas vêm pelos commits alcançáveis).
- Não verificado: Windows, macOS Intel, remoto autenticado/rede real, e o caminho de helper preso no meio do protocolo no app real (só no host).
