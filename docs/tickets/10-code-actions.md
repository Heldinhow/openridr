# 10: Oferecer somente code actions do contexto atual

**What to build:** Exibir e aplicar apenas ações efetivamente retornadas pelo csharp-ls para documento/posição/contexto atuais, protegendo revisões e mantendo diagnósticos pendentes até republicação.

**Blocked by:** 05 — Instalar csharp-ls com confirmação e carregar análise; 07 — Proteger buffers e resolver conflitos de disco.

**Status:** ready-for-agent

**Category:** enhancement

- [ ] Solicitar ações na posição/revisão atual, respeitar capacidades e oferecer só as retornadas. Resposta vazia/indisponível não cria quick fix com base isolada em código CS nem promete catálogo de ações.
- [ ] Antes de aplicar/resolver ação, revalidar documento, contexto, revisão e todas as edições envolvidas. Alteração concorrente de buffer/disco ou resposta tardia invalida a ação pendente sem sobrescrever conteúdo; cancelamento não modifica documentos.
- [ ] Aplicar somente operações suportadas/validadas; comando retornado não vira execução shell arbitrária. Host verifica trust e limites da operação, inclusive quando invocado diretamente; operação não suportada explica motivo/log em vez de êxito fictício.
- [ ] Após aplicação, preservar modificado/decisão de salvar e marcar snapshot anterior obsoleto; não remover diagnóstico como corrigido antes de publicação atual. Ausência de análise mantém edição.
- [ ] Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), com menu/ação acessível, foco/teclado e estados ingleses honestos.

## Mapeamento à SPEC

AC-13 (ações retornadas, aplicação e não antecipar correção), AC-11 (proteção de buffer), AC-15 (parcial: obsolescência após edição; apresentação Problems em 06), AC-01/07, AC-32/33. SPEC P0.3 e Integrações .NET.

## Validação

UI Mac com csharp-ls real: registrar ações realmente disponíveis e ausência sem quick fix fabricado; não exigir ação que a versão não retorna. Boundary controlada cobre ação/edit/command suportados e rejeitados, revisão alterada, cancelamento, resposta tardia e conflito. Se nenhuma ação real puder ser exercitada, registrar limitação E2E e evidência faltante, não aprovar com mocks; documentar por ambiente.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
