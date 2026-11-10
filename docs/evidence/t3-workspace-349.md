# Workspace inspirado no T3 — #349

Base de implementação: `a0fefe6efe30b0bf8a296f5b855b217398b6d34e`. A direção foi autorizada na #348; a aprovação visual desta fatia ainda cabe ao mantenedor.

## Referência e comparação

A captura pública [T3 Code app-desktop](https://t3.codes/_astro/app-desktop.BHuzLVLF.webp) foi observada em `/tmp/openridr-t3-reference.webp`; a revisão que ela retrata não está identificada. Foi consultado também `apps/web/src/index.css` de `pingdotgg/t3code@53bcb9af7a37dd0b4df11a7d6fa0106e9e5b7356`, como referência, não como arquivo copiado. Sem app T3 em execução, estilos computados e estados interativos não foram observados.

Na captura de referência, uma navegação escura contínua ocupa a lateral esquerda, separada por uma linha fina da área principal preta; o conteúdo, abas/controles e superfície secundária ganham distinção por contraste discreto. Na [captura do OpenRidr em 1600×1000](t3-workspace-349/restricted-1600.webp), a árvore é contínua ao lado do editor **e** da tool window, em vez de terminar acima dela; abas, toolbar, sintaxe, gutter e painel usam o mesmo vocabulário escuro. [1100×700](t3-workspace-349/restricted-1100.webp) mostra a janela menor. São capturas reais do Chromium web em modo restrito, não mockups. O T3 observado mostra conversas, não código ou build: editor, execução, debug, rails, status e tool window são adaptações necessárias de IDE, não equivalentes nem promessa pixel a pixel. Marca e funções do T3 não foram reutilizadas; ícones e fontes já existentes continuam sujeitos às atribuições existentes.

## Verificação

- `bun install --frozen-lockfile`; `bun run build`: passaram. `bun test tests/unit`: 156 passaram.
- TDD pelo fluxo público de workspace: o cenário de layout 1100×700 falhou antes da mudança (árvore terminava em y=432, tool window em y=664); após a mudança, três tamanhos passaram. A rodada posterior de `ui-layout.spec.ts` e o cenário de aparência de `settings.spec.ts`: **8 passaram**. Exercita abertura restricted e trusted, arquivo, redimensionamento por mouse/teclado, tabs, truncamento, densidade compact/comfortable, cancelamento e persistência.
- `qa4-eval-failed.spec.ts`: **1 passou**, incluindo erro de avaliação em Problems. O cenário de loading em `build.spec.ts` observou mensagem e botão desabilitado, mas **falhou** na comparação exata do `title`: o texto real inclui o nome da ação como prefixo; não conta como passe completo.
- Rodada completa combinada de layout/settings: **10 passaram, 2 falharam**. Os dois de Settings enviam atalhos `Meta` de macOS no Chromium Linux (diálogo por ⌘, não abre; atalho capturado não inclui ⌘). Não foram considerados passes.
- `build.spec.ts` cenário de build real falhou ao tentar salvar com `Meta+s` no Linux; o build com erro e sua navegação não foram verificados. Outro cenário de loading falhou conforme acima. Não se afirma que essas falhas sejam causadas ou resolvidas por este diff.
- Doctor: SDK 8.0.408 presente, Chromium via `PLAYWRIGHT_CHANNEL=chromium`, TMPDIR curto em filesystem do projeto; bundle macOS e agent-desktop indisponíveis. O detector Impeccable retornou `[]` para os arquivos alterados.

**Não verificado:** app Tauri macOS ARM64 e WebKit, atalhos nativos, Windows e macOS Intel; diagnóstico/breakpoint em sessão de debug real; inspeção sistemática de contraste em todos os estados. A captura trusted contém caminhos locais e ficou fora do repositório. A aprovação visual do mantenedor e expansão #350 permanecem pendentes.
