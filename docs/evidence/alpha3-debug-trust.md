# Evidência — alfa 3: Debug sem pergunta, confiança no aviso e dados de teste isolados

Máquina: macOS arm64, Google Chrome (Playwright `channel: "chrome"`), .NET SDK 8.0.408 em
`~/.dotnet`, netcoredbg 3.2.0-1092 em `~/Library/Application Support/OpenRidr/tools/netcoredbg`.
Host E2E com `OPENRIDR_HOME` temporário (`/var/folders/.../T/openridr-e2e-home`) e
`OPENRIDR_TRUST_STORE=off`. Branch `fix/debug-no-prompt-trust-ux`.

## Ticket 35 — Debug sem modal

O spec one-click copia `fixtures/OrderDesk` para um diretório temporário, abre a cópia, confia no
projeto, espera o restore automático, marca o breakpoint da linha 3 e clica `toolbar-debug` uma
vez. O teste afirma `debug-consent-dialog` ausente, `dap-experimental-note` visível e a sessão real
em `Paused` com frame em `Program.cs:3`.

```sh
PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/debugger.spec.ts -g "one click"
# ✓ 1 tests/e2e/debugger.spec.ts:514:1 › one click on Debug enables the flag, builds implicitly
#   and stops at a breakpoint (8.8s)
# 1 passed (25.5s)

PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/ui-fold.spec.ts
# ✓ 4 tests/e2e/ui-fold.spec.ts (inclui "debug frames and locals are in the first fold while paused",
#   com o netcoredbg real copiado para o app home temporário)
# 4 passed (22.7s)

PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/execution-gates.spec.ts
# ✓ 4 tests/e2e/execution-gates.spec.ts
# 4 passed (1.6s)
```

O que essas execuções observaram:

- A flag experimental estava desligada (o host a reseta ao abrir outro workspace) e nenhum
  `debug-consent-*` apareceu; o clique em Debug ligou a flag, reusou o netcoredbg presente e
  chegou a `Paused` sem clique intermediário.
- `dap-experimental-note` aparece no painel Debug enquanto a flag está ativa.
- `executionGates.debugDisabledReason` não bloqueia mais por adaptador ausente nem por flag
  desligada; os demais motivos (trust, SDK, avaliação, configuração, startup, build em andamento e
  sessão ativa) continuam.

## Não verificado

- App Tauri/WKWebView empacotado: a prova é pela UI no Chrome sobre a ponte HTTP de dev; o bundle
  `.app` não foi reexecutado nesta rodada.
- Windows x64 e macOS Intel: não executados.
