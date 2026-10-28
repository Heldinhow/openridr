# OpenRidr — especificação do MVP C#/.NET e das sete telas aprovadas

Status: ready-for-agent
Category: enhancement
Data: 27/10/2026
Fase: especificação para revisão de Helder; execução não autorizada.

As telas foram aprovadas pelo usuário para serem convertidas nesta spec. Isso não constitui aprovação da spec, autorização para criar tickets, implementar, instalar ferramentas ou fazer commits. O status de triagem acima não substitui esse ponto de parada.

## Problem Statement

Quem desenvolve aplicações C#/.NET em macOS ou Windows precisa de um ambiente aberto que feche o ciclo abrir solução, entender projetos, editar, compilar, executar e testar sem montar manualmente integrações desconectadas. Um editor com sintaxe C# ou execução de comandos genéricos não resolve seleção de SDK, avaliação MSBuild, contexto de execução, descoberta de testes ou lançamento correto do debugger.

OpenRidr deve oferecer esse ciclo pequeno e confiável, com identidade própria e compatibilidade de atalhos IntelliJ/Rider, sem reutilizar código, componentes ou marcas proprietárias. O resultado esperado não é paridade com uma IDE comercial: é permitir trabalhar em uma solução SDK-style com console, biblioteca e testes, entendendo exatamente o que está pronto, incompleto, bloqueado ou não verificado.

Há riscos concretos que a interface não pode esconder: repositórios executam código durante avaliação e tarefas; o SDK do host de análise difere do SDK do projeto; diagnósticos podem representar revisões diferentes; um processo encerrado pode não informar exit code; e disponibilidade de um adaptador não prova compatibilidade de debug por arquitetura.

O diretório local do projeto foi inspecionado e está vazio, sem checkout Git, instruções locais, glossário, ADRs, módulos ou testes a reutilizar. OpenIdea foi pesquisado remotamente, não está integrado localmente. Suas capacidades e seams são candidatos de reaproveitamento sujeitos a auditoria, não funcionalidades existentes do OpenRidr.

## Solution

Entregar um aplicativo desktop aberto para **macOS e Windows juntos no P0**, apoiado na base/inspiração OpenIdea, com sete telas coerentes: confiança/toolchain, editor/Build, Solution Explorer, Problems, Debug, Terminal e Test Runner. A ordem de prioridade herdada permanece P0.1 a P0.7; dependências precedem consumidores, sem transformar essa ordem em tickets nesta etapa.

### Recorte funcional e compatibilidade

- Uma solução SDK-style por vez, com console, biblioteca e projeto de testes; abertura direta de projeto também deve ser possível. Havendo múltiplas soluções ou contextos ambíguos, pedir seleção em vez de escolher silenciosamente.
- .NET 8+ é a linha suportada; net6 é best effort, sem equivalência de garantia. SDKs, runtimes, referências e demais pré-requisitos de cada alvo precisam estar disponíveis. Validar net8 não prova todos os TFMs 8+.
- Análise com csharp-ls 0.28.0 hospedado em SDK .NET 10. SDK do host, SDK do projeto, regras de global.json e TFM são informações distintas.
- Debug básico com netcoredbg 3.2.0, condicionado a asset compatível verificado e prova DAP. macOS ARM64 permanece experimental, flag OFF por padrão e não bloqueante de release. Ausência de asset Mac Intel 3.2 deixa Debug indisponível nessa arquitetura, não autoriza baixar outra versão silenciosamente.
- Test Runner VSTest genérico no P0, incluindo xUnit pelo adapter já configurado no projeto. Não é exclusivo de xUnit nem promessa de todos os frameworks/adapters. MTP não suportado neste recorte.

### P0.1 — Confiança, SDKs e instalação confirmada

Ao abrir um workspace, mostrar caminho completo e `Restricted mode`, com confiança desmarcada. É permitido ler texto de solução, projeto e global.json e inventariar ferramentas globais em contexto neutro. Isso não autoriza avaliação MSBuild, restore, csharp-ls, tarefas, build, run, debug, runner ou PTY no repositório.

`Open text only` mantém edição sem execução. `Trust and inspect project` autoriza a inspeção executável do workspace: resolver SDK do projeto, avaliar MSBuild, solicitar restore explicitamente e carregar análise quando seus pré-requisitos estiverem satisfeitos. Não deixar restore implícito de build/run/test contornar essa solicitação. Erros preservam acesso à edição de texto.

Mostrar host de análise ausente separadamente de SDK do projeto ausente. SDK 10 não substitui o SDK fixado por global.json; não reescrever esse arquivo nem migrar o projeto. SDK ausente recebe instruções oficiais e nova detecção; não há instalador de SDK de sistema autorizado por esta spec.

O app instala ferramentas após plano e confirmação próprios. O plano informa ferramenta, versão exata, fonte/URL, OS/arquitetura, destino absoluto fora do repositório, operação/comando e permissões. csharp-ls usa instalação local de ferramenta via NuGet.org; netcoredbg usa asset oficial Samsung da versão aprovada. Sem asset compatível verificado, bloquear instalação. Não instalar ferramenta globalmente nem pedir elevação implicitamente.

