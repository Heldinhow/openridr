# Evidência — Cancelar push, update e fetch (issue #150)

Data: 2026-11-07. Alvo verificado: macOS arm64 (Apple Silicon). Windows e macOS Intel **não** foram executados.

Código sob teste: `6b50741` (host + shells + UI dos tickets #206–#208, no desenho assíncrono) sobre `9097444` (`origin/main`). Desenho: [SPEC-GIT-NETWORK-CANCEL](../SPEC-GIT-NETWORK-CANCEL.md) — o Git de rede virou **start** (id do host, estado no snapshot), como Build/Run/Tests.

## Precondições (não são passe da UI)

- O E2E cria o próprio repositório descartável (`setupRepo` em `tests/e2e/git.spec.ts`); nada roda `git` no checkout do app.
- **O remoto que não responde** é um `net.Server` local que **aceita a conexão e nunca responde** — o mesmo cenário dos testes de host e da #136. Fechado no `finally` do teste.
- A nota "nothing to cancel" usa um remoto real (bare local) e um hook `post-merge` que segura o merge por 3 s: é o que garante que o clique chegue **depois** do fim do passo de rede.

## Host tests (não é prova de UI)

`cargo test -p openridr-host --lib` → **469 passed**, 0 failed, 14 ignored.
`cargo test -p openridr-host --test host` → **111 passed**, 0 failed, 27 ignored.

Os três testes de integração (`--test host`) provam, cada um, o contrato inteiro pelo seam `Host::dispatch`:

- `a_push_start_answers_at_once_and_its_cancel_publishes_the_cancelled_state`, `an_update_start_answers_…`, `a_fetch_start_answers_…` — o start volta em **menos de 1 s** com `{ id: "git:N" }` e o snapshot publica `running` com o `kind` certo; um `git.status` despachado **com a operação em voo responde** (nada segura o host); um segundo start é recusado com `invalid_state`; `git.cancel { id }` devolve `true` e o snapshot publica `cancelled` com a mensagem do app em **menos de 5 s** (o prazo é 120 s/60 s, então o cancelamento é o único motivo possível do retorno); `HEAD`, árvore, branch e refs de tracking ficam intactos; nenhum descendente do `git` sobrevive (`pgrep` pela porta do listener, pulado onde não existe).
- Cancelar com id desconhecido, com id já encerrado ou sem nada em voo devolve `cancelled: false` e não mata nada.

Os unitários de `git.rs` cobrem o registro: `the_host_issues_the_id_and_refuses_a_second_operation`, `the_end_of_the_network_step_is_atomic_with_the_cancel`, `a_cancel_for_a_finished_operation_does_not_stop_the_next_one`, `a_cancel_after_the_network_step_reports_nothing_to_stop` (o merge local do Update não é cancelável) e os três `*_stopped_by_the_cancel_returns_before_the_deadline` (retorno em menos de 5 s, mensagem do app, estado local intacto, nenhum descendente vivo).

## Falha-antes medida

Com o `if net.cancelled(id)` removido do laço de espera de `run_with_deadline` (o resto igual), `a_push_start_answers_at_once_and_its_cancel_publishes_the_cancelled_state` reprova em **5,0158 s** com `the cancel must return well before the 120 s deadline: 5.0158165s` — o cancel não tem efeito. Com um listener que segura a conexão além do prazo, o estado terminal só aparece em **120,06 s** com `Push stopped: the remote did not answer within 120 seconds…`, ou seja, sem a correção o push roda até o prazo. Revertido depois da medição (`git diff` do laço idêntico ao commitado).

## Playwright (modo web, só `tests/e2e/git.spec.ts`)

`bunx playwright test tests/e2e/git.spec.ts` → **11 passed** (1,4 min), incluindo os dois cenários novos:

- **`cancelling a push, an update and a fetch returns in seconds while the app answers`** — com o `origin` apontando para o listener que não responde: Push → `Cancel push` devolve `Push cancelled: … may already have reached the remote…` com o diálogo aberto; um `git.status` pelo mesmo bridge responde em **menos de 5 s durante o push**; Escape no diálogo em voo cancela em vez de fechar; Update → `Cancel update` mostra `Update cancelled: … nothing was merged…` e **não existe `MERGE_HEAD`** depois; Fetch → `Cancel fetch` mostra `Fetch cancelled: …`.
- **`an update whose fetch already finished says there is nothing to cancel`** — remoto real + hook `post-merge` de 3 s: o clique em `Cancel update` chega com o fetch já terminado, o host responde `cancelled: false` e o diálogo mostra `Nothing to cancel: the update is already merging.`; o merge termina sozinho e o diálogo fecha com a mensagem do app.

Os 9 cenários que já existiam no spec continuam verdes (o contrato novo do `git.push` — start que devolve id, "sem remoto" como `failed` no snapshot — está coberto pelo cenário de log/checkout/pull/push).

**Uma rodada falhou por carga, não por código**: com a máquina ocupada (a suíte inteira levou 2,5 min contra 1,4 min), `full-file diff shows the whole file…` estourou `toHaveCount` em 29 s; isolado passa em 5,7 s e a rodada seguinte passou inteira. É um teste de diff, sem relação com o cancelamento — fica registrado como intermitência de carga, não como verde.

**Dois defeitos de UI que só a prova de UI pegou**, corrigidos na mesma rodada: o botão de cancelar aparecia enquanto o start ainda estava em voo (sem id para nomear, o clique não fazia nada) e o Escape do diálogo usava uma closure anterior ao `pending` (o handler ficava com `pending: null` e o cancel não tinha id). Ambos estão no commit `6560827`.

## O que ficou sem prova

- **App Tauri real (`agent-desktop`)**: pela decisão de 2026-11-07 (“A superfície de verificação de UI segue o que mudou”, `docs/decisions.md`), interface pura se prova no Playwright em modo web; a checagem no binário ficou opcional e **não foi feita**. O que ela cobriria — a janela e o menu nativo — não muda nesta frente.
- **Windows e macOS Intel.**
- **Cancelar o merge local do Update**: fora de escopo por decisão da #136; o que se prova é que o cancel ali devolve `false` e a UI diz isso.
- **A ponte de dev (`host-http`) medida à mão**: a fila/overtake saiu e o laço atende uma requisição por vez; o comportamento é o mesmo do E2E, que roda a mesma ponte, mas não houve medição isolada de latência nesta rodada.

## Capturas

Os artefatos do Playwright (`test-results/`, `trace.zip`) não são versionados; as asserções acima são a prova.
