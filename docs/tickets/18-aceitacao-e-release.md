# 18: Consolidar aceitação multiplataforma e auditoria de release

**What to build:** Consolidar evidências reais do ciclo P0 e auditoria dos componentes distribuídos, explicitando compatibilidade/limites por arquitetura e impedindo release com gates obrigatórios sem prova. Não introduzir tardiamente trust, acessibilidade, logs ou proteções já exigidos em cada fatia.

**Blocked by:** 08 — Completar e navegar definições entre projetos; 09 — Renomear entre projetos com prévia válida; 10 — Oferecer somente code actions do contexto atual; 12 — Executar a saída avaliada e cancelar processos; 14 — Depurar por DAP com breakpoints, frames e locals; 15 — Usar PTY real e confirmar fechamento; 17 — Executar testes selecionados ou todos e preservar resultados.

**Status:** ready-for-agent

**Category:** enhancement

- [ ] Percorrer UI desktop → host → ferramentas reais: restrito/negativos trust → confiar/SDKs/árvore → restore explícito → LSP/diagnósticos/completion/definição/rename/ações quando retornadas → build falho/navegar/corrigir/rebuild → Run → descoberta/execução/cancelamento testes; PTY e Debug têm percursos próprios reais.
- [ ] Mapear **AC-01 a AC-34**, incluindo cada cláusula compartilhada/parcial do índice, a evidência ou limitação explícita por macOS Intel, macOS ARM64 e Windows x64. Guardar SO/arquitetura/versões, SDKs host/projeto, TFM, asset/fonte, comandos/contexto/revisão, logs, resultado e exit conhecido/unknown; sem percentuais/contagens inventados ou aprovação de não executados.
- [ ] Validar núcleo P0 em Mac e Windows juntos: Windows via CI real se viável ou pendência documentada; CI apenas CLI/contratos não comprova UI desktop. Registrar lacuna não aprova gate release. Arquitetura indisponível fica não verificada, não “passou”; não reduzir entrega a um OS inicial.
- [ ] Auditar sete telas/tema/strings inglesas, foco, nomes acessíveis, teclado/atalhos por SO, estados textuais, motivos de bloqueio e rolagem local em tamanhos reais, usando referências visuais como orientação, não testes executados. Repetir trust direto no host, revogação, preservação de buffers/logs e processos remanescentes em cada consumidor.
- [ ] Registrar matriz Debug separada: Intel sem asset 3.2 compatível mantém indisponibilidade/sem substituição; Windows x64 precisa smoke DAP próprio; ARM64 experimental OFF não bloqueante enquanto assim declarado. Retirada da flag só com DAP real `--interpreter=vscode` em ARM64, launch/breakpoint/stepping/stop, async/multi-thread, logs e processos encerrados. Spike CLI não prova DAP; #256 permanece risco não reproduzido sem evidência própria.
- [ ] Declarar .NET 8+ somente nos alvos/pré-requisitos comprovados; fixture net8/SDK 8.0.408 ao lado de host10 não aprova todos os TFMs. net6 best-effort separado; registrar versões reais de SO sem mínimos inventados, sem promessa Linux/outras arquiteturas Windows.
- [ ] Auditar revisão/origem/licenças/avisos de OpenIdea Apache-2.0, csharp-ls MIT, netcoredbg MIT, dependências transitivas, assets/ícones/binários efetivamente incorporados/baixados e alterações/redistribuição. Preservar obrigações e identidade própria; sem vsdbg/C# Dev Kit/código ou marca Rider/ReSharper. Auditoria documental anterior não equivale a auditoria completa nem parecer jurídico.
- [ ] Produzir decisão rastreável de release: separar aprovado, bloqueado e não verificado; gates obrigatórios pendentes impedem declarar P0 aceito/distribuição liberada. Manter flags/limites e artefatos de evidência acessíveis; publicar Mac e Windows juntos somente com gates satisfeitos. Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket).

## Mapeamento à SPEC

AC-32/33/34 (consolidação/regressão UI, matriz/release e auditoria de distribuição). AC-01–AC-31 (validação integradora de todas as cláusulas, não substitui sua entrega nos tickets responsáveis). Testing Decisions, matriz/gate ARM64, Licenças e distribuição e [mapa completo](README.md#mapa-completo-de-ac-para-tickets).

## Validação

Evidência desktop real Mac, Windows real/CI conforme capacidade e limitações declaradas, incluindo paths com espaços, CRLF/posições, permissões, filhos/cancelamento/exit unknown, mensagens tardias, revisões/conflitos e consentimento/download/extração. Reusar apenas infraestrutura auditada; testes de boundary complementam E2E, não o substituem. Conferir mapa AC por ambiente e manifestos/avisos de distribuição; falha ou ausência de prova obrigatória deixa gate aberto. Este ticket documental não executa nem aprova release agora.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
