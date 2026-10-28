# Tickets P0 — OpenRidr

## Escopo e autorização

Fonte normativa: [SPEC completa](../SPEC.md); decisões atualizadas: `/tmp/openridr-handoff-3.md` e solicitação aprovada de 27/10/2026. A autorização atual supera o ponto de parada histórico da SPEC **somente para criar estes 18 tickets, este índice e um commit local `docs: add P0 tickets`**. Não autoriza implementação, instalação, execução do app, checkout, fork, remote, push ou PR. A quebra já foi aprovada; não há novo quiz. A estratégia de validação abaixo está decidida pelo handoff, substituindo seu estado histórico de proposta na SPEC.

Tracker local em `docs/tickets/` por autorização explícita; regras globais de triagem aplicadas por ausência de overrides locais. Cada ticket tem um único estado `ready-for-agent` e categoria `enhancement`; isso não autoriza sua execução. Implementação futura: exclusivamente modelo `opencode-go/deepseek-v4.1-flash`, nunca variante `-free`, após autorização própria.

O repositório contém documentação, não uma implementação comprovada. A base OpenIdea é candidata sujeita a auditoria: não presumir APIs, paths internos, integrações ou testes herdados existentes. Nenhuma fixture, ferramenta ou capacidade do app foi criada/verificada nesta tarefa.

## Índice e grafo de bloqueios

Blockers são requisitos funcionais anteriores, não uma fila artificial. Trabalhar na fronteira liberada do DAG após autorização; referências abaixo usam os IDs dos arquivos e os títulos dos tickets.

| Ticket | Bloqueado por | Entrega demonstrável |
|---|---|---|
| [01 — Abrir solução ou projeto em modo restrito](01-abertura-restrita.md) | Nenhum | Shell desktop mínimo auditado, escolha de contexto e edição segura sem execução |
| [02 — Confiar no workspace e separar SDKs](02-trust-e-sdks.md) | 01 | Trust no host, revogação e inventário host/projeto independentes |
| [03 — Avaliar MSBuild e mostrar a árvore real](03-arvore-msbuild.md) | 02 | Modelo avaliado, contexto e erro de avaliação recuperável |
| [04 — Restaurar explicitamente e recarregar contexto](04-restore-explicito.md) | 03 | Restore consentido, contexto parcial e invalidação de snapshots |
| [05 — Instalar csharp-ls com confirmação e carregar análise](05-csharp-ls.md) | 04 | Instalação protegida e LSP real com SDKs separados |
| [06 — Navegar diagnósticos por documento e revisão](06-diagnosticos.md) | 05 | Problems real, navegação e obsolescência sem falsa limpeza |
| [07 — Proteger buffers e resolver conflitos de disco](07-buffers-e-conflitos.md) | 01 | Ciclo completo de decisões sem perda de edições |
| [08 — Completar e navegar definições entre projetos](08-completion-e-definicao.md) | 05 | Completion e definição reais console↔biblioteca |
| [09 — Renomear entre projetos com prévia válida](09-rename.md) | 05, 07 | Prévia, cancelamento e rejeição de edições obsoletas |
| [10 — Oferecer somente code actions do contexto atual](10-code-actions.md) | 05, 07 | Ações LSP atuais com aplicação segura |
| [11 — Compilar contexto e navegar erros no output](11-build.md) | 04, 06 | Build explícito, logs brutos e erros identificados |
| [12 — Executar a saída avaliada e cancelar processos](12-run.md) | 11 | Run do build identificado e término observado da árvore |
| [13 — Instalar debugger com confirmação e gates de plataforma](13-instalacao-debugger.md) | 02 | Adaptador verificado e indisponibilidades/flag honestas |
| [14 — Depurar por DAP com breakpoints, frames e locals](14-debug-dap.md) | 11, 13 | Launch Debug real, stepping e stop com evidência |
| [15 — Usar PTY real e confirmar fechamento](15-terminal-pty.md) | 02 | Shell/cwd identificados, entrada e Ctrl+C reais |
| [16 — Descobrir testes VSTest com identidade e escopo](16-descoberta-vstest.md) | 04 | Descoberta real e vazio/adapter/falha/MTP distintos |
| [17 — Executar testes selecionados ou todos e preservar resultados](17-execucao-testes.md) | 16 | Resultados/detalhes reais e cancelamento parcial |
| [18 — Consolidar aceitação multiplataforma e auditoria de release](18-aceitacao-e-release.md) | 08, 09, 10, 12, 14, 15, 17 | Matriz/evidências completas, auditoria e gates de distribuição |

