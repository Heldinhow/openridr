# 06: Navegar diagnósticos por documento e revisão

**What to build:** Receber diagnósticos reais csharp-ls em Problems, navegar à posição conhecida e distinguir revisão coberta de conteúdo editado. Completar a primeira fatia 01–06 sem depender de Build, debugger ou tickets posteriores.

**Blocked by:** 05 — Instalar csharp-ls com confirmação e carregar análise.

**Status:** ready-for-agent

**Category:** enhancement

- [ ] Mostrar severidade da fonte/configuração, código, mensagem, posição quando conhecida, documento, escopo/sessão e revisão; separar origem `LSP · csharp-ls` de categoria Compiler/Analyzer/Unknown. Sem inferir Analyzer apenas pelo nome/código nem prometer cobertura não observada.
- [ ] Clicar diagnóstico com posição abre documento e posição corretos; sem posição, oferecer mensagem/log sem inventar navegação. Documento alterado não recebe navegação/estado como se snapshot velho fosse atual.
- [ ] Editar marca diagnóstico anterior obsoleto/`Awaiting diagnostics`, sem declarar correção. Publicação vazia atual permite `No problems reported` apenas no documento/revisão cobertos; restore parcial e analyzers não verificados continuam explícitos.
- [ ] Guardar snapshots por origem/contexto/revisão sem mesclar silenciosamente; chegada tardia de sessão/documento/contexto anterior não limpa o atual. A estrutura permite contribuição Build identificada em 11, mas nenhum build é pré-requisito desta entrega.
- [ ] Falha, espera, reinício LSP e revogação preservam logs/edição e deixam diagnósticos antigos identificados; não converter ausência de servidor/publicação em solução limpa.
- [ ] Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), incluindo teclado/foco/nome acessível na lista e detalhes, sem depender só de cor.

## Mapeamento à SPEC

AC-14 (parcial: LSP, estrutura e origem/categoria; Build em 11), AC-15 (parcial: revisão e documento LSP; snapshot Build em 11/12), AC-13 (parcial: não antecipar correção, consumidor em 10), AC-01/07, AC-32/33. SPEC P0.3. A união 01–06 já demonstra árvore MSBuild e diagnósticos reais.

## Validação

Pela UI Mac, na fixture real provocar diagnóstico, navegar, editar e observar obsolescência até republicação; documento sem problemas não limpa a solução. Boundary LSP: publicações vazias/tardias, revisão trocada, posição desconhecida, CRLF/posições Windows, desconexão e cancelamento/reinício. Evidência real csharp-ls é obrigatória; mensagens controladas só cobrem limites. Registrar SDK/contexto/revisão/log e limitações por ambiente.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
