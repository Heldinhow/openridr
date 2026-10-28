# 09: Renomear entre projetos com prévia válida

**What to build:** Renomear símbolo entre console e biblioteca via csharp-ls, revisando o conjunto real de mudanças antes de aplicar, com cancelamento sem efeitos e rejeição de snapshot obsoleto.

**Blocked by:** 05 — Instalar csharp-ls com confirmação e carregar análise; 07 — Proteger buffers e resolver conflitos de disco.

**Status:** ready-for-agent

**Category:** enhancement

- [x] Solicitar rename apenas com capacidade/contexto atual; apresentar prévia por documento das edições retornadas pelo LSP, incluindo referências console↔biblioteca da fixture. Não fabricar rename textual quando o servidor não puder responder.
  - Nota 2026-10-28: sessão real csharp-ls 0.28.0 anuncia `renameProvider` com `prepareProvider`; `prepareRename` aceitou 2:20 e `textDocument/rename` devolveu 3 documentos/5 edições (Core/Order.cs, Cli/Program.cs, Tests/OrderTests.cs). O host recusa `WorkspaceEdit` com operação de arquivo, URI fora do workspace, intervalo inválido ou arquivo ilegível; não há fallback textual.
- [x] Cancelar prévia não modifica documento, buffer ou disco. Aplicar exige decisão explícita e valida novamente todas as revisões/documentos envolvidos, inclusive arquivos ainda não abertos e conflitos em disco.
  - Nota 2026-10-28: E2E cancela com revisão e três arquivos byte-idênticos; Apply re-lê todos os documentos no host, exige revisão/conteúdo/conflito dos buffers e hash de disco dos não abertos antes de uma única atualização de abas; não grava nada.
- [x] Se qualquer documento mudar durante solicitação/prévia/confirmação, invalidar o conjunto pendente e rejeitar aplicação obsoleta; não aplicar silenciosamente só parte de uma prévia inválida nem perder edições locais. Falha de aplicação mantém alterações recuperáveis e informa o que ocorreu.
  - Nota 2026-10-28: E2E cobre arquivo não aberto alterado no disco, buffer de origem alterado, resposta tardia após cancelar com servidor pausado e revogação de trust; em todos a prévia permanece visível com motivo e nenhum buffer/arquivo muda.
- [x] Atualizar revisões/modificado conforme alterações realmente aplicadas; salvar continua explícito e protegido por 07. Diagnósticos anteriores não viram corrigidos só porque o rename foi aplicado.
  - Nota 2026-10-28: abas abertas ganham `revision+1` com `savedRevision` inalterado; arquivos não abertos viram abas `1/0` com `hash` base; o save explícito subsequente gravou somente `Core/Order.cs` e as outras duas abas continuaram modificadas. A UI diz que diagnósticos não são marcados corrigidos.
- [x] Revogação/troca de workspace/sessão bloqueia operação pendente no host e impede resposta tardia de alterar outro contexto. Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), com prévia navegável por teclado e log de falha.
  - Nota 2026-10-28: `lsp.rename` está em `EXECUTABLE_METHODS` e responde `trust_required` após revogação; a UI rejeita prévia com sessão/workspace/contexto trocados. Shift+F6 e o botão focável “Rename symbol…” abrem o prompt; a prévia é um painel com foco no título e ações por teclado. Troca de workspace não foi dirigida por E2E (a view é desmontada); o gate de contexto do host e o descarte tardio têm teste unitário/boundary.

## Mapeamento à SPEC

AC-12 (parcial: rename, prévia, cancelar e concorrência; completion/definição em 08), AC-11 (preservação dos buffers), AC-15 (não antecipar diagnóstico atual), AC-01/07, AC-32/33. SPEC P0.3 e contratos de revisão.

## Validação

UI real Mac: prévia console↔biblioteca, cancelar sem diff de conteúdo, aplicar em revisão válida e verificar referências. Boundary LSP/arquivo: editar buffer/arquivo em disco durante prévia, resposta tardia, falha de escrita e desconexão; provar rejeição integral da prévia obsoleta e preservação de dados. Evidência por ambiente com versões/contexto/revisão/log; nenhum rename simulado substitui servidor real.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
- 2026-10-28: Implementado e verificado em macOS 27.2 arm64 com csharp-ls 0.28.0 real. `lsp.rename` usa `prepareRename` quando anunciado e `textDocument/rename`, com timeout de 5 s, trust/capability/documento e descarte de resposta de outra sessão/versão; o `WorkspaceEdit` é convertido por UTF-16 no host em prévia por documento (`baseContent`/`newContent`/hashes/revisões), recusando o conjunto inteiro fora do workspace, intervalo inválido, arquivo ilegível ou acima dos caps. Aplicar é só em buffers, revalida revisões/hashes (inclusive não abertos) e nunca grava. UI: Shift+F6/botão focável, prompt acessível, prévia navegável, Cancelar inerte, staleness integral e motivo textual. Evidência: [docs/evidence/09-rename.md](../evidence/09-rename.md). Windows x64, macOS Intel e CRLF não executados; sem `$/cancelRequest`.