### Primeira fatia: 01–06

Ao concluir 01–06 é possível, pela UI desktop real, abrir solução/projeto sem executar seu conteúdo, editar com segurança mínima, confiar, identificar os dois SDKs, avaliar a árvore MSBuild, solicitar restore e instalar/carregar csharp-ls para navegar diagnósticos reais por revisão. Essa demo não espera 07–18, Build, Run nem debugger. O ticket 07 completa fluxos de conflitos, não introduz segurança básica tardiamente. Os tickets 08–17 ampliam o ciclo sem serem pré-requisitos artificiais dessa fatia.

## Gates compartilhados obrigatórios desde cada ticket

Estas cláusulas integram os critérios de **todos** os tickets; o 18 consolida evidência, não posterga segurança, acessibilidade, logs ou qualidade até o fim.

- **G1 — Trust e fronteira:** todo consumidor executável valida trust e contexto no host, inclusive chamadas diretas que contornem a UI, atalhos e respostas tardias. Antes de trust, apenas leitura textual e inventário neutro fora do workspace; sem MSBuild, restore, LSP, tarefas, build/run/debug, runner ou PTY no repositório. Revogação bloqueia novas execuções imediatamente, encerra serviços e solicita término de processos ativos; remanescentes/decisão pendente ficam visíveis. Troca/fechamento de workspace preserva edições e encerra ou relata processos. Nem fim de protocolo nem pedido de cancelamento provam limpeza. Build/run/test não fazem restore implícito.
- **G2 — Contexto e integridade:** identificar operação, workspace, projeto/configuração/TFM, revisão salva/buffer, escopo, estado e logs pertinentes; invalidar contexto/TargetPath antigo. Respostas tardias não substituem contexto atual. Preservar edições, resultados parciais e output bruto. Estado de processo, cancelamento solicitado, conclusão de protocolo e exit code são distintos; código ausente é `unknown`, nunca zero presumido. Executar binário resolvido com argumentos separados, cwd/ambiente explícitos, sem interpolar nomes em shell; PTY humano é exceção intencional.
- **G3 — Consentimento e instaladores:** trust, confirmação de instalação e opt-in experimental são independentes. Plano inclui versão/fonte/URL/asset aplicável, SO/arquitetura, destino absoluto fora do repo, operação/comando e permissões; mudança material exige reconfirmação. Cancelar plano não baixa/executa. Origens/redirecionamentos oficiais, destino e extração segura sem escape; integridade/autenticidade verificadas pelos mecanismos disponíveis e limites registrados, sem inventar hash/assinatura. Sem global install, elevação implícita, downgrade ou substituição silenciosa de ferramenta válida por parcial. Estados/logs/retry sobrevivem; progresso só medido. SDK de sistema não é instalado.
- **G4 — UI e privacidade:** UI em inglês e docs PT-BR; composição das [sete telas](../wireframes.md) e [referência visual](../index.html), tema escuro/cobre, cores com texto/ícone, sans/mono, árvore/editor/toolbar/painel inferior e rolagem local sem perder ações/foco. Cada controle funcional tem nome acessível, foco visível, teclado e motivo de bloqueio legível desde sua introdução. Keymap padrão IntelliJ/Rider sem seletor: Mac Run Ctrl+R, Debug Ctrl+D, breakpoint Cmd+F8; Windows Run Shift+F10, Debug Shift+F9, breakpoint Ctrl+F8; F8/F7 step over/into em ambos. Atalhos respeitam os mesmos gates. Logs locais acessíveis após falha/cancelamento, sem registrar deliberadamente segredos/feed credentials nem publicar dados automaticamente; compartilhar é decisão consciente. Sem telemetria/upload/serviço remoto obrigatório.
- **G5 — Validação e proveniência:** cada AC/cláusula recebe evidência ou limitação explícita **por ambiente**: SO/arquitetura/versão, ferramentas/asset/origem, SDK host/projeto, TFM, contexto, comandos, revisão, logs, resultado e exit conhecido/unknown. Resultado não executado é não verificado, nunca aprovação automática. Mocks/contratos complementam, não substituem E2E real. Evidência documental/desenho não prova execução. Auditar interfaces/licenças da base e componentes efetivamente incorporados desde a introdução; preservar avisos e identidade OpenRidr. Auditoria final ausente não significa liberação jurídica.

