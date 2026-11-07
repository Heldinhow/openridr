# Evidência — Fetch explícito no painel Git (issue #113)

Data: 2026-11-06. Alvo verificado: macOS 27.x arm64 (Apple Silicon). Windows e macOS Intel **não** foram executados.

Código sob teste: `65c8bec` (host, #137) e `049398a` (UI, #138), mais as correções da revisão independente, sobre `f870388` (`origin/main`: #135 do #112, #130 do #111). Versão `0.3.0-alpha.11`. As capturas S1–S3 vieram do primeiro bundle; S4 e S5 foram refeitas com o bundle das correções (mensagem de prazo e estado ocupado mudaram).

Bundle: `src-tauri/target/debug/bundle/macos/OpenRidr.app`, `CFBundleShortVersionString` `0.3.0-alpha.11`, construído nesta sessão do próprio checkout (`bun install` + `bun run tauri build --debug --bundles app`) às 22:06 sobre o conteúdo que virou `65c8bec` + `049398a`.

UI dirigida **somente** no app Tauri real via `agent-desktop` (sem navegador, sem Playwright de sessão), com `OPENRIDR_HOME` isolado, o `.sln` do clone em argv e state próprio (`VERIFY_OPENRIDR_DESKTOP_STATE=$TMPDIR/verify-openridr-113-desktop`). Uma instância por vez (PID 4654), fechada pelo PID gravado.

## Precondições (não são passe da UI)

- **Um state do fixture local**: `scripts/git-remote-fixture.sh` criou `openridr-git-remote.8Wyqyl` com `origin.git` (bare), `user` e `peer` a partir do conteúdo público de `fixtures/OrderDesk`. Sem rede, sem login, sem `git config --global` (identidade só `--local` em cada clone).
- **Estado criado por shell, declarado**: a contraparte (`peer`) publicou `5036245` em `main` e a branch `published-113` (`252dde6`) antes de a UI agir. O clone `user` ficou em `1515ef3` com `## main...origin/main` — sem `↓`, exatamente o sintoma da issue.
- **O app não foi autor de nenhum estado de Git nesta rodada**: ele só leu e fez fetch. Nenhum commit, merge ou checkout foi feito pela UI.

## Quadro por cenário

| # | Cenário | Prova fora da UI (comando → resultado) | Status |
|---|---|---|---|
| S1 | O menu de branches oferece **Fetch**, e o Refresh continua sem rede | `git status -sb` no clone → `## main...origin/main` (sem contadores, refs velhas). UI: menu com `Upstream origin/main`, `main (current)`, `origin/main remote`, e os botões `New branch…`, `Refresh`, **`Fetch`**, `Commit…`, `Update…`, `Push…`; a barra mostra `main` sem `↓` | **passou** |
| S2 | Fetch atualiza o contador e a lista de branches remotas | UI: toast `Fetched origin: main is 0 ahead, 1 behind origin/main.`; a barra passa a `main ↓1`; a lista ganha `origin/published-113 remote`; `main (current) ↓1`. Fora da UI: `git status -sb` → `## main...origin/main [behind 1]`, `git for-each-ref refs/remotes` → `origin`, `origin/main`, `origin/published-113`, `git rev-parse --short HEAD` → `1515ef3` (inalterado), `git status --porcelain` → vazio | **passou** |
| S3 | `--prune` remove a branch apagada na contraparte | `peer`: `git push origin --delete published-113`; `user` ainda listava `origin/published-113`. Depois do clique em Fetch na UI: a lista mostra só `origin/main`; `git for-each-ref refs/remotes` → `origin`, `origin/main`. Nenhuma branch local, commit ou arquivo mudou (`HEAD` continua `1515ef3`, árvore limpa) | **passou** |
| S4 | Remoto que não responde não congela o app e promete só o que um fetch não toca | `git remote set-url origin https://192.0.2.1/openridr-113.git` (TEST-NET-1, não roteável) feito por shell. UI: clique em Fetch às `22:34:24`; o botão passou a `Fetching…` (`aria-busy`, captura S5) e, aos `22:35:38`, a mensagem `Fetch stopped: origin did not answer within 60 seconds. Your branch, index and working tree were not touched; refs already received stay.` em vermelho (`role=alert`), com o menu aberto e o app respondendo (o snapshot seguinte saiu normalmente). Fora da UI: `git for-each-ref refs/remotes` → `origin`, `origin/main` (nada movido), `HEAD` `1515ef3`, `pgrep -fl "192.0.2.1"` → nenhum descendente `git-remote-http` vivo. O remoto foi restaurado para o bare local depois | **passou** |
| S5 | O botão mostra que está trabalhando, e o app sobrevive ao estouro | Durante S4 o botão lia `Fetching…` com `aria-busy` (captura `s5-fetching-state.png`); depois do estouro voltou a `Fetch`, o menu continuou abrindo e o app seguiu utilizável | **passou** |

## Host tests (não é prova de UI)

`cargo test -p openridr-host -- fetch` → **11 passed** (o filtro também pega `debugger::tests::only_official_https_hosts_are_fetched`, que é de outra frente):

- `fetch_moves_the_behind_counter_and_the_remote_branch_list` — bare remoto real + clone `peer`: `behind` vai de 0 para 1, `HEAD` e a árvore não mudam, `origin/published` aparece na lista, mensagem nomeia o remoto e o contador.
- `fetch_prunes_a_branch_the_counterpart_deleted_and_keeps_local_work` — a ref de tracking some; uma branch **local** com o mesmo nome continua no mesmo commit.
- `fetch_is_allowed_with_a_dirty_tree_and_around_an_in_progress_merge` — merge conflitado real em andamento: fetch passa, `MERGE_HEAD` e o arquivo com marcadores continuam intactos.
- `fetch_without_a_remote_is_refused_with_the_apps_reason` — `git_no_remote`, nada roda.
- `fetch_reports_a_credential_failure_in_the_apps_own_words` — a classe de credencial vira mensagem do app (sem "terminal prompts"); qualquer outra falha mantém a cauda do stderr.
- `a_fetch_that_outruns_its_deadline_is_killed_and_changes_nothing` — prazo já esgotado: `failed`, mensagem "did not answer within", retorno em menos de 10 s e nenhuma ref movida.
- `a_fetch_against_a_remote_that_never_answers_returns_at_the_deadline` — listener TCP local que aceita a conexão e não responde, prazo de 1 s: retorna em segundos, nenhuma ref muda e **nenhum** `git remote-http`/`git-remote-http` sobrevive (`pgrep` pelo endereço). Falha-antes medida: sem o grupo de processos próprio, os dois processos ficam vivos segurando os pipes.
- `a_failing_fetch_never_echoes_credentials_from_the_remote_url` — remoto `https://user:secret-token@127.0.0.1:1/…`: o token não aparece nem na mensagem nem no `output`. Medido no teste: nesse formato de falha o próprio Git já omite o userinfo, então o mascaramento do app é provado onde o `output` é montado, com um stderr que carrega a credencial (`https://***@example.com`).
- `fetch_prune_cannot_reach_refs_outside_the_remote_tracking_namespace` — `remote.origin.fetch` espelhando em `refs/mirror/*`: o fetch do app não **cria**, não **atualiza** e não **apaga** essas refs (falha-antes medida duas vezes: sem o refspec explícito `refs/mirror/doomed` é apagada; sem `--refmap=` `refs/mirror/main` é criada).
- `fetch_resolves_the_remote_from_the_branch_configuration` — remoto `team/origin` (nome com `/`) resolvido por `branch.main.remote`; upstream local (`.`) cai no primeiro remoto.

Bateria completa: `cargo test -p openridr-host` → **496 passed, 0 failed, 64 ignored** (os ignorados são os testes isolados por processo, como antes). `cargo clippy --workspace --all-targets -- -D warnings` limpo; `cargo fmt --all` aplicado.

Falha-antes: sem `git::fetch` o host não tem como produzir `behind 1` sem `git pull`; os testes novos chamam `fetch`/`fetch_with`, que não existiam antes deste ticket. As guardas de regressão medidas foram duas: sem `process_group(0)` o teste do listener reprova listando os `git-remote-http` sobreviventes, e sem o refspec explícito o teste do refspec hostil reprova porque `refs/mirror/doomed` é podada.

## Onde o stderr cru do git ficou

O texto mostrado ao usuário é do app; a saída do git continua em `Operation.output`. O caso de prazo é o único em que o `output` pode estar vazio (o filho é morto antes de escrever) — a mensagem não depende dele.

## Não verificado

- **Windows e macOS Intel**: não executados.
- **Remoto autenticado/rede real** (HTTPS/SSH, credential helper respondendo): o remoto da rodada é um bare local; o caminho de credencial foi provado só pelo teste de host (`fetch_reports_a_credential_failure_in_the_apps_own_words`), que exercita o mapeamento da mensagem, não um servidor que peça credencial.
- **Prazo de 60 s no app real contra um remoto que aceita a conexão e trava no protocolo**: o cenário S4 usou `192.0.2.1` (TEST-NET-1), que não responde; o caminho do helper `git-remote-http` preso no meio do protocolo é provado só no teste de host com listener local (prazo de 1 s), não no app real.
- **Find Action `Fetch…` aberto pelo teclado**: a entrada existe e está wired (`git.fetch` com `gitFetchBlockedReason`), mas a rodada abriu o menu de branches pelo clique; o caminho do palette não foi exercitado no app real.
- **Botão desabilitado sem remoto no app real**: provado só pelo teste de host (`git_no_remote`) e pela leitura do `gitFetchBlockedReason`; não houve clone sem remoto aberto no app.
- **Push e pull pendurados**: fora de escopo, registrado em #136.
