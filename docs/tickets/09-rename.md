# 09: Renomear entre projetos com prévia válida

**What to build:** Renomear símbolo entre console e biblioteca via csharp-ls, revisando o conjunto real de mudanças antes de aplicar, com cancelamento sem efeitos e rejeição de snapshot obsoleto.

**Blocked by:** 05 — Instalar csharp-ls com confirmação e carregar análise; 07 — Proteger buffers e resolver conflitos de disco.

**Status:** ready-for-agent

**Category:** enhancement

- [ ] Solicitar rename apenas com capacidade/contexto atual; apresentar prévia por documento das edições retornadas pelo LSP, incluindo referências console↔biblioteca da fixture. Não fabricar rename textual quando o servidor não puder responder.
- [ ] Cancelar prévia não modifica documento, buffer ou disco. Aplicar exige decisão explícita e valida novamente todas as revisões/documentos envolvidos, inclusive arquivos ainda não abertos e conflitos em disco.
- [ ] Se qualquer documento mudar durante solicitação/prévia/confirmação, invalidar o conjunto pendente e rejeitar aplicação obsoleta; não aplicar silenciosamente só parte de uma prévia inválida nem perder edições locais. Falha de aplicação mantém alterações recuperáveis e informa o que ocorreu.
- [ ] Atualizar revisões/modificado conforme alterações realmente aplicadas; salvar continua explícito e protegido por 07. Diagnósticos anteriores não viram corrigidos só porque o rename foi aplicado.
- [ ] Revogação/troca de workspace/sessão bloqueia operação pendente no host e impede resposta tardia de alterar outro contexto. Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), com prévia navegável por teclado e log de falha.

## Mapeamento à SPEC

AC-12 (parcial: rename, prévia, cancelar e concorrência; completion/definição em 08), AC-11 (preservação dos buffers), AC-15 (não antecipar diagnóstico atual), AC-01/07, AC-32/33. SPEC P0.3 e contratos de revisão.

## Validação

UI real Mac: prévia console↔biblioteca, cancelar sem diff de conteúdo, aplicar em revisão válida e verificar referências. Boundary LSP/arquivo: editar buffer/arquivo em disco durante prévia, resposta tardia, falha de escrita e desconexão; provar rejeição integral da prévia obsoleta e preservação de dados. Evidência por ambiente com versões/contexto/revisão/log; nenhum rename simulado substitui servidor real.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
