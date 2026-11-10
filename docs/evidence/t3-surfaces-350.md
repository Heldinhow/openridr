# Paleta única no app inteiro — #350

Escopo: promover a linguagem visual aprovada na #349 (PR #351, squash `16e8b16`) do workspace para
welcome, setup, trust, menus, diálogos, configurações e painéis. Referência e limites da comparação
com o T3 Code estão em [t3-workspace-349.md](t3-workspace-349.md); este documento cobre só a expansão.

## O que mudou

- Os tokens `--workspace-*` e o alias em `.workspace-view` saíram. A paleta passou a valer no `:root`:
  `--bg` `#090909`, `--panel` `#101010`, `--raised` `#1b1b1b`, `--chrome` `#050505`, `--select`
  `#282828`, `--select-row` `#262626`, `--select-row-focus` `#343434`, aba ativa `#252525`/`#3a3a3a`.
- `.chrome` (barra do setup/welcome) e `.toolbar` usam `--chrome`, como a barra do workspace.
- `.menu-content` passou a `--raised`, igual aos demais popovers e diálogos, para separar do chrome preto.
- O terminal usa `#30466b` como seleção, no lugar do azul antigo do Rider.

## Capturas

Chromium web, 1440×900, fixture `OrderDesk` copiada para `/tmp` (sem caminho pessoal):

- [welcome](t3-surfaces-350/welcome.webp) — tela inicial, navegação lateral e lista de recentes
- [trust prompt](t3-surfaces-350/trust-prompt.webp) — diálogo modal sobre o welcome
- [settings](t3-surfaces-350/settings.webp) — diálogo de configurações
- [menu](t3-surfaces-350/menu.webp) — menu suspenso da barra

## Verificação

- `bun run build`: passou. `bun test tests/unit`: 156 passaram.
- `ui-layout.spec.ts`: 7 passaram. O teste de paleta agora exige a **mesma** paleta no welcome e no
  workspace (`#090909`/`#050505`) e reprova se alguém voltar a separar as superfícies.
- `settings.spec.ts`: 3 passaram, 2 falharam (`⌘,` e `⌘⇧F9`) — ambiente, Chromium no Linux.
- `actions.spec.ts`: 5 falharam, todas em atalho `Meta`; `focus-after-navigation.spec.ts`: 4 falharam
  em `⌘F`/`⌘⇧A`; `medium-nav-git-dialogs.spec.ts`: 4 falharam em `⇧⌘K`/`⌥⌘W`. Mesma causa de ambiente.
- `panel-disclosures.spec.ts` (foco por Tab) e `medium-nav-layout.spec.ts` (Find Usages dockado)
  falham **também** numa cópia limpa de `origin/main` (`/tmp/base350`): são anteriores a esta frente.
- `microcopy.spec.ts`: 3 passaram. `acceptance-fixes.spec.ts`: 7 passaram.
- Auditoria de contraste própria (script descartável, não versionado) sobre welcome, trust, workspace
  e settings: os únicos textos abaixo de 4,5:1 são controles **desabilitados** (`--disabled` `#737373`,
  4,3:1 e 4,01:1), que a WCAG isenta; nenhum texto habilitado ficou abaixo do piso.
- Detector Impeccable em `http://127.0.0.1:1520/` (welcome, 1440×900): `[]`.

**Não verificado:** app Tauri macOS ARM64 e WebKit, atalhos nativos, Windows e macOS Intel; estados de
erro de cada diálogo em captura; contraste medido em todos os painéis e densidades.
