# 01: Trust decision persistida por projeto (host)

**What to build:** Um projeto aberto pela primeira vez não tem Trust decision; depois que o usuário decide (trusted ou restricted) a decisão sobrevive a reinícios e o host a devolve no snapshot. Projetos já confiados no formato antigo continuam confiados. Ver spec: Trust store, métodos novos, Abertura.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] `trusted.json` é um mapa caminho→decisão; lê o formato lista como `trusted`; arquivo ilegível não é sobrescrito por leitura
- [x] `trust.get`, `trust.set` (qualquer caminho; no workspace aberto aplica/revoga com as garantias existentes) e `trust.revoke` grava `restricted`
- [x] `workspace.open` só aplica trust se a decisão gravada é `trusted`; snapshot traz `trust.decision` (trusted|restricted|null)
- [x] `cargo test` cobre migração, ilegível, set/get, revoke→restricted, store off; `bun run check` verde
- [x] decisions.md/implementation-notes.md atualizados; commit `feat(trust): ...`

## Comments
- 2026-11-01: implementado no host (trusted.rs, lib.rs, testes). cargo test do crate com 2 falhas de ambiente alheias ao ticket; bun run check verde.