Estados visíveis: plano pendente, downloading, installing, verifying, ready, cancelled e failed, com cancelamento, log e retry explícitos. Informar progresso apenas quando a fonte o fornece. Antes de declarar sucesso, verificar executável e versão; isso não comprova carregamento LSP nem funcionamento DAP. Registrar estado parcial após falha/cancelamento e impedir que seja tratado como instalação pronta.

Confiança no repositório, confirmação de instalação e habilitação experimental são três decisões independentes. Revogar confiança bloqueia imediatamente novas execuções; serviços do workspace devem ser encerrados, e processos ativos exigem confirmação de encerramento. Enquanto o encerramento aguarda decisão, exibir os processos remanescentes e não afirmar isolamento ou limpeza já concluídos.

### P0.2 — Uma solução e árvore avaliada

Solution Explorer mostra solução, projetos, fontes, TFM, frameworks, referências entre projetos e pacotes quando conhecidos; oculta bin/obj. Contorno textual antes de trust não se apresenta como modelo avaliado. Após trust, MSBuild é a fonte de propriedades e itens; LSP não é fonte do modelo de solução.

Projetos com erro de avaliação continuam visíveis, com causa, log, abertura do arquivo e retry. Restore incompleto sinaliza dependências/análise parciais e oferece restore explícito. Mudanças relevantes em projeto/imports devem invalidar o contexto e permitir recarga, sem usar TargetPath antigo como atual. A saída executável é obtida de TargetPath avaliado para configuração/TFM selecionados, não de uma convenção presumida.

### P0.3 — Editor e LSP essencial

Manter árvore à esquerda, editor flexível, toolbar de contexto e painel inferior com Build, Problems, Debug, Terminal e Tests. Abas e breadcrumb identificam arquivo/projeto/TFM; buffer modificado é visível. Abrir, editar e salvar texto funciona mesmo sem análise. Fechar ou substituir buffer modificado exige decisão explícita, inclusive em conflito com alteração em disco: comparar, recarregar ou preservar edições, nunca sobrescrever silenciosamente.

csharp-ls fornece completion, definição, rename entre projetos com prévia e diagnósticos. Code actions aparecem somente quando retornadas para documento/posição atuais; não prometer quick fix pelo código CS isoladamente. Edições de rename/code action respeitam revisão e conflitos; invalidar a prévia se os documentos mudarem, sem descartar buffers ou aplicar edição desatualizada.

Estados de análise incluem indisponível, carregando, pronta para operações disponíveis, cobertura parcial, aguardando diagnósticos e falha, com log e reinício explícito. Falha ao carregar projeto não deve paralisar o editor. Não exibir porcentagem ou contagem de arquivos inventadas.

Problems separa origem (`LSP · csharp-ls` ou `Build · dotnet`) de categoria (`Compiler`, `Analyzer` ou `Unknown` quando não identificada). Severidade vem da fonte/configuração. Preservar origem, escopo, execução e revisão; não mesclar snapshots silenciosamente nem presumir cobertura de analyzers.

Depois de editar, diagnóstico anterior fica obsoleto/`Awaiting diagnostics`, não resolvido por antecipação. Uma publicação vazia só sustenta `No problems reported` para o documento/revisão verificados, não para toda a solução. Build anterior permanece vinculado à sua execução e ao conteúdo salvo, independentemente do buffer atual.

### P0.4 — Startup, build/run e output

Selecionar explicitamente projeto executável de startup, configuração e TFM. Build e Run expõem contexto, comando, logs stdout/stderr, estado, cancelamento e exit code quando conhecido. Não executar biblioteca como startup nem escolher contexto ambíguo silenciosamente. Run exige build válido do contexto selecionado; build falho bloqueia launch e oferece navegação ao erro.

Saída é transmitida e preservada. Diagnósticos clicáveis exigem posição conhecida; linhas não reconhecidas continuam no log bruto. Parser conservador de texto é a proposta inicial, sem pressupor JSON universal de dotnet build. Conteúdo alterado desde o build recebe aviso; salvar/reconstruir deve ser decisão explícita antes de afirmar execução do fonte atual.

### P0.5 — Debug launch básico condicionado à plataforma

Em plataforma habilitada e com adaptador verificado, build Debug bem-sucedido antecede launch da saída gerenciada avaliada. Usar DAP de netcoredbg, não passar dotnet run como se fosse a DLL gerenciada. Breakpoints distinguem pendente e verificado pelo adaptador; gutter separa breakpoint de número de linha e indica linha atual.

Sessão mostra preparação, rodando, pausada, encerrada, cancelada ou falha. Em pausa, oferecer continue, step over, step into, stop, frames e locals do frame selecionado; rodando não mantém locals antigos como atuais. Log do adaptador e output da aplicação permanecem disponíveis. Se terminar sem exit code, mostrar `exit unknown`, nunca inventar zero nem concluir limpeza de processos apenas pelo término da sessão DAP.

No Mac ARM64, `Apple Silicon debugger: experimental · disabled` explica Debug bloqueado e Run disponível. Opt-in experimental separado não instala o adaptador nem comprova compatibilidade. A flag só pode ser retirada depois da prova DAP real descrita em Testing Decisions, incluindo async e multi-thread. O relato upstream #256 é risco não reproduzido neste trabalho, não falha universal comprovada.

