# Tickets — Abrir do Finder e do terminal

Fonte: `docs/SPEC-OPEN-WITH.md`. Decisões: entrada de 2026-11-02 em `docs/decisions.md`.

| # | Ticket | Bloqueado por |
|---|--------|---------------|
| 01 | Caminho externo e `.slnx` no host | — |
| 02 | Bundle e recepção no shell Tauri | 01 |
| 03 | Launcher `openridr`, docs e evidência | 02 |

Ordem de commit: 01, 02, 03. Cada ticket roda os testes que declara e commita só os próprios
caminhos (`git commit --only -- <paths>`). Sem push, remote ou PR.
