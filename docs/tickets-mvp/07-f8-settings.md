# 07 — F8: Settings

**What to build:** diálogo de Settings com busca e árvore, páginas Appearance, Keymap, Editor, Build e Version Control, aplicando ao vivo e persistindo no app home. Tema escuro explícito, claro indisponível com motivo.

**Blocked by:** 02

**Status:** done — `settings.spec`, `actions.spec`, `shell.spec` e `zz-acceptance.spec` verdes; `bunx tsc --noEmit` e `cargo test -p openridr-host` verdes.

- [x] Settings abre pela barra e por atalho, com busca no topo e árvore lateral; buscar por nome leva à página/campo.
- [x] Appearance mostra tema escuro selecionado, claro desabilitado com motivo, e zoom/densidade aplicados ao vivo.
- [x] Keymap lista o perfil ativo, permite trocar entre "Rider (macOS)" e "IntelliJ", remapear uma ação, e resetar; o efeito é imediato ou no próximo reload, sempre declarado.
- [x] Editor aplica família, tamanho e indentação/tabs ao CodeMirror ao vivo e a status bar reflete a indentação.
- [x] Build mostra os SDKs detectados, o host de análise e permite Redetect; não instala SDK nem altera `global.json`.
- [x] Version Control mostra o caminho do git em uso, somente leitura.
- [x] Cancel descarta e Save persiste em `settings.json`; valor inválido é recusado com mensagem e não grava.
- [x] `bunx tsc --noEmit`, `cargo test -p openridr-host` e a suíte Playwright completa passam; commit local `feat(F8): ...`.

## Comments

- O diálogo é global e não tem override por projeto; o rascunho aplica ao vivo (zoom, densidade, fonte/indentação do CodeMirror e remap de teclas) e Cancel restaura o snapshot da abertura. Só Save chama `settings.write`.
- A busca filtra páginas e campos pelo nome, escolhe a página quando o acerto é inequívoco e marca o campo correspondente; sem resultados a árvore diz "No setting matches".
- Keymap reusa o `KeymapPanel` extraído do `KeymapDialog` da F2 (mesmos `data-testid`, perfil, remap, reset e conflitos); o diálogo standalone continua no menu ••• e segue persistindo na hora.
- Editor usa compartments do CodeMirror (`EditorState.tabSize`, `indentUnit`, tema de fonte), então mudanças valem sem remontar a view; a status bar lê a mesma configuração (`4 spaces`, `Tabs (2)`).
- `git.info` (novo, com trust) resolve o `git` do PATH e roda `git --version` somente para exibir caminho/versão; nenhum repositório é aberto e a página é read-only.
- Host valida `appearance` (zoom 80–150, densidade) e `editor` (família, 8–32 px, indentação 1–12, tabs) antes de substituir o arquivo; payload inválido mantém o `settings.json` anterior.
- E2E: `tests/e2e/settings.spec.ts` cobre abrir por botão e ⌘,, busca, páginas, Cancel, valor inválido, persistência e reload, remap no rascunho, Redetect sem tocar `global.json` e caminho do git read-only.
