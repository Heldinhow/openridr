# 25 — Auditoria visual: Settings e Keymap

**What to build:** Tornar o keymap uma tabela compacta e remover ruído de caminhos absolutos/settings; explicitar a ausência de tema claro.

**Source:** `docs/UI-AUDIT.md` M4/M11/L8; F2 #16–17 e F8 #65–69; decisão F8.

**Blocked by:** nenhum.

**Status:** done — `bun run check`, `settings.spec.ts` (5/5) e teste de keymap (1/1) verdes; evidência em `docs/evidence/25-audit-settings-keymap.md`.

- [x] Keymap apresenta cabeçalhos e linhas tabulares compactas (ação, grupo/atalho, ações); remove rótulos duplicados e Reset global sem contexto, preservando remap/conflito/reset.
- [x] Settings não expõe caminho absoluto de settings.json repetido em páginas; VCS continua mostrando caminho do git read-only mesmo em Restricted mode, com trust exigido para consulta conforme contrato.
- [x] Appearance comunica tema escuro atual e tema claro indisponível com motivo, sem select de uma única opção que pareça editável.
- [x] Responsividade em 1100×700 e foco/nomes acessíveis preservados; não remover testids ou comportamento de persistência.
- [x] E2E cobre keymap, estado Restricted/Trusted de VCS e Appearance; executar build e E2E aplicável, anotando o que não foi verificado.


## Comments

- Capturas: `44-settings-keymap.png`, `45-keymap-conflict.png`, `89-settings-1100.png`, `12-restricted-settings-vcs.png`, `41-settings-appearance.png`.
- Tema claro permanece fora do MVP por decisão explícita; este ticket só melhora a representação da indisponibilidade.
