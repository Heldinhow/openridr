# Confinamento de acesso a arquivos: as duas falhas "conhecidas" do `cargo test --workspace`

Branch: `fix/file-access-confinement` (de `origin/main`, v0.3.0-alpha.7). Data: 2026-11-04.
Escopo: `file_access_is_confined_to_the_workspace` e `evaluation_returns_the_real_msbuild_model_for_every_project`.

## Resumo

| Teste | Classificação | Causa |
|---|---|---|
| `file_access_is_confined_to_the_workspace` | **teste dependente do ambiente (c)** sobre um **defeito real de contrato (severidade baixa)**: negação não fail-closed / oráculo de existência | `resolve_in_root` canonicalizava **antes** de checar a contenção, então um alvo **inexistente** fora do workspace respondia `not_found` e um alvo **existente** respondia `outside_workspace` |
| `evaluation_returns_the_real_msbuild_model_for_every_project` | **dependência de ambiente (c)** | a fixture `OrderDesk` não tem `obj/`, então `restored` é `not_restored`; o teste exigia `restored` sem preparar o estado |

**Não houve escape de leitura nem de escrita.** Nenhum vetor testado (30 casos, abaixo) conseguiu ler, escrever, renomear ou apagar fora do workspace; o arquivo externo de controle terminou intacto em todas as execuções. O defeito é de contrato/observabilidade, não de confinamento.

## Antes: falha exata

```
$ export PATH="$HOME/.bun/bin:$HOME/.cargo/bin:$PATH"
$ cargo test --workspace
...
---- file_access_is_confined_to_the_workspace stdout ----
thread 'file_access_is_confined_to_the_workspace' panicked at crates/host/tests/host.rs:156:9:
assertion `left == right` failed: expected ../../../../../../etc/hosts to be rejected
  left: "not_found"
 right: "outside_workspace"

---- evaluation_returns_the_real_msbuild_model_for_every_project stdout ----
thread 'evaluation_returns_the_real_msbuild_model_for_every_project' panicked at crates/host/tests/host.rs:1136:5:
assertion `left == right` failed
  left: String("not_restored")
 right: "restored"

test result: FAILED. 78 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 45.80s
```

## Diagnóstico do teste 1

### O código

`crates/host/src/lib.rs:7268` (`resolve_in_root`, usado por `resolve_in_workspace`) fazia:

1. `candidate = root.join(raw)` (ou `raw` se absoluto);
2. `fs::canonicalize(&candidate)` → em erro, `not_found`;
3. só então `canonical.starts_with(root)` → `outside_workspace`.

`fs::canonicalize` exige que o caminho exista. Logo, a resposta dependia do que existe **fora** do workspace: um caminho de fuga inexistente morria em `not_found` no passo 2 e nunca chegava ao passo 3.

Ordem idêntica existia em mais quatro fronteiras:

- `resolve_path_in_workspace` (`lib.rs:7155`, create/rename/delete) — canonicalizava o **pai** antes de checar contenção;
- `resolve_reveal_in_workspace` (`lib.rs:7243`) — "Show in Finder";
- `workspace.tree` (`lib.rs:5226`) — listagem de diretório;
- `nuget::resolve_project` (`crates/host/src/nuget.rs:155`) — alvo do NuGet.

### Por que o teste passava em alguns checkouts

O laço original usava `"../../../../../../etc/hosts"` a partir da raiz do workspace. Só que `/etc/hosts` é alcançado de forma determinística quando o `..` satura em `/` (checkout raso, com ≤ 5 componentes de caminho). Num checkout fundo — `/Users/helder/.t3/worktrees/openridr/<worktree>/fixtures/OrderDesk` — 6 níveis acima caem em `/Users/helder/etc/hosts`, que não existe, e o resolver respondia `not_found`. `git log -S` mostra que o resolvedor já nasceu assim em `89adba0` (`feat(workspace): open solution in restricted mode with safe text editing`), junto com o teste: a dependência de profundidade existe desde o primeiro commit dessa fatia, não é regressão.

