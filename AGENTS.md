# AGENTS.md — OpenRidr

IDE C# inspirada no Rider: Tauri 2 + Rust (host) + React/TS (UI). Alvo verificado hoje: macOS ARM64. Windows e macOS Intel não foram executados — não os trate como suportados sem evidência.

## Leia antes de mexer

- `docs/SPEC.md` — contrato do produto (P0).
- `docs/SPEC-MVP-RIDER.md` e `docs/tickets-mvp/` — fatias F1–F9 e critérios de aceite.
- `docs/decisions.md` — decisões incrementais; siga-as e registre uma nova em vez de reabrir uma antiga.
- `docs/implementation-notes.md` — como cada ticket foi feito e o que não foi verificado.
- `docs/REVIEW-MVP-RIDER.md` e `docs/RETRO-MVP-RIDER.md` — dívida e lições conhecidas.

## Estrutura

```text
src/                 React/TS da UI (editor, painéis, git, testes, terminal)
crates/host/         lógica e estado do host, sem dependência de Tauri
crates/host-http/    ponte HTTP só de dev/E2E (127.0.0.1, nunca distribuída)
src-tauri/           janela Tauri; chama o mesmo dispatch do host
tests/e2e/           specs Playwright contra host real + fixture real
fixtures/            soluções C# de teste (OrderDesk, MissingSdk, ...)
docs/                SPECs, decisões, tickets, notas, evidência
```

A UI fala com o host por `dispatch`; não duplique lógica de host no React.

## Comandos

```sh
bun install
bun run dev            # Vite (UI em 1420)
bun run tauri dev      # app desktop completo
bun run build          # tsc --noEmit + vite build
bun run check          # tsc --noEmit + cargo check --workspace --all-targets
bun run hooks          # uma vez por clone: aponta core.hooksPath para .githooks
bun run test:e2e       # Playwright; não rode junto com dev (portas 1420/4317)
bun run host:http      # ponte HTTP de dev na 4317
```

`check` não inclui `cargo fmt --check` nem `clippy -D warnings`: o código atual não passa em nenhum dos dois. Não reformate o repo inteiro dentro de um ticket; formate apenas o que tocar até essa dívida ser paga.

## .NET SDK 8.0.408

- Cada fixture fixa o SDK no próprio `global.json` (`8.0.408`, `rollForward: disable`). Não altere o pin nem o roll-forward.
- O host de análise usa o maior SDK estável 10.x detectado; o projeto usa o SDK do `global.json`.
- O app apenas detecta SDKs (PATH, `~/.dotnet`, `~/Library/Application Support/OpenRidr/sdks`). Nunca instala nem edita `global.json`.
- Qualquer raiz detectada pode satisfazer o pin: `~/.dotnet` é tão válido quanto `OpenRidr/sdks/8.0.408`. Specs e código não podem exigir o caminho gerenciado; o contrato é a versão resolvida (e o `sdkRoot` absoluto).
- Ferramentas vão para `~/Library/Application Support/OpenRidr/tools/<nome>/<versão>`; nunca global.

## E2E

- `bun run test:e2e` sobe o host e o Vite sozinho, na 1420 (UI) e 4317 (host). Portas alternativas: `OPENRIDR_E2E_UI_PORT` e `OPENRIDR_E2E_HOST_PORT`.
- No Mac com o SDK em `~/.dotnet`, rode com `PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet DOTNET_CLI_UI_LANGUAGE=en bun run test:e2e` (ou `bunx playwright test tests/e2e/<spec>.ts` para um spec).
- `playwright.config.ts` usa `channel: "chrome"` por padrão. Sem Chrome (ex.: VPS Linux), rode `PLAYWRIGHT_CHANNEL=chromium bun run test:e2e`.
- Os specs usam `fixtures/OrderDesk` de verdade e restauram o arquivo; workers = 1 porque host e arquivos são estado compartilhado.
- Specs que dependem de dotnet/csharp-ls/netcoredbg real fazem `skip` quando o ambiente não tem a ferramenta. Skip não é passe: leia o motivo e registre "não verificado".
- Não rode a suíte inteira por reflexo durante outra execução; as portas e a fixture são compartilhadas.

## Regras

- **`data-testid` são contrato**: não renomeie nem remova os existentes; cada fatia adiciona os seus e mantém `role="tab"` no cabeçalho inferior.
- **Zero dependência nova** de UI/runtime sem decisão registrada e aprovação explícita.
- Um componente por arquivo; comentários em inglês; uma função de erro por fronteira (use `messageOf`/`requireAction` em vez de repetir mapeamento).
- **Prova real, não inventada**: o que o ambiente não permitiu executar vai como "não verificado" no `implementation-notes.md` e no ticket.
- **Commit**: um por ticket/fatia, mensagem convencional (`feat(git): ...`, `fix(search): ...`, `docs: ...`, `chore: ...`), com código, testes e checklist atualizado.
- **Stage explícito**: commite caminhos com `git commit --only -- <paths>`; nunca use `git add -A` nem altere o index do usuário. Deixe `dist/`, `target/`, `test-results/` e afins fora (ver `.gitignore`).
- **Sem push, remote ou PR** a partir de sessão de agente. O hook de pre-commit roda `bun run check`; não use `--no-verify`.
