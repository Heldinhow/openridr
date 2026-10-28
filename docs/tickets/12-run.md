# 12: Executar a saída avaliada e cancelar processos

**What to build:** Executar o console de startup a partir de TargetPath MSBuild atual e de build válido identificado, transmitindo saída e deixando claro qual conteúdo salvo está rodando. Cancelamento observa a árvore de processos, não só um estado da UI.

**Blocked by:** 11 — Compilar contexto e navegar erros no output.

**Status:** ready-for-agent

**Category:** enhancement

- [ ] Run exige projeto executável/configuração/TFM explícitos e build válido bem-sucedido desse contexto. Build falho, avaliação/TargetPath inválido ou biblioteca startup bloqueiam launch com motivo e navegação/log pertinentes.
- [ ] Lançar saída gerenciada obtida da avaliação atual, nunca caminho presumido; expor comando, cwd/contexto, revisão salva/build e logs stdout/stderr/estado. Run não dispara restore implícito nem ignora SDK/regras do projeto.
- [ ] Buffer ou conteúdo salvo alterado depois do build recebe aviso; oferecer decisão explícita para salvar/reconstruir ou usar snapshot anterior identificado quando cabível. Nunca afirmar fonte atual verificado nem sobrescrever edições sem decisão.
- [ ] No console real, observar saída esperada e exit quando fornecido; preservar log após término/falha/cancelamento. Exit ausente é unknown, não sucesso presumido.
- [ ] Cancelar solicita término da árvore real, incluindo filhos; distinguir solicitação, resultado observado, falha/remanescentes e exit. Revogação bloqueia novas execuções no host e oferece confirmação de término mantendo processos visíveis enquanto pendente.
- [ ] Aplicar G1–G5 do [índice](README.md#gates-compartilhados-obrigatórios-desde-cada-ticket); Run permanece independente de debugger/flag e o atalho Ctrl+R Mac/Shift+F10 Windows respeita os mesmos gates.

## Mapeamento à SPEC

AC-03, AC-10 (TargetPath usado), AC-15 (snapshot Build anterior), AC-17 (Run/startup/contexto), AC-18 (bloquear/recuperar/Run console), AC-19 (decisão e identidade build/fonte), AC-20 (Run/árvore/exit), AC-01/07, AC-23/24 (parcial: Run independente de debug), AC-32/33.

## Validação

UI Mac → build/run real da fixture, alterar buffer e salvo após build, trocar contexto e comprovar saída/revisão. Boundary processos: filhos, cancelamento, falha de término, resposta tardia, ausência de exit e revogação direta/pendente. Testar paths com espaços e equivalente Windows real quando disponível; registrar lacunas sem substituir E2E por mocks nem aprovar release unilateral.

## Comments

- 2026-10-27: Quebra aprovada; publicação documental, implementação não autorizada.
