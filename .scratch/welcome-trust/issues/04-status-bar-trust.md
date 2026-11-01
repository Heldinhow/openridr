# 04: Indicador fora da barra de cima; modo na barra de status

**What to build:** A barra de cima não mostra mais Trusted/Restricted nem banner; o modo aparece discreto na barra de status e dali se muda a decisão do projeto aberto.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] `mode-badge`, `trust-banner`, `trust-prompt*` removidos da toolbar; nada reaparece ao reabrir
- [ ] `status-mode` é botão com popover (Trust/Restrict via `trust.set`), Esc e retorno de foco
- [ ] Todos os specs que usavam `mode-badge`/`trust-banner` migrados para `status-mode`/prompt (gates, trust-sdk, workspace, ui-layout, zy-restore-interrupt, restore, code, csharp-ls, zz-acceptance)
- [ ] Exceção de testids já registrada em decisions.md; `bun run check` e specs afetados verdes; commit `refactor(trust): ...`

## Comments
