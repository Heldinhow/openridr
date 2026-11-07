# Evidência — Diretório neutro da ferramenta em um lugar só (issue #93)

Data: 2026-11-07. Alvo verificado: macOS Apple Silicon. Windows e macOS Intel **não** foram executados.

Código sob teste: `ffb7de0` (host, ticket #153) sobre `ccd5ce4` (`origin/main`). Versão `0.3.0-alpha.11`.

`dotnet` do PATH: `~/.dotnet` primeiro (SDKs 8.0.408 e 10.0.100), que é o único caminho que satisfaz o pin `8.0.408` dos fixtures. Sem `~/.dotnet` no PATH, `run_start_reports_restore_required_when_assets_are_invalidated_after_a_build` falha com `restore_required` (o `/usr/local/share/dotnet` só tem 8.0.416) — é a armadilha já documentada no `AGENTS.md`, não esta frente.

## O que a issue pedia, medido

| # | Medição | Comando | Resultado |
|---|---|---|---|
| S0 | A reprodução da issue no `origin/main` (antes desta frente) | `fresh=$(mktemp -d); TMPDIR=$fresh cargo test -p openridr-host --lib tools::tests::staged_verification` | **1 passed** — o `#90` já tinha corrigido o sintoma em `verify_installed`; o que faltava era o critério 2 (passo compartilhado) |
| S1 | **Falha-antes** com a criação removida de `neutral_cwd_in` (patch local, não commitado) | `cargo test -p openridr-host --lib tools::tests::the_neutral_cwd` | **FAILED** — `assert!(neutral.is_dir())` falso |
| S2 | **Falha-antes** da reprodução da issue, com a criação removida | `fresh=$(mktemp -d); TMPDIR=$fresh cargo test -p openridr-host --lib tools::tests::staged_verification` | **FAILED** — `called Result::unwrap() on an Err value: "Could not run <tmp>/staging/csharp-ls --version."`, exatamente o texto da issue |
| S3 | **Passa-depois**, teste novo | `cargo test -p openridr-host --lib tools::tests::the_neutral_cwd` | **1 passed** (0,00 s) |
| S4 | **Passa-depois**, reprodução da issue com `TMPDIR` fresco | `fresh=$(mktemp -d); TMPDIR=$fresh cargo test -p openridr-host --lib tools::tests::staged_verification` | **1 passed** (1,01 s) |
| S5 | O passo existe em um lugar só | `grep -n "create_dir_all(&cwd)\|neutral_cwd()" crates/host/src/tools.rs` | `neutral_cwd()` em `plan`/`verify_installed`/`verify_bundled`/worker de install; nenhum `create_dir_all` do diretório neutro fora de `neutral_cwd_in` |
| S6 | Formatação e lints | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings` | limpos (fmt ok; clippy `Finished` sem warning) |

O teste novo (`the_neutral_cwd_is_created_before_the_launcher_runs`) usa um `tempfile::tempdir()` como raiz, assere que `openridr-neutral` **não existe** ali, chama `neutral_cwd_in` e assere que existe. Não mexe em `TMPDIR` do processo: a criação é provada sem depender do estado da máquina, que é o que escondia o bug numa máquina quente.

## Bateria completa

`PATH="$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT=$HOME/.dotnet cargo test -p openridr-host`:

- `--lib`: **389 passed**, 0 failed, 11 ignored (inclui o teste novo).
- `--test bundled_analysis`: 11 passed; `--test debugger`: 1 passed; `--test test_discovery`: 15 passed; os demais binários verdes.
- `--test host`: **106 passed**, 27 ignored, e **3 falhas** — `tools_install_is_real_verified_and_reused`, `lsp_start_requires_tool_sdk_and_current_restore` e `lsp_loads_the_real_solution_republishes_diagnostics_and_stops_on_revoke`, todas aos ~120 s.

As três falhas são **de rede, não do diff**, e medidas assim:

- `prepare_local_package` baixa o `csharp-ls.0.28.0.nupkg` (21.643.814 bytes) com prazo de 120 s. Nesta rede, `curl` do mesmo pacote mediu **151,19 s** (`http=200 size=21643814`): o download não cabe no prazo.
- O mesmo teste, com o `tools.rs` **revertido para o `origin/main`** (`git checkout -- crates/host/src/tools.rs`), falha igual em 120,34 s — não é regressão desta frente.
- Quando o download terminou dentro do prazo, o teste passou: `tools_install_is_real_verified_and_reused ... ok` em **108,79 s**.
- Rodar os três com `--test-threads=1` não ajuda: são 3 × 120 s, cada um estourando o prazo próprio.

Registrado em [`test-stability.md`](test-stability.md) como falha de ambiente/rede observada, não como passe.

## Não verificado

- **App Tauri real**: a mudança é de host, sem superfície de UI (nenhum `data-testid`, nenhuma tela). Não houve sessão de `agent-desktop`; o caminho de install real no app já tem evidência própria da frente de tools (`docs/evidence/tests-nuget-tools-setup/desktop.md`) e não muda de comportamento — o diretório neutro é o mesmo caminho, agora criado antes do spawn.
- **Playwright / E2E**: não rodado (smoke noturno, decisão da #67; nenhuma regra de UI mudou).
- **Windows e macOS Intel**.
- **Custo declarado, não provado em teste**: a mensagem "Could not create the neutral install directory" deixou de existir; o caso (TMPDIR não gravável) não tem teste, porque o spawn falha logo depois com `ENOENT` e a mensagem de spawn.
