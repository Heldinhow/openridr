# Projetos fora do diretório da solution — evidência

Issue [#52](https://github.com/Heldinhow/openridr/issues/52); tickets [#143](https://github.com/Heldinhow/openridr/issues/143) (escopo + avaliação), [#144](https://github.com/Heldinhow/openridr/issues/144) (confinamento), [#145](https://github.com/Heldinhow/openridr/issues/145) (análise/breakpoints/problemas), [#146](https://github.com/Heldinhow/openridr/issues/146) (execução + app real).
Branch `feat/issue-52-projects-outside-solution`, base `origin/main` `dcb19ea`. macOS ARM64, SDK do projeto `8.0.408` (`~/.dotnet`), `DOTNET_CLI_UI_LANGUAGE=en`.

## Resumo

| O que | Prova |
|---|---|
| Projeto fora do diretório é avaliado com TFM, TargetPath e fontes | `evaluation_includes_a_project_outside_the_solution_directory` |
| Escopo exposto, recalculado e limitado | `the_scope_is_recomputed_when_the_solution_gains_a_project`, unidade `scope::tests` |
| Nada fora do escopo abre, escreve ou é analisado | `the_scope_admits_the_outside_project_and_refuses_everything_else`, `the_tree_and_nuget_see_the_outside_project_inside_the_scope_only` |
| Breakpoint em fonte de fora | `breakpoints_accept_a_file_of_the_outside_project_and_nothing_beyond_the_scope` |
| Problema de build de fora navegável | unidade `build::tests::a_diagnostic_in_a_project_outside_the_root_is_navigable_inside_the_scope` |
| Restore, build, run e testes do projeto de fora | `restore_build_and_run_work_for_a_project_outside_the_solution`, `a_test_project_outside_the_solution_is_discovered_and_runs` |
| App real (árvore, editor, diagnóstico, breakpoint, build, run) | seção "App real" abaixo |

## Falha-antes (base `origin/main` `dcb19ea`)

Mesmos arquivos de teste e a mesma fixture, num worktree limpo da base:

```
$ cargo test -p openridr-host --test host -- evaluation_includes_a_project_outside an_edit_in_a_project_outside the_scope_is_recomputed the_scope_admits_the_outside_project the_tree_and_nuget_see breakpoints_accept_a_file restore_build_and_run_work_for_a_project_outside
test result: FAILED. 0 passed; 7 failed; 0 ignored; 0 measured; 127 filtered out

$ cargo test -p openridr-host --test test_discovery -- a_test_project_outside_the_solution
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out
```

Motivos observados, um por teste:

| Teste | Falha na base |
|---|---|
| `evaluation_includes_…` | `[Lib] failed · [OutsideApp] failed · [Lib.Tests] failed` → evaluation `partial` em vez de `ok` |
| `an_edit_in_a_project_outside_…` | `stale: false` depois de editar o `.csproj` de fora |
| `the_scope_is_recomputed_…` | `workspace.scope` é `null` |
| `the_scope_admits_…` | `file.read` do arquivo de fora → `outside_workspace` |
| `the_tree_and_nuget_see_…` | outline do projeto de fora é `unknown` |
| `breakpoints_accept_a_file_…` | `debug.setBreakpoints` no arquivo de fora → `outside_workspace` |
| `restore_build_and_run_work_…` | `restore.projects` só lista `App`; os projetos de fora não entram no plano |
| `a_test_project_outside_…` | evaluation `partial` antes de qualquer descoberta |

## Depois (host)

```
$ cargo test -p openridr-host
unittests src/lib.rs         374 passed; 0 failed; 11 ignored
tests/debugger.rs             11 passed; 0 failed; 11 ignored
tests/host.rs                107 passed; 0 failed; 27 ignored
tests/test_discovery.rs        1 passed; 0 failed;  1 ignored
tests/test_run.rs             15 passed; 0 failed; 14 ignored
tests/trust_model.rs           1 passed; 0 failed;  1 ignored
tests/corporate_proxy.rs      11 passed; 0 failed;  0 ignored
tests/… (demais binários)      7 passed; 0 failed
```

```
$ bun run check            # tsc --noEmit + cargo check --workspace --all-targets
$ bun test tests/unit      # 89 pass, 0 fail
$ cargo fmt --all          # sem diff pendente
$ cargo clippy --workspace --all-targets -- -D warnings   # limpo
```

Os 27 ignorados de `tests/host.rs` são os "isolated leaf" que só rodam em processo próprio (contrato do harness); não são falha.

## Matriz de confinamento

Tudo com a fixture real (`fixtures/OutsideSolution`: raiz `app/`, projetos em `lib/`, `outside-app/`, `tests/`) e um diretório `outside/` **fora** do escopo, com arquivo de controle.

| Vetor | Esperado | Observado |
|---|---|---|
| `file.read` de `lib/Greeting.cs` (absoluto) | lê | lê |
| `file.write` de `lib/Greeting.cs` com hash atual | grava | grava; conteúdo em disco confere |
| `file.create`/`rename`/`delete` em `lib/` | funciona | funciona; arquivo some/aparece como esperado |
| `file.read` de `lib/LinkOut/Secret.cs` (symlink de dentro para fora) | `outside_workspace` | `outside_workspace` |
| `file.read` de `lib/Dangling.cs` (symlink pendente para fora) | `outside_workspace` | `outside_workspace` |
| `file.read` de `app/LinkIn/Greeting.cs` (symlink da raiz para diretório do escopo) | lê | lê |
| `file.read` de `<tmp>/outside/Secret.cs` (absoluto fora) | `outside_workspace` | `outside_workspace` |
| `file.read` de `../outside/Secret.cs` e `../outside` | `outside_workspace` (por forma) | `outside_workspace` |
| `file.write` de `<tmp>/outside/Secret.cs` | `outside_workspace`, arquivo intacto | `outside_workspace`; SHA do arquivo de controle inalterado |
| `workspace.tree` do diretório de fora | lista | lista `Greeting.cs` com caminho absoluto |
| `workspace.tree` de `outside/` e `../outside` | `outside_workspace` | `outside_workspace` |
| `workspace.symlinks` com caminho de fora e caminho fora do escopo | responde o de dentro, omite o de fora | 1 entrada (`symlink: true`), a de fora omitida |
| `workspace.symbolKinds` de `lib/Greeting.cs` | `class` | `class` |
| `nuget.planUpdate` no projeto de fora | plano criado | plano criado, `project` absoluto |
| `nuget.planUpdate` em `.csproj` fora do escopo | `outside_workspace` | `outside_workspace` |
| `debug.setBreakpoints`/condição no arquivo de fora | aceita | aceita; breakpoint na lista do DAP |
| `debug.setBreakpoints` fora do escopo (absoluto e `..`) | `outside_workspace` | `outside_workspace` |
| diagnóstico MSBuild com caminho de fora | item navegável | `Line::Problem` com path absoluto e posição 0-based |
| diagnóstico MSBuild fora do escopo / `..` | texto (`Rejected`) | `Rejected` |
| definição LSP em arquivo de fora | localização com path absoluto | idem; recusada sem o escopo |

## App real

_(preenchido na verificação com `agent-desktop`; ver abaixo)_

## Não verificado

- Windows e macOS Intel.
- `ProjectReference` para projeto fora que não está no `.sln`: não é modelado (limite declarado na spec).
- Busca Files/Text, git status/gutter/diff e cwd do terminal continuam ancorados na raiz (limite declarado). "Show all files" de um projeto de fora lista o diretório do projeto, que está no escopo.
- Playwright: não é prova desta feature (decisão da #67).
