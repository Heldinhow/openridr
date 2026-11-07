# Evidência — Cancelar push, update e fetch (issue #150)

Data: 2026-11-07. Alvo verificado: macOS arm64 (Apple Silicon). Windows e macOS Intel **não** foram executados.

Código sob teste: _a preencher_ (host + shells + UI dos tickets #206–#208 no desenho assíncrono) sobre `9097444` (`origin/main`).

Desenho: [SPEC-GIT-NETWORK-CANCEL](../SPEC-GIT-NETWORK-CANCEL.md). O Git de rede passou a ser **start** (id do host, estado no snapshot), como Build/Run/Tests.

## Precondições (não são passe da UI)

- **Um state do fixture local**: `scripts/git-remote-fixture.sh` cria `origin.git` (bare), `user` e `peer` a partir do conteúdo público de `fixtures/OrderDesk`. Sem rede, sem login, sem `git config --global`.
- **O remoto que não responde** é um listener TCP local (`python3`, `127.0.0.1:<porta>`) que **aceita a conexão e nunca responde** — o mesmo cenário dos testes de host e da #136. Morto pelo PID no fim (`lsof -nP -iTCP:<porta>` → vazio).

## Host tests (não é prova de UI)

_a preencher: `cargo test -p openridr-host` com a lista dos testes novos e o resultado._

## Falha-antes medida

_a preencher: com o `if net.cancelled(id)` removido do laço de espera, o teste de 5 s reprova; tempo medido e erro cru._

## Quadro por cenário (app Tauri real, `agent-desktop`)

| # | Cenário | Prova fora da UI (comando → resultado) | Status |
|---|---|---|---|
| S1 | **Push cancelado** devolve o controle em segundos, com a mensagem do app | _a preencher_ | _a preencher_ |
| S2 | **Update cancelado** devolve o controle em segundos, sem merge pela metade | _a preencher_ | _a preencher_ |
| S3 | **Fetch cancelado** devolve o controle em segundos | _a preencher_ | _a preencher_ |
| S4 | **Escape no diálogo em voo** cancela em vez de fechar | _a preencher_ | _a preencher_ |
| S5 | **"Nothing to cancel"** quando não há operação de rede em voo | _a preencher_ | _a preencher_ |
| S6 | O app **responde durante** a operação (status/editor) | _a preencher_ | _a preencher_ |
| S7 | Um push **saudável** continua funcionando depois de tudo | _a preencher_ | _a preencher_ |

## Ponte de dev (`host-http`)

_a preencher: `/health`, `git.status` durante uma operação em voo, ordem preservada sem a fila._

## O que ficou sem prova

_a preencher_

## Capturas

_a preencher_
