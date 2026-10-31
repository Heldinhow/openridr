# Resumo das execuções da matriz de aceite (2026-10-30, macOS ARM64)

Resumo versionado; os logs completos ficaram em `/tmp/t01/m/` (fora do repositório, descartáveis). Comando base:
`PATH=$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet DOTNET_CLI_UI_LANGUAGE=en TERM=xterm-256color bunx playwright test tests/e2e/<spec>.ts`, um processo por vez, portas 1420/4317 livres.

## Histórico (antes das correções D-01…D-11)

| Spec | Commit | Resultado | Exit |
|---|---|---|---|
| restore, evaluation, build, run, run-configs, workspace, debugger | `0587ee3` | 46 passed | 0 |
| gates, restore, run, install-plan, ui-layout | `9bf0bec` | 21 passed | 0 |
| csharp-ls | `6c323ed` | 8 passed, 2 timeout (`:1145`, `:1427`) | 1 |
| git 5, nuget 4, settings 5, actions 6, search 6, tests 6, testsuite 2, trust-sdk 3, shell 4, code 3, editor-overflow 2, disabled-states 3, ui-fold 4, ui-layout 7, install-plan 3, environment 1, gates 2 | `9bf0bec` | todos passed | 0 |
| terminal | `9bf0bec` | 2 failed com `TERM=dumb`; 4 passed com `xterm-256color` | 1 / 0 |
| zz-acceptance | `9bf0bec` | jornada falhou, UI audit passou | 1 |
| `cargo test --workspace --no-fail-fast` | `9bf0bec` | lib 173, debugger 9, test_discovery 9, test_run 5 ok; host.rs 47 ok / 13 falhas | 101 |

## Após as correções (estado atual)

| Item | Commit | Resultado | Exit |
|---|---|---|---|
| `cargo test -p openridr-host --test host` | `614edbd` | 60 passed | 0 |
| `cargo test -p openridr-host --lib terminal` | `8e45eda` | 13 passed | 0 |
| terminal com `TERM=dumb` | `8e45eda` | 4 passed | 0 |
| zz-acceptance (jornada + UI audit) | `3bbc30e` | 2 passed | 0 |
| find-usages | `1bc3d02` | 1 passed | 0 |
| csharp-ls (10 testes) | `7eb1cb2` / `0f00ae7` | 10 passed, 0 skips | 0 |
| tests 6, terminal 4, debugger 12 | `3bbc30e` | todos passed | 0 |
| gates + workspace + trust-sdk + shell | `3bbc30e` | 25 passed | 0 |
| gutter | `0f00ae7` | 1 passed | 0 |
| code | `0f00ae7` | 3 passed | 0 |
| `bun run check` | `0f00ae7` | ok | 0 |

Os issues do roadmap (`12-causa-timeout-csharp-ls-rename.md`, `14-defeitos-da-matriz-d01-d02-d03-d05.md`) vivem em `/Users/helder/Projetos/openridr/.scratch/roadmap/issues/` e **não são rastreados** por este repositório.
