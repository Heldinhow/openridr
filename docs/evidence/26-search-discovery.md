# Ticket 26 — Busca: contagem e descoberta de tipos

Ambiente: macOS 27.2 arm64; Chrome via Playwright headed, `OPENRIDR_E2E_SLOW_MO=300`.

## Verificação

- `bun run build` — passou (`tsc --noEmit` + Vite build).
- `OPENRIDR_E2E_SLOW_MO=300 DOTNET_ROOT="$HOME/Library/Application Support/OpenRidr/sdks/8.0.408" PATH="$HOME/Library/Application Support/OpenRidr/sdks/8.0.408:$HOME/.dotnet/tools:$PATH" bunx playwright test tests/e2e/search.spec.ts --headed` — 5 passed.
- A nova E2E controla a resposta `lsp.workspaceSymbol` para provar dois resultados visíveis, truncamento declarado com 523 símbolos totais, e sugestão Classes para um tipo encontrado sem membros. Não é prova da cobertura/truncamento do csharp-ls real.
- A E2E existente continua verificando `Analysis not ready` sem trust e `No results` em buscas reais sem correspondência. Estados de erro/retry não foram alterados.

Não executada: suíte E2E completa; LSP/csharp-ls real para busca de símbolos e limites. Sem capturas novas.
