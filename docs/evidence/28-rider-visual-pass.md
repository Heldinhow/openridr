# Comparação Rider — continuação visual

Data: 29/10/2026. As referências Rider são `docs/evidence/rider-reference/` (OrderDesk, macOS ARM64). A janela nativa do OpenRidr anterior foi observada via REA com Gabagool e tamanho diferente; por isso a observação não prova equivalência pixel a pixel. As capturas de teste abaixo usam cópias temporárias de OrderDesk e Chrome em macOS ARM64.

- Janela e editor: `/tmp/openridr-rider-shell/workspace-1440x900.png`; painel inferior atravessa a largura sob a árvore e o editor. `ui-layout.spec.ts` e `shell.spec.ts` passaram 11/11 em 1600×1000, 1440×900 e 1100×700.
- Debug real: `/tmp/openridr-rider-editor/14-dap-paused.png` e `14-dap-stepped.png`; netcoredbg 3.2.0-1092 existente, restore/build reais, breakpoint, frames, locals, step into/out/over e término observados. O teste confirmou que a linha de breakpoint e a linha pausada mantêm sua cor mesmo quando ativa.
- Build, Run e Terminal: `/tmp/openridr-rider-panels/11-build-ok.png`, `11-build-failed.png`, `12-run-output.png`, `15-terminal-running.png`; processos reais. A seleção de 23 testes de Build, Run, Git, Terminal, Tests e NuGet passou 23/23. Os testes NuGet dessa seleção usam `nuget.probe`: não provam consulta real ao feed.
- Search, Settings, Run Configurations e editor: seleção de 19 testes passou 19/19; Search foi novamente testado 6/6 após corrigir a prévia anterior durante leitura demorada. O teste de atraso controlado falhou antes da correção e passou depois.
- O teste de trust e processos passou com uma decisão pendente aberta automaticamente, seguido de término observado. O resumo pode ser reaberto quando não há decisão.
- `bun run tauri build --debug --bundles app --no-sign` gerou bundle macOS; uma cópia com identificador isolado foi aberta e capturada por REA (`ev_c268ef5f09b3277ef86a9dec1b3ec759175c49422e607ce9c074692b00dd477d`). A árvore de acessibilidade não expôs o conteúdo WebKit (`ev_803a70bfbc0bac143322bc0533ffbee409d8400d588727ed6a855d87433daa7a`), e o controle da janela expirou; apenas a tela inicial e detecção do SDK foram observadas no app reconstruído.

As capturas e logs temporários não são versionados. A suíte completa e a consulta real de NuGet devem ser registradas separadamente após sua execução. Windows e macOS Intel não foram verificados.

Durante as primeiras rodadas, alguns specs ignoraram `OPENRIDR_SCREENSHOT_DIR` e sobrescreveram capturas preexistentes modificadas em `docs/evidence/screenshots/`. Não havia cópia conhecida de seus estados anteriores. Os specs foram ajustados para respeitar as variáveis de saída antes da suíte completa.

## Consulta real NuGet e terminal

A consulta sem probe em cópia temporária de OrderDesk passou com restore real e SDK 8.0.408: 3 pacotes e 3 desatualizados, com a fonte habilitada `https://api.nuget.org/v3/index.json` e aviso de que a origem por pacote não é reportada. Captura: `/tmp/openridr-rider-nuget-real.png`; execução: `.scratch/nuget-real.spec.ts`. A fonte antes falhava porque o comando pedia um formato JSON não suportado; a [documentação da Microsoft](https://learn.microsoft.com/dotnet/core/tools/dotnet-nuget-list-source) lista `short` e `detailed`. A suíte do parser NuGet passou 13/13.

O terminal em `/tmp/openridr-rider-terminal/15-terminal-running.png` mostra a saída real após agrupar controles e metadados. O teste de altura falhou antes do ajuste (29,8 px), passou depois (mínimo de 50 px) e também exercitou comando, Ctrl+C e fechamento. A suíte completa anterior passou 101/101; após a alteração de NuGet e terminal, os quatro specs afetados passaram 15/15.
