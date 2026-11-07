# HANDOFF — #54 (releases macOS Intel e Windows)

Estado em 2026-11-07, sessão encerrada por teto de contexto. Este doc é o ponto de entrada da
próxima sessão; ele não substitui a issue #54 nem a spec.

## Onde está

- Worktree: `/Volumes/DevSSD/t3-worktrees/openridr/feat-issue-54-intel-windows-releases`
- Branch (integration branch): `feat/issue-54-intel-windows-releases`, HEAD `7042ead`, já com
  `origin/main` (inclui #55, o updater) mergeado.
- Push feito; a branch está em `origin/feat/issue-54-intel-windows-releases`.
- PR ainda **não** aberto.

## Docs e decisões

- Spec: `docs/SPEC-PLATAFORMAS.md`.
- Decisão: `docs/decisions.md`, entrada `2026-11-07 — Releases para macOS Intel e Windows`.
- Glossário: `GLOSSARY.md` ganhou **Release target**, **Bundled pair**, **Installable artifact**,
  **Install smoke**.
- Tickets: #199 (host Windows/Intel), #200 (bundle por plataforma), #201 (build Windows NSIS),
  #202 (build macOS x64), #203 (smoke doc), #204 (release, liberado pela #55), #205 (SignPath,
  needs-triage), #280 (dotnet tool install no runner Windows).

## O que já passou (com URL do run)

Workflow novo `.github/workflows/platforms.yml` (`workflow_dispatch` + noturno + push na `main`),
fora dos required checks. O `ci.yml` não mudou.

- macOS Intel (`macos-15-intel`): **verde** — `cargo test -p openridr-host` + clippy `--workspace
  --all-targets -D warnings`. Run: https://github.com/Heldinhow/openridr/actions/runs/42416148794
- macOS arm64 (local): `cargo test -p openridr-host` 462 + 108 + 15 + 11 + 7 + 2 + 1, clippy limpo,
  `cargo check --target x86_64-pc-windows-gnu --all-targets` limpo (mingw-w64 instalado no Mac).
- Windows (`windows-2025`): **vermelho em 4 testes** (ver abaixo). Run:
  https://github.com/Heldinhow/openridr/actions/runs/42416148794
- O que já está verde no Windows: lib 426/426, `bundled_analysis` 3/3, `debugger` unit 4/4,
  `corporate_proxy` (o teste de proxy ficou Unix-only), `test_discovery`/`test_run` ainda não.

## O que falta (decisão do orquestrador: (a) para 1, 2 e 4; (b) para 3)

### 1. `evaluation_includes_a_project_outside_the_solution_directory` (trivial)

`crates/host/tests/host.rs` (~1962): `outside["targetPath"].as_str().ends_with("lib/bin/Debug/net8.0/Lib.dll")`
usa `/`; no Windows o host devolve `\`. Trocar por `Path::new(...).ends_with(...)`.

### 2. `the_tree_and_nuget_see_the_outside_project_inside_the_scope_only` (trivial)

`crates/host/tests/host.rs` (~2276): o outline do host devolve `//?/C:/...` e o teste compara com
`support::portable(...)` (`C:/...`). O ponto que monta o outline ainda não passa por
`fsutil::portable_path` — achar no `lib.rs`/`evaluation.rs` (o `workspace_symbol_items` em
`lib.rs:8900` já tem o `replace('\\', "/")` sem tirar o prefixo; o outline é outro sítio) e usar
`portable_path`.

### 3. `tools_install_is_real_verified_and_reused` e `lsp_start_requires_tool_sdk_and_current_restore`

`dotnet tool install` no runner Windows morre com
`NuGet.Configuration.ConfigurationDefaults`. Já isolado: `USERPROFILE`/`APPDATA`/`LOCALAPPDATA` para
o app home, `TEMP`/`TMP` para o temp do install, `HOME`/`TMPDIR` removidos no Windows
(`crates/host/src/tools.rs`, `isolate_child_home`). Não resolveu. Issue: **#280**. Decisão: marcar
os dois testes como `#[cfg(not(windows))]` com comentário apontando a #280 e seguir.

### 4. Bug real do produto: path verbatim (`\\?\`) no MSBuild (o item mais importante)

`test_discovery` e `test_run` mostram que o host passa `\\?\C:\...\app\App.sln` ao `dotnet msbuild`
e o MSBuild falha:

