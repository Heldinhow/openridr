# Evidência — skill `verify-openridr`

Data: 2026-11-03. macOS ARM64. Rodada 1 (seções 1–5): `OpenRidr 0.3.0-alpha.4`, branch `chore/verify-openridr`. Rodada 2 (seção 6, app Tauri real): `0.3.0-alpha.5`, branch `chore/verify-app-real`. Caminhos pessoais trocados por `~` e `<tmp>`. Capturas em [verify-openridr/](verify-openridr/) (`app-*.jpg` = rodada 2).

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

### 6. Superfície C — app Tauri real dirigido por `agent-desktop` (rodada 2)

Pré-condições: `agent-desktop permissions` → Acessibilidade e Gravação de Tela `granted`. O doctor avisou `installed app 0.3.0-alpha.4 != package.json 0.3.0-alpha.5`; para não sobrescrever o app do usuário em `/Applications`, foi construído um bundle de debug com `bun run tauri build --debug --bundles app` (`src-tauri/target/debug/bundle/macos/OpenRidr.app`, `CFBundleShortVersionString` 0.3.0-alpha.5) e passado a `desktop.sh up <bundle>`. Home isolado em `<tmp>/verify-openridr-desktop/home`, cópia da fixture em `<tmp>/verify-openridr-desktop/work`. O home real do usuário não foi alterado (`grep -c verify-openridr ~/Library/Application Support/OpenRidr/trusted.json` → 0). Uma queda da sessão do agente derrubou a primeira instância; o fluxo foi refeito na segunda, e a decisão de Trust conferida em disco nas duas.

| # | Lacuna | Ação na janela real | Estado observado | Captura |
|---|---|---|---|---|
| 1 | Abrir workspace (argv, mesmo caminho do CLI) | `desktop.sh up` com a `.sln` em argv | `list-windows`: 1 janela `OpenRidr` 1440x900; árvore de acessibilidade com `dialog` `Trust “OrderDesk.sln”?` | [app-01](verify-openridr/app-01-trust-prompt-argv.jpg) |
| 3 | Trust: confiar | `click` em **Trust project** (AXPress) | `trusted.json` = `{"<tmp>/verify-openridr-desktop/work/OrderDesk/OrderDesk.sln": "trusted"}`; combobox `Trusted mode`; `recent.json` com o caminho | [app-02](verify-openridr/app-02-trusted-tree.jpg) |
| 2 | Árvore + editor | `click` em `Program.cs` | árvore: `OrderDesk · 3 projects`, `OrderDesk.Cli · console · net8.0`, `OrderDesk.Core · library`, `OrderDesk.Tests · test`; campo `Editor` com o conteúdo real do arquivo (`using OrderDesk.Core;` … `Console.WriteLine`), `Ln 1, Col 1`, "Evaluated · revision 2" | [app-03](verify-openridr/app-03-editor-program-cs.jpg) |
| 5 | Build | `click` em **Build** (toolbar) | `Build succeeded · exit 0`, 0 warnings/errors; `OrderDesk.Cli.dll` existe na **cópia** (`<tmp>/verify-openridr-desktop/work/OrderDesk/src/OrderDesk.Cli/bin/Debug/net8.0/`, 5120 bytes) | [app-04](verify-openridr/app-04-build.jpg) |
| 5 | Test: descobrir | aba **Tests** → **Discover tests** | `Discovered 4 tests · exit 0` (`Id_is_preserved`, `Total_is_preserved`, `Deliberate_failure_for_runner_evidence`, `Skipped_until_fixture_changes`) | [app-05](verify-openridr/app-05-tests-discover.jpg) |
| 5 | Test: rodar | **Run all** | `Run completed · 4 results · exit 1`: 2 passed, 1 failed, 1 skipped. O failed é a falha deliberada da fixture (`Deliberate_failure_for_runner_evidence`), o skip também é da fixture; o `exit 1` é o esperado | [app-06](verify-openridr/app-06-tests-run-all.jpg) |
| 3 | Trust: restaurar a decisão | `desktop.sh down`, reabrir o binário com o **mesmo** home e a mesma `.sln` em argv | sem diálogo de Trust; direto em `Trusted mode`; `trusted.json` inalterado | [app-08](verify-openridr/app-08-reopen-trusted-no-prompt.jpg) |
| 5 | Debug (smoke) | aba **Debug** → **Start debugging** | sessão `Terminated`, banner `macOS arm64 debugging is experimental ... approved netcoredbg 3.2.0-1092 build`, saída do debuggee `Order A-104: R$ 42,00` | [app-07](verify-openridr/app-07-debug-tab.jpg), [app-09](verify-openridr/app-09-debug-start.jpg) |
| 4 | "Abrir com" pelo terminal | com o app aberto (pid 13565): `OPENRIDR_APP=<bundle de debug> sh scripts/openridr <tmp>/.../OrderDesk.Core.csproj` (exit 0) | `pgrep -x openridr` continuou só no pid 13565 (entregou à instância aberta, sem segunda instância); a UI mostrou `Trust “OrderDesk.Core.csproj”?` | [app-10](verify-openridr/app-10-open-with-cli-dialog.jpg) |
| 3 | Trust: Restricted | **Open in Restricted mode** | `trusted.json` ganhou `"<tmp>/.../OrderDesk.Core.csproj": "restricted"` (a `.sln` segue `trusted`); combobox `Restricted mode`; árvore `OrderDesk.Core · 1 project` | [app-11](verify-openridr/app-11-open-with-restricted.jpg) |

