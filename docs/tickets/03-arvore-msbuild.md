# 03: Avaliar MSBuild e mostrar a árvore real

**What to build:** Após trust e SDK adequado, navegar uma árvore de solução derivada de avaliação MSBuild real, incluindo projetos que falharam. A UI explica o que foi avaliado e permite recuperar falhas sem parar a edição.

**Blocked by:** 02 — Confiar no workspace e separar SDKs.

**Status:** ready-for-agent

**Category:** enhancement

- [x] Avaliar MSBuild somente com trust no host, sem restore implícito; substituir o contorno textual por modelo avaliado identificado por workspace/contexto/revisão. LSP não fornece o modelo de solução.
- [x] Mostrar solução, console, biblioteca e testes, fontes, TFMs/frameworks, referências entre projetos e pacotes quando conhecidos; ocultar bin/obj. Dados desconhecidos/parciais não são inventados.
- [x] Selecionar configuração/TFM explicitamente quando ambíguos e obter propriedades/itens e TargetPath dessa avaliação, não de convenção de diretório. Não usar snapshot antigo após mudança de contexto.
- [x] Projeto com erro permanece na árvore; causa, log, abrir arquivo e retry funcionam. Restore incompleto sinaliza contexto/dependências/análise parciais e não declara solução limpa; execução do restore será entregue em 04.
- [x] Edição e mudança relevante em projeto/import invalidam avaliação anterior visivelmente; oferecer recarga e nunca apresentar TargetPath invalidado como atual. Resposta tardia de contexto anterior não substitui a seleção vigente.
- [x] Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), incluindo revogação durante avaliação, logs preservados e árvore acessível por teclado sem cor como único estado.

## Evidência

Implementação, comandos, JSON bruto do MSBuild, falha MSB4019, logs e captura: [docs/evidence/03-arvore-msbuild.md](../evidence/03-arvore-msbuild.md). Avaliado em macOS 27.2 arm64 com SDK de projeto 8.0.408 e host 10.0.100; Windows e macOS Intel não executados. Restore real, consumidores de build/run/debug e LSP ficam fora desta fatia.

## Mapeamento à SPEC

AC-08 (árvore avaliada), AC-09 (erro/recuperação e parcialidade), AC-10 (parcial: modelo/TargetPath/invalidação; reload após restore em 04 e consumidores 11/12/14), AC-01/07 (gates deste consumidor), AC-32/33. SPEC P0.2 e Avaliação e ferramentas.

## Validação

Preparar na implementação a fixture real OrderDesk descrita no índice quando necessária à demo; na UI Mac navegar console/biblioteca/testes e abrir falha controlada de avaliação, log e retry. Comparar propriedades/itens com MSBuild real. Boundary: imports alterados, contexto trocado com resposta tardia, SDK ausente, paths com espaços e revogação/cancelamento durante avaliação. Evidência/limitação por ambiente, sem supor módulos herdados nem cobertura de TFMs não testados.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
- 2026-10-28: Implementado e validado (Rust 15+23, Playwright 14). Avaliação por projeto com MSBuild 8.0.408, multi-TFM explícito, falha parcial com log/retry, invalidação por save/mtime e token tardio. Evidência em [docs/evidence/03-arvore-msbuild.md](../evidence/03-arvore-msbuild.md). Limitações: Windows/Intel não executados, `.slnx`/solution filters e mapeamentos de plataforma não cobertos, retry reavalia todo o escopo.
