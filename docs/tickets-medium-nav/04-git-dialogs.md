# 04 — Foco e Esc nos diálogos Push/Update (d14)

**Fonte:** `docs/SPEC-MEDIUM-NAV-SEARCH.md` §7. **Achado:** QA-DEVFLOW d14.
**Bloqueado por:** nenhum. **Status:** ready-for-agent.

## Objetivo

Diálogos Push (`⇧⌘K`) e Update (`⌥⌘W`) abertos por atalho tomam foco, não deixam a digitação
cair no buffer oculto e fecham com Esc.

## Pontos de código

- `src/git.tsx` (`BranchWidget`, diálogos `push-dialog` e `update-dialog`, ~l. 528–590):
  - `autoFocus` no botão de confirmação (`push-confirm` / `update-confirm`), que já é o
    primeiro controle do rodapé; alternativa: `tabIndex={-1}` no `.modal` + ref/focus.
  - `onKeyDown` no `.modal`: `Escape` → `setDialog(null)` e `preventDefault`/`stopPropagation`.
  - Não fechar enquanto `dialogBusy` (uma operação em voo não deve perder o diálogo); Esc
    durante busy é ignorado.
- Não mudar o fluxo de confirmação, os gates nem os textos.

## Teste primeiro

Criar `tests/e2e/medium-nav-git-dialogs.spec.ts`. Copiar `fixtures/OrderDesk` para um temp,
`git init`, commit, criar um bare remote, `git remote add origin`, `git push -u origin main`
(para o Update ter upstream). Abrir o workspace confiado e `Program.cs`; focar o editor.

1. `Meta+Shift+K`: `push-dialog` visível; `push-confirm` focado; digitar `x` não altera o
   buffer (comparar `editor.innerText()`); `Escape` fecha.
2. `Meta+Alt+W`: `update-dialog` visível; `update-confirm` focado; `Escape` fecha.
3. Repetir o Push com o diálogo aberto e confirmar que o buffer continua sem `dirty-marker`.

Rodar o spec e ver falhar antes de implementar.

## Aceite

- [ ] Push e Update por atalho focam o confirmar; digitação não vai ao buffer.
- [ ] Esc fecha os dois diálogos; busy não é interrompido.
- [ ] Teste alvo verde; `bun run check` verde.
- [ ] Commit local `fix(git): focus and escape for push/update dialogs` (só caminhos deste ticket).
