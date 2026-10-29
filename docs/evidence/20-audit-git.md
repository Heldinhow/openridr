# Ticket 20 — auditoria visual Git

## Ambiente e verificações

- Ambiente: macOS 27.2 arm64, Chrome headed; suíte Playwright com `slowMo: 300`.
- `bun run check` — passou (`tsc --noEmit` + `cargo check --workspace --all-targets`).
- `bun run build` — passou (`tsc --noEmit` + Vite build); aviso preexistente do chunk JavaScript acima de 500 kB.
- `PATH="$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT="$HOME/.dotnet" DOTNET_CLI_UI_LANGUAGE=en bunx playwright test --config=.playwright-headed.config.ts tests/e2e/git.spec.ts` — 4/4 passaram. A config headed temporária não foi mantida.
- Os testes usam repositórios temporários, verificam diff longo com overflow horizontal, caminho extenso no Commit/log, metadados, feedback único e commit seletivo/amend. O conteúdo do diff é texto cru do host.

## Capturas

Capturas novas foram gravadas fora do repositório para não alterar nem versionar screenshots existentes:

- `/private/var/folders/n4/bjx54c1d3_d7438735kj02sr0000gn/T/opencode/ticket20-git-diff.png`
- `/private/var/folders/n4/bjx54c1d3_d7438735kj02sr0000gn/T/opencode/ticket20-git-log.png`

## Limitações

- A suíte E2E completa não foi rodada; apenas `tests/e2e/git.spec.ts` (4/4) e seu cenário adicional headed.
- Nenhuma operação remota (pull/push) foi executada contra um remote real; os cenários existentes só comprovam gates/confirmação em repositório temporário.
- Windows não executado; esta auditoria altera somente apresentação React/CSS.
