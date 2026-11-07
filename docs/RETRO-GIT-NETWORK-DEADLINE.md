# RETRO — Prazo em push e pull (issue #136)

Rodada de 2026-11-07, `44b81f2` (+ `ac168a7` da spec), macOS arm64. Spec: [SPEC-GIT-NETWORK-DEADLINE](SPEC-GIT-NETWORK-DEADLINE.md). Evidência: [evidence/git-network-deadline.md](evidence/git-network-deadline.md).

## O que se sustentou

- **O prazo vale no binário, não só no host.** No app real, um push contra um servidor que aceita e não responde voltou em t+131 s com `Push stopped: origin did not answer within 120 seconds…`; um Update contra o mesmo servidor voltou em t+69 s com `Update stopped: … Nothing was merged…`. Nenhum dos dois deixou estado pela metade: bare intacto no push, sem `MERGE_HEAD` e com `HEAD`/árvore intactos no update.
- **Reusar a máquina da #113 pagou.** `run_with_deadline` (grupo de processos, `kill_group`, drenagem por canal) já estava certo; push e o fetch do pull só precisaram de prazo, mensagem e teste. O diff de código ficou em ~170 linhas, quase metade de teste.
- **Separar fetch de merge tornou a mensagem verdadeira.** "Nothing was merged" só é garantível porque o merge nunca começa quando o fetch estoura — a estrutura que o #112 já tinha escolhido por outro motivo (provar a posse do abort) resolveu a semântica do timeout de graça.
- **Push saudável não quebrou.** Com o remoto real de volta, `Pushed main to origin.` e o bare recebeu o commit: o prazo não virou atalho para matar operação legítima.
- **A falha-antes foi medida, duas vezes.** Com o `run()` sem prazo e um peer que fecha em 10 s, as duas chamadas voltam com o erro cru do Git; com um peer que nunca fecha, `timeout 45` mata o teste do push com exit 124. O teste novo reprova sem a correção, que é o que o retro da #113 pediu.

## O que quebrou / custou

- **A premissa do endereço não roteável estava errada para o push.** `192.0.2.1` (TEST-NET-1) parecia o cenário óbvio — foi o da prova da #113 — mas o Git desiste da **conexão** em 75 s e mostra o erro dele: o prazo de 120 s nunca dispara. A primeira rodada no app real produziu `fatal: unable to access … Failed to connect to 192.0.2.1 port 80 after 75005 ms`, o que parece defeito e é só o timeout do cliente vencendo a corrida. Para o fetch (60 s) o endereço funciona por acidente: 60 s < 75 s.
- **A prova no app real precisou de um servidor local que aceita e não responde.** É o cenário honesto (proxy ou servidor travado), mas não é "rede real": o que se prova é o prazo e o kill, não o comportamento de um remoto de verdade.
- **Duas rodadas de app para uma captura.** A primeira mensagem de Update saiu cortada no recorte (o diálogo é mais alto do que a área que eu recortei) e precisou de uma segunda sessão; para não repetir o setup de ferramentas, o `OPENRIDR_HOME` da primeira foi copiado para um state novo — o trust voltou a perguntar, o setup não.
- **O recorte das capturas tem custo e limite.** O painel NuGet da sessão mostrava o caminho do SDK com o diretório home, então as imagens foram recortadas para o diálogo/painel Git; o `screenshot --app` do `agent-desktop` sai deslocado ~135 px lógicos à esquerda, então recortes a partir de `x=0` perdem a borda esquerda da janela.
- **`bun install` antes do primeiro commit.** Worktree nova não tem `node_modules`, e o hook de pre-commit roda `bun run check` (`tsc`); sem instalar, o commit morre em `tsc: command not found`.

## Revisão independente (rodada 1) — dois achados médios, corrigidos

Ambos eram da mensagem do push morto, não do prazo:

- **Nomear o remoto podia mandar a pessoa conferir o servidor errado.** `git push` sem argumento de repositório resolve o destino por `branch.<nome>.pushRemote`, depois `remote.pushDefault`, depois o remoto do upstream — e o app só lê o último. A mensagem passou a dizer "the remote" e a não sugerir o Fetch (que busca o remoto do upstream) como forma de conferir; o diálogo continua mostrando o remoto que o app pretende usar.
- **"Sua branch, index e árvore não foram tocados" não é garantido no push.** O hook `pre-push` roda antes da transferência e pode escrever no repositório; a frase saiu. O fetch mantém a promessa porque não roda hook que toque a árvore.
- **Consequência:** a captura `s1-push-stopped.png` é da redação anterior à revisão. O texto atual é o dos testes de host, e a tela não foi refeita por falta do slot da GUI (a #52 estava com a janela). O comportamento provado (prazo dispara, app mostra a mensagem do app, estado local intacto fora de hook) não mudou.

## Lições

- **Prazo contra endereço "que não responde" pode provar o timeout do cliente, não o do app.** Antes de usar TEST-NET-1 como cenário de prazo, medir qual dos dois vence: o cliente tem timeout próprio de conexão (75 s no Git) e ele pode ser menor que o teto que se quer provar.
- **"Teto" e "mensagem do app" são promessas diferentes.** O que a issue pede é espera limitada; a redação do app só é garantida quando a conexão é aceita e o servidor cala. Escrever isso no spec e na evidência evita vender "todo push morto mostra a nossa frase".
- **Reusar o mecanismo da frente anterior é o caminho mais curto para a mesma garantia.** A #113 deixou `run_with_deadline` com kill de grupo e drenagem limitada; aqui não houve mecanismo novo, só prazo e mensagem — e nenhum dos achados da #113 (kill de grupo, promessa de mensagem) voltou.
- **Teste de prazo só é guarda se o cenário for determinístico.** O listener local que aceita e não responde é o único jeito de o teste falhar por ausência do prazo, e não por acaso da rede.

## Follow-ups

- **Cancelar push/pull em andamento pela UI**: issue filha (enhancement, *Related to #136*). O prazo libera o host; cancelar exige estado de operação em voo no host.
- **Prazo no merge local** (hook `pre-merge-commit` preso) continua fora, como no `git commit`.
- **`git clone`/`ls-remote`/submodules**: não existem no host hoje; se entrarem, entram sob o mesmo contrato de teto.
- Não verificado: Windows, macOS Intel, remoto autenticado/rede real, e a fila de chamadas atrás do lock durante a espera (não medida no app real).
