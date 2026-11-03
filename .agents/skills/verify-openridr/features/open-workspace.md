# 1. Abrir solution / workspace

## Sub-features

- Abrir por caminho absoluto (`.sln`, `.slnx`, `.csproj`, pasta); a pasta resolve para o único `.sln`/projeto dentro, ou pede escolha com 2+ (`workspace-candidates`).
- Caminho relativo com `baseDir`; extensão em maiúsculas; caminho inexistente/recusado mantém o workspace aberto.
- Lista de recentes na tela inicial (só entram projetos já decididos no Trust); busca; menu ⋯ por linha.
- Restaura as abas salvas da sessão ao reabrir (`session.read`/`session.write`; desligado no host dos E2E).

## How to get to it (user POV)

Tela inicial ("Welcome to OpenRidr"): campo "Workspace path", botão "Open path"; botão "Open" (seletor de pasta); ou linha dos recentes. Depois aparece o prompt de Trust (feature 3) e então o workspace.

## Driving it

- **A**: `bunx playwright test tests/e2e/open-external.spec.ts tests/e2e/welcome.spec.ts` (o primeiro é só host via `POST /call`, rápido; o segundo usa a UI).
- **B**: `web.sh up`; `browser.open http://127.0.0.1:1520/`; `browser.type` em `[data-testid="workspace-path-input"]` com o `.sln` impresso, `submit:true`. Estado final: `curl ... '{"method":"app.state","params":{}}'` → `workspace.kind` = `solution`, `workspace.path` = caminho canônico (`/private/tmp/...` no macOS), `workspace.candidates` vazio.
- **C**: `desktop.sh up` já passa o `.sln` em argv; a UI deve mostrar o prompt de Trust ou o workspace (ver feature 4).

Prova de que funciona: `workspace.kind/path` corretos no host **e** a UI mostra o nome (`OrderDesk.sln`) no topo e o prompt/árvore; recusa deixa o workspace anterior intacto.

## Gotchas

- Caminhos do macOS são canonicalizados: `/tmp/x` vira `/private/tmp/x`. Compare com `realpath`.
- Abrir um `.csproj` isolado vira `kind: project`; abrir **pasta** com um `.csproj` usa o caminho do projeto como chave de trust.
- O host dos E2E roda com `OPENRIDR_TRUST_STORE=off` e `OPENRIDR_SESSION_STORE=off`: todo open pergunta Trust e nada é restaurado. `web.sh` roda com os stores ligados.
- Não abra `fixtures/` direto: restore/build criam `bin/obj`. Use a cópia.