No Mac Intel, sem asset compatível verificado para a versão aprovada, mostrar indisponibilidade explícita e bloquear instalação/launch de debug; não prometer cobertura universal Mac. Windows x64 também exige smoke real, não apenas presença do binário. Limitações de debug não podem desabilitar análise, Run ou testes que tenham pré-requisitos satisfeitos.

### P0.6 — Um terminal PTY básico

Após trust, abrir um único PTY com shell disponível descoberto e cwd explícitos. Oferecer entrada real, stdout/saída de terminal e Ctrl+C, inclusive para dotnet test manual como complemento ao runner. Não presumir zsh ou PowerShell instalado.

Shell ativo exige confirmação ao fechar: manter terminal ou encerrar shell e fechar. Mostrar saída do shell e código quando conhecido, sem atribuí-lo ao último comando. Retorno ao prompt não é término do shell. Preservar estado/log de encerramento e indicar falhas de processo.

### P0.7 — Test Runner VSTest básico

Selecionar projeto/TFM e descobrir testes por adapter VSTest já instalado no projeto. Descoberta e execução têm estados próprios, seleção identificável e log: idle, discovering/running, completed, empty, failed ou cancelled conforme a operação. `Run all` executa todos do escopo; `Run selected` somente os selecionados. Sem seleção válida, não executar outro conjunto silenciosamente.

Apresentar resultados passed, failed e skipped conforme reportados, além de testes pendentes/rodando quando conhecidos; cancelled pertence à operação e só a um teste quando houver informação da fonte. Cancelamento preserva resultados parciais, não converte ausentes em passed/skipped nem declara sucesso. Separar falha de infraestrutura de falha de asserção e manter escopo/revisão da execução.

Detalhes exibem mensagem, stack trace, stdout/stderr disponíveis e logs/resultados brutos. Output sem atribuição comprovada a um teste permanece no nível da execução. Navegação só quando houver posição conhecida; caso contrário mostrar indisponibilidade e oferecer log, sem adivinhar linha pelo nome do teste.

Adapter ausente recebe orientação explícita, não alteração automática de packages. Ausência de testes é diferente de descoberta falha. VSTest/TRX é proposta técnica a validar para resultados, não garantia de descoberta por TRX. Se o projeto usa MTP, não executar comandos VSTest nem convertê-lo; mostrar exatamente `Microsoft.Testing.Platform is not supported in this version. Use the terminal.`

### Apresentação e interação aprovadas

UI em inglês desde v1, documentação PT-BR. Tema escuro integral, cobre para seleção, cores semânticas sempre acompanhadas de texto/ícone; sans na interface e mono em código/logs. Preservar densidade desktop, hierarquia e estados alternativos das sete telas; a referência 1440 × 900 e proporções dos desenhos orientam composição, não garantem dimensões fixas em todos os monitores. Painéis/logs podem rolar localmente sem esconder ações essenciais ou perder foco.

Controles funcionais devem ter nome acessível, foco visível, uso por teclado e motivo legível quando desabilitados. Os botões desabilitados do protótipo são ilustração, não especificação de desabilitar todas as ações do produto. Estados de falha não dependem apenas de cor.

Keymap padrão `Keymap: IntelliJ / Rider`, sem seletor adicional, apenas compatibilidade de atalhos e identidade OpenRidr:

| Ação | macOS | Windows |
|---|---|---|
| Run | Ctrl+R | Shift+F10 |
| Debug | Ctrl+D | Shift+F9 |
| Toggle breakpoint | Cmd+F8 | Ctrl+F8 |
| Step over | F8 | F8 |
| Step into | F7 | F7 |

Não substituir Cmd por Ctrl mecanicamente; atalhos obedecem aos mesmos gates de trust, contexto e plataforma dos botões.

### Critérios de aceitação P0

Cada item abaixo é obrigação verificável futura, não evidência de execução nesta fase.

**P0.1 — Segurança/toolchain/instalação**

- [ ] **AC-01** Abrir workspace não confiável mostra caminho completo, trust desmarcado e edição textual; observar ausência de MSBuild, restore, LSP, tarefas, build/run/debug, runner e PTY do workspace.
- [ ] **AC-02** Inventário de SDKs ocorre fora do workspace; após trust, SDK do projeto respeita global.json/rollForward e aparece separado do host 10 e do TFM. SDK faltante não altera global.json; edição continua disponível.
- [ ] **AC-03** Restore requer ação explícita; build/run/test não contornam a decisão por restore implícito. Falha de restore mantém contexto parcial, log e opção de nova tentativa.
- [ ] **AC-04** Cancelar plano de instalação não executa download/comando. Confirmar mostra versão/fonte/asset/arquitetura/destino/operação/permissões exatos; trust e opt-in experimental não substituem confirmação.
- [ ] **AC-05** Exercitar downloading/installing/verifying, sucesso, cancelamento, falha e retry; logs sobrevivem e instalação parcial não é pronta. Sucesso exige executável/versão corretos; SDK de sistema não é instalado.
- [ ] **AC-06** Asset incompatível/ausente, origem não validada ou verificação falha bloqueiam instalação/uso; não há downgrade silencioso nem instalação global. Percentual só aparece se a fonte fornecer medida real.
- [ ] **AC-07** Revogar trust bloqueia novas execuções e solicita término dos processos ativos; serviços são encerrados e qualquer processo ainda ativo/encerramento não confirmado permanece visível, sem alegação falsa de limpeza.