Detalhes do Debug: o home isolado não tinha netcoredbg; em vez de baixar (o que instalaria algo), `<tmp>/verify-openridr-desktop/home/tools` foi um symlink para o `~/Library/Application Support/OpenRidr/tools` que o usuário já tinha (`netcoredbg/3.2.0-1092`, `csharp-ls/0.28`), removido ao final; nada foi instalado nem alterado ali. O `csharp-ls` **não** foi reconhecido por esse caminho (a barra seguiu em `Analysis: not installed` / `Install analysis`, que não foi clicado).

Limpeza: `desktop.sh down` (pid gravado), `pgrep -x openridr` vazio, portas 1520/4417 livres. `<tmp>/verify-openridr-desktop` fica (descartável).

## Não verificado

| O quê | Por quê |
|---|---|
| Editar e **salvar** pela janela real | `agent-desktop type` no campo `Editor` (CodeMirror) devolveu `ACTION_FAILED` e o arquivo/`Save` ficaram intactos; `press cmd+s` exige foco (`--headed`), que o skill reserva à permissão do usuário. Salvar segue provado só na superfície B/spec |
| "Abrir com" pelo **Finder** (menu) e arrastar para o **Dock** | só o caminho `scripts/openridr` → `open -a` (LaunchServices) foi dirigido; a GUI do Finder/Dock não foi acionada |
| Abrir **pasta** com 2+ solutions e troca de workspace com diálogo `externalOpen` na janela real | não exercitado nesta rodada (cobertos pelo spec `open-external`) |
| Debug com breakpoint, passos e locals na UI | só a sessão curta (inicia, imprime, termina) foi observada |
| `csharp-ls` (análise, diagnósticos, completion) no app real | não reconhecido no home isolado nem instalado, por regra desta rodada |
| Cancel, revogar Trust e Esc na janela real | só confiar e Restricted foram dirigidos; o resto segue provado pelos specs `trust-prompt` |
| Windows e macOS Intel | nunca executados |
| TLA+/TLC | sem Java nesta máquina (`/usr/bin/java` é só stub). O candidato 2 é só proposta |
| Falhas conhecidas do HEAD (`find-usages`, `run-configs`, `tests`, Rust `file_access_is_confined_to_the_workspace`) | não relacionadas; não foram reexecutadas |
