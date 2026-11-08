# Evidência — a avaliação roda o muxer com `cwd` na raiz da workspace (issue #313)

Data: 2026-11-08. macOS ARM64 (Apple Silicon), `~/.dotnet` com SDK **8.0.408** e **10.0.403**
(nenhum prerelease instalado). Branch `fix/issue-313-evaluation-cwd-root`, base `8526889`.

## Reprodução medida à mão (antes de qualquer código)

`/tmp/X/` sem `global.json`; `/tmp/X/src/global.json` = `{ "sdk": { "version": "8.0.408",
"rollForward": "disable" } }`; `/tmp/X/src/App/App.csproj` com `net10.0`. Todos os comandos com
`DOTNET_ROOT=$HOME/.dotnet` e `DOTNET_MULTILEVEL_LOOKUP=0`:

| Comando | Resultado |
|---|---|
| `dotnet --version` com cwd `/tmp/X` (o que o snapshot pergunta) | `10.0.403` |
| `dotnet msbuild src/App/App.csproj -getProperty:… -getProperty:NETCoreSdkVersion` com cwd `/tmp/X/src/App` (a avaliação de antes) | exit 0, `NETCoreSdkVersion` = **`8.0.408`** |
| o mesmo com cwd `/tmp/X` (a avaliação de agora) | exit 0, `NETCoreSdkVersion` = **`10.0.403`** |

A avaliação **não falha** em nenhum dos dois: a divergência era silenciosa — o snapshot nomeava
`10.0.403` e o `-getProperty` da avaliação rodava `8.0.408`. Quem usava o SDK do snapshot era o
`dotnet build`, que já rodava na raiz.

## O teste vermelho (a reprodução no host)

`crates/host/tests/host.rs::a_global_json_between_the_root_and_the_project_does_not_change_the_evaluation_sdk`:
um `tempdir` com `X.sln` na raiz, `src/global.json` fixando a linha baixa e `src/App/App.csproj` na linha
alta. As versões saem de `sdk::inventory()` via `muxer_pin_pair()`, nunca hardcoded; sem duas linhas
estáveis ≥ 5 na mesma raiz o teste imprime `not verified:` e retorna.

Revertendo **só** o `cwd` (`.current_dir(project.parent())`, com o resto da mudança no lugar):

```
assertion `left == right` failed: the evaluation must run the muxer with the SDK the snapshot names
  left: "8.0.408"
 right: "10.0.403"
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 146 filtered out; finished in 0.39s
```

Com a mudança:

```
test a_global_json_between_the_root_and_the_project_does_not_change_the_evaluation_sdk ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 146 filtered out; finished in 0.52s
```

O teste lê o `NETCoreSdkVersion` do stdout do MSBuild que o próprio host guarda no log do projeto — a
pergunta do #310 ("qual SDK rodou de verdade"), agora respondida pelo log da avaliação em vez de um
segundo subprocesso do teste. É por isso que o comando ganhou `-getProperty:NETCoreSdkVersion`.

## Nenhum teste dependia do `cwd` antigo

Os `global.json` dos testes e das fixtures estão todos na raiz da workspace que eles abrem
(`fixtures/MultiConsole` e `fixtures/OrderDesk` pinam na raiz; `fixtures/OutsideSolution/app/global.json`
fica no diretório da solution), então a suíte inteira é a prova: **658 passed, 0 failed, 71 ignored**,
com o mesmo número de testes de antes mais o novo. Os 71 `ignored` não ficam de fora: são os leaves
isolados, que o teste pai roda num processo próprio via `support::run_isolated_leaf` e exige `1 passed`
no filho — se um deles falha, o pai falha.

A consequência que a mudança cria e a suíte confirma: um projeto **fora** do diretório da solution
(`fixtures/OutsideSolution/lib`, `outside-app`, `tests`) passa a ser avaliado sob o `global.json` da raiz
também — antes rodava no próprio diretório e o muxer resolvia o maior SDK estável. É a mesma resposta que
restore/build/run/test já davam para esse projeto, e `evaluation_includes_a_project_outside_the_solution_directory`
continua verde.

## O resto do que rodou

| Comando | Resultado |
|---|---|
| `cargo test -p openridr-host` (o job `rust` do CI) | 658 passed, 0 failed, 71 ignored |
| `cargo test -p openridr-host --test host -- a_global_json_between_the_root_and_the_project` | 1 passed |
| `cargo fmt --all -- --check` | limpo |
| `cargo clippy --workspace --all-targets -- -D warnings` | limpo |
| `PLAYWRIGHT_CHANNEL=chromium OPENRIDR_E2E_UI_PORT=1920 OPENRIDR_E2E_HOST_PORT=4917 bunx playwright test tests/e2e/evaluation.spec.ts tests/e2e/sdk-prerequisites.spec.ts` | **14 passed** (50,0 s) |

Chrome não está instalado nesta máquina, por isso `PLAYWRIGHT_CHANNEL=chromium`. Nenhum spec de E2E mudou:
a mudança é de host e o caso comum (pin na raiz, ou nenhum pin) já era o mesmo antes.

## A falha de `test_run` que apareceu no caminho

Uma das execuções da suíte completa falhou em
`crates/host/tests/test_run.rs::real_run_reports_trx_outcomes_positions_and_only_the_selected_identity`
(`test_run.rs:297`, a asserção do `position.path`). É a **segunda observação** da linha que a tabela de
[`test-stability.md`](test-stability.md) já tem para esse teste (a primeira foi na sessão da #55, mesma
asserção, `Null` no lugar de `tests/OrderDesk.Tests/OrderTests.cs`); a linha foi atualizada com esta.
Medido: 1 falha em ~22 execuções do binário `--test test_run` e em 1 de 4 execuções da suíte completa;
4 execuções do teste sozinho (`--exact`) verdes. A asserção é do mapeamento TRX → posição
(`position_from_stack`, que usa a raiz da workspace, nunca o `cwd` da avaliação), então não há relação
com esta mudança; a causa continua não determinada.

## Não verificado

- **Windows e macOS Intel**, como no resto do produto.
- **`src-tauri`**: não tocado; o job `rust-tauri` é informativo e não roda aqui.
- **A causa da falha de `test_run` acima**: o valor recebido na asserção não foi preservado nesta
  observação; registrado na tabela, não mascarado.