**P0.2 — Solução**

- [ ] **AC-08** Abrir uma solução ou projeto; pedir escolha quando houver ambiguidade. Mostrar console, biblioteca e testes com referências/TFM/pacotes avaliados e bin/obj ocultos; modo restrito apresenta apenas contorno textual.
- [ ] **AC-09** Projeto com avaliação falha permanece visível, com abrir arquivo/log/retry; restore incompleto deixa explícita a cobertura parcial, sem declarar solução limpa.
- [ ] **AC-10** Mudança de contexto/projeto invalida avaliação antiga; TargetPath exibido e usado corresponde ao contexto MSBuild selecionado, nunca a caminho inferido ou snapshot obsoleto.

**P0.3 — Editor/análise/Problems**

- [ ] **AC-11** Sem solução, host ou LSP disponível, abrir/editar/salvar texto continua possível. Abas/breadcrumb/modificado são coerentes; fechar com edição ou conflito em disco não perde conteúdo sem escolha explícita.
- [ ] **AC-12** Na fixture real, completion e definição funcionam e rename console↔biblioteca apresenta prévia antes de aplicar. Cancelar prévia não modifica documentos; mudanças concorrentes invalidam a edição pendente.
- [ ] **AC-13** Code action só é oferecida se retornada no contexto atual; aplicar não marca diagnóstico corrigido antes de nova publicação. Ausência de ação não fabrica quick fix.
- [ ] **AC-14** Problems mantém severidade, código, mensagem, posição quando conhecida, origem, categoria e revisão. Categoria não identificada é Unknown; LSP/build não são mesclados apagando origem.
- [ ] **AC-15** Alterar buffer deixa diagnóstico/build anterior obsoleto com identificação. Publicação vazia atual só permite ausência de problemas no documento coberto; restore incompleto/analyzers não verificados não viram garantia global.
- [ ] **AC-16** Carregamento, falha, log e reinício do LSP são exercitados; coexistência host SDK 10 com projeto fixado em SDK 8 é provada. Falha mantém edição e log, sem migração silenciosa.

**P0.4 — Build/run**

- [ ] **AC-17** Selecionar startup executável/configuração/TFM; biblioteca ou contexto ambíguo não inicia silenciosamente. Comando, contexto, stdout/stderr e estado são visíveis.
- [ ] **AC-18** Build falho bloqueia Run/Debug; erro com posição abre fonte; texto não reconhecido fica no output bruto. Corrigir, salvar e recompilar permite executar o console com saída esperada.
- [ ] **AC-19** Buffer alterado depois do build não é apresentado como fonte executado verificado; execução/snapshot identifica revisão salva e exige decisão antes de sobrescrever ou salvar edições.
- [ ] **AC-20** Cancelar build/run solicita término real da árvore de processos e informa resultado observado; exit conhecido é preservado, exit ausente permanece desconhecido, e falha de encerramento fica explícita.

**P0.5 — Debug**

- [ ] **AC-21** Onde houver asset/plataforma habilitada, smoke real confirma build Debug prévio, launch da saída avaliada, breakpoint pendente→verificado, pausa, continue, step over/into, frames, locals e stop com logs.
- [ ] **AC-22** Build falho/adaptador ausente bloqueiam launch; breakpoint sem símbolos continua pendente; locals de pausa anterior não aparecem como atuais ao continuar. Exit ausente não vira zero nem prova término do processo.
- [ ] **AC-23** Mac ARM64 inicia com flag OFF e aviso experimental, Debug bloqueado, Run independente. Opt-in não instala ferramenta nem comprova compatibilidade; retirar a flag exige o gate DAP/async/multi-thread abaixo.
- [ ] **AC-24** Mac Intel sem asset 3.2 compatível verificado mostra indisponibilidade e não baixa versão divergente. Windows x64 requer prova DAP própria. Limitações de debug não bloqueiam análise/run/testes disponíveis nem implicam debug universal nos Macs.

**P0.6 — Terminal**

- [ ] **AC-25** Após trust, um PTY usa shell realmente disponível/cwd visíveis; entrada, output e Ctrl+C funcionam nos dois SOs. Antes de trust não abre shell no workspace.
- [ ] **AC-26** Fechar shell ativo oferece manter ou encerrar; saída/código representam o shell, não o último comando, e prompt ativo não é mostrado como shell encerrado. Encerramento e processos remanescentes têm evidência própria.

**P0.7 — Test Runner**

