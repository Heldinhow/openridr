# 05 — d19: Problems com altura útil e uma única contagem

Status: ready-for-agent
Achado: QA-DEVFLOW-2026-11-02 §19.

## Aceite

- `defaultShellLayout.bottom.size` sobe de 280 para 320 px; o clamp de b15 continua valendo.
- Cabeçalho (`problems-counts`), barra de status (`status-diagnostics`) e rótulo da aba usam a mesma soma: diagnósticos atuais + problemas atuais de build + problemas atuais de descoberta de testes + projetos com avaliação falha.
- O texto "No analysis session has published diagnostics" só aparece quando não há nenhuma linha de nenhuma origem.
- A seção Build continua identificada como origem separada, mas o número não contradiz o cabeçalho.

## Teste

E2E: build real com 1 erro; cabeçalho e status dizem `1 error`, a aba diz `Problems (1)` e o aviso de "no analysis session" não existe. Teste de altura padrão: sem layout no localStorage, abrir Problems mede 320 px.
