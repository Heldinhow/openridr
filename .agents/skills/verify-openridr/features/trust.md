# 3. Trust por projeto

## Sub-features

- Pergunta uma vez por projeto: **Trust project**, **Open in Restricted mode**, **Cancel** (Esc = Cancel). A decisão fica em `<app home>/trusted.json` (caminho canônico → `trusted`|`restricted`); sem entrada = nunca decidido.
- Reabrir: `trusted` entra direto em Trusted; `restricted` ou indecidido entra em Restricted (sem prompt se já decidido).
- Cancel não deixa rastro (sem recente, sem decisão).
- Mudar depois: badge/status (`status-mode`), `trust-revoke`, menu ⋯ dos recentes (`trust.set` sem abrir).
- Gate: todo método executável (`build.start`, `terminal.*`, `lsp.start`, ...) responde `trust_required` sem trust; revogar para processos/builds em andamento.
- Diálogo: modal, página de trás inerte, foco preso (Tab cicla Trust → Restricted → Cancel).

## How to get to it (user POV)

Abrir qualquer projeto novo (feature 1) → diálogo `Trust "<nome>"?`; depois, indicador "Trusted mode"/"Restricted mode" na barra de status e escudo na barra direita.

## Driving it

- **A**: `bunx playwright test tests/e2e/trust-prompt.spec.ts` (sobe o próprio host em 4318 com `OPENRIDR_TRUST_STORE=on` e `OPENRIDR_HOME` temporário; é a prova de decisão persistida). Complementares: `status-mode`, `trust-sdk`, `welcome` (menu ⋯).
- **B** (provado em 2026-11-03): abrir o `.sln` (feature 1) → `browser.snapshot` mostra `trust-prompt-dialog` → `browser.capture` → `browser.click` em `button[data-testid="trust-prompt-trust"]`. Estado final: `cat <state>/home/trusted.json` (o `<state>` é impresso pelo `web.sh up`) contém `{"<state>/work/OrderDesk/OrderDesk.sln": "trusted"}`; `app.state` → `workspace.mode: "trusted"`, `trust.decision: "trusted"`, `trust.granted: true`; `status-mode` = "Trusted mode".
- **Regra de modelo** (rodar junto quando mexer em `crates/host/src/{lib,trusted}.rs`): `cargo test -p openridr-host --test trust_model` (sequências aleatórias fixas contra um modelo de referência; invariantes I1–I6 em `docs/VERIFICACAO-FORMAL.md`) e `cargo test -p openridr-host trusted`.
- **C**: `desktop.sh up`; o prompt deve aparecer na janela; `agent-desktop find --role button --name "Trust project"` e `click`.

Prova de que funciona: decisão no disco + estado do host + UI coerentes; reabrir (novo `Host` com o mesmo home) respeita a decisão; sem trust, uma chamada executável devolve `trust_required`.

## Gotchas

- O host compartilhado dos E2E desliga o store: um spec que reabre o mesmo projeto e espera "não perguntar" precisa de host próprio (como `trust-prompt.spec.ts`).
- A chave é o caminho **resolvido** do workspace (para pasta com `.csproj`, o caminho do `.csproj`), canonicalizado.
- `grant` sem `remember` vale só para a sessão; o botão da UI manda `remember: true`.
- `trusted.json` ilegível nunca é sobrescrito (`trusted_unreadable`): não "conserte" apagando sem avisar o usuário.
- Nunca aponte para o `~/Library/Application Support/OpenRidr` real: a decisão do usuário seria alterada.