- [ ] **AC-27** Descoberta real via adapter VSTest do projeto encontra testes xUnit na fixture; projeto/TFM e seleção são explícitos. Run all e Run selected respeitam escopo e não executam substitutos por falta de seleção.
- [ ] **AC-28** Exercitar passed/failed/skipped; preservar mensagem, stack e output realmente disponíveis. Navegar quando posição fornecida e oferecer log sem navegação inventada quando não fornecida.
- [ ] **AC-29** Cancelar descoberta/execução preserva resultados parciais e marca operação cancelled; itens sem resultado não viram passed/skipped. Falha de infraestrutura e falha de teste são distinguíveis, com logs e retry.
- [ ] **AC-30** Testar vazio, adapter ausente e descoberta falha como estados distintos; nenhuma ação instala/edita packages silenciosamente. Contagens/resultados pertencem à execução e revisão identificadas.
- [ ] **AC-31** Projeto MTP recebe a mensagem exata especificada e fallback de terminal; nenhum comando VSTest nem conversão de projeto é aplicado. TRX não é tratado como mecanismo de descoberta.

**Transversal — UI e release**

- [ ] **AC-32** As sete telas mantêm composição/tema aprovados, strings inglesas, foco visível, nomes acessíveis, operação por teclado e estados textuais; atalhos por SO respeitam todos os bloqueios dos botões.
- [ ] **AC-33** Registrar a matriz macOS Intel, macOS ARM64 e Windows x64 para o núcleo P0; publicar Mac e Windows juntos, com status separado de debug por arquitetura e limitações explícitas. Nenhum resultado de desenho/documentação conta como prova.
- [ ] **AC-34** Antes de distribuir, auditar licenças/avisos de componentes e assets realmente incorporados/baixados, preservar obrigações aplicáveis e identidade própria; ausência de auditoria não é declaração de liberação jurídica.

## User Stories

1. As a C# developer, I want to open a solution or project, so that I can start working without assembling a command sequence manually.
2. As a developer, I want to choose between ambiguous solutions, so that the IDE does not analyze or run the wrong workspace.
3. As a developer, I want to see the full workspace path before granting trust, so that I know which repository may execute code.
4. As a security-conscious developer, I want repositories to open in restricted mode by default, so that inspection does not execute their build logic.
5. As a developer, I want to edit text without trusting the repository, so that I can inspect and repair source safely.
6. As a developer, I want to revoke trust and review active processes, so that I can stop further execution without hidden background work.
7. As a developer, I want to see analysis host SDK, project SDK, global.json rules and target framework separately, so that I can diagnose the correct prerequisite.
8. As a developer, I want missing SDK guidance and redetection, so that I can fix my environment without silent system installation or project migration.
9. As a developer, I want the analysis host to coexist with my pinned project SDK, so that language services do not force me to change the project toolchain.
10. As a developer, I want to request restore explicitly, so that package retrieval and project execution do not happen as hidden side effects.
11. As a developer, I want a complete tool installation plan before confirming, so that I can review origin, version, destination and permissions.
12. As a developer, I want repository trust and tool installation consent to be independent, so that trusting source does not authorize downloads.
13. As a developer, I want installation logs, cancellation and retry, so that I can recover from network, permission or verification failures.
14. As a developer, I want incompatible or unverified debugger assets to be blocked, so that the IDE does not silently install a different toolchain.
15. As a developer, I want tool readiness to require executable and version verification, so that partial installation is not reported as success.
16. As a developer, I want an evaluated solution tree with projects and dependencies, so that I can understand the actual build context.
17. As a developer, I want generated build directories hidden in the explorer, so that source navigation remains focused.
18. As a developer, I want failed projects and incomplete restore to remain visible, so that missing analysis is explainable and repairable.
19. As a developer, I want project changes to invalidate stale evaluation, so that execution uses the selected current context.
20. As a developer, I want to open, edit and save files in identifiable tabs, so that I can work even while analysis is unavailable.
21. As a developer, I want explicit choices for unsaved buffers and disk conflicts, so that my edits are not silently lost.
22. As a developer, I want C# completion and go-to-definition, so that I can navigate and write code across project references.
23. As a developer, I want a preview of cross-project rename edits, so that I can review and cancel a semantic change before applying it.
24. As a developer, I want stale edit previews rejected, so that concurrent changes are not overwritten by outdated language-service edits.
25. As a developer, I want contextual code actions only when supplied by the language server, so that suggested fixes reflect the current document and position.
26. As a developer, I want language-service loading, failure, logs and restart actions, so that incomplete analysis does not look like a clean solution.
27. As a developer, I want diagnostic origin and rule category shown independently, so that I can distinguish compiler, analyzer and reporting source.
28. As a developer, I want diagnostic and build snapshots tied to their scope and revision, so that old failures and live reports are not confused.
29. As a developer, I want edited documents marked as awaiting diagnostics, so that a local edit is not mistaken for a verified fix.
30. As a developer, I want empty diagnostic reports limited to their verified document scope, so that I do not assume full-solution cleanliness.
31. As a developer, I want to select startup project, configuration and target framework, so that build and execution use the intended executable context.
32. As a developer, I want build logs and source navigation for known error positions, so that I can fix failures without losing raw output.
33. As a developer, I want failed builds to block launch, so that I do not accidentally run a stale or invalid result.
34. As a developer, I want to see when source differs from the last build, so that I know which saved revision an execution represents.
35. As a developer, I want to run the console and see its output, so that I can validate behavior from the IDE.
36. As a developer, I want build and run cancellation with honest termination status, so that I can stop work without assuming unknown exit codes or cleanup.
37. As a developer on a verified debug platform, I want to launch the evaluated Debug output, so that breakpoints correspond to the selected build.
38. As a developer, I want pending and verified breakpoints distinguished, so that I know whether the adapter accepted a stop location.
39. As a developer, I want continue, step over, step into and stop, so that I can inspect basic execution flow.
40. As a developer, I want frames and current locals while paused, so that I can understand state without stale values appearing live.
41. As a developer, I want adapter logs and an unknown-exit state when necessary, so that missing DAP events are not reported as successful termination.
42. As an Apple Silicon developer, I want experimental debugging disabled by default with a separate opt-in, so that unverified support is never implied by installation or trust.
43. As a Mac Intel developer, I want an explicit unavailable-debug state when no verified compatible asset exists, so that I can still use editing, analysis, run and tests without misleading promises.
44. As a developer, I want one terminal with an identified shell and working directory, so that I can run complementary commands manually.
45. As a developer, I want real terminal input and Ctrl+C, so that I can interact with and interrupt command-line tools.
46. As a developer, I want confirmation before closing an active terminal, so that I do not accidentally terminate work.
47. As a developer, I want shell exit distinguished from command output, so that the IDE does not invent per-command results.
48. As a developer, I want to select a test project and target framework and discover tests through its VSTest adapter, so that the runner matches my project configuration.
49. As an xUnit developer, I want the basic runner to use my project's configured VSTest adapter, so that I can test without an IDE-specific package rewrite.
50. As a developer, I want to run all discovered tests or only selected tests, so that I can control execution scope.
51. As a developer, I want passed, failed and skipped results with run identity, so that I can distinguish current evidence from earlier runs.
52. As a developer, I want failure messages, stack traces and available test output, so that I can investigate a failure without discarding raw logs.
53. As a developer, I want source navigation only when a test result provides a known position, so that the IDE does not send me to a guessed location.
54. As a developer, I want discovery and execution cancellation to preserve partial results, so that unfinished tests are not mislabeled as passing or skipped.
55. As a developer, I want empty discovery, missing adapter and infrastructure failure shown separately, so that I can choose the correct recovery action.
56. As a developer, I want adapter guidance without automatic package changes, so that the IDE respects ownership of my project dependencies.
57. As a developer using Microsoft.Testing.Platform, I want an explicit unsupported message and terminal fallback, so that the IDE does not apply incompatible VSTest commands.
58. As a developer, I want the approved English dark interface with accessible focus and textual states, so that important actions and failures remain understandable without relying on color.
59. As a developer moving between macOS and Windows, I want the documented IntelliJ/Rider-compatible default shortcuts, so that familiar actions work without implying a proprietary integration.
60. As a developer evaluating OpenRidr, I want compatibility and experimental limits stated per platform, so that I can distinguish supported workflows from unproven debugger behavior.

