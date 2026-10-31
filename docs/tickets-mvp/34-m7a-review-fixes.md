# 34 — M7a: correções da revisão final

- [x] (A) Resumo/aviso dos popups docados não é mais ocultado em janelas baixas; só espaçamento/tamanho (`.editor-popup p`).
- [x] (B) `role="status"` do editor (`editor-lsp-status`) fica visível com preview aberto: uma linha, reticências, texto completo em `title`. Comentário CSS corrigido.
- [x] (C) Popups docados movidos para depois de `.editor-body` no DOM (`WorkspaceView.tsx`); `order:2` e o combinador `~` removidos → Tab segue a ordem visual.
- [x] E2E: summary "Nothing has been applied" visível, status visível, popup depois do editor, 1ª ação + Close/Apply/Cancel dentro do painel, editor ≥100 px (1100×700, Problems aberto). Vermelho sem a correção: `docs/evidence/logs/m7-ticket34-red.md`.
- [x] Nit Rust: `use super::*;` no topo de `mod tests` (`lib.rs`).
- [x] Docs: `decisions.md` (tetos 40%→50% como histórico, mudança também a 1440×900, ticket 34), tickets 31 (supersessões, sem caminho pessoal), `implementation-notes.md` (nota do `code.spec.ts:81` atualizada), `UI-AUDIT.md` (M7a resolvido / M7b aberto).

- [x] Follow-up: `conflict-banner` fica entre breadcrumb e editor (só deixou de ficar abaixo dos popups); `workspace.spec.ts` "detects a disk conflict…" verde. `.editor-popup p` em 0,75rem. Linha dupla/formatação dos hunks Rust corrigida. Vermelho isolado de status e de ordem DOM anexado ao log.

M7b segue aberto. Não verificado: reprova no `.app` (native-proof-4), Windows, Intel.