```
MSB3202: The project file "\\?\C:\...\app\..\lib\Lib.csproj" was not found.
```

O prefixo verbatim que o `fs::canonicalize` adiciona no Windows quebra o MSBuild (qualquer build,
não só projeto fora do root). Decisão: **uma função que tira o prefixo verbatim (`\\?\` e `//?/`)
antes de passar caminho a processo externo**, sem dependência nova, usada em todos os pontos que
chamam `dotnet`/`msbuild`, com teste unitário que falha antes. Candidatos:
`crates/host/src/evaluation.rs` (`invoke`, `plan_project`), `crates/host/src/build.rs`,
`crates/host/src/test_run.rs`/`test_discovery.rs` (os comandos), `crates/host/src/tools.rs`.
`fsutil::portable_path` já faz a versão string; falta a versão `PathBuf` para argv.

Depois disso, rodar o `platforms.yml` e fechar o #199 (comentar e fechar a issue).

### 5. Antes do PR: remover a branch temporária do `platforms.yml`

`.github/workflows/platforms.yml` tem `push: branches: [main, feat/issue-54-intel-windows-releases]`
para a prova. **Voltar para `[main]`** num commit antes do PR (a branch específica foi só para
disparar o run; `workflow_dispatch` não funciona antes do merge porque o arquivo não está na
`main`).

### 6. Depois do #199: #200 → #201/#202 em paralelo → #203 → #204

- #200: `bundled.rs` (par por plataforma) + `debugger.rs` (`bundled_binary_for`), com testes.
- #201: `scripts/vendor-windows.ts`, `src-tauri/tauri.windows.conf.json` (NSIS currentUser),
  `package-windows` no `platforms.yml`.
- #202: `vendor-analysis.sh` por `TAURI_ENV_TARGET_TRIPLE`, `vendor-netcoredbg.sh` sem binário no
  x64, `package-macos-x64` no `platforms.yml`.
- #203: `docs/SMOKE-PLATAFORMAS.md` + `docs/evidence/plataformas.md` + README/RELEASE-ALPHA.
- #204: liberado pela #55 (mesclada). Integrar os 3 anexos ao `release.yml`/`release-macos.sh` novos
  (updater do arm64 não pode quebrar; o x64 não entra no `latest.json`).

## Regras que continuam valendo

- Processo: `/implement-spec` com `delegate_task` + `isolation:"worktree"`, target
  `{providerInstanceId:"pi", model:"mistral/deepseek-v4-1-flash-1017", options:{thinking:"max"}}`.
  O provider falhou 2x nesta sessão; se falhar, faça você mesmo e registre.
- Skills: carregar de verdade e citar o caminho no aviso (`~/.agents/skills/<nome>/SKILL.md`).
  `verify-openridr` e `code-review` antes do PR; `/retro` no fim.
- Commit: `PATH="$HOME/.bun/bin:$PATH" sh scripts/commit-only.sh "msg" <paths>`; nunca
  `--no-verify`; sem `git stash`.
- UI: Playwright no modo web (`tests/e2e`, portas 1420/4317) para interface pura — **pedir o slot do
  E2E ao orquestrador** antes; `agent-desktop` só para o nativo (instalador/DMG/primeira abertura),
  com slot e state próprio.
- Não tocar em `DESIGN.md`/`PRODUCT.md`; sem dependência nova sem decisão.
- PR: título conventional, corpo com `Closes #54` + `## Evidence`; risk:high (release/CI/assinatura),
  merge é do Helder. Avisar o orquestrador em cada marco pelo canal `t3 thread send`.

## Ambiente

- `bun install` já rodado nesta worktree; `node_modules` presente.
- mingw-w64 instalado no Mac (`x86_64-w64-mingw32-gcc`), target `x86_64-pc-windows-gnu` adicionado:
  `cargo check -p openridr-host --target x86_64-pc-windows-gnu --all-targets` é a prova local do
  Windows (não roda testes).
- `.gitattributes` novo (`* text=auto eol=lf`): o checkout do runner Windows converte LF→CRLF e isso
  quebrava fixtures/testes. Não remover.
- Teste instável visto no macOS: `run_start_reports_restore_required_when_assets_are_invalidated_after_a_build`
  falhou uma vez na suíte e passou isolado; registrar se repetir.
- Não rodar `bun run test:e2e` junto com `bun run dev` (portas 1420/4317).
