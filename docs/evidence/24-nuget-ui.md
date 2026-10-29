# Ticket 24 — NuGet sem pacotes e tabela

Data: 29/10/2026. Ambiente: macOS arm64; Chrome headed, `OPENRIDR_E2E_SLOW_MO=300`.

## Verificações executadas

- `cargo test -p openridr-host nuget::tests` — 13 passaram, 0 falharam. Inclui framework omitido para projeto sem pacotes, projeto/frameworks inválidos, multi-TFM, merge de outdated, CPM e plano de update.
- `bun run build` — TypeScript e Vite concluídos. Vite emitiu o aviso já conhecido de chunk acima de 500 kB.
- `PATH="$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT="$HOME/.dotnet" DOTNET_CLI_UI_LANGUAGE=en OPENRIDR_E2E_SLOW_MO=300 bunx playwright test tests/e2e/nuget.spec.ts --headed` — 4 passaram: bloqueio pré-trust, lista com pacotes, projeto sem pacotes e CPM.
- O caso sem pacotes e falha de consulta de fontes usa `nuget.probe` isolado; não executa dotnet, update nem restore. Fontes desconhecidas aparecem como nota neutra enquanto o resultado de pacotes permanece independente.

## Limitações

- Não foi executado `dotnet list package` contra a fixture nem consultada rede real nesta fatia; o comportamento real do CLI está coberto por fixture JSON do parser e o E2E usa probe determinístico.
- Windows e macOS Intel não verificados.
