# 02 — F2: Registro de ações e keymap

**What to build:** menus, botões de barra, faixas e a paleta passam a ler de um registro central de ações, com atalhos por perfil ("Rider (macOS)" padrão e "IntelliJ"), remap persistido, conflito avisado e motivo em ação indisponível. Inclui Find Action (⇧⌘A) e a storage mínima de settings usada pelo keymap.

**Blocked by:** 01

**Status:** done — F2 entregue; ações/keymap, Find Action e settings storage cobertos por spec novo, com tsc/cargo e specs de regressão verdes.

- [x] Toda ação de menu/barra/atalho do MVP é registrada com id estável, título, grupo, atalho padrão, disponibilidade e executar; nenhum menu monta item fora do registro.
- [x] ⇧⌘A abre Find Action, filtra por título/id/grupo, mostra o atalho atual e executa a ação com Enter.
- [x] O perfil "Rider (macOS)" é o padrão e produz os atalhos da spec (⌘T, ⇧⌘T, F12, ⌃F5, ⌥F5, ⇧F5, ⌘U, ⇧F12, ⌥↩, ⌥⇧⌘K, ⇧⌘K, ⇧Esc).
- [x] O perfil "IntelliJ" existe com os atalhos correspondentes; trocar o perfil em `settings.json` e recarregar muda os atalhos sem editar código.
- [x] Remap do usuário persiste no app home (`settings.json`), vence perfil e padrão, e pode ser resetado.
- [x] Duas ações no mesmo atalho geram aviso no registro, mantêm ambas listáveis e não quebram a execução.
- [x] Ação indisponível aparece desabilitada com motivo textual (trust, análise, build, fatia futura) na paleta e no menu.
- [x] Com foco no editor, ⌘/ (comentar), ⌘F, setas e Tab continuam sob controle do CodeMirror; o handler global ignora `defaultPrevented` e campos de texto.
- [x] `settings.read`/`settings.write` no host gravam JSON válido no app home, recusam payload inválido e não exigem trust.
- [x] `bunx tsc --noEmit`, `cargo test -p openridr-host` e a suíte Playwright completa passam; commit local `feat(F2): ...`.