## Implementation Decisions

### Base e fronteiras

Adotar OpenIdea como base/inspiração de shell desktop Tauri/Rust e TypeScript/React, editor CodeMirror e terminal xterm, conforme pesquisa remota. Antes de reaproveitar qualquer integração, auditar sua revisão, interfaces e limites. Não existe checkout local que permita fixar agora módulos, APIs, nomes de serviços ou testes herdados como garantias.

Manter fronteiras conceituais pequenas: UI/editor apresenta intenção e estado; host desktop valida confiança, consentimentos e contexto e supervisiona processos; processos externos executam MSBuild/CLI, LSP, DAP e shell. Não reconstruir compilador, motor semântico ou framework universal de plugins. LSP e DAP são protocolos distintos, não uma API semântica única.

A superfície de cada operação deve representar conceitualmente identidade, workspace/origem, contexto selecionado, revisão/escopo, estado de ciclo de vida, logs, erro, cancelamento e processo associado quando houver. Estes são contratos de comportamento, não prescrição de endpoints, assinaturas ou schemas internos.

### Estado, erros e processos

Separar solicitação de cancelamento, confirmação de término de processo, conclusão do protocolo e exit code. Um evento DAP de encerramento não prova que toda a árvore morreu; um exit do shell não prova resultado do comando anterior. Não sobrescrever resultados prévios com novos estados sem identificação de execução. Guardar logs acessíveis após falha/cancelamento e evitar que resposta tardia de operação anterior seja aplicada como estado atual de outro contexto.

Host é responsável por executar binários resolvidos com argumentos separados e cwd/ambiente apropriados; não interpolar nomes de projeto/arquivo em comandos shell. PTY é a exceção intencional para interação humana. Validar limites UI→host também no host, não apenas desabilitando botões. Encerrar ou relatar processos remanescentes nas trocas/fechamento de workspace, sem perder edições nem afirmar término sem evidência.

### Avaliação e ferramentas

Delegar semântica de projeto ao MSBuild após trust. Leitura textual serve apenas para inspeção inicial. Resolução de SDK do projeto respeita suas regras e é independente do host de análise; ambiente de lançamento do LSP deve provar essa separação, não assumir que trocar cwd resolve qualquer caso. Incompatibilidade real resulta em log, capacidade indisponível e edição preservada, não alteração automática do projeto.

