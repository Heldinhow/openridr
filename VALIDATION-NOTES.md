# Validação — pós-review

Arquivo ausente na worktree recebida; instruções de portas e isolamento vieram do handoff. Base `5031a5a`, correção `9aa95d3`. O handoff reportou 51/51 na base; não substituir esse histórico por um resultado posterior.

## Standards

- Inicial: **1 hard P2 + 1 smell P3**. P2 corrigido restaurando os titles condicionais originais, sem mudar gates. P3 explicitamente adiado: os dois algoritmos locais curtos não justificam abstração adicional nesta correção.
- Final: **0 hard pendentes; 1 smell aceito/adiado**. Testes existentes ampliados sem remover asserts.

## Spec

- Inicial: **0 findings**; escopo funcional preservado. A correção restaura os motivos legíveis exigidos para controles desabilitados.
- Sem alteração de CSS/layout; o único efeito visual novo são tooltips restaurados.

## Execução isolada

- Diretório desta execução: `/private/var/folders/n4/bjx54c1d3_d7438735kj02sr0000gn/T/opencode/openridr-review-9yX7P2`.
- Backup anterior aos testes: `before-tests.tar.gz` contém fixtures, `docs/evidence` e `test-results`, inclusive arquivos ignorados. Nenhum processo alheio encerrado; portas 15420/15317 estavam livres antes da execução. Playwright inicia e encerra seus próprios servidores, com `reuseExistingServer: false`.
- `OPENRIDR_E2E_UI_PORT=15420 OPENRIDR_E2E_HOST_PORT=15317 OPENRIDR_E2E_HOME=<diretório acima>/home bun run test:e2e`.
- `bun run build`: **PASS** (TypeScript + Vite); warning já existente de chunk JS >500 kB.
- Full E2E: **51 passed (5.1m)**, zero falhas e zero skips. Log: `<diretório acima>/full-e2e.log`.
- Standalone posterior: **2 passed (8.2s)**. Log: `<diretório acima>/standalone.log`. **6/6 imagens** geradas em `standalone-screenshots/` e lidas individualmente: welcome, workspace vazio e editor, em 1440×900 e 1100×700. Sem cortes/overflow horizontal ou regressão aparente. Os asserts também cobrem scroll local, resize e navegação por teclado.
- Build repetido após a suíte: **PASS**; log em `<diretório acima>/build.log`.

Após a suíte completa, a captura standalone usa o mesmo isolamento e `bun run test:e2e tests/e2e/ui-layout.spec.ts`, com `OPENRIDR_SCREENSHOT_DIR=<diretório acima>/standalone-screenshots`. Os dois testes capturam welcome, workspace vazio e workspace com editor em 1440×900 e 1100×700: seis imagens. Não executar simultaneamente com a suíte completa.

Evidências pós-full preservadas em `after-full-e2e.tar.gz`. Restaurados **57 arquivos pré-existentes de evidência**, listados em `restored-evidence-files.txt`: validado que cada original do backup era idêntico ao HEAD e que cada arquivo atual ainda era idêntico ao arquivo pós-suite, antes de restaurar. Fixtures versionadas sem diff. Nenhum arquivo desconhecido descartado; home temporário, backups, logs e seis screenshots permanecem disponíveis fora do checkout.

Os processos próprios observados na suíte foram 29884 (Playwright), 29995 (host 15317) e 30036 (Vite). Após o standalone, todos estavam encerrados e `lsof` não encontrou listeners nas portas 15420/15317. O teardown dos runners bastou; nenhum processo alheio foi encerrado.

Limites: não foi repetida validação desktop nativa nem teste manual com leitor de tela. O teste verifica os titles originais para os dois controles nos estados resolvido, trust revogado e SDK ausente. P3 continua adiado, não corrigido implicitamente.

## Commits

Lista histórica completa desta série em `REDESIGN-REPORT.md`; base `5031a5a`, correção `9aa95d3`. O commit posterior de documentação registra os resultados, sem reescrever os commits anteriores.
