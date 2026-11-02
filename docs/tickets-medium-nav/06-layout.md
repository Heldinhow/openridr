# 06 — Find Usages sem sobrepor a faixa inferior, banner sem empurrar e tools drawer (b9, b8)

**Fonte:** `docs/SPEC-MEDIUM-NAV-SEARCH.md` §9–10. **Achados:** QA-BUGHUNT b9, b8.
**Bloqueado por:** nenhum. **Status:** ready-for-agent.

## Objetivo

Find Usages docado não rouba a altura da tool window inferior em janelas normais;
notificações não deslocam abas/editor; Workspace tools não estoura na horizontal.

## Pontos de código

- `src/styles.css`:
  - **b9 (Find Usages):** remover a regra
    `.workspace-view:not(.bottom-closed):has(.editor-popup) { grid-template-rows: ... 96px ... }`
    (linha ~588). O preview já tem `max-height: min(50%, 300px)` e cabe no editor; a faixa
    inferior mantém `--output-height`. Ajustar o comentário do ticket 32 que ficou obsoleto.
  - **b9 (banner):** `.workspace-view` passa a ter a linha 2 do grid em `0` e
    `.workspace-view > .workspace-notices` vira sobreposição:
    `position:absolute; top:100px; right:calc(34px + var(--tools-width, 0px) + 12px);
    width:min(520px, calc(100vw - 120px)); max-height:30vh; overflow-y:auto; z-index:20;`
    (o `.workspace-view` já é `position:relative`). Notificações não mudam a altura de
    `.editor-pane`/`.tabbar`.
  - **b8:** no drawer, `overflow-x:hidden`; em `.workspace-tools-body` e filhos diretos,
    `min-width:0`; em `code`, `pre`, `dd`, `p` e `summary` do drawer,
    `overflow-wrap:anywhere`; `pre` com `white-space:pre-wrap`. Nada de rolagem horizontal.
- `src/WorkspaceView.tsx`: sem mudança funcional; se necessário, apenas classes/estrutura.
- Atualizar `tests/e2e/editor-overflow.spec.ts`: a linha de notices não ocupa mais 168 px;
  recalcular a altura esperada do editor (700 − 52 − 0 − 294 − 4 − 28 = 322) e manter a
  prova de que o empty editor rola localmente.

## Teste primeiro

Criar `tests/e2e/medium-nav-layout.spec.ts`:

1. **b9 Find Usages:** abrir workspace confiado, `Program.cs`, aba Build aberta, rodar Find
   Usages (csharp-ls; `test.skip` se `tools.plan` não tiver `present`). Medir:
   - `.output` com altura ≥ 200 px (não 96);
   - `find-usages` e `.output` sem sobreposição (`popup.bottom <= output.top`);
   - as abas da faixa inferior continuam visíveis.
2. **b9 banner:** copiar a fixture, abrir workspace, apagar `src/OrderDesk.Core/Order.cs` do
   disco e clicar na linha da árvore: `app-log` visível; `tabbar`/`editor-pane` mantêm o
   mesmo `y` antes e depois (diferença ≤ 1 px).
3. **b8:** abrir Workspace tools e o install plan em 1440×900 e 1100×700; o drawer tem
   `scrollWidth <= clientWidth + 1` e `scrollLeft === 0`; o plano continua legível (campos e
   confirmar visíveis ou rolagem vertical, como no `install-plan.spec.ts`).

Rodar o spec e ver falhar antes de implementar.

## Aceite

- [ ] Find Usages não reduz a faixa inferior em 1440×900/1100×700.
- [ ] Banner não move abas/editor.
- [ ] Tools drawer sem overflow horizontal nos dois tamanhos.
- [ ] `install-plan.spec.ts` e `editor-overflow.spec.ts` continuam verdes (ajustados quando
      o contrato mudou).
- [ ] Teste alvo verde; `bun run check` verde.
- [ ] Commit local `fix(layout): docked preview, notices and tools drawer` (só caminhos deste ticket).
