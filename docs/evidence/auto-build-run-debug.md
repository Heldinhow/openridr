# Evidência — build implícito em Run/Debug (2026-11-01)

Máquina: macOS arm64, Google Chrome (Playwright `channel: "chrome"`), .NET SDK 8.0.408 em
`~/.dotnet`, netcoredbg 3.2.0-1092 instalado no app home padrão. Branch
`fix/auto-build-on-run`.

## UI real (host real, fixture sem `bin/`/`obj/`)

As specs copiam `fixtures/OrderDesk` para um diretório temporário com o filtro
`!/[\\/](bin|obj)([\\/]|$)/`, abrem a cópia pela UI, confiam no projeto e esperam o
restore automático. Antes do Run, o teste afirma `build-state = "No build has run"`.

```sh
PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/run.spec.ts \
  -g "builds the startup project when there is no build"
# ✓ one click on Run builds the startup project when there is no build (5.6s)

PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/debugger.spec.ts \
  -g "builds implicitly"
# ✓ one click on Debug enables the flag, builds implicitly and stops at a breakpoint (6.5s)
```

O que essas execuções observaram:

- Run: clique na toolbar sem build → `run-state` passa por `Building…`/`Running…`/`Run exited`;
  `build-state` termina `Build succeeded · exit 0`; `run-log` contém `Order A-104`.
- Debug: clique na toolbar com a flag experimental desligada → diálogo `debug-consent-dialog` →
  `Enable and debug` → `dap-state` passa por `Building` → sessão real pausa em `Program.cs:3`.
  O adaptador é o netcoredbg real copiado para o app home temporário da execução.

Espécimes completos também rodaram nesta rodada e passaram: `execution-gates.spec.ts` 4,
`gates.spec.ts` 9, `run.spec.ts` 4, `debugger.spec.ts` 12, `zz-acceptance.spec.ts` 2.

## Host (dotnet real)

```sh
cargo test -p openridr-host --test host run_implicitly_builds_and_runs_when_there_is_no_build
cargo test -p openridr-host --test host a_failed_implicit_build_fails_the_run
cargo test -p openridr-host --test host cancelling_a_run_waiting_for_its_build
cargo test -p openridr-host --test host debug_start_gates
cargo test -p openridr-host --test host stopping_a_debug_session_waiting
cargo test -p openridr-host --test host debug_implicitly_builds_and_starts
```

Todos passaram, incluindo os casos de falha (diagnósticos permanecem no Build e nenhum processo
`run:`/`debug:` nasce) e de cancelamento (build e lançamento ficam `cancelled`, sem exit
inventado). `bun run check` (tsc + `cargo check --workspace --all-targets`) passou.

Suíte completa `cargo test -p openridr-host --no-fail-fast`: 184 unit, 9 `debugger`, 69 `host`,
9 `test_discovery` e 5 `test_run` passaram; a única falha é a de baseline já registrada
`file_access_is_confined_to_the_workspace` (depende da profundidade do worktree e é anterior a
esta mudança).

## Não verificado

- Windows x64 e macOS Intel: não executados; o build implícito usa o mesmo `dotnet build` do
  build explícito e herda a limitação já registrada para essas plataformas.
- App Tauri/WKWebView empacotado: a prova é pela UI servida ao Chrome e pela ponte HTTP de
  dev; o bundle não foi reexecutado nesta rodada.
- O teste de Debug com sucesso real faz `skip` (não falha) quando o netcoredbg 3.2.0-1092 não
  está no app home padrão da máquina executora.
