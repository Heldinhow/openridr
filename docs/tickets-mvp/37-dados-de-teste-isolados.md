# 37 — Dados de teste isolados e higiene do repositório

Fonte: [SPEC-ALPHA3-DEBUG-TRUST](../SPEC-ALPHA3-DEBUG-TRUST.md), D3/D4.

- [x] `tests/e2e/environment.spec.ts` prova que `app.state.app.appHome === e2eHome` (host E2E nunca aponta para o app home real).
- [x] `package.json`: `host:http` usa `.scratch/host-home` quando `OPENRIDR_HOME` não está definido, para a ponte manual não escrever `trusted.json`/`recent.json` reais.
- [x] `.gitignore`: `.openchamber/`, `.playwright-mcp/`, `.scratch/design-ref/`, `.scratch/ui-shape/`, `docs/evidence/logs/*`.
- [x] Docs: `decisions.md`, `implementation-notes.md`, `AGENTS.md` (comando `host:http` isolado) e `docs/tickets-mvp/README.md` (índice 35–37).

Aceite: `bun run check`; `bunx playwright test tests/e2e/environment.spec.ts` verde; `git status --short` sem os caminhos de D4; `trusted.json`/`recent.json` reais com mtime inalterado após o E2E (verificar antes/depois).
