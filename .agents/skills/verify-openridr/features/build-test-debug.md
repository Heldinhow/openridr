# 5. Build / test / debug

## Sub-features

- **Build** (`toolbar-build`, painel `build-panel`): `dotnet build --no-restore -c Debug` no projeto de startup; log bruto, problemas navegáveis, cancelar, aviso de buffers sujos (`build-dirty-dialog`). Restore é automático após o Trust (`restore-state`).
- **Run** (`toolbar-run`, `run-panel`): constrói se preciso e roda o assembly; recusa fonte salva depois do build.
- **Test** (`tests-panel`): descoberta VSTest, execução com TRX (`tests-run-*`), cancelar parcial; projeto MTP/sem adaptador mostra mensagem exata.
- **Debug** (`toolbar-debug`, `dap-panel`): netcoredbg via DAP; breakpoints, passos, locals. No macOS ARM64 o host liga a flag experimental na sessão e **baixa** o netcoredbg oficial para `<app home>/tools`. **Não há diálogo de consentimento**: o clique em Debug é o consentimento (`src/useExecutionFlow.ts`: "there is no consent question").
- Gates: sem Trust ou sem restore, cada ação fica desabilitada com motivo textual (`*-disabled-reason`) e o host responde `trust_required`/`restore_required`.

## How to get to it (user POV)

Toolbar superior (martelo = Build, ▶ = Run, besouro = Debug) ou faixa inferior de tool windows; atalhos Cmd+F9 (build), Ctrl+R (run saved build), Ctrl+D (debug, macOS).

## Driving it

- **A** (provado em 2026-11-03 para build): `bunx playwright test tests/e2e/build.spec.ts` → build real com SDK 8.0.408, `build-state` "Build succeeded · exit 0", erro de compilação navegável. Para run/test/debug: `run.spec.ts`, `tests.spec.ts`, `testsuite.spec.ts`, `debugger.spec.ts`, `execution-gates.spec.ts`. O debug real precisa de rede (instala o netcoredbg) e do `dotnet`; sem eles o spec faz `skip`: registre "não verificado".
- **B**: abrir + Trust (feature 3), esperar `restore-state` "Restore succeeded" e `eval-state` "Evaluated", `browser.click` em `[data-testid="toolbar-build"]`, esperar `[data-testid="build-state"]` com "Build succeeded". Efeito colateral: `src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll` existe **na cópia** em `<state>/work` (o `<state>` é impresso pelo `web.sh up`).
- **C**: mesmo fluxo pela árvore de acessibilidade; para debug, basta build atual do mesmo contexto — o host baixa o netcoredbg na primeira vez (precisa de rede) e não pergunta nada.

Prova de que funciona: estado do painel + `.dll` em disco (build); saída do programa no `run-log`; contagens/resultados no `tests-run-counts`; `dap-state` paused no breakpoint (debug).

## Gotchas

- `dotnet` e o SDK 8.0.408 precisam estar no PATH (`$HOME/.dotnet`): `export PATH="$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT=$HOME/.dotnet`.
- Falhas conhecidas do HEAD: 2 em `tests.spec.ts`, 2 em `run-configs.spec.ts`, `find-usages` intermitente. Não as atribua à sua mudança sem comparar com o HEAD.
- Build demorado e restore concorrente: espere pelo estado, não por tempo fixo.
- `debug.start` só roda com build atual do mesmo contexto; trocar configuração exige `evaluation.run`.
- Não rode a suíte inteira por reflexo; portas 1420/4317 e a fixture são compartilhadas.
