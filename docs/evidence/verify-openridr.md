# Evidência — skill `verify-openridr`

Data: 2026-11-03. macOS ARM64, `OpenRidr 0.3.0-alpha.4`, branch `chore/verify-openridr`. Caminhos pessoais trocados por `~` e `<tmp>`. Capturas em [verify-openridr/](verify-openridr/).

Ambiente: `PATH` com `~/.bun/bin`, `~/.dotnet`, `~/.dotnet/tools`; `DOTNET_ROOT=~/.dotnet`; SDKs 8.0.408 e 10.0.100.

## Provado de verdade

### 1. Doctor do skill

`sh .agents/skills/verify-openridr/scripts/doctor.sh`:

```text
ok    bun 1.3.9
ok    cargo present
ok    node_modules present
ok    dotnet SDK 8.0.408 (fixtures pin it, rollForward disabled)
ok    Chrome (Playwright default channel)
ok    installed app 0.3.0-alpha.4 matches package.json
note  agent-desktop "accessibility":{"state":"denied"
note  desktop surface also needs the screen unlocked and awake
```

### 2. Superfície A — Playwright (Chrome real, host real, fixture `OrderDesk` real)

`bunx playwright test tests/e2e/trust-prompt.spec.ts tests/e2e/open-external.spec.ts` → **11 passed (13.1s)**:

```text
✓  1 open-external › a relative solution path resolved against baseDir opens the solution
✓  2 open-external › an uppercase extension opens and a .csproj stays a project
✓  3 open-external › .slnx opens as a solution and the outline lists its projects in file order
✓  4 open-external › a directory with two solutions asks for a choice and lists the candidates
✓  5 open-external › refused paths keep the open workspace and never replace it
✓  6 trust-prompt › asks once for a new project, then reopening goes straight in
✓  7 trust-prompt › a Restricted decision is remembered and not asked again
✓  8 trust-prompt › Cancel and Esc return to the welcome screen without opening the project
✓  9 trust-prompt › Cancel on a prompt raised from inside the IDE restores the previous workspace
✓ 10 trust-prompt › the page behind the prompt is inert and a backdrop click keeps focus in the dialog
✓ 11 trust-prompt › focus is trapped: Tab cycles Trust, Restricted, Cancel and Enter activates the focused button
```

`bunx playwright test tests/e2e/build.spec.ts tests/e2e/solution-explorer.spec.ts` → **7 passed (24.9s)**:

```text
✓ 1 build › a real build reports its raw log, fails on a compile error and navigates to it
✓ 2 build › build waits for the automatic restore and asks before using unsaved buffers
✓ 3 solution-explorer › Rider-style rows: counts, solution folders, nested sources and Dependencies
✓ 4 solution-explorer › toolbar: collapse/expand all, filter, minimize
✓ 5 solution-explorer › show all files lists bin/obj/dotfiles as no index and keeps sources indexed
✓ 6 solution-explorer › Restricted project menu blocks execution, offers Trust and edits the csproj
✓ 7 solution-explorer › trusted menu: console can run, library builds; folders show counts
```

### 3. Superfície B — UI dirigida contra o host isolado (portas 1520/4417)

Passos (`scripts/web.sh up`, navegador do OpenChamber, `scripts/web.sh down`):

1. `web.sh up` sobe o host em 4417 e o Vite em 1520 com `OPENRIDR_HOME=<tmp>/home` e copia `fixtures/OrderDesk` para `<tmp>/work/OrderDesk`.
2. Abrir `<tmp>/work/OrderDesk/OrderDesk.sln` pelo formulário da tela inicial → aparece o prompt de Trust ([01-trust-prompt.jpg](verify-openridr/01-trust-prompt.jpg)).
3. Clicar **Trust project** → solution tree com projetos ([02-trusted-solution-tree.jpg](verify-openridr/02-trusted-solution-tree.jpg)).
4. Abrir `Program.cs` na árvore → editor com o arquivo ([03-editor-program-cs.jpg](verify-openridr/03-editor-program-cs.jpg)).
5. Estado persistido e estado do host:

```text
trusted.json  = {"/private/tmp/verify-openridr/work/OrderDesk/OrderDesk.sln": "trusted"}
app.state     → mode: trusted, workspace.kind: solution, decision: trusted, granted: true
```

6. `web.sh down`: portas 1520 e 4417 livres, nenhum processo sobrando.

### 4. "Abrir com" — partes não-GUI

```text
$ sh scripts/openridr-selfcheck.sh
ok: openridr self-check passed

$ cargo test --manifest-path src-tauri/Cargo.toml
test tests::argv_ignores_program_name_and_finder_flags ... ok
test tests::file_urls_become_local_paths_and_other_schemes_are_ignored ... ok
test result: ok. 2 passed; 0 failed
```

### 5. Prova de conceito formal

```text
$ cargo test -p openridr-host --test trust_model
test trust_lifecycle_matches_the_reference_model_on_random_sequences ... ok
test result: ok. 1 passed; 0 failed; finished in 1.37s
```

Teste de mutação: com o `Revoke` gravando `Some(true)` o teste **falhou**; revertido e passou de novo. Detalhes em [../VERIFICACAO-FORMAL.md](../VERIFICACAO-FORMAL.md).

## Não verificado

| O quê | Por quê |
|---|---|
| Superfície C: app Tauri real dirigido por `agent-desktop` | `agent-desktop status`: Accessibility e Screen Recording `denied`; `snapshot` devolve `PERM_DENIED`. O usuário precisa conceder as duas permissões e deixar a tela desbloqueada. Só `list-windows` funcionou (uma janela 1440x900 abriu com a `.sln` em argv). Nenhuma captura da tela foi guardada. |
| "Abrir com" pelo Finder/Dock (LaunchServices) | depende da superfície C |
| Test e Debug (feature 5) na UI | só **build** foi provado (A). Os specs de test/debug dependem de `csharp-ls`/`netcoredbg`, que não foram rodados nesta sessão |
| Windows e macOS Intel | nunca executados |
| TLA+/TLC | sem Java nesta máquina (`/usr/bin/java` é só stub). O candidato 2 é só proposta |
| Falhas conhecidas do HEAD (`find-usages`, `run-configs`, `tests`, Rust `file_access_is_confined_to_the_workspace`) | não relacionadas; não foram reexecutadas |