## Estratégia de validação decidida

Seam principal: **UI desktop → host → ferramentas/processos reais**. No Mac executar o fluxo de verdade; Windows via CI real se viável, ou registrar pendência com ambiente/motivo/evidência faltante. CI que exercita apenas CLI/contratos sem interação desktop não comprova a UI. Lacuna documentada não equivale a gate de release aprovado; Mac e Windows permanecem juntos no P0.

Fixture **futura, ainda inexistente**: solução SDK-style OrderDesk.Cli (console) → OrderDesk.Core (biblioteca), mais OrderDesk.Tests com xUnit e adapter VSTest configurado; net8.0, SDK 8.0.408 fixado por global.json com rollForward disabled, ao lado do SDK 10 usado pelo host csharp-ls 0.28.0. Criar fixture/harness somente na implementação autorizada e quando necessários à fatia; nenhum SDK será instalado nesta tarefa. Incluir erros controlados, restore incompleto, referências/rename e testes passed/failed/skipped; não copiar contagens ilustrativas como expectativas reais.

Boundaries complementares: UI→host direto negativo para trust, consentimento/download/redirecionamento/verificação/extração, argumentos/paths com espaços/separadores, permissões, CRLF/posições Windows, cancelamento e filhos, desconexão, resposta tardia, revisão/conflito e ausência de exit. Casos determinísticos podem controlar artefatos/servidores na boundary; ainda precisam de smoke com ferramentas oficiais reais. Registrar a lacuna desktop quando só contratos forem viáveis, sem esconder o gate restante.

| Ambiente P0 | Núcleo obrigatório | Debug |
|---|---|---|
| macOS Intel | Trust, SDKs separados, ferramentas disponíveis, árvore/editor/LSP, build/run, VSTest, PTY e UI real | Sem asset 3.2 compatível verificado: indisponível, sem substituir versão |
| macOS ARM64 | Mesmo núcleo, independente do debugger | Experimental OFF por padrão; opt-in separado e experimento não bloqueante de release |
| Windows x64 | Mesmo núcleo; CI real se viável e lacunas desktop explícitas, PTY/término próprios do SO | Asset compatível e smoke DAP próprio obrigatório; versão do binário não basta |

Registrar versões reais dos SOs; não inventar mínimos nem estender cobertura a Linux/outras arquiteturas Windows. .NET 8+ apenas para alvos/pré-requisitos efetivamente comprovados; net8 não prova todos os TFMs; net6 best-effort, sem garantia equivalente.

O spike ARM64 em modo CLI **não prova DAP**. Retirar a flag exige netcoredbg 3.2.0 real em Mac ARM64 com `--interpreter=vscode`: launch, breakpoint, stepping, stop, async e multi-thread, logs DAP/host/app, versões/comandos e processos realmente encerrados. Opt-in/asset/versão não substituem essa prova. Upstream #256 continua risco relatado não reproduzido, não falha universal. Mac Intel indisponível não dispensa Windows DAP; limites de debugger não bloqueiam análise/Run/testes disponíveis.

## Mapa completo de AC para tickets

Cada linha refere-se ao AC integral na SPEC. “Parcial” limita a contribuição da fatia, não reduz a obrigação; gates compartilhados continuam válidos. O ticket 18 reúne evidência por ambiente de **todas** as linhas, sem ser a primeira implementação dos gates.