Instalador usa somente origens oficiais aprovadas, versão explícita e asset adequado ao SO/arquitetura. Validar origem e redirecionamentos, destino e extração segura; impedir escape do diretório autorizado. Verificar integridade/autenticidade pelos mecanismos efetivamente disponíveis e registrar o que foi e não foi verificado; não afirmar que toda release oferece assinatura ou hash verificável. Não marcar artefato divergente/corrompido como pronto. Mudança material de plano exige nova confirmação. Instalações parciais não substituem ferramenta válida silenciosamente.

Preservar logs sem publicar automaticamente dados locais; não registrar deliberadamente segredos de ambiente/credenciais de feeds. Logs podem conter caminhos e conteúdo do projeto, e eventual compartilhamento deve ser ação consciente do usuário. Não adicionar serviço remoto, telemetria ou upload como dependência destes fluxos.

### Integrações .NET

Fixar csharp-ls 0.28.0, host SDK 10, netcoredbg 3.2.0 e contratos LSP/DAP efetivamente suportados. Metadados virtuais e Roslyn LS futuro não ampliam P0. Negociar capacidades e respeitar estados/ordem dos protocolos; não expor controles que a sessão não pode executar.

Proposta inicial para Build é saída textual com interpretação conservadora de posições. Proposta inicial para runner é descoberta/seleção VSTest e ingestão de resultados estruturados quando disponíveis, incluindo TRX, com output bruto preservado. Validar identidade de testes e filtros de seleção, sem assumir que nome de exibição basta ou que TRX descobre testes. Evitar dependência de packages adicionais impostos ao projeto do usuário. Um logger específico só será considerado se a validação mostrar necessidade, sem transformar isso em implementação aprovada agora.

### Licenças e distribuição

Fontes permitidas para esta base: OpenIdea Apache-2.0, csharp-ls MIT e netcoredbg MIT, com preservação de licenças, avisos e obrigações aplicáveis a alterações/redistribuição. Auditar dependências transitivas, assets, ícones e eventual distribuição/download de binários antes de liberar o produto. Pesquisa documental não é auditoria concluída nem parecer jurídico. Compatibilidade de atalhos não concede direitos de branding; não usar vsdbg, C# Dev Kit ou código/componentes Rider/ReSharper.

## Testing Decisions

### Estratégia proposta — confirmar na revisão de Helder

O seam **mais alto proposto** é a interação real **UI desktop → host → processos externos**, com fixture .NET real. Esta estratégia foi comunicada como proposta; não há confirmação de Helder, testes locais, harness instalado ou suíte herdada comprovada. A revisão desta spec deve confirmar a estratégia antes da implementação.

Percurso principal: abrir fixture restrita → comprovar bloqueios → confiar → resolver SDKs/avaliar → solicitar restore → carregar análise → completion/definição/rename com prévia → build falho navegável → corrigir/rebuild → Run → descobrir/executar testes → cancelar e inspecionar logs. Debug entra em percurso separado conforme plataforma/asset/flag. PTY é exercitado de verdade nos dois sistemas, não substituído por simples output estático.

Fixture proposta: uma solução SDK-style com dois projetos de aplicação, console OrderDesk.Cli referenciando biblioteca OrderDesk.Core, mais projeto OrderDesk.Tests com xUnit e adapter VSTest. Alvo net8.0; global.json fixa SDK 8.0.408 com rollForward desabilitado, instalado ao lado do SDK 10 do host. A fixture ainda não existe. Preparar estados controlados de erro de compilação, referência entre projetos, rename, restauração incompleta e testes que passam/falham/são ignorados. Os snapshots ilustrativos das telas não são uma execução única nem contagens obrigatórias de produção.

### Camadas complementares

Reutilizar seams e infraestrutura de testes upstream somente depois de auditoria. Priorizar evidência observável na fronteira real; não provar o fluxo com mocks de funções internas triviais. Quando E2E desktop for impraticável, cobrir contratos de LSP, DAP, processos/PTY, avaliação/CLI, resultados VSTest e instalador nas respectivas boundaries, declarando a lacuna de UI restante.

Testes de protocolo/contrato devem cobrir mensagens tardias, cancelamento, desconexão, posições/revisões obsoletas, resultado parcial, erro e ausência de exit code. Casos determinísticos de falha de download/verificação/extração e consentimento podem usar artefatos/servidores controlados na fronteira externa, complementados por smoke de instalação da origem aprovada. Isso não substitui smoke com csharp-ls e netcoredbg reais.

Inspecionar ausência de processos antes de trust e tentativas diretas na fronteira do host. Exercitar paths com espaços, separadores e escaping, CRLF/posições no Windows, permissões, processos filhos, cancelamento e encerramento em cada OS. Conferir foco/teclado, motivo de controles desabilitados, texto semântico e preservação de edições/logs nas sete telas.

### Matriz de aceitação e release

| Ambiente | Núcleo P0 obrigatório | Debug |
|---|---|---|
| macOS Intel | Trust, ferramentas disponíveis, SDKs separados, árvore, editor/LSP, build/run, VSTest e PTY | Somente se asset 3.2 compatível for verificado e smoke DAP passar; na ausência, validar indisponibilidade explícita sem substituição silenciosa |
| macOS Apple Silicon | Mesma sequência do núcleo, independente do debugger | Flag experimental OFF por padrão; validar bloqueio/opt-in separado. Experimento não bloqueia release nem equivale a suporte comprovado |
| Windows x64 | Mesma sequência do núcleo, com shell/PTY e encerramento de processos Windows | Asset adequado e smoke DAP real para launch/breakpoint/step/frames/locals/stop; não basta verificar versão |

