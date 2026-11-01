# 02: Recentes com data e remoção (host)

**What to build:** A lista de recentes devolve nome-fonte de verdade com último acesso e a Trust decision de cada um, e permite remover uma entrada sem afetar a decisão. Formato antigo de `recent.json` continua legível.

**Blocked by:** 01

**Status:** ready-for-agent

- [x] `recent.json` guarda `{path, openedAt}`; lê a lista antiga (sem data); mais recente primeiro, sem duplicatas, teto de 30
- [x] `recent.list` devolve `{projects:[{path, openedAt?, decision}]}` omitindo caminhos que não existem; `recent.remove {path}`
- [x] A UI atual (toolbar/App) continua funcionando com a nova forma
- [x] `cargo test` cobre migração, remoção e decisão por entrada; E2E de recentes existentes passa; commit `feat(recent): ...`

## Comments

- 2026-11-01: implementado (`feat(recent)`). `openedAt` em epoch ms; `recent.remove` devolve a lista atualizada.
