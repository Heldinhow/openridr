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
DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/debugger.spec.ts
# ✓ 12 tests (inclui "macOS arm64 requires the experimental flag and shows the real asset")
# 12 passed (1.3m)

PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/ui-fold.spec.ts
# ✓ 4 tests/e2e/ui-fold.spec.ts (inclui "debug frames and locals are in the first fold while paused",
#   com o netcoredbg real copiado para o app home temporário)
# 4 passed (22.7s)

PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/execution-gates.spec.ts
# ✓ 4 tests/e2e/execution-gates.spec.ts
# 4 passed (1.6s)

PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/zz-acceptance.spec.ts -g "real P0 journey"
# ✓ 1 zz-acceptance.spec.ts:171 › real P0 journey: restricted, trust, SDKs, evaluation, restore,
#   LSP, build, run, tests, PTY and DAP (36.7s)
# 1 passed (41.3s)
```

O que essas execuções observaram:

- A flag experimental estava desligada (o host a reseta ao abrir outro workspace) e nenhum
  `debug-consent-*` apareceu; o clique em Debug ligou a flag, reusou o netcoredbg presente e
  chegou a `Paused` sem clique intermediário.
- `dap-experimental-note` aparece no painel Debug enquanto a flag está ativa.
- `executionGates.debugDisabledReason` não bloqueia mais por adaptador ausente nem por flag
  desligada; os demais motivos (trust, SDK, avaliação, configuração, startup, build em andamento e
  sessão ativa) continuam.
- Dois specs que assumiam o gate antigo foram alinhados: `debugger.spec.ts` (toolbar Debug agora
  fica habilitada sem adaptador; o teste espera a avaliação) e `zz-acceptance.spec.ts` (Debug
  habilitado após build falho; e a espera do rebuild passa a exigir que a revisão do build avance,
  em vez de aceitar o "Build succeeded" do build anterior).

## Ticket 36 — Confiar no próprio aviso

O spec abre uma cópia temporária em Restricted, confere os três avisos (`build-disabled-reason`,
`run-disabled-reason`, `dap-disabled-reason`) com seus botões e clica `build-trust`. Sem checkbox
no aviso.

```sh
PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/gates.spec.ts
# ✓ 10 tests/e2e/gates.spec.ts (inclui "Restricted warnings grant trust from the reason itself")
# 10 passed (21.8s)
```

O que a execução observou: em Restricted, `build-trust`, `run-trust` e `dap-trust` aparecem com o
rótulo "Trust this project"; nenhum `trust-checkbox` dentro dos avisos; após `build-trust`,
`status-mode` fica "Trusted mode", os motivos de Restricted somem e os botões de trust
desaparecem. O grant é feito por `trust.grant {remember: true}` (a persistência é a mesma coberta
pelos specs de trust). `ui-layout.spec.ts` 7/7 confirma que a barra superior continua sem
indicador de confiança e que o layout não mudou.

## Ticket 37 — Dados de teste isolados

O host E2E já subia com `OPENRIDR_HOME: e2eHome` e `OPENRIDR_TRUST_STORE=off` em
`playwright.config.ts`; o spec agora prova o contrato pela ponte real:

```sh
PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/environment.spec.ts
# ✓ 1 environment.spec.ts:4 E2E ports are validated and the loopback proxy rejects foreign origins
# ✓ 2 environment.spec.ts:23 the E2E host uses the isolated app home
# 2 passed (1.7s)
```

`bun run host:http` (sem `OPENRIDR_HOME`) respondeu `app.state` com
`appHome = <repo>/.scratch/host-home`, não o app home real:

```sh
bun run host:http &
curl -s -X POST http://127.0.0.1:4317/call -H 'content-type: application/json' \
  -d '{"method":"app.state","params":{}}'
# ok: True appHome: .../alpha3/.scratch/host-home
```

Os arquivos reais não são tocados pelos specs. A captura (`stat -f '%N %m %z'` + `shasum -a 256`)
foi feita antes e depois das rodadas; o diff ficou vazio em todas:

- antes de qualquer execução → depois de `debugger` (one-click), `ui-fold`, `execution-gates`,
  `gates` e `environment`: **UNCHANGED**. Estado capturado: `trusted.json` mtime `1793568932`,
  15306 bytes, SHA-256 `c0a9918b…dacdd9`; `recent.json` mtime `1793568993`, 4205 bytes, SHA-256
  `dba03a04…3eee7`.
- `debugger -g "one click"` + `environment.spec.ts`, depois `trust-prompt.spec.ts`, depois
  `welcome.spec.ts`: cada uma comparada antes/depois, **UNCHANGED**.

Achado fora do escopo do ticket (pendência): `cargo test -p openridr-host` escreve nos mesmos
arquivos reais. As duas rodadas de fechamento (uma normal e uma `--no-fail-fast`) gravaram
projetos temporários de `host.rs` (`.tmp*/Many.sln`, `Sleeper.csproj`, `Policy.csproj`, …); o
mtime final ficou `1793569994` (18:53:14 local), sem nenhuma escrita posterior das execuções E2E.
É o comportamento anterior à mudança (testes de host que não fixam `OPENRIDR_HOME` pelo processo
inteiro); nada de Rust foi alterado neste ticket.

`.gitignore` cobre os caminhos de higiene (D4) mais `.scratch/host-home/`, o diretório criado pelo
`host:http` isolado. `git check-ignore` confirmou a regra para `.scratch/host-home/trusted.json`.

## Não verificado

- App Tauri/WKWebView empacotado: a prova é pela UI no Chrome sobre a ponte HTTP de dev; o bundle
  `.app` não foi reexecutado nesta rodada.
- Windows x64 e macOS Intel: não executados; o `host:http` isolado usa expansão POSIX de shell.