### Prova empírica do defeito (e da ausência de escape)

Um harness temporário (não commitado) disparou 30 vetores pelo `dispatch` real contra um workspace em `tempdir` com arquivo e diretório externos de controle:

| Vetor | Antes | Depois |
|---|---|---|
| `file.read` `/etc/hosts` (existe) | `outside_workspace` | `outside_workspace` |
| `file.read` `/etc/nope-xyz` (não existe) | **`not_found`** | `outside_workspace` |
| `file.read` `../nope.txt` | **`not_found`** | `outside_workspace` |
| `file.write` `../pwned.txt` | **`not_found`** | `outside_workspace` |
| `file.write` `../../../../../tmp/x` | **`not_found`** | `outside_workspace` |
| `workspace.tree` `../nope` | **`not_found`** | `outside_workspace` |
| symlink de arquivo → fora (read/write) | `outside_workspace` | `outside_workspace` |
| symlink de diretório → fora (read/write/create/rename/tree) | `outside_workspace` | `outside_workspace` |
| symlink em **componente intermediário** (`sub/Link.cs`, `sub/LinkDir/x`) | `outside_workspace` | `outside_workspace` |
| `file.create` sobre symlink existente | `already_exists` (`create_new`) | `already_exists` |
| `file.rename` sobre symlink no destino | troca o link, não o alvo | idem |
| `file.delete` de symlink de diretório | apaga o link | idem |
| `file.delete` de `.` / raiz | `outside_workspace` | `outside_workspace` |

Efeito de segurança real: **oráculo de existência**. `not_found` × `outside_workspace` respondia "existe fora do workspace?" para qualquer caminho absoluto, sem trust nenhum (`file.read` é read-only e não pede trust). Além disso a negação não era fail-closed: o host `stat`ava o caminho de fuga antes de decidir.

## Correção

Duas checagens, nesta ordem, em **todos** os resolvedores (a léxica não substitui a canônica):

1. **Léxica primeiro** — `fsutil::normalize_lexically` + `fsutil::stays_inside` (`crates/host/src/fsutil.rs:181` e `:204`) normalizam `.`/`..` **sem tocar no disco** e recusam por forma, com a mesma resposta (`outside_workspace`) para alvo existente e inexistente. `..` acima da raiz satura na raiz, como o SO resolve.
2. **Canônica depois** — mantida a checagem `starts_with(root)` sobre o `canonicalize`, que é o que impede um symlink dentro do workspace (componente final **ou** intermediário) de resolver para fora.

Pontos alterados:

| Arquivo:linha | Fronteira |
|---|---|
| `crates/host/src/lib.rs:7270` | `resolve_in_root` — `file.read`, `file.write`, LSP (`didOpen`/`didChange`/completion/definition/rename/code action), breakpoints e demais `debug.*`, `workspace.symbolKinds` |
| `crates/host/src/lib.rs:7209` | `resolve_path_in_workspace` — `file.create`, `file.rename`, `file.delete` (checa o pai antes de canonicalizá-lo) |
| `crates/host/src/lib.rs:7314` | `resolve_reveal_in_workspace` — `file.reveal` |
| `crates/host/src/lib.rs:5229` | `workspace.tree` — listagem de diretório |
| `crates/host/src/nuget.rs:167` | `nuget::resolve_project` — plano/aplicação de pacotes |

`file.create` continua usando `create_new` (não segue symlink existente), `file.delete` decide por `symlink_metadata` (apaga o link, não o alvo) e `file.rename` substitui o link do destino em vez do alvo — comportamento preservado e agora coberto por teste.

## Testes

`crates/host/tests/host.rs`:

- `file_access_is_confined_to_the_workspace` (reescrito, mesmo nome) — workspace dentro de uma pasta pai, então os caminhos "um nível acima" **existem de verdade** e nada depende da profundidade do checkout nem do `/etc` local. Cobre: escape absoluto existente e inexistente, `..` existente e inexistente, `..` profundo, escape por componente intermediário, `path_not_allowed` para path vazio, e read/write por symlink de arquivo, symlink de diretório e symlink intermediário — com o arquivo externo verificado intacto.
- `file_operations_refuse_escapes_and_leave_the_outside_untouched` (novo) — `file.create`, `file.rename` e `file.delete` recusam alvo absoluto fora, `..`, caminho profundo e pasta symlinkada; o diretório externo continua com exatamente os mesmos nomes; apagar o symlink de diretório apaga só o link; e as três operações **dentro** do workspace continuam funcionando (prova de que a recusa é confinamento, não superfície quebrada).
- `the_directory_listing_refuses_escapes_by_shape` (novo) — `workspace.tree` com `..`, `../..`, absoluto existente/inexistente e symlink de diretório.
- `fsutil::lexical_normalization_decides_containment_without_the_disk` (unitário, `crates/host/src/fsutil.rs:212`) — normalização e contenção por forma, incluindo `..` saturando na raiz e prefixo parecido (`/work/app` × `/work/applications`).

## Teste 2: `evaluation_returns_the_real_msbuild_model_for_every_project`

`restored` é tri-estado derivado de `obj/project.assets.json` + `obj/project.nuget.cache` (`crates/host/src/restore.rs:381`), nunca de heurística. A fixture `fixtures/OrderDesk` (com `bin/` e `obj/` no `.gitignore`) chega sem `obj/`, então o host responde `not_restored` — correto. O teste exigia `restored` sem preparar esse estado: dependência de `dotnet restore` prévio e de `~/.nuget`, ou seja, ambiente.

Correção sem enfraquecer a asserção: o teste agora trabalha numa **cópia própria** da fixture (`fixture_copy`) e chama `stage_restore_artifacts`, que escreve `obj/project.assets.json` e `obj/project.nuget.cache` (`{"success": true}`) **depois** da cópia, para que o mtime fique mais novo que o do `.csproj`. Com isso `cli["restored"] == "restored"` continua sendo asserção exata e determinística, sem rede. Restore real segue provado por `restore_reports_the_exact_command_and_creates_real_assets`, e o estado oposto (`not_restored` numa cópia limpa) por `evaluation_of_a_fresh_copy_with_spaces_creates_no_restore_artifacts`.

## Depois

```
$ export PATH="$HOME/.bun/bin:$HOME/.cargo/bin:$PATH"
$ cargo test --workspace
test result: ok. 236 passed; 0 failed; 0 ignored   (unittests src/lib.rs)
test result: ok.  14 passed; 0 failed; 0 ignored   (tests/debugger.rs)
test result: ok.  82 passed; 0 failed; 0 ignored   (tests/host.rs)
test result: ok.   9 passed; 0 failed; 0 ignored   (tests/test_discovery.rs)
test result: ok.   5 passed; 0 failed; 0 ignored   (tests/test_run.rs)
test result: ok.   1 passed; 0 failed; 0 ignored   (tests/trust_model.rs)
test result: ok.   0 passed; 0 failed; 0 ignored   (host-http main, doc-tests)
```

Antes: 78 ok / 2 falhas em `tests/host.rs`. Depois: 82 ok / 0 falhas (3 testes novos + 1 unitário novo).

```
$ bun run check      # tsc --noEmit + cargo check --workspace --all-targets
exit=0
```

## Não verificado

- App Tauri real e Playwright/E2E: outra sessão estava usando o app (1420/4317) e a tarefa proibia abrir Vite/tauri; a UI nunca precisou mudar para essa correção (o contrato dos erros é o mesmo `outside_workspace`). Nenhum `data-testid` tocado.
- Windows e macOS Intel: só o alvo macOS ARM64 foi executado. A normalização léxica trata `Prefix`/`RootDir` de forma genérica, mas não há evidência de execução nesses alvos.
- TOCTOU entre a resolução e o I/O: `file.write` já tinha uma rechecagem antes do `rename` e um comentário explícito sobre a janela restante; esta correção não a fecha nem a amplia (um atacante local trocando um diretório do workspace entre a checagem e a escrita continua fora de escopo).
