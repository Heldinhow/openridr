# 37 — Dados de teste isolados e higiene do repositório

Fonte: [SPEC-ALPHA3-DEBUG-TRUST](../SPEC-ALPHA3-DEBUG-TRUST.md), D3/D4.

- [x] `tests/e2e/environment.spec.ts` prova que `app.state.app.appHome === e2eHome` (host E2E nunca aponta para o app home real).
- [x] `package.json`: `host:http` usa `.scratch/host-home` quando `OPENRIDR_HOME` não está definido, para a ponte manual não escrever `trusted.json`/`recent.json` reais.
- [x] `.gitignore`: `.openchamber/`, `.playwright-mcp/`, `.scratch/design-ref/`, `.scratch/ui-shape/`, `docs/evidence/logs/*`.
- [x] Docs: `decisions.md`, `implementation-notes.md`, `AGENTS.md` (comando `host:http` isolado) e `docs/tickets-mvp/README.md` (índice 35–37).

- [x] `crates/host/src/tools.rs::app_home()` também isola o `cargo test`: `OPENRIDR_HOME`, depois binário de teste (`cfg!(test)` ou `target/*/deps`) → `std::env::temp_dir()/openridr-test-home-<pid>`, senão o app home real. `crates/host/tests/host.rs` prova o caminho (`app_home_never_points_at_the_real_application_support_directory`).
- [x] `src/WorkspaceView.tsx`: o erro do grant do aviso é `trust-grant-error` (o `trust-error` do popover de `TrustControls` não é reusado).

Aceite: `bun run check`; `bunx playwright test tests/e2e/environment.spec.ts` verde; `git status --short` sem os caminhos de D4; `trusted.json`/`recent.json` reais com mtime inalterado após o E2E (verificar antes/depois).

Revisão: `cargo test -p openridr-host --no-fail-fast` agora deixa `trusted.json`/`recent.json` reais com mtime `1793569994`, 17324/4287 bytes e SHA-256 `1d921185d2e8f159…`/`1cd68d5fe3414205…` **iguais** antes e depois; os testes escrevem em `…/T/openridr-test-home-<pid>/`. A única falha é a de baseline `file_access_is_confined_to_the_workspace`. O comando `host:http` com expansão POSIX de shell continua sem verificação em Windows.
