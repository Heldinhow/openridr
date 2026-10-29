# Aproximação visual ao Rider — plano de implementação

**Objetivo:** aproximar a experiência visual do OpenRidr ao Rider, preservando as funções existentes e verificando cada área com estados equivalentes.

**Abordagem:** usar as capturas existentes e observações REA das janelas nativas como referência; implementar em fatias independentes, sem novas dependências e mantendo os contratos de testid. Registrar resultados e limites em `docs/implementation-notes.md` e no ticket da fatia. Não presumir suporte além de macOS ARM64.

**Tecnologia:** React/TypeScript, CSS, Tauri 2, host Rust e Playwright.

## Preparação

- [ ] Preservar o trabalho não commitado e o índice do usuário; o commit `ec49ae8` deixou quatro caminhos como alterações inversas no índice original. Não incluir essas diferenças em novos commits.
- [ ] Comparar cada tela com o mesmo projeto, estado, tema e tamanho de janela quando possível. Usar `docs/evidence/rider-reference/` e `docs/evidence/rider-compare/`; marcar comparações de estados diferentes como parciais.
- [ ] Verificar as decisões em `docs/decisions.md`, o ticket relevante e os critérios em `docs/SPEC-MVP-RIDER.md` antes de cada alteração.

## 1. Janela principal e Solution Explorer

**Arquivos:** `src/WorkspaceView.tsx`, `src/styles.css`, `src/rider-chrome.css`, `src/shell/toolWindows.ts`, `tests/e2e/ui-layout.spec.ts`, `tests/e2e/evaluation.spec.ts`.

**Em andamento:** o título visível passou para “Solution”; a região mantém o nome acessível anterior. Referências e pacotes avaliados ficam recolhidos sob “Dependencies”, acessível pelo teclado. `evaluation.spec.ts`, `ui-layout.spec.ts` e `shell.spec.ts` passaram 15/15, e `bun run check` passou. Captura no Chrome com a solução OrderDesk avaliada em `/tmp/openridr-evaluation-tree.png`; a comparação com o Tauri reconstruído e os demais ajustes ainda estão pendentes.

- [ ] Capturar Rider e OpenRidr com solução e arquivo equivalentes; observar toolbar, barras laterais, árvore, faixa de ferramentas e rodapé.
- [ ] Reduzir o peso visual de Workspace tools e dos controles de avaliação, mantendo visíveis os estados de trust, avaliação, restore e erro e mantendo acessíveis as ações existentes.
- [ ] Ajustar hierarquia, ícones, seleção e espaçamento da árvore usando a referência; manter os paths, seleção e expansão funcionais.
- [x] Confirmar que o painel inferior inicia fechado em uma instalação nova e que a preferência existente permanece ao recarregar; não substituir o layout salvo.
- [ ] Executar `PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/ui-layout.spec.ts tests/e2e/evaluation.spec.ts`; inspecionar as capturas nas três dimensões e rodar `bun run check`.

## 2. Editor

**Arquivos:** `src/CodeEditor.tsx`, `src/WorkspaceView.tsx`, `src/styles.css`, `src/rider-chrome.css`, `tests/e2e/editor-overflow.spec.ts`, `tests/e2e/code.spec.ts`, `tests/e2e/debugger.spec.ts`.

- [ ] Comparar o mesmo arquivo C# aberto nas duas aplicações: abas, margens, gutter, tipografia, sintaxe, seleção, alterações e breakpoint.
- [ ] Corrigir diferenças comprovadas sem alterar edição, salvamento, foco ou contratos de teste.
- [ ] Verificar arquivo longo, seleção, breakpoint e navegação; executar `bunx playwright test tests/e2e/editor-overflow.spec.ts tests/e2e/code.spec.ts` com as variáveis .NET documentadas e o teste focado de breakpoint quando o adaptador estiver disponível. Registrar skips como não verificados.

## 3. Painéis de trabalho

**Arquivos:** `src/shell/BottomToolWindow.tsx`, `src/WorkspaceView.tsx`, `src/DebugPanel.tsx`, `src/git.tsx`, `src/styles.css` e specs correspondentes em `tests/e2e/`.

- [ ] Capturar Build e Run após execução real; ajustar hierarquia e logs e executar os specs `build.spec.ts` e `run.spec.ts`.
- [ ] Capturar Debug pausado com variáveis reais; ajustar disposição e controles e executar o spec `debugger.spec.ts` com netcoredbg real, se disponível.
- [ ] Capturar Tests com resultados reais e Terminal com comando real; ajustar estados e executar `tests.spec.ts`, `testsuite.spec.ts` e `terminal.spec.ts` conforme as ferramentas disponíveis.
- [ ] Capturar Git com repositório, alterações e histórico reais; ajustar Commit, Log e diff e executar `git.spec.ts`.
- [x] Verificar Problems e NuGet com dados reais; Problems foi exercitado por diagnósticos de Build e csharp-ls, e NuGet por consulta ao SDK 8.0.408 e feed em cópia temporária. `nuget.spec.ts` usa probe e passou 4/4; a consulta real passou 1/1. A localização configurada não prova a origem individual dos pacotes.

## 4. Busca, configurações e diálogos

**Arquivos:** `src/WorkspaceView.tsx`, `src/runConfigs.tsx`, `src/styles.css`, `tests/e2e/search.spec.ts`, `tests/e2e/settings.spec.ts`, `tests/e2e/run-configs.spec.ts`.

- [x] Comparar Search Everywhere com resultados e verificar se preview é possível sem inventar conteúdo ou ignorar erros do host. A prévia já existia; a troca de resultado agora limpa a prévia anterior durante leitura e mostra carregamento, com teste de atraso controlado.
- [ ] Comparar Settings/Keymap e Run Configurations com valores reais; ajustar somente controles que correspondam a recursos existentes.
- [ ] Executar os três specs listados e testar navegação por teclado, estado vazio e erros relevantes.

## 5. Integração e entrega

- [ ] Reconstruir e abrir o Tauri no macOS ARM64; comparar novamente os estados equivalentes e verificar os fluxos modificados na janela nativa.
- [ ] Executar `bun run check` e, sem outros servidores usando as portas, `bun run test:e2e` com as variáveis .NET documentadas; relatar falhas e skips sem contá-los como passes.
- [ ] Atualizar tickets, notas e evidências; criar um commit convencional por fatia usando caminhos explícitos e sem incluir alterações preexistentes. Não fazer push.