| AC | Tickets e cláusulas/contribuições |
|---|---|
| AC-01 | 01 edição/restrição; 02 host-enforcement; 03–06 e 08–17 negativos por consumidor (G1) |
| AC-02 | 02 inventário neutro e regras SDK; 05 coexistência real host10/projeto8 |
| AC-03 | 04 restore/recarga; 11, 12, 16, 17 impedem restore implícito |
| AC-04 | 05 plano csharp-ls; 13 plano debugger; G3 em ambos |
| AC-05 | 05 ciclo instalador/LSP; 13 ciclo instalador/debugger; sem SDK installer |
| AC-06 | 05 origem/versão/verificação NuGet aplicáveis; 13 asset/arquitetura/extração; G3 |
| AC-07 | 02 revogação/serviços/processos; 03–06 e 08–17 aplicam por consumidor (G1) |
| AC-08 | 01 abertura/ambiguidade/contorno parcial; 03 árvore MSBuild completa |
| AC-09 | 03 falha de avaliação; 04 restore parcial/recarga; 05 cobertura parcial LSP |
| AC-10 | 03 propriedades/TargetPath; 04 invalidação; 11 contexto build; 12 saída Run; 14 saída Debug |
| AC-11 | 01 edição mínima segura sem solução/host/LSP; 05 falha mantém edição; 07 fluxo completo de buffers/conflitos |
| AC-12 | 08 completion/definição; 09 rename/prévia/cancelamento/concorrência |
| AC-13 | 10 ações retornadas/aplicação; 06 republicação de diagnósticos, sem correção presumida |
| AC-14 | 06 diagnósticos LSP e estrutura/origem/categoria; 11 diagnósticos Build sem fusão |
| AC-15 | 06 revisão/documento/publicação vazia; 04/05 limites restore/analyzers; 11/12 build/revisão salva obsoleta |
| AC-16 | 05 carga/falha/log/reinício/coexistência SDK; 02 detecção separada |
| AC-17 | 11 seleção/comando/output Build; 12 startup/Run e contexto |
| AC-18 | 11 falha/erro/log/rebuild; 12 bloqueio/Run console; 14 bloqueio Debug |
| AC-19 | 11 identidade build salvo; 12 decisão fonte/build; 07 preservação de edições |
| AC-20 | 11 cancelar Build; 12 cancelar Run/árvore/exit desconhecido |
| AC-21 | 14 smoke DAP real completo; 11 build Debug; 13 asset/plataforma habilitados |
| AC-22 | 14 bloqueios/pendência/locals/exit; 11 falha build; 13 adaptador ausente |
| AC-23 | 13 flag ARM64 OFF/opt-in; 14 gate DAP/async/multi-thread; 18 mantém limitação na release |
| AC-24 | 13 Mac Intel sem asset e limites; 14 smoke Windows x64; 18 matriz e ausência de cobertura universal |
| AC-25 | 15 PTY/shell/cwd/entrada/Ctrl+C em ambos SOs e gate trust |
| AC-26 | 15 confirmação/shell exit/processos remanescentes |
| AC-27 | 16 descoberta xUnit/adapter/escopo/identidade; 17 selected/all sem substituição |
| AC-28 | 17 resultados/detalhes/output/navegação apenas comprovada |
| AC-29 | 16 cancelar descoberta/falha infra; 17 cancelar execução/resultados parciais/falha teste |
| AC-30 | 16 vazio/adapter/falha/packages; 17 contagens/resultados por execução/revisão |
| AC-31 | 16 detecção MTP/mensagem/sem VSTest/TRX não descobre; 17 conserva gate; 15 terminal complementar |
| AC-32 | 01–17 G4 em cada superfície introduzida, sem adiamento; 18 regressão das sete telas/atalhos |
| AC-33 | 01–17 G5 por ambiente/fatia; 18 consolida matriz núcleo P0 Mac+Windows/debug separado e gate release |
| AC-34 | 01 auditoria da base mínima; 05/13 ferramentas/assets e G5 nas demais incorporações; 18 auditoria completa de distribuição |

## Estado desta entrega documental

Tickets publicados como critérios futuros, sem marcar AC funcional cumprido. Não existem aprovação automática de release, resultado de teste do app, compatibilidade comprovada nem auditoria jurídica concluída. Pendências materiais são verificadas durante implementação autorizada e registradas em evidências, não resolvidas por este índice.
