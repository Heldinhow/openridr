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
| `sh scripts/ci-risk-tier-selfcheck.sh` | **39 casos, 39 ok** |
| Mesmo autoteste com `dash` (o `/bin/sh` do runner) | **39 casos, 39 ok** |
| Falha-antes: stub `echo low` no lugar do script | **33 de 39 falham**, inclusive os três `err-*` |
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
| `sh tests/shell/ci-risk-gate.test.sh` | **14 casos, todos ok** |
| Mesmo teste com `dash` | **14 casos, todos ok** |
| Mutação 1: remover `$3 >= latest` (aprovação de qualquer revisão) | reprova `approval-before-head` e `mixed-histories` |
| Mutação 2: aceitar qualquer `author_association` | reprova `approval-without-write-access` |

Casos: `low`/`medium` não chamam o GitHub (sem gate); aprovação depois do head; aprovação antes do head
(reprova); aprovação de `CONTRIBUTOR` (reprova); review `DISMISSED` (reprova); `COMMENTED` (reprova);
nenhuma review (reprova); aprovação **no mesmo segundo** do head (passa); histórico misto com uma
aprovação válida depois de uma velha (passa, e o log nomeia quem aprovou); PR errado (o stub `gh` falha
na URL → exit 2); lista de commits vazia (exit 2); tier desconhecido (exit 2); `REPO` ausente (exit 2).

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
| `sh tests/shell/revert-rate.test.sh` | **8 casos, todos ok** |
| Mesmo teste com `dash` | **8 casos, todos ok** |
| Falha-antes: stub de saída fixa | **8 de 8 falham** |
| Dados reais, `--month 2026-10` | `\| 2026-10 \| 1 \| 0 \| 0.0% \|` |
| Dados reais, `--month 2026-11` | `\| 2026-11 \| 56 \| 0 \| 0.0% \|` (mês em curso) |

Casos: janela nos dois lados (PR mergeado no mês seguinte, revert commitado no mês seguinte),
`Revert "…"` do botão do GitHub, tipo convencional `revert(ci)!:`, revert de revert contando contra o PR
da reversão, duas reversões do mesmo PR contando uma, revert que nomeia PR fora da janela
(`unattributed`), revert sem referência (`unattributed`), mês sem PR mergeado (exit 1), mês mal
formado / arquivo ausente / JSON que não é array (exit 2).

O número real é 0% porque **não existe commit de revert na `main`** (493 commits em 2026-11-06,
`git log --grep='Revert'` vazio) — não é falso negativo do classificador. A tabela e o comando estão em
[revert-rate.md](revert-rate.md).

Um bug foi encontrado e corrigido com os dados reais: o `--search` usava `merged:$month-01..$month-31`,
que numa janela de 30 dias perde o dia 31 — e de fato devolvia 1 PR para outubro em vez de 55 para
novembro. Agora só o limite inferior vai para o `--search` e a janela exata é a do `jq`.

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

## verify-openridr

Não se aplica: nenhuma feature de produto, nenhum `data-testid`, nenhum arquivo de `src/` ou `crates/`
foi tocado. A superfície verificável aqui é shell + YAML, e ela foi exercitada de verdade (74 casos de
teste de comportamento, mais o run real). Nenhum app Tauri foi aberto — abrir o app não provaria nada
sobre um gate que só roda no GitHub.

## O que não foi verificado

- **Aprovação do mantenedor e o rerun**: o run fica vermelho até o Helder aprovar o PR #131 e alguém
  rodar `gh run rerun --failed`. Até lá, o caminho "aprovação válida → verde" não tem prova real (tem
  prova de unidade: 14 casos).
- **Auto-merge no tier baixo**: `allow_auto_merge` está **desligado** no repo (decisão do Helder, comando
  em [ci-main-ruleset.md](../ci-main-ruleset.md)). O passo roda, falha e emite `::warning::`; nenhum PR
  de risco baixo foi mergeado por auto-merge ainda.
- **PR de fork**: o token é read-only, então rotular falha e o job segue. Repo sem forks: não exercitado.
- **`risk:low` e `risk:medium` num run real**: só o `high` foi exercitado (este PR). Os outros dois têm
  prova de unidade (self-check do tier) e nenhuma prova de ponta a ponta.
- **`dash` é uma aproximação do `/bin/sh` do runner**: os testes rodaram com `dash 0.5.13.5` do
  Homebrew, não com o `dash` do Ubuntu 24.04.
- **Revert rate em mês fechado**: a linha de 2026-11 é de um mês em curso; a primeira medição de mês
  completo é a de 2026-10, com um único PR.
