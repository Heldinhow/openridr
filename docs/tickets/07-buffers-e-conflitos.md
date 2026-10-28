# 07: Proteger buffers e resolver conflitos de disco

**What to build:** Completar o ciclo de decisões para buffers modificados e alterações externas em disco, mantendo conteúdo recuperável em fechamento, troca de workspace e aplicação futura de edições semânticas. A segurança mínima já existe em 01.

**Blocked by:** 01 — Abrir solução ou projeto em modo restrito.

**Status:** ready-for-agent

**Category:** enhancement

- [ ] Fechar aba/app, substituir documento ou trocar workspace com edição oferece salvar, preservar/cancelar e descarte somente explícito. Cancelar não altera disco/buffer nem prossegue o fechamento; falha ao salvar mantém conteúdo e explica erro.
  - Nota 2026-10-28: aba e workspace provados por E2E (Save/Save all/Keep editing/Discard, falha de escrita com buffer e diálogo preservados). App: `beforeunload` provado no Chrome; `onCloseRequested`/`destroy()` implementados e compilados, mas a janela nativa não foi dirigida (TCC) — item permanece aberto por essa lacuna.
- [x] Detectar edição externa concorrente antes de sobrescrever; oferecer comparar, recarregar mediante decisão explícita ou preservar edições. Mostrar qual versão está no buffer/disco, sem substituir silenciosamente uma pela outra.
- [x] Abas/breadcrumb/modificado e revisão do documento permanecem coerentes após salvar, recarregar, preservar ou cancelar; mudanças de disco/contexto invalidam edições pendentes que dependam de snapshot antigo.
  - Nota 2026-10-28: troca de disco invalidada pelo conflito proativo; edições semânticas (09/10) ainda não existem e devem usar o mesmo `expected_hash`.
- [x] A proteção é válida sem solução/SDK/LSP e também quando host/serviço falha; mudanças concorrentes durante confirmação/comparação são revalidadas antes da gravação. Edições semânticas futuras não contornam a proteção.
- [x] Troca/fechamento não oculta processos remanescentes ou confirmações de término de 02; preservar arquivos não significa encerrar serviços. Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket) à superfície disponível, sem exigir LSP para esta fatia.

## Mapeamento à SPEC

AC-11 (completa conflitos/fechamento; edição mínima em 01), AC-12/13 (parcial: proteção/revisão para consumidores 09/10), AC-19 (parcial: decisões sobre buffer; build/Run em 11/12), AC-32/33. SPEC P0.3 e Estado, erros e processos.

## Validação

UI real: editar, alterar externamente, comparar, recarregar/preservar/cancelar, fechar e trocar workspace; conferir conteúdo em disco e buffer após cada decisão. Boundary: falha de escrita/permissão, nova alteração durante diálogo, paths com espaços e CRLF; asserts de nenhuma perda/sobrescrita silenciosa. Registrar evidência por ambiente, sem criar teste/fixture nesta tarefa documental.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
- 2026-10-28: Implementado em commit local `feat(buffers)`; evidência em [07-buffers-e-conflitos](../evidence/07-buffers-e-conflitos.md). Aba/workspace/app cobertos em UI restrita; fechamento nativo Tauri só compilado (TCC), por isso o primeiro item segue aberto. Windows/Intel e CRLF não executados.
