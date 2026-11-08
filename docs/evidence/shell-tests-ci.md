# Evidência — job de CI rodando `tests/shell/*.test.sh` sob dash (issue #269)

Data: 2026-11-07. Ambiente local: macOS 27.2, arm64, `/bin/sh` = bash 3.2.57, `/bin/dash` presente.
Base: `c35f60b` (tip de `origin/main`). Branch: `ci/269-shell-tests`.

Origem: candidato 2 do retro [`RETRO-DOCTOR-DISK-SPACE.md`](../RETRO-DOCTOR-DISK-SPACE.md). O `AGENTS.md`
já pedia `dash <arquivo>` antes de empurrar um PR que mexe em `scripts/` ou `tests/shell/`, mas o pedido
dependia de memória: nenhum job rodava esses testes e um teste de shell quebrado passava despercebido.

## O que o job faz

`.github/workflows/ci.yml` ganhou o job `shell` (`name: Shell (dash, tests/shell)`, `ubuntu-latest`,
`timeout-minutes: 10`): um `checkout` com `persist-credentials: false` e um passo,
`dash scripts/ci-shell-tests.sh`. Ele entra no `needs` do agregador `ci` e **não** em `PR_ONLY`: roda em
todo evento, então um `skipped` dele é falha, não passe.

**Por que dash.** O `/bin/sh` do runner ubuntu é dash; o `/bin/sh` do macOS é bash 3.2.57, que aceita
`local`, `[[ ]]` e `10#`. Um teste escrito na máquina do dev pode passar lá e falhar no runner — e o
contrário também — sem que nada registre o resultado. Rodar sob dash nos dois lugares deixa o resultado
local e o do CI no mesmo shell.

**Por que a lista é glob.** `scripts/ci-shell-tests.sh` descobre `tests/shell/*.test.sh` por glob, nunca
por lista escrita à mão: um teste novo passa a rodar sem editar o workflow nem o runner. Um glob que não
casa nada sai **2**, então o job não consegue ficar verde rodando zero testes.

**Por que a lógica está num script.** `tests/shell/ci-shell-tests.test.sh` exercita o runner localmente,
como `ci-aggregate.test.sh` faz com o agregador; YAML inline não tem teste local.

**shellcheck.** Nenhum teste de `tests/shell/` precisa dele hoje: o job `governance` já roda shellcheck
pelo `actionlint`, e nada aqui é lintado por conta própria. O único check obrigatório continua sendo o
agregador `ci`, então o job novo adiciona um gate sem tocar no ruleset.

## Comandos e resultados (local)

| Comando | Resultado |
|---|---|
| `dash scripts/ci-shell-tests.sh` | exit 0, **9 passed, 0 failed under dash (7s)**, `real 6.61` |
| `sh scripts/ci-shell-tests.sh` | exit 0, **9 passed, 0 failed under dash (9s)**, `real 9.48` |
| `dash tests/shell/ci-shell-tests.test.sh` | 6 casos `ok`, `all cases passed`, exit 0 |
| `sh tests/shell/ci-shell-tests.test.sh` | 6 casos `ok`, `all cases passed`, exit 0 |
| `dash tests/shell/ci-workflow-bootstrap.test.sh` | `all cases passed`, exit 0 (os blocos extraídos do `ci.yml` continuam achados e verdes) |
| `dash tests/shell/ci-aggregate.test.sh` | `all cases passed`, exit 0 |
| `tests/shell/zz-broken.test.sh` com `exit 1` → `dash scripts/ci-shell-tests.sh` | **exit 1**, `FAIL - …/zz-broken.test.sh (exit 1, 0s)`, `9 passed, 1 failed` |
| o mesmo depois de apagar o arquivo | exit 0, `9 passed, 0 failed` |
| `shellcheck -s sh scripts/ci-shell-tests.sh tests/shell/ci-shell-tests.test.sh` | exit 0, sem achado (0.10.0) |
| `actionlint` (com shellcheck no PATH) | exit 0, sem achado (1.7.12) |
| `zizmor --offline --min-severity low .` | `No findings to report. Good job!`, exit 0 (1.32.0) |
| `NEEDS=<… "shell":{"result":"success"}>` … `sh scripts/ci-aggregate.sh` | exit 0, `every required job passed` |
| `NEEDS=<… "shell":{"result":"failure"}>` … `sh scripts/ci-aggregate.sh` | exit 1, `shell: failure` |

