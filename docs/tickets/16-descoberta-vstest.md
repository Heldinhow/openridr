# 16: Descobrir testes VSTest com identidade e escopo

**What to build:** Selecionar projeto/TFM e descobrir testes reais através do adapter VSTest já configurado no projeto, preservando identidade para seleção posterior e distinguindo vazio, adapter ausente, falha e MTP.

**Blocked by:** 04 — Restaurar explicitamente e recarregar contexto.

**Status:** ready-for-agent

**Category:** enhancement

- [ ] Descoberta real encontra testes xUnit da fixture pelo adapter VSTest do projeto; escopo/projeto/TFM, comando/contexto/revisão e identidade de cada teste são explícitos. Não tratar display name isolado como identidade nem prometer todos os frameworks/adapters.
- [ ] Separar idle, discovering, completed, empty, failed e cancelled, com log/retry e operação identificada. Cancelar preserva descoberta parcial; resposta tardia de outro contexto/operação não altera seleção atual.
- [ ] Diferenciar nenhum teste, adapter ausente e falha de descoberta/infraestrutura com orientação própria. Não instalar/editar packages silenciosamente, não tratar TRX como mecanismo de descoberta nem ausência de resultado como sucesso.
- [ ] Detectar MTP antes de emitir VSTest e mostrar exatamente `Microsoft.Testing.Platform is not supported in this version. Use the terminal.` Não converter projeto nem executar comando VSTest; indicar terminal complementar sem depender de 15 para entregar o bloqueio/mensagem.
- [ ] Host exige trust e contexto, sem restore implícito na descoberta/testes; dependências faltantes apontam para restore explícito. Revogação/cancelamento observam processo/árvore e preservam resultados/logs, com exit unknown quando necessário.
- [ ] Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket), com árvore/lista/seleção acessíveis e contagens ligadas à operação/revisão.

## Mapeamento à SPEC

AC-03, AC-27 (parcial: descoberta real/escopo/identidade; executar selected/all em 17), AC-29 (parcial: cancelar descoberta e falha infra), AC-30 (vazio/adapter/falha e contagens descoberta), AC-31 (MTP e TRX), AC-01/07, AC-32/33. SPEC P0.7 e Integrações .NET.

## Validação

UI Mac → ferramentas reais → fixture xUnit/adapter VSTest: descobrir e selecionar testes identificáveis. Na implementação preparar casos vazio, sem adapter, falha e MTP; observar mensagem exata e ausência de VSTest para MTP e nenhum restore/package edit oculto. Boundary: nomes duplicados, filtros/identidades ambíguos, cancelamento com parciais, troca TFM/revisão, resposta tardia e revogação direta. Registrar comandos/logs/exit e limitações por ambiente; TRX/mock não prova descoberta real.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
