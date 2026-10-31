# Ticket 25 — auditoria de Settings e Keymap

## Ambiente e comandos

- macOS arm64, Chrome headed, viewport 1100×700; `OPENRIDR_E2E_SLOW_MO=300`.
- `bun run build` — passou (`tsc --noEmit` + Vite build; aviso existente de chunk >500 kB).
- `bun run check` — passou (`tsc --noEmit` + `cargo check --workspace --all-targets`).
- `cargo test -j1 -p openridr-host git::tests::info_reports_the_git_that_path_would_run --lib` — passou 1/1.
- `PATH="$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT="$HOME/.dotnet" DOTNET_CLI_UI_LANGUAGE=en OPENRIDR_E2E_SLOW_MO=300 bunx playwright test tests/e2e/settings.spec.ts --headed` — passou 5/5.
- Mesmo prefixo de ambiente com `bunx playwright test tests/e2e/actions.spec.ts -g "keymap captures a key" --headed` — passou 1/1.

## Evidência observada

- Keymap apresenta cabeçalho de tabela, grupo/atalho e ações compactas; remap, conflito, reset individual e persistência passaram na UI real.
- Appearance mostra Dark como estado e explica a indisponibilidade de Light sem seletor.
- Settings e Keymap não exibem o caminho interno de `settings.json`.
- Em Restricted mode, VCS mostra o executável resolvido no PATH e explica que a consulta Git requer trust; chamada direta a `git.info` devolve `trust_required`. Em Trusted, caminho e versão aparecem.
- Layout 1100×700 manteve a tabela dentro do diálogo; controles de remap/reset têm nomes acessíveis.
- Nenhuma captura foi criada ou incluída.

## Limitações

- Suíte E2E completa, Tauri nativo, leitor de tela/WCAG formal, Windows x64 e macOS Intel não verificados. O teste focado confirma nomes acessíveis/foco do controle, não auditoria formal de acessibilidade.
