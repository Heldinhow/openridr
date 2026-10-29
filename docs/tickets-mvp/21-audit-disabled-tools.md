# 21 — Auditoria visual: ações desabilitadas e planos de ferramentas

**What to build:** Fazer estados indisponíveis parecerem indisponíveis e expor os campos materiais do plano de instalação antes da confirmação.

**Source:** `docs/UI-AUDIT.md` A4/A5/M1/M6; `docs/SPEC-MVP-RIDER.md` F1/F2, SPEC P0 §§98,100; decisão do ticket 05 em `docs/decisions.md`.

**Blocked by:** nenhum.

**Status:** done — build e E2E focado 3/3 verdes; ver `docs/evidence/21-audit-disabled-tools.md`.

- [x] `:disabled` em botões primary/danger, inputs e selects tem tratamento visual consistente que não imita ação habilitada; motivo textual permanece disponível/acessível.
- [x] Aplicar o tratamento a trust, install/confirm, campos read-only de run config, terminal em execução e itens de menu/contexto.
- [x] Plano csharp-ls e netcoredbg expõe comando, destino, staging, permissões/limites e integridade/digest antes de `Confirm and install`; confirmação não aparece isolada antes dos detalhes.
- [x] Workspace tools não corta cards/controles; rolagem/altura não oculta ações nem rouba altura desnecessária do editor.
- [x] E2E verifica bloqueios e conteúdo dos planos com flag experimental off/on, sem executar instalação adicional; preservar gates e testids.
- [x] Executar build e E2E aplicável; registrar limitações reais.

## Comments

- Capturas: `03-toolchain-restricted.png`, `22a-install-plan.png`, `38-run-config-default.png`, `71-editor-context-menu.png`, `20-workspace-tools.png`, `78-debug-plan.png`.
- A auditoria não exercitou instalação nova de ferramenta; testes devem usar os seams existentes e não baixar/instalar por conveniência.
- Implementação/verificação: ver `docs/evidence/21-audit-disabled-tools.md`; capturas novas não foram adicionadas ao commit.
