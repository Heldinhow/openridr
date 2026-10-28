# 17: Executar testes selecionados ou todos e preservar resultados

**What to build:** Executar somente a seleção válida ou todos os testes do escopo VSTest escolhido, exibindo resultados/detalhes realmente reportados e cancelamento com parciais sem transformar ausências em sucesso.

**Blocked by:** 16 — Descobrir testes VSTest com identidade e escopo.

**Status:** ready-for-agent

**Category:** enhancement

- [x] `Run all` executa todos do projeto/TFM/escopo escolhidos; `Run selected` apenas identidades selecionadas válidas. Validar filtros/identidade com ferramenta real, inclusive nomes de exibição duplicados. Sem seleção válida ou descoberta obsoleta, bloquear/solicitar nova seleção, nunca substituir por todos silenciosamente.
- [x] Exibir idle/running/completed/failed/cancelled e estado de resultado passed/failed/skipped conforme fonte; pendente/rodando só quando conhecidos. Vincular contagens/resultados a operação, contexto e revisão; não misturar execuções nem assumir fonte atual após edição.
- [x] Detalhes preservam mensagem, stack trace, stdout/stderr disponíveis e logs/resultados brutos. Output sem atribuição comprovada permanece na execução; navegar somente quando fonte fornece posição, caso contrário explicitar indisponibilidade e oferecer log, sem adivinhar pelo nome.
- [x] Cancelar execução mantém resultados parciais e operação cancelled; sem resultado não vira passed/skipped. Só atribuir cancelled ao teste se fonte disser. Separar falha de infraestrutura de asserção, conservar logs e permitir retry explícito.
- [x] Usar resultados estruturados disponíveis, incluindo TRX quando validado, sem supor que descobre testes ou impor logger/package adicional sem necessidade comprovada e decisão própria. Conservar gate MTP, trust e proibição de restore implícito no host, inclusive acesso direto.
- [x] Revogação, desconexão, troca de contexto e resultados tardios não contaminam execução atual; cancelamento observa árvore/exit conhecido ou unknown/remanescentes. Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), com controles/lista/detalhes acessíveis.

## Mapeamento à SPEC

AC-03, AC-27 (selected/all), AC-28 (resultados/detalhes/navegação), AC-29 (execução/cancelamento parcial/falhas), AC-30 (contagens por execução/revisão, sem alteração de packages), AC-31 (gate mantido), AC-01/07, AC-32/33. SPEC P0.7, sem cobertura/debug test/perfis.

## Validação

UI real Mac com fixture xUnit/VSTest: Run all e subset verificável, passed/failed/skipped, detalhes com/sem posição/output atribuível e teste longo cancelado preservando parciais. Boundary: seleção vazia/inválida/duplicada, filtros, resultado atrasado, TRX parcial/corrompido quando usado, infra falha, processo filho/exit ausente e revogação. Provar nenhum restore implícito/MTP VSTest. Registrar evidência ou limitação por ambiente sem converter mocks em aceitação E2E.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
- 2026-10-28: Implementado e provado em `docs/evidence/17-execucao-testes.md`. Run all real da fixture: 2 passed, 1 failed, 1 skipped (TRX `ok`); Run selected: 1 passed; cancelamento real do teste longo: `cancelled`, exit 143, 0 resultados (sem resultado vira passed/skipped); TRX parcial preservado no teste unitário; TRX ausente/exit 1 vira falha de infraestrutura; revogação sela. Limitações em Windows/Intel/CRLF e teorias sem evidência.