As três ferramentas de lint não estão instaladas nesta máquina (sem `brew`, `go` nem `cargo`): os
binários das versões pinadas no workflow foram baixados das release pages para `/tmp` e rodados de lá.
Nada foi adicionado ao repositório.

## Saída medida

`dash scripts/ci-shell-tests.sh` (caminho da worktree abreviado para `<repo>`):

```text
shell tests: <repo>/tests/shell/*.test.sh under dash
ok   - <repo>/tests/shell/ci-aggregate.test.sh (0s)
ok   - <repo>/tests/shell/ci-risk-gate.test.sh (1s)
ok   - <repo>/tests/shell/ci-shell-tests.test.sh (0s)
ok   - <repo>/tests/shell/ci-workflow-bootstrap.test.sh (1s)
ok   - <repo>/tests/shell/doctor.test.sh (0s)
ok   - <repo>/tests/shell/e2e-smoke-issue.test.sh (0s)
ok   - <repo>/tests/shell/git-remote-fixture.test.sh (2s)
ok   - <repo>/tests/shell/pre-commit.test.sh (2s)
ok   - <repo>/tests/shell/revert-rate.test.sh (1s)
shell tests: 9 passed, 0 failed under dash (7s)
```

O arquivo novo aparece na lista (`ci-shell-tests.test.sh`), o que é a prova de que o glob o pegou sem
ninguém escrever o nome dele em lugar nenhum.

## A falha reprova o job (medido, não deduzido)

`printf '#!/bin/sh\nexit 1\n' > tests/shell/zz-broken.test.sh` e `dash scripts/ci-shell-tests.sh`:

```text
shell tests: <repo>/tests/shell/*.test.sh under dash
FAIL - <repo>/tests/shell/zz-broken.test.sh (exit 1, 0s)
shell tests: 9 passed, 1 failed under dash (6s)
```

Exit **1**. Os outros nove continuaram rodando depois da falha (é o que `9 passed` mostra), e o arquivo
foi apagado em seguida: a rodada seguinte voltou a `9 passed, 0 failed` com exit 0.

O `tests/shell/ci-shell-tests.test.sh` cobre o mesmo caminho de dentro da suíte, sem tocar em
`tests/shell/`: um diretório de mentira com um teste que falha e um que passa, e a asserção de que o que
passa **rodou** (arquivo marcador), não só de que o nome dele apareceu.

## Os testes falsos provam que rodaram sob dash

Cada teste falso do `ci-shell-tests.test.sh` recusa rodar se `BASH_VERSION` estiver definida. Medido:

| Comando | Resultado |
|---|---|
| `SHELL_TESTS_DIR=<dir> SHELL_TESTS_SHELL=sh dash scripts/ci-shell-tests.sh` | exit 1, `BASH_VERSION is set: this test did not run under dash` |
| `SHELL_TESTS_DIR=<dir> dash scripts/ci-shell-tests.sh` | exit 0, `1 passed` |

Ou seja: o guarda não é decorativo — com o shell do harness (bash no macOS) ele reprova, e o runner
padrão (dash) passa.

## Não verificado

- **Nenhum runner ubuntu foi executado nesta sessão.** O job `shell` só é medido de verdade no run do PR;
  o tempo de parede do job (checkout + suíte) e o dash da distro não foram observados aqui. O que está
  medido é a suíte: **7 s** localmente, muito abaixo do `timeout-minutes: 10`.
- **Sem rede:** lido no código, não medido com a rede bloqueada. `gh`, `bun`, `cargo` e `df` são stubs nos
  testes; `scripts/git-remote-fixture.sh` cria repositórios bare locais e não faz login nem clone remoto.
- **`git-remote-fixture.test.sh` imprime um `skip -`** localmente: o caso do launcher precisa do bundle de
  debug (`src-tauri/target/debug/bundle/macos/OpenRidr.app`), que não existe nesta worktree. O arquivo sai
  com exit 0, então o runner o conta como `ok`. No ubuntu o caso vai pular pelo mesmo motivo — o skip é do
  teste, não do runner, e o runner não o esconde.
- **O run do PR é a única medida no ubuntu.** Até o primeiro run verde, o que existe é a suíte medida no
  macOS sob dash; a rodada do PR (link abaixo) é a evidência do job no runner.
