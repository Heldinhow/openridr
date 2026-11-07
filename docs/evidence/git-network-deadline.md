# Evidência — Prazo em push e pull (issue #136)

Data: 2026-11-06/07. Alvo verificado: macOS 27.x arm64 (Apple Silicon). Windows e macOS Intel **não** foram executados.

Código sob teste: `44b81f2` (host, ticket #147) sobre `dcb19ea` (`origin/main`, fetch com prazo da #113). Versão `0.3.0-alpha.11`.

Bundle: `src-tauri/target/debug/bundle/macos/OpenRidr.app`, `CFBundleShortVersionString` `0.3.0-alpha.11`, construído nesta sessão do próprio checkout (`bun install` + `bun run tauri build --debug --bundles app`) sobre `44b81f2`.

UI dirigida **somente** no app Tauri real via `agent-desktop` (sem navegador, sem Playwright de sessão), com `OPENRIDR_HOME` isolado, o `.sln` do clone em argv e state próprio (`VERIFY_OPENRIDR_DESKTOP_STATE=/tmp/verify-136-desktop`). Uma instância por vez (PID 8207 na rodada 1, 40867 na rodada 2), fechada pelo PID gravado; `pgrep -x openridr` vazio no fim.

## Precondições (não são passe da UI)

- **Um state do fixture local**: `scripts/git-remote-fixture.sh` criou `openridr-git-remote.DMp0fP` com `origin.git` (bare), `user` e `peer` a partir do conteúdo público de `fixtures/OrderDesk`. Sem rede, sem login, sem `git config --global`.
- **Estado criado por shell, declarado**: o clone `user` ganhou um commit local `9051862` (`local change before the deadline proof`) para o push ter o que enviar. Baseline do bare: `6b951f8`. O app não foi autor de nenhum estado de Git nesta rodada — ele só leu, tentou push/update e, no S3, empurrou o commit que já existia.
- **O remoto que não responde** é um listener TCP local (`python3`, `127.0.0.1:18736`) que **aceita a conexão e nunca responde** — o mesmo cenário do teste de host. Ele foi morto pelo PID no fim (`lsof -nP -iTCP:18736` → vazio).

## Quadro por cenário

| # | Cenário | Prova fora da UI (comando → resultado) | Status |
|---|---|---|---|
| S0 | **Medição que abriu um achado**: push contra `192.0.2.1` (TEST-NET-1) **não** exercita o prazo de 120 s | Push disparado pela UI às `23:49:44`; aos ~75 s o diálogo mostrou o erro **cru do Git**: `fatal: unable to access 'http://192.0.2.1/repo.git/': Failed to connect to 192.0.2.1 port 80 after 75005 ms: Couldn't connect to server` (`s0-push-testnet1-git-own-timeout.png`). O prazo do app (120 s) nunca disparou: o Git desiste da **conexão** antes. O bare continuou em `6b951f8`, o clone em `[ahead 1]` | **falhou (esperado diferente)** — o endereço não roteável não prova o prazo do push; o fetch da #113 escapa porque 60 s < 75 s |
| S1 | Push contra um servidor que aceita e não responde para no prazo, com a mensagem do app | `git remote set-url origin http://127.0.0.1:18736/repo.git` (shell). UI: clique em Push às `23:52:30`; botão em `Pushing…` (captura `s1-push-waiting.png`); às `23:54:45` (t+131 s) o diálogo leu `Push stopped: origin did not answer within 120 seconds. The push may already have reached the remote: use Fetch to check the branch before pushing again. Your branch, index and working tree were not touched.` (`s1-push-stopped.png`). **A redação mudou depois da revisão independente** (o remoto não é nomeado e o estado local não é prometido, porque `git push` resolve `pushRemote`/`pushDefault` e um hook `pre-push` pode ter rodado); o texto atual é o dos testes de host e a captura é da redação anterior — não refeita por falta do slot da GUI. Fora da UI: bare `main` **continua** `6b951f8`, clone `## main...origin/main [ahead 1]`, `branch.main.remote` continua `origin` (o push nem chegou a escrever upstream) | **passou** |
| S2 | Depois do estouro o app volta a responder e mostra o estado real | Ainda com o remoto no listener, o painel Git foi aberto pela UI: `Current branch: main · upstream origin/main · 1 ahead, 0 behind`, lista de branches com `main (current) ↑1` e `origin/main remote`, e o Git log carregou os dois commits (`s1-after-deadline.png`) — chamadas de `git.status`/`git.branches`/`git.log` respondidas **depois** do prazo | **passou** |
| S3 | Update contra o mesmo servidor para no prazo de 60 s e não deixa merge pela metade | UI: clique em Update às `23:58:08`; botão em `Updating…`; às `23:59:17` (t+69 s) o diálogo leu `Update stopped: origin did not answer within 60 seconds. Nothing was merged; your branch, index and working tree were not touched, and refs already received stay.` (`s2-update-stopped.png`). Fora da UI: `.git/MERGE_HEAD` **não existe**, `HEAD` continua `9051862`, `git status -sb` → `## main...origin/main [ahead 1]` | **passou** |
| S4 | O prazo não quebra um push saudável | Remoto restaurado para o bare local (shell). UI: Push → toast `Pushed main to origin.` (`s3-push-ok.png`). Fora da UI: `git -C origin.git rev-parse --short main` → `9051862`; clone → `## main...origin/main` (sem ahead) | **passou** |

## O prazo é teto, não promessa de mensagem

O contrato que a issue pede é **não travar o host**, e ele vale nos dois modos de falha, mas a frase do app só é garantida em um deles:

- **Conexão aceita e servidor calado** (S1/S3, e o teste de host): o prazo do app dispara e a mensagem é a dele.
- **Remoto que descarta pacotes** (S0, `192.0.2.1`): o próprio Git desiste da conexão antes — 75 s medidos — e o diálogo mostra o erro cru do Git. A espera continua limitada (75 s < 120 s), só não termina na nossa frase.

Por isso nada na UI ou nas mensagens promete que todo push/pull morto termina com o texto do app, e o `PUSH_DEADLINE` não foi baixado para "ganhar a corrida" do timeout de conexão do Git: o prazo é teto de espera, não orçamento de transferência nem garantia de redação.

## Host tests (não é prova de UI)

`cargo test -p openridr-host --lib -- git::tests` → **54 passed**, incluindo:

- `a_push_against_a_remote_that_never_answers_returns_at_the_deadline` — listener que aceita e não responde, prazo de 1 s: volta `failed` em menos de 15 s com a mensagem do app, **sem** "Pushed" e **sem** promessa de upstream, refs do bare intactas, `HEAD` local intacto e nenhum descendente vivo (`pgrep` pela porta).
- `an_update_against_a_remote_that_never_answers_returns_at_the_deadline` — mesmo listener: `failed` em menos de 15 s, mensagem com "Nothing was merged" e **sem** `merge --abort`, `HEAD` e árvore intactos, sem `MERGE_HEAD`, `status.operation` nulo, refs de tracking inalteradas, nenhum descendente vivo.
- `a_fetch_against_a_remote_that_never_answers_returns_at_the_deadline` — o caso da #113 continua verde com o helper de listener extraído (`silent_remote`).

**Falha-antes medida duas vezes** (código trocado para `run()` sem prazo, depois restaurado):

1. Com o peer do teste fechando em 10 s: as duas chamadas esperaram os 10 s e voltaram com o erro **cru** do Git (`Recv failure: Connection reset by peer`) em vez da mensagem do app — os dois testes reprovam.
2. Com o peer que nunca fecha: `timeout 45 cargo test … a_push_against_a_remote_that_never_answers` → **exit 124**, ou seja, a chamada não retorna em 45 s. É o congelamento que a issue descreve.

## O que ficou sem prova

- **A fila atrás do lock durante a espera não foi medida no app real.** O lock global do `host_call` continua no lugar (decisão da frente): durante os 120 s/60 s as outras chamadas de host ficam enfileiradas. A prova acima é o **retorno no prazo** e o app respondendo depois dele, não "o app nunca congela".
- **Cancelamento pela UI** não existe (issue filha).
- **Hook `pre-merge-commit` preso** no merge local do Update: sem prazo por decisão, não exercitado.
- **Windows e macOS Intel**: não executados.
- **Remoto autenticado e rede real**: não exercitados; o cenário de prazo usa servidor local que aceita e não responde.
- **A redação atual da mensagem do push não foi vista no app**: a captura S1 é da versão anterior à revisão independente (a mudança tira o nome do remoto e a promessa sobre o estado local). O texto atual é o dos testes de host; a tela não foi refeita por falta do slot da GUI. A mensagem do update não mudou e a captura S3 continua valendo.

## Capturas

`docs/evidence/git-network-deadline/` — `s0-push-testnet1-git-own-timeout.png`, `s1-push-dialog.png`, `s1-push-waiting.png`, `s1-push-stopped.png`, `s1-after-deadline.png`, `s2-update-dialog.png`, `s2-update-waiting.png`, `s2-update-stopped.png`, `s3-push-ok.png`. As capturas foram recortadas para a área do diálogo/painel: o painel NuGet da sessão mostrava o caminho do SDK com o diretório home, que não vai para a evidência.
