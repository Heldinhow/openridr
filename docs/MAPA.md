# Mapa do repositório — onde fica o quê

Use este mapa antes de `grep`/`ls` em `src/` ou `crates/`. Os arquivos grandes estão em **negrito**: leia por trecho (`grep -n`, depois `sed -n`), nunca inteiros. Tamanhos são aproximados (nov/2026).

## Achar a dona de uma coisa em 1 comando

| Você tem | Rode |
|---|---|
| um `data-testid` | `grep -rn 'data-testid="build-panel"' src` (o contrato está em [implementation-notes](implementation-notes.md#testids-estáveis-e2e)) |
| um método do host (`build.start`) | `grep -n '"build.start"' crates/host/src/lib.rs` → o `if method == ...` chama a função `start_build`; o módulo é `crates/host/src/build.rs` |
| o wrapper TS de um método | `grep -n 'build.start' src/host/client.ts` (tipos do snapshot também estão ali) |
| um atalho/ação | `src/actions/keymap.ts` (teclas) e `src/actions/registry.ts` (ações) |
| um termo do domínio | [GLOSSARY.md](../GLOSSARY.md) |
| a cor/ícone de um projeto ou arquivo | `src/solutionVisuals.ts` (tipo e glifo) e `--icon-*` em `src/styles.css` (cor) |
| uma decisão ("por quê?") | `grep -n '^## ' docs/decisions.md \| grep -i <tema>`; entradas "Substituída" já vêm marcadas |
| como um ticket foi feito | `grep -n '^## ' docs/implementation-notes.md \| grep -i <tema>` |
| o spec de uma feature | tabela "Feature → docs" em [README.md](README.md) |

## UI — `src/` (React 18 + TS + CodeMirror)

| Arquivo | O que tem |
|---|---|
| `main.tsx`, `App.tsx`, `nativeMenu.ts` | entrada; alterna Welcome × SetupGate × Workspace; recebe "abrir com" do Finder/Dock; guard que cancela o menu do WebKit fora de campo de texto (produção) |
| `SetupGate.tsx` | gate de setup obrigatório (progresso, erro, retry, runtime) antes do Trust/workspace |
| `welcome/` | tela inicial: recentes, toolchain, formulário de caminho |
| **`WorkspaceView.tsx`** (~5.200 linhas) | **um único componente** que monta o workspace: abas, editor, painéis, diálogos, atalhos globais. Começa em `export function WorkspaceView`. Antes dele só há helpers puros. Fluxos já extraídos: `useExecutionFlow.ts`, `useEvaluationFlow.ts`, `useRunConfigs.ts` |
| **`CodeEditor.tsx`** (~1.200) | CodeMirror: gutter, breakpoints, marcas de diagnóstico, completion, rename |
| `SolutionTree.tsx`, `TreeRow.tsx`, `solutionNodes.ts`, `solutionMenu.ts` | Solution Explorer (modelo dos nós, menu de contexto; `TreeRow` é o único dono do `preventDefault` e do teclado `Shift+F10`) |
| `solutionVisuals.ts`, `FileIcon.tsx`, `icons/` | tipo visual de projeto/arquivo/símbolo (ícone + token `--icon-*`); glifos vendorizados em `icons/fileGlyphs.tsx` (licenças em `THIRD-PARTY-NOTICES.md`); cores em `styles.css` |
| `shell/` | tool windows (faixas), barra de status, modo Restricted/Trusted |
| `actions/` | keymap, registro de ações, Search Everywhere (`FindAction`), diálogo de Keymap |
| `useExecutionFlow.ts`, `executionGates.ts`, `executionCommands.ts` | Build/Run/Debug: fluxo, gates (por que está desabilitado) e comandos |
| `BuildPanel`, `RunPanel`, `DebugPanel` (+ `Debug*.tsx`), `TestsPanel`, `TerminalPanel`, `ProblemsPanel`, `NuGetPanel`, `FindInFiles`, `FindUsagesPanel`, `ProcessPanel` | um painel por arquivo |
| `ToolchainPanels.tsx` | painéis de SDK, restore, diagnóstico/reinstalação de ferramentas, LSP, adaptador de debug |
| `git.tsx` (~1.000) | tudo de Git da UI (status, commit, branches, diálogos) |
| `runConfigs.tsx`, `SettingsDialog.tsx` | Run configurations; Settings |
| `TrustControls.tsx`, `TrustPrompt.tsx`, `SetupScreen.tsx` | Trust e tela de setup |
| `host/client.ts` | **única** fronteira com o host: Tauri (`host_call`) ou HTTP de dev. Contém os tipos do `Snapshot` |
| **`styles.css`** (~3.200), `rider-chrome.css` | todo o CSS; busque por classe (`grep -n '\.build-panel' src/styles.css`) |

## Host — `crates/host/src/` (Rust, sem Tauri)

`lib.rs` (~7.700 linhas) é `Host` + `dispatch`: a tabela de métodos (`if method == "x.y"`), os gates de trust e `EXECUTABLE_METHODS`. Os **testes unitários de `lib.rs` ficam no fim** (`mod tests`, ~linha 7.020): ignore-os ao procurar código de produção. Testes de fronteira: `crates/host/tests/{host,debugger,test_discovery,test_run}.rs`.

| Módulo | Responsabilidade |
|---|---|
| `evaluation.rs`, `outline.rs`, `sdk.rs`, `hostroot.rs` | avaliação MSBuild, outline de texto e scan de tipo C# (`workspace.symbolKinds`), inventário de SDKs, `DOTNET_ROOT` do csharp-ls |
| `bundled.rs` | o par embutido em `Contents/Resources` (`csharp-ls/CSharpLanguageServer.dll` + runtime privado `dotnet/`); overrides de debug `OPENRIDR_BUNDLED_CSHARP_LS`/`OPENRIDR_BUNDLED_DOTNET_ROOT`. Quem verifica o par é `tools::sync_bundled` |
| `restore.rs`, `build.rs`, `run.rs`, `run_configs.rs` | restore explícito, build, run, configurações salvas |
| `lsp.rs`, `diagnostics.rs` | cliente LSP (csharp-ls), snapshots de diagnóstico |
| `dap.rs`, `debugger.rs` | cliente DAP; instalação/gates do netcoredbg |
| `test_discovery.rs`, `test_run.rs` | VSTest: descoberta e execução (TRX) |
| `terminal.rs`, `process.rs` | PTY; registro de processos e término |
| `tools.rs`, `nuget.rs`, `git.rs`, `search.rs` | instalação de ferramentas, NuGet, Git, busca de arquivos/texto |
| `trusted.rs`, `recent.rs`, `settings.rs`, `fsutil.rs`, `logbuf.rs`, `error.rs` | estado persistido no app home, logs limitados, erros |

Estado do app: `~/Library/Application Support/OpenRidr` (`OPENRIDR_HOME` troca; nos E2E é isolado).

## Outras pastas

| Pasta | O que é |
|---|---|
| `crates/host-http/` | ponte HTTP **só de dev/E2E** (127.0.0.1:4317) |
| `src-tauri/` | janela Tauri; `src/main.rs` expõe `host_call` e a recepção de "abrir com"; `tauri.conf.json` tem versão e bundle. `resources/{netcoredbg,csharp-ls,dotnet}/` são gerados no build (`scripts/vendor-*.sh`) a partir de artefatos pinados em `vendor/` |
| `tests/e2e/` | Playwright contra host real; arquivos `medium-*`, `qa-*`, `zz-acceptance` etc. por onda de trabalho; helpers `.ts` sem `.spec` |
| `fixtures/` | `OrderDesk` (principal), `MultiConsole`, `BrokenImport`, `MissingSdk`, `CrashApp` |
| `scripts/` | `install-app.sh` (`bun run app:install`), `openridr` (CLI "abrir com"), `openridr-selfcheck.sh`, `commit-only.sh` (`scripts/commit-only.sh "msg" <paths>`: `add -N` nos novos + `commit --only`), `release-macos.sh` (release: preflight `--check`, ensaio `--dry-run`, auditoria de Mach-O `--audit`, caminho completo), `release-selfcheck.sh`, `release-notes.sh` + `release-notes-selfcheck.sh`; `vendor-*.sh` geram as árvores assinadas |
| `.github/workflows/release.yml` | release por tag `v*`: preflight, keychain temporário, `scripts/release-macos.sh`, notas e `gh release create`; ver [release-signing.md](release-signing.md) |
| `.agents/skills/verify-openridr/` | skill de verificação: `SKILL.md`, `features/` (uma por feature), `scripts/{web,desktop,doctor}.sh`. Portas próprias 1520/4417. Evidência em `docs/evidence/verify-openridr.md` |
| `crates/host/tests/trust_model.rs` | teste de propriedade do ciclo de trust contra um modelo de referência (ver `docs/VERIFICACAO-FORMAL.md`) |
| `.githooks/` | pre-commit = `bun run check` |
| `.scratch/` | rascunho de sessões (capturas, scripts de auditoria visual). **Não é código do produto**; ignore ao buscar. Está no `.gitignore` e não é versionado |
| `docs/` | ver [README.md](README.md) |
