# 19 — Auditoria visual: saída e hierarquia de Build/Run/Tests/Debug

**What to build:** Ajustar os painéis de execução para que a informação primária (stdout do Run, árvore/resultados de testes e frames/locals) apareça sem precisar atravessar metadados; preservar logs completos com rolagem local.

**Source:** `docs/UI-AUDIT.md` A1/A2/L1/L2; decisões funcionais em `docs/decisions.md` tickets 11, 12, 14, 16 e 17. Este ticket é visual, não muda gates nem estados do host.

**Blocked by:** nenhum.

**Status:** ready-for-agent

- [x] Run torna a saída do programa visível na primeira área útil após executar; manter build metadata acessível sem empurrar stdout para fora da dobra.
- [x] Tests mostra árvore e resultados/status como conteúdo principal; Debug mostra frames e locals sem serem subordinados a metadados da sessão.
- [x] Logs extensos rolam localmente e controles/ações essenciais permanecem acessíveis.
- [x] Estados vazios “No program has run” e “No build has run” têm aparência informativa, não de controles clicáveis.
- [x] Corrigir textos duplicados/erros de pontuação observados em Debug sem mudar estados reportados.
- [x] E2E cobre layout/visibilidade em execução, testes e pausa de debug; preservar todos os testids.
- [x] Executar build, testes de unidade aplicáveis e E2E da fatia; registrar ambiente não verificado em `docs/implementation-notes.md` e neste ticket.

## Comments

- Capturas: `docs/evidence/ui-audit/25-run-output.png`, `61-tests-discovered.png`, `62-tests-results.png`, `81-debug-paused.png`, `83-debug-ended.png`, `18-editor-program.png`.
- Não adicionar uma nova tool window Run sem necessidade: primeiro reutilizar os painéis/slots existentes e manter o menor diff que exponha a saída.
- Verificação desta fatia e capturas novas fora do repositório: `docs/evidence/19-audit-run-tests-debug.md` e `<tmp>/opencode/ticket19-evidence/`.
