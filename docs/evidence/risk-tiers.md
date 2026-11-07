# Evidência — risco de PR por caminho tocado (issue #74)

Spec: [SPEC-RISK-TIERS.md](../SPEC-RISK-TIERS.md). Decisões: `docs/decisions.md` (2026-11-06, "Risco de
PR por caminho tocado"). Tickets: #123 (tier), #122 (agregador), #124 (revert rate), #125 (job `risk`),
#126 (docs). PR: [#131](https://github.com/Heldinhow/openridr/pull/131).

Ambiente: macOS Apple Silicon, `bun 1.3.9`, `rustc 1.99.0`, SDK .NET 8.0.408, `gh` 2.83.1,
`actionlint 1.7.12`, `zizmor 1.32.0`, `shellcheck 0.11.0`, `dash 0.5.13.5`. Base: `origin/main`
`f870388` (merge na branch `6aa8f1b`).

## `scripts/ci-risk-tier.sh` (#123)

| Prova | Resultado |
|---|---|
| `sh scripts/ci-risk-tier-selfcheck.sh` | **44 casos, 44 ok** |
| Mesmo autoteste com `dash` (o `/bin/sh` do runner) | **44 casos, 44 ok** |
| Falha-antes: stub `echo low` no lugar do script | **33 de 44 falham**, inclusive os três `err-*` |
| Falha-antes da rodada 1: o script anterior aos três consertos | **3 de 44 falham** (`cargo-target-dep`, `cargo-same-dependency-both-sides`, `cargo-unparseable-value`) |
| Falha-antes da rodada 2: o script anterior ao conserto do header indentado | **1 de 44 falha** (`cargo-indented-header`: esperado `high`, veio `medium`) |
| `sh scripts/ci-risk-tier.sh origin/main HEAD` no branch | `high`, com os motivos: `.github/workflows/ci.yml` (CI), `AGENTS.md` (processo), `scripts/ci-*` (CI), e os docs como `low` |

Casos cobertos: uma família de caminho por caso (`docs/**`, `*.md` na raiz, `AGENTS.md`, `*.css`,
`src/**`, `crates/**`, `crates/*/tests/**`, `tests/**`, `.github/workflows/**`, `.github/rulesets/**`,
`scripts/release*`, `scripts/vendor-*`, `scripts/ci-*`, entitlements, `tauri.conf.json`, `trusted.rs`,
`fixtures/**`, `rust-toolchain.toml`, caminho desconhecido), mistos (`docs+src` = medium,
`src+workflow` = high, `docs+tests` = low, `AGENTS.md+docs` = high), dependência (`Cargo.lock`,
`bun.lock`, manifesto adicionado, dependência acrescentada/removida em `Cargo.toml`, `version` bump =
medium, dependência acrescentada em `src-tauri/Cargo.toml`, dependência acrescentada em
`package.json`, **reordenar** dependência = medium, `scripts` do `package.json` = medium, versão de
dependência trocada = high) e erros (sem argumento, ref inexistente, diff vazio = exit 2).

Duas armadilhas encontradas e corrigidas durante a implementação, ambas com o autoteste como prova:

1. `bump` terminava em `[ "$value" -gt "$rank" ] && rank=$value`; sob `set -e` o `&&` que devolve falso
   aborta o script. Virou `if`.
2. O loop sobre caminhos rodava com globbing ligado: um caminho que parecesse padrão seria expandido.
   Virou `set -f` em volta do loop.
3. O caso `pkg-scripts-only` reprovava **corretamente** no começo: o fixture acrescentava
   `"devDependencies": {}`, que é uma tabela nova e portanto mudança de dependência. O fixture foi
   corrigido, não o script.

## `scripts/ci-aggregate.sh` (#122)

| Prova | Resultado |
|---|---|
| `sh tests/shell/ci-aggregate.test.sh` | **13 casos, todos ok** |
| Mesmo teste com `dash` | **13 casos, todos ok** |
| Mutação da asserção: `rust: failure` → `rust: bogus` no teste | reprova com `stderr does not contain 'rust: bogus'` (a asserção é viva) |

Casos: PR tudo verde; `push` e `workflow_dispatch` com os dois PR-only `skipped`; PR com `governance`
pulado (**reprova**); PR com `risk` pulado (**reprova**); `rust: failure`; `security: cancelled`;
`push` com `rust: skipped` por acidente (**reprova**); `risk` ausente do `needs` num PR (**reprova** —
garantia nova: remover o gate da lista deixa de ser silencioso); `NEEDS` vazio e não-objeto (exit 2);
`PR_ONLY`/`EVENT` ausentes (exit 2).

## `scripts/ci-risk-gate.sh` (#125)

| Prova | Resultado |
|---|---|
| `sh tests/shell/ci-risk-gate.test.sh` | **17 casos, todos ok** |
| Mesmo teste com `dash` | **17 casos, todos ok** |
| Falha-antes: o gate anterior (comparação por timestamp) contra o teste novo | **12 de 17 falham** |
| Falha-antes da rodada 2: o gate anterior ao conserto do `COMMENTED` | **1 de 17 falha** (`comment-after-approval`) |
| Mutação: aceitar qualquer `author_association` | reprova `approval-without-write-access` |

Casos: `low`/`medium` não chamam o GitHub (sem gate); aprovação no head (passa); aprovação num commit
anterior (reprova); aprovação retirada por `CHANGES_REQUESTED` posterior (reprova); retirada por
`DISMISSED` (reprova); reaprovação depois da mudança de ideia (passa); aprovação de `CONTRIBUTOR`
(reprova); `COMMENTED` (reprova); nenhuma review (reprova); histórico misto, um revisor pediu mudanças e
outro aprovou o head (passa, e o log nomeia quem aprovou); PR errado (o stub `gh` falha na URL → exit 2);
erro de API na lista de reviews (exit 2, nunca passe); tier desconhecido (exit 2); `REPO` ausente
(exit 2); `HEAD_SHA` ausente (exit 2).

O stub `gh` aplica o `--jq` de verdade no fixture com o `jq` real, então o teste exercita a expressão
que roda no CI, não uma reimplementação dela.

## Run real no GitHub (PR #131) — o gate bloqueando o próprio PR

Run `42322621771`, head `6aa8f1b`, tier calculado `high`:

| Job | Resultado |
|---|---|
| `TypeScript (tsc + unit)` | pass (20 s) |
| `Governance (workflows, PR title, PR body)` | pass (22 s) — inclui `actionlint` **com shellcheck** |
| `security` (gitleaks) | pass (14 s) |
| `Rust (fmt, clippy, host tests)` | pass (3 min 41 s) |
| `Risk tier and maintainer approval` | **fail (13 s)** — esperado |
| `ci` (agregador) | **fail (10 s)** — `required jobs did not pass: risk: failure` |

Passos do job `risk`: `Compute the risk tier` **success** (tier `high`), `Label the pull request`
**success** (o PR ficou com `risk:high`), `Require the maintainer for a high-risk pull request`
**failure** com
`::error::high-risk pull request: it needs an approval from the maintainer of the current revision (last commit 2026-11-06T22:50:10Z). Approve the pull request, then re-run the failed jobs with: gh run rerun --failed <run-id>`,
`Arm auto-merge for a low-risk pull request` **skipped** (correto: o tier é alto).

Ou seja: o caminho de risco alto **não pode** ser mergeado sem o mantenedor, e o agregador `ci` (o check
obrigatório do ruleset) reprova por causa dele — sem nenhum check obrigatório novo e sem tocar no
ruleset.

## Lint e shell

- `actionlint 1.7.12` (compilado de `github.com/rhysd/actionlint/cmd/actionlint@v1.7.12`) → exit 0.
  O primeiro run no GitHub **reprovou** o `governance` com dois achados de `shellcheck` que o
  `actionlint` local não via (a máquina não tinha `shellcheck`): `SC2006` (crases dentro de um `echo` em
  `run:`) e `SC2016` (expressão em aspas simples). Depois de instalar `shellcheck 0.11.0` aqui, os dois
  apareceram localmente e foram corrigidos; `actionlint` com shellcheck → exit 0.
- `shellcheck --shell=sh` nos oito scripts novos (`scripts/ci-risk-tier.sh`,
  `ci-risk-tier-selfcheck.sh`, `ci-aggregate.sh`, `ci-risk-gate.sh`, `revert-rate.sh` e os três
  `tests/shell/*.test.sh`) → **0 achados**. Corrigidos no caminho: `CDPATH='' cd` em vez de `CDPATH= cd`
  (SC1007), `10#` trocado por `${var#0}` (SC3052 — conversão de base é indefinida em POSIX sh, e o
  `/bin/sh` do runner é dash), e uma variável não usada (SC2034).
- `zizmor 1.32.0 --offline --min-severity low .` → sem achado (8 ignorados, 2 suprimidos: os mesmos do
  `main`). Com `--persona pedantic`, o branch tinha **2 achados novos** `undocumented-permissions` no
  `ci.yml` (as permissões do job `risk`); resolvidos comentando as permissões **na mesma linha** (a
  forma que o zizmor reconhece) — `ci.yml` volta a 0 achados no modo pedantic.

## `scripts/revert-rate.sh` (#124)

| Prova | Resultado |
|---|---|
| `sh tests/shell/revert-rate.test.sh` | **12 casos, todos ok** |
| Mesmo teste com `dash` | **12 casos, todos ok** |
| Falha-antes: stub de saída fixa | **8 de 12 falham** |
| Falha-antes da rodada 1: o script anterior | **3 de 12 falham** (`positional-month`, `paginated-reverts`, `not-json`) |
| Falha-antes da rodada 2: o script anterior ao conserto do basename | **1 de 12 falha** (`same-basename-different-dirs`) |
| Dados reais, `--month 2026-10` | `\| 2026-10 \| 1 \| 0 \| 0.0% \|` |
| Dados reais, `--month 2026-11` | `\| 2026-11 \| 57 \| 0 \| 0.0% \|` (mês em curso) |

Casos: mês posicional; duas páginas de `--paginate` (dois arrays concatenados) contando as duas; janela
nos dois lados (PR mergeado no mês seguinte, revert commitado no mês seguinte),
`Revert "…"` do botão do GitHub, tipo convencional `revert(ci)!:`, revert de revert contando contra o PR
da reversão, duas reversões do mesmo PR contando uma, revert que nomeia PR fora da janela
(`unattributed`), revert sem referência (`unattributed`), mês sem PR mergeado (exit 1), mês mal
formado / arquivo ausente / JSON que não é array (exit 2).

O número real é 0% porque **não existe commit de revert na `main`** (493 commits em 2026-11-06,
`git log --grep='Revert'` vazio) — não é falso negativo do classificador. A tabela e o comando estão em
[revert-rate.md](revert-rate.md).

Bugs encontrados e corrigidos com os dados reais e com a revisão: o `--search` usava
`merged:$month-01..$month-31`, que numa janela de 30 dias perde o dia 31 — e de fato devolvia 1 PR para
outubro em vez de 55 para novembro; agora o intervalo fecha no último dia real do mês (com bissexto) e
bater o teto de 500 resultados é erro, não truncamento silencioso. O `gh api --paginate` imprime um array
por página e o script lia só o primeiro: um mês com mais de 100 commits via só a primeira página; a saída
passa por `jq -s 'add // []'`.

## Suíte completa (uma vez, no fim)

- `bun run check` (hook de pre-commit): `tsc --noEmit` 0 erros + `cargo check --workspace --all-targets`
  limpo.
- `bun run test:unit` → **86 testes, 0 falhas** (298 expects, 12 arquivos).
- `cargo fmt --all -- --check` e `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` limpos.
- `cargo clippy --workspace --all-targets -- -D warnings` limpo.
- `cargo test -p openridr-host` → **480 passando, 0 falhando**, 64 ignorados do harness de folha
  isolada. Uma rodada de três falhou em
  `lsp_loads_the_real_solution_republishes_diagnostics_and_stops_on_revoke` (a folha isolada parou em
  `ensure_tool`: `tools.install` terminou `failed` com log vazio); rodado sozinho em seguida passa
  (75,6 s) e as outras duas rodadas completas ficaram verdes. Registrado como reincidência da classe
  "restore/LSP falham no run completo" em [test-stability.md](test-stability.md); o diff do branch não
  toca `crates/`.

## Revisão independente (rodada 1) e o que mudou

Revisão (Pi, `openai-codex/gpt-6.1-terra`, `thinking=medium`) sobre `7a352d5`. Todo achado foi corrigido;
nenhum ficou "para depois". A tabela é o registro de que a revisão foi lida, não um resumo do diff.

| Achado | Severidade | O que era | Correção e prova |
|---|---|---|---|
| O gate executava código do PR | alta | `actions/checkout` num `pull_request` traz o merge commit: `ci-risk-tier.sh`/`ci-risk-gate.sh` vinham do PR, então um PR que os editasse se classificava baixo ou se aprovava | classificador, gate e agregador são extraídos de `origin/<base>` e executados de lá; bootstrap explícito (`::warning::` + linha no resumo) quando a base ainda não tem o arquivo |
| Aprovação ancorada em `committer.date` | alta | data de commit é controlável: commit novo com data antiga herdava aprovação de revisão que ninguém viu | a review tem de ter `commit_id` **igual** ao head do PR; a API de commits saiu do caminho. Falha-antes: 12 de 16 casos do teste novo reprovam o gate antigo |
| `APPROVED` antigo sobrevivia a `CHANGES_REQUESTED` | alta | a busca pegava qualquer aprovação no histórico, sem olhar o estado mais recente do revisor | redução por revisor pelo `submitted_at` mais recente; casos `withdrawn-by-changes-requested`, `withdrawn-by-dismissal` e `reapproved` |
| Manifesto comparado na ponta da base | média | base e PR acrescentando a mesma dependência davam "sem mudança" (médio) | `git show` do manifesto usa o merge base do diff de caminhos; caso `cargo-same-dependency-both-sides` (falha-antes) |
| Extrator TOML ignorava `[target.'cfg(..)'.dependencies.foo]` e descartava valor multilinha | média | dependência acrescentada ali virava médio; valor que começa com `[` era lido como cabeçalho e o resto da tabela sumia | qualquer tabela cujo nome mencione `dependencies` conta (mais `patch`/`replace`); linha que começa como cabeçalho sem ser faz o script falhar fechado. Casos `cargo-target-dep` e `cargo-unparseable-value` (falha-antes) |
| `--paginate` do `gh api` só lia a primeira página | média | mês com mais de 100 commits via só os primeiros 100 | `jq -s 'add // []'`; caso `paginated-reverts` (falha-antes) |
| Busca de PRs mergeados sem limite superior e com teto mudo | média | histórico grande empurrava o mês para fora dos 500 resultados | intervalo fecha no último dia real do mês (com bissexto) e bater o teto é erro |
| `gh api ... \| sort \| tail -1` mascarava erro | média | o status era do `tail`: erro de API virava "lista vazia" | saída capturada e checada antes de ordenar; caso `api-failure` |
| Falha do auto-merge afirmava a causa | baixa | qualquer erro era anunciado como "falta `allow_auto_merge`" | a saída real do `gh` vai para o `::warning::` e para o resumo; o setting aparece como hipótese |
| Mês só por `--month` | baixa | o ticket escreveu `revert-rate.sh [mês]` | as duas formas valem; caso `positional-month` (falha-antes) |

O que **não** foi corrigido, e por quê: com `pull_request`, o YAML que roda é o do merge commit, então um
PR que apague o passo do gate do próprio `ci.yml` ainda pode ficar verde. Esta frente tira a decisão das
mãos do PR (classificador e gate vêm da base) e deixa a brecha residual num diff de YAML, que é
`risk:high` e visível; fechar de vez exige `pull_request_target` para o gate e um terceiro check
obrigatório no ruleset — settings, portanto issue filha ([#139](https://github.com/Heldinhow/openridr/issues/139))
e decisão do Helder.

## Revisão independente (rodada 2) — o re-check das correções

Re-check (Pi, `openai-codex/gpt-6.1-terra`, `thinking=medium`) sobre o commit de correção `a6080c9`. Veredito: **findings** — 2 altos e 3 médios, todos corrigidos na rodada 2.

| Achado | Severidade | O que era | Correção e prova |
|---|---|---|---|
| O bootstrap executava código do PR | alta | quando a base não tinha os scripts, o job rodava a cópia do PR com um `::warning::` — aviso não é gate | sem script na base, o tier é **forçado a `high`** e a aprovação é conferida por um filtro `jq` inline fixo no YAML; o agregador idem (regra inline equivalente). Nenhum caminho executa script do PR |
| Cabeçalho TOML indentado era invisível | alta | a gramática permite espaço antes de `[tabela]`; indentar `[dependencies]` e mudar uma entrada dava "sem mudança" (médio) | reconhecimento aceita indentação; caso `cargo-indented-header` com o par de branches cuja única diferença mora na tabela indentada (falha-antes: `medium`) |
| `COMMENTED` retirava aprovação | média | um comentário posterior ao APPROVED tornava o PR impossível de mergear até nova aprovação | a redução por revisor considera só `APPROVED`/`CHANGES_REQUESTED`/`DISMISSED`; caso `comment-after-approval` (falha-antes) |
| `gh api \| jq` mascarava falha do `gh` | média | mesmo defeito do `\| sort \| tail` da rodada 1, num lugar que passou | páginas capturadas em arquivo, `gh` checado, `jq` depois |
| Dois fixtures de mesmo nome se sobrescreviam | média | a normalização derivava o caminho do `basename` | dois caminhos fixos e distintos; caso `same-basename-different-dirs` (falha-antes) |

O que o re-check confirmou correto, sem ressalva: `commit_id == head` rejeita aprovação de revisão anterior e `commit_id` nulo; redução por revisor com aprovações repetidas e paginação fora de ordem; manifesto no merge base (adicionado/deletado falha fechado); `jq -s 'add // []'` para um array, vários, vazio e escalar; rolagem de dezembro e fevereiro bissexto; aviso de auto-merge e mês posicional.

## verify-openridr

Não se aplica: nenhuma feature de produto, nenhum `data-testid`, nenhum arquivo de `src/` ou `crates/`
foi tocado. A superfície verificável aqui é shell + YAML, e ela foi exercitada de verdade (74 casos de
teste de comportamento, mais o run real). Nenhum app Tauri foi aberto — abrir o app não provaria nada
sobre um gate que só roda no GitHub.

## O que não foi verificado

- **Aprovação do mantenedor e o rerun**: o run fica vermelho até o Helder aprovar o PR #131 e alguém
  rodar `gh run rerun --failed`. Até lá, o caminho "aprovação válida → verde" não tem prova real (tem
  prova de unidade: 14 casos).
- **Auto-merge no tier baixo**: `allow_auto_merge` está **ligado** no repo desde 2026-11-06
  (`gh api repos/Heldinhow/openridr --jq .allow_auto_merge` → `true`, autorizado pelo Helder), mas nenhum
  PR `risk:low` passou pelo job ainda: o caminho "arma auto-merge → checks verdes → squash automático"
  não tem prova de ponta a ponta. O caminho de falha (setting desligado → `::warning::`) também não foi
  observado depois de ligado.
- **PR de fork**: o token é read-only, então rotular falha e o job segue. Repo sem forks: não exercitado.
- **`risk:low` e `risk:medium` num run real**: só o `high` foi exercitado (este PR). Os outros dois têm
  prova de unidade (self-check do tier) e nenhuma prova de ponta a ponta.
- **A extração do código confiável da base num run real**: o passo está no YAML e foi exercitado só pelo
  `actionlint`/`zizmor`; no run deste PR ele cai no caminho de bootstrap (a base ainda não tem os
  scripts), que é o único caminho que este PR pode provar. O caminho "base tem o script" só tem prova
  local (`git show origin/main:scripts/...` com os arquivos do `main`).
- **Uma revisão `APPROVED` de verdade**: nenhuma review foi submetida neste PR até a escrita desta
  evidência; o caminho "aprovação válida → verde" tem prova de unidade (16 casos), não de ponta a ponta.
- **`dash` é uma aproximação do `/bin/sh` do runner**: os testes rodaram com `dash 0.5.13.5` do
  Homebrew, não com o `dash` do Ubuntu 24.04.
- **Revert rate em mês fechado**: a linha de 2026-11 é de um mês em curso; a primeira medição de mês
  completo é a de 2026-10, com um único PR.
