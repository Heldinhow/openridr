# Ticket 21 — indisponibilidade visual e planos de ferramentas

## Ambiente e comandos

- macOS arm64, Chrome headed, workspace temporário copiado da fixture `OrderDesk`.
- `bun run build` — passou (`tsc --noEmit` + Vite build); Vite reporta o aviso já existente de chunk acima de 500 kB.
- `PATH="$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT="$HOME/.dotnet" DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/csharp-ls.spec.ts -g "disabled trust and install plan" --headed` — passou 1/1.
- Mesmo prefixo de ambiente com `bunx playwright test tests/e2e/debugger.spec.ts -g "macOS arm64 requires the experimental flag" --headed` — passou 1/1.
- Mesmo prefixo de ambiente com `bunx playwright test tests/e2e/run-configs.spec.ts -g "saved run configurations drive build and run" --headed` — passou 1/1.

## Evidência observada

- Trust desabilitado mantém o motivo visível e associado por `aria-describedby`; o estilo computado não mantém a cor de ação.
- O plano csharp-ls está aberto ao ser carregado e mostra versão, origem, plataforma, destino, staging, comando, cwd, permissões, integridade e estado do destino antes do botão de confirmação.
- No fluxo netcoredbg ARM64, flag OFF mostra plano bloqueado; o opt-in explicitamente confirmado replaneja para flag ON. Os campos do plano e a ação ficam no disclosure; diretório de instalação permaneceu ausente. Não houve download nem instalação.
- Os testids existentes foram preservados. A faixa de workspace tools continua com rolagem interna e limite de altura de 30 vh.

## Limitações

- E2E focado foi headed, mas não configurado com `slowMo:300`; não foram preservadas capturas novas. O spec existente escreveu duas capturas de debugger durante a execução; como já estavam limpas antes do teste, foram restauradas e ficaram fora da alteração.
- Não executados: suíte E2E completa, fluxo Tauri nativo, leitor de tela/WCAG formal, macOS Intel e Windows x64. O spec de run-config confirmou campos derivados desabilitados e o cursor `not-allowed` no input; não comparou o estilo computado de cada tipo de controle individualmente.