Mac e Windows entram juntos; não reduzir a entrega a um OS inicial. Não foram aprovadas versões mínimas concretas de macOS/Windows: registrar versões reais na evidência, declarar alcance validado e não inventar cobertura de versões antigas. Outras arquiteturas Windows e Linux não fazem parte desta matriz P0. net6 recebe registro best effort; outros TFMs .NET 8+ precisam de pré-requisitos e evidência específicos, não herdam aprovação automaticamente da fixture net8.

**Gate obrigatório para retirar a flag ARM64:** executar em Mac arm64 real netcoredbg no modo `--interpreter=vscode` (DAP), demonstrando launch, breakpoint, stepping e stop, incluindo fluxos async e multi-thread; guardar logs DAP/host/aplicação, versões, comandos e evidência de encerramento dos processos. Não aceitar teste de outro modo do debugger, presença do asset, versão do executável ou opt-in como substitutos. #256 continua risco relatado não reproduzido até haver evidência própria.

Mapear cada AC-01 a AC-34 a uma evidência ou limitação explícita por ambiente. Registrar OS/arquitetura, versões/asset/fonte, SDK host/projeto, TFM, contexto, comandos, revisão, logs, exit conhecido/desconhecido e resultado observado. Sem métricas de desempenho, percentuais de cobertura ou progresso inventados. Resultados não executados permanecem não verificados; nada nesta fase declara compatibilidade comprovada.

## Out of Scope

- Implementação, instalação, checkout/fork, tickets e commits nesta etapa de especificação; somente revisão do documento.
- P1: testes avançados como cobertura/gutters/debug test, perfis e launchSettings.json, usages, format, settings ampliadas e busca de símbolos, sujeitos a priorização posterior.
- P2: MTP a avaliar; attach, conditional/log breakpoints, watches/evaluate, políticas avançadas de exceção, NuGet UI, templates, multi-target avançado e abas/splits de terminal.
- Roslyn LS alternativo, navegação avançada de metadata e motor semântico próprio no P0.
- Hot reload/Edit and Continue, designers WPF/WinForms, profiling, .NET Framework legado, suporte completo Razor/Blazor, Unity/Unreal, desenvolvimento remoto e IA embutida.
- Instalador de SDK de sistema, migração automática de global.json/TFM, instalação silenciosa de adapters/packages, fallback silencioso de versões e promessa universal de frameworks, adapters ou arquiteturas.
- Paridade Rider/ReSharper, backend/componente proprietário, uso de marca JetBrains, vsdbg e C# Dev Kit.

## Further Notes

### Fontes e precedência

- Telas aprovadas: [HTML offline](index.html) e [wireframes ASCII](wireframes.md); decisões e prioridades: [README](README.md), no diretório `/Users/helder/.opencode/plan/openridr-ui/`.
- Handoff lido: `/tmp/openridr-handoff.md`; notas remotas de apoio: `/Users/helder/.config/openchamber/chats/2026-10-27/session-1b9558e8-ecc3-4178-8b36-e6b8bd290c33/openidea-dotnet-notes.md` e `rider-like-architecture-notes.md` no mesmo diretório.
- Base remota pesquisada: [OpenIdea, revisão 48ac2094fc62b51b730a59d4ae6530392ef7f18b](https://github.com/l0ng-ai/openidea/tree/48ac2094fc62b51b730a59d4ae6530392ef7f18b). Não há checkout/fork local nem integração garantida pelas notas.
- Ferramentas: [csharp-ls 0.28.0](https://github.com/razzmatazz/csharp-language-server/releases/tag/0.28.0), [netcoredbg 3.2.0-1092](https://github.com/Samsung/netcoredbg/releases/tag/3.2.0-1092), [relato upstream #256](https://github.com/Samsung/netcoredbg/issues/256). Referências herdadas, sem nova pesquisa ou execução nesta etapa.

As fontes visuais ainda contêm textos históricos de “aguardando aprovação das telas”; a solicitação atual confirma essa aprovação. Esta spec passa a aguardar sua própria revisão, sem modificar retrospectivamente os arquivos aprovados. Recomendações antigas de um único SO inicial, runner apenas posterior ou escopo adicional cedem às decisões atuais: Mac/Windows juntos, VSTest básico P0 e debug condicionado por arquitetura.

### Pendências para a revisão, sem reabrir decisões aprovadas

Confirmar a estratégia de teste proposta no seam UI desktop→host→processos externos e revisar os critérios da spec. Disponibilidade Mac Intel 3.2, compatibilidade real csharp-ls/SDKs, validação VSTest/TRX, provas por plataforma e auditoria de distribuição são verificações futuras, não fatos assumidos. A flag ARM64 continua OFF e não bloqueia release enquanto experimental.

**Ponto de parada: aguardar Helder revisar esta SPEC. Não criar tickets, implementar, instalar, fazer commit nem avançar automaticamente.**
