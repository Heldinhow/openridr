# 07 — F8: Settings

**What to build:** diálogo de Settings com busca e árvore, páginas Appearance, Keymap, Editor, Build e Version Control, aplicando ao vivo e persistindo no app home. Tema escuro explícito, claro indisponível com motivo.

**Blocked by:** 02

**Status:** ready-for-agent

- [ ] Settings abre pela barra e por atalho, com busca no topo e árvore lateral; buscar por nome leva à página/campo.
- [ ] Appearance mostra tema escuro selecionado, claro desabilitado com motivo, e zoom/densidade aplicados ao vivo.
- [ ] Keymap lista o perfil ativo, permite trocar entre "Rider (macOS)" e "IntelliJ", remapear uma ação, e resetar; o efeito é imediato ou no próximo reload, sempre declarado.
- [ ] Editor aplica família, tamanho e indentação/tabs ao CodeMirror ao vivo e a status bar reflete a indentação.
- [ ] Build mostra os SDKs detectados, o host de análise e permite Redetect; não instala SDK nem altera `global.json`.
- [ ] Version Control mostra o caminho do git em uso, somente leitura.
- [ ] Cancel descarta e Save persiste em `settings.json`; valor inválido é recusado com mensagem e não grava.
- [ ] `bunx tsc --noEmit`, `cargo test -p openridr-host` e a suíte Playwright completa passam; commit local `feat(F8): ...`.
