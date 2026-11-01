# 36 — Confiar no próprio aviso de bloqueio

Fonte: [SPEC-ALPHA3-DEBUG-TRUST](../SPEC-ALPHA3-DEBUG-TRUST.md), D2.

- [x] `BuildPanel` recebe `trusted`/`onTrust` e mostra `build-trust` e `run-trust` dentro de `build-disabled-reason`/`run-disabled-reason` quando o motivo é Restricted mode; rótulo "Trust this project". Sem checkbox.
- [x] `DebugPanel` recebe `trusted`/`onTrust` e mostra `dap-trust` dentro de `dap-disabled-reason` no mesmo caso.
- [x] `WorkspaceView` implementa `grantTrust()` único: `trust.grant {remember: true}`, atualiza snapshot, limpa mensagem de sucesso e mostra erro `trust-grant-error`/mensagem se falhar (nome próprio, porque `trust-error` é o erro do popover de `TrustControls`). A escolha é persistida.
- [x] Nenhum indicador de confiança novo na barra superior; `status-mode` e menu "⋯" dos recentes continuam sendo os únicos lugares.
- [x] E2E `gates.spec.ts`: em Restricted, os três avisos aparecem com seus botões; clicar `build-trust` deixa `status-mode` em "Trusted mode" e os motivos de Restricted somem. Nenhum checkbox exigido.
- [x] Docs: `decisions.md` e `implementation-notes.md`.

Aceite: `bun run check`; `bunx playwright test tests/e2e/gates.spec.ts` verde; captura/evidência do aviso com o botão.

Revisão: o erro do grant é `trust-grant-error`, não `trust-error` — esse nome já é do erro do popover de `TrustControls`, e os dois podiam coexistir na tela. `gates.spec.ts` 10/10 depois da troca.
