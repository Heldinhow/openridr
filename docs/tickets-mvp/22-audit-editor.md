# 22 — Auditoria visual: popups, rename e ações do editor

**What to build:** Aproximar code actions/Find Usages do cursor e tornar preview/controles de rename e toolbar do editor utilizáveis em alturas/larguras reais.

**Source:** `docs/UI-AUDIT.md` A6/M7/M13/L6/L9; F1 #10 e F7 #59–62 da SPEC; decisões F7 e ticket 09/10.

**Blocked by:** nenhum.

**Status:** completed

- [x] Code actions e Find Usages abrem como popups próximos à posição do editor (**superseded: painel docado abaixo do editor, ver `decisions.md` “M7: previews docados” e ticket 31**), com ajuste para não sair dos limites da viewport.
- [x] Preview de rename mantém `Apply`/`Cancel` visíveis sem rolar faixa oculta; campo inline destaca o identificador e Enter/Esc mantêm o contrato existente.
- [x] Cancelar rename limpa a mensagem de status; mensagens não reaparecem em Debug/Find Usages sem relação.
- [x] Em 1100×700, Rename, Code Actions e Save continuam nomeados e acessíveis (visível ou tooltip/aria-label coerente), sem encostar/cortar a borda.
- [x] Remover contadores internos de revisão/salvo do breadcrumb, mantendo indicação clara de buffer modificado.
- [x] Ícones de rail já tinham nome acessível/foco global; removidos glifos decorativos redundantes da árvore, mantendo `aria-expanded`.
- [x] E2E cobre bounds do popup/controles em viewport estreita e estado do buffer; semântica LSP e testids existentes preservados.
- [x] Build e E2E visual isolado executados; limites de verificação registrados abaixo.

## Comments

- Capturas: `72-code-actions.png`, `74-rename-preview.png`, `73-rename-inline.png`, `75-find-usages.png`, `88-layout-1100.png`, `18-editor-program.png`.
- Não redesenhar/alterar operações LSP: preview ainda revalida contexto, buffers e disco conforme decisões existentes.
- Verificado: `bun run build`; `tests/e2e/editor-overflow.spec.ts` em Chrome headed, slowMo 300 ms (2/2). O fluxo real `csharp-ls.spec.ts` e a suíte E2E completa não foram executados; os logs/capturas de evidência já estavam modificados pelo usuário e não foram sobrescritos. Windows/macOS Intel e janela Tauri nativa não verificados.
