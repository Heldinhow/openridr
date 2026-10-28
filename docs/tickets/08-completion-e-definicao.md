# 08: Completar e navegar definições entre projetos

**What to build:** Usar csharp-ls real para completion e go-to-definition no console e biblioteca referenciados, respeitando documento/posição/revisão e capacidades da sessão.

**Blocked by:** 05 — Instalar csharp-ls com confirmação e carregar análise.

**Status:** ready-for-agent

**Category:** enhancement

- [ ] Na fixture real, solicitar/aceitar completion em contexto atual e navegar definição console↔biblioteca a documento/posição fornecidos pelo servidor. Não inferir destino por texto nem implementar motor semântico próprio.
- [ ] Negociar capacidades e mostrar indisponível/parcial/carregando/falha conforme a sessão; host ausente, projeto não carregado ou resposta sem destino válido não bloqueiam edição nem fabricam resultado.
- [ ] Cancelamento, troca de documento/posição/revisão ou sessão invalida resposta pendente; resultado tardio não insere texto no lugar errado. Aplicar edição de completion somente a revisão válida sem sobrescrever buffer concorrente.
- [ ] Trust/revogação são verificados no host para início/reinício/solicitação aplicável; atalhos e comandos respeitam os mesmos gates. Proteção mínima de buffer de 01 permanece em navegação/abertura.
- [ ] Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), com operação por teclado, foco previsível, logs e limites de escopo evidentes.

## Mapeamento à SPEC

AC-12 (parcial: completion e definição; rename separado em 09), AC-01/07, AC-32/33. SPEC P0.3 e Integrações .NET. Não é preciso concluir rename, Build ou Debug para demonstrar esta fatia.

## Validação

UI Mac → csharp-ls real na fixture: completar referência à biblioteca e navegar definição entre projetos, registrando contexto/SDK/revisão. Boundary LSP: resposta tardia, capacidade ausente, cancelamento/desconexão, edição concorrente, CRLF/posições Windows e destino indisponível. Casos controlados não substituem E2E; registrar limitações por ambiente.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
