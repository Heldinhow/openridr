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

App Tauri instalado a partir desta branch, macOS ARM64, `OPENRIDR_HOME` em diretório temporário, workspace = cópia de `fixtures/OutsideSolution` em `$TMPDIR` (raiz `app/`, projetos em `lib/`, `outside-app/` e `tests/`). Capturas em [`projects-outside-solution/`](projects-outside-solution/); todos os caminhos visíveis nas imagens são de `$TMPDIR`.

| # | Ação | Estado observado | Prova de host correspondente |
|---|---|---|---|
| [s1](projects-outside-solution/s1-trust-prompt.png) | Abrir `app/App.sln` | diálogo `Trust “App.sln”?` com o texto "Trust authorizes executable inspection of this solution and of the project directories it references." — o alcance do escopo declarado antes de confiar | `tests/trust_model.rs` |
| [s2](projects-outside-solution/s2-tree-outside.png) | `Trust project` | árvore `App · 4 projects`: `App` sem selo; `Lib`, `OutsideApp` e `Lib.Tests` com o selo `outside`; status `Trusted mode · Evaluated… · Analysis: Partial coverage` | `the_scope_admits_the_outside_project_and_refuses_everything_else` |
| [s3](projects-outside-solution/s3-editor-outside-file.png) | Abrir `Greeting.cs` na árvore | aba `Greeting.cs`, breadcrumb `Lib / Greeting.cs / net8.0`, status `Lib > Greeting.cs` | `file.read` do absoluto (matriz acima) |
| [s4](projects-outside-solution/s4-breakpoint-outside-file.png) | Clicar na gutter da linha 5 de `Greeting.cs` | ponto de breakpoint e linha realçada num arquivo fora da raiz | `breakpoints_accept_a_file_of_the_outside_project_and_nothing_beyond_the_scope` |
| [s5](projects-outside-solution/s5-build-outside-project.png) | `Build startup 'OutsideApp'` | `Build succeeded · exit 0`; log com `Lib → …/lib/bin/Debug/net8.0/Lib.dll` e `OutsideApp → …/outside-app/bin/Debug/net8.0/OutsideApp.dll`; 0 warnings, 0 errors, 0,93 s | `restore_build_and_run_work_for_a_project_outside_the_solution` |
| [s6](projects-outside-solution/s6-run-outside-project.png) | `Run` do `OutsideApp` | `Run exited · exit 0`, saída `outside app: Hello, outside!`, `Process finished with exit code 0.` | idem |
| [s7](projects-outside-solution/s7-edit-outside-file.png) | Editar `Greeting.cs` (fora) | aba com `Modified` e ponto de buffer sujo, `// outside edit` no editor, status `1 modified buffer` | `an_edit_in_a_project_outside_the_solution_marks_the_model_stale` |
| [s8](projects-outside-solution/s8-diagnostic-outside-file.png) | Escrever `int broken = ;` no arquivo de fora e salvar | `Problems`: "Awaiting diagnostics for …/lib/Greeting.cs (revision 2)"; `Analysis · csharp-ls`: `0 publications · 0 items · awaiting 1 document` — **limite**, ver abaixo | — (o host guarda o que chega; nada chegou) |
| [s9](projects-outside-solution/s9-definition-outside-file.png) | `F12` sobre `Greeting` em `Program.cs` (dentro da raiz) | navegou para `Greeting.cs` (fora): aba ativa, cursor em `Ln 3, Col 21` na declaração da classe, breadcrumb `Lib / Greeting.cs / net8.0` | `definition_location` com escopo (unidade) |

s1–s8 foram capturadas pela sessão anterior na mesma instância; s9 foi refeita nesta sessão com o cursor sobre `Greeting` (a primeira tentativa estava em `using Lib;`, e um namespace não tem localização — a resposta "no location" era correta e não dizia nada sobre a fronteira).

### Limite medido: csharp-ls não publica diagnóstico de arquivo fora da raiz

Controle medido nesta sessão, no mesmo app e no mesmo workspace: `int broken = ;` em `Program.cs` (projeto `App`, dentro da raiz) rendeu `1 error · 0 warnings` com `error CS1525: Invalid expression term ';'` em ~12 s e `Diagnostics: 1 publication · 1 item`. O mesmo erro em `Greeting.cs` (projeto `Lib`, fora da raiz) ficou em `0 publications · 0 items` depois de 2+ minutos, com o documento aberto e o servidor respondendo `textDocument/documentSymbol` para ele (log do LSP em s9).

**Hipótese, não investigada nesta frente:** o csharp-ls parece filtrar por workspace folder — o documento fora da pasta do workspace não entra na compilação que publica diagnósticos, embora o servidor o conheça (definição e símbolos funcionam). O host não filtra nada: guarda o diagnóstico que chega e recusa o que vem de fora do escopo, e a matriz acima prova o caminho de um diagnóstico **MSBuild** de fora. Consequência para quem usa: em projeto fora da raiz, o sublinhado e a lista de problemas do LSP ficam vazios até o build; `Build · dotnet` continua reportando. Fica como limite declarado, com issue própria se o Helder quiser perseguir.

## Não verificado

- Windows e macOS Intel.
- `ProjectReference` para projeto fora que não está no `.sln`: não é modelado (limite declarado na spec).
- Busca Files/Text, git status/gutter/diff e cwd do terminal continuam ancorados na raiz (limite declarado). "Show all files" de um projeto de fora lista o diretório do projeto, que está no escopo.
- Testes (aba Tests) do projeto de fora: provados no host (`a_test_project_outside_the_solution_is_discovered_and_runs`), não clicados no app real.
- Playwright: não é prova desta feature (decisão da #67).
