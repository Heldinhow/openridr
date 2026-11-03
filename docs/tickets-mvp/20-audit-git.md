# 20 — Auditoria visual: Git diff, caminhos e log

**What to build:** Tornar o diff por arquivo revisável e os nomes/histórico do Git legíveis sem truncamento destrutivo.

**Source:** `docs/UI-AUDIT.md` A3/M8/L5; requisito F6 #50–52 da `docs/SPEC-MVP-RIDER.md` e decisão F6.

**Blocked by:** nenhum.

**Status:** done — `bun run check`, `bun run build` e `tests/e2e/git.spec.ts` verdes; ver `docs/evidence/20-audit-git.md`.

- [x] Selecionar arquivo mostra diff em diálogo amplo, com linhas/cabeçalhos legíveis e rolagem horizontal quando necessária; o texto recebido não é transformado.
- [x] Caminhos de arquivos quebram em segmentos sem perder o nome completo; tooltip mantém o caminho integral.
- [x] Git log distribui hash, mensagem, autor, data e parents em colunas que podem quebrar linhas sem sobreposição; feedback de commit aparece uma vez.
- [x] Cabeçalho do painel Commit permanece “Commit”, sem branch; mantém o estilo comum de `panel-title`.
- [x] E2E cobre diff longo, caminho longo, log e feedback único; seleção seletiva/amend e testids existentes preservados.
- [x] Executar build/check e E2E Git aplicável; limitações registradas em `docs/evidence/20-audit-git.md`.

## Comments

- Capturas: `docs/evidence/ui-audit/50-git-commit.png`, `51-git-diff.png`, `53-git-log.png`.
- Não alterar comportamento de commit, branch, pull ou push; nenhuma operação remota foi validada pelo audit.
- Capturas headed Chrome, slowMo 300 ms, fora do repositório: `<tmp>/opencode/ticket20-git-diff.png` e `ticket20-git-log.png`.
