# Evidência — menu nativo não engole atalho do keymap (issue #111)

Alvo: macOS Apple Silicon, app Tauri real dirigido por `agent-desktop`. Spec em
[SPEC-NATIVE-MENU-KEYMAP](../SPEC-NATIVE-MENU-KEYMAP.md); tickets #120 (menu), #129 (tecla com
Option) e #121 (esta aceitação).

## O que rodou

- **Bundle sob teste:** `src-tauri/target/debug/bundle/macos/OpenRidr.app`, `CFBundleShortVersionString`
  `0.3.0-alpha.11`, construído da branch `fix/issue-111-update-shortcut` no commit `113e62a`
  (`#120` em `3e2bd8f` + `#129` em `113e62a`). Foi o **segundo** build: o primeiro (`3e2bd8f`) serviu
  para achar o defeito de `#129`, descrito abaixo.
- **Lançador:** `.agents/skills/verify-openridr/scripts/desktop.sh up <bundle>` com `OPENRIDR_HOME`
  isolado (state privado em `$TMPDIR`), uma instância por vez (checagem de `pgrep` antes e PID gravado
  depois), `down` no fim (`stopped`, PID morto).
- **Workspace:** `scripts/git-remote-fixture.sh` — bare `origin.git` + clone `user` (aberto pelo app)
  + clone `peer`, tudo em `$TMPDIR`; solução `user/OrderDesk.sln` passada em argv pelo override
  `VERIFY_OPENRIDR_DESKTOP_SOLUTION`. Baseline `a22f9fc`, upstream `origin/main` configurado, árvore
  limpa.
- **Navegador/Playwright:** não usados. O defeito só existe onde existe menu nativo; toda a prova é do
  app real.

## Falha antes (reprodução, app instalado 0.3.0-alpha.9)

| Ação | Estado resultante | Status |
|---|---|---|
| Ler o menu File | `Close Window` com ⌘W (`AXMenuItemCmdChar = W`, modificadores 0) e `Close All` com ⌥⌘W (modificadores 2 = ⌥) | falha |
| Pressionar `⌥⌘W` (com a janela em foco) | PID 60340 morto (`pgrep -x openridr` vazio), nenhum diálogo | falha |
| Pressionar `⌘W` num workspace aberto | PID morto, nenhum diálogo | falha |

O `Close All` não vem do Tauri: o AppKit o injeta no menu File a partir do item com o seletor
`performClose:`. Medido num spike AppKit descartável (fora do repo): com `performClose:` + ⌘W o
injentado nasce com ⌥⌘W; com `performClose:` sem acelerador, nasce sem tecla; sem nenhum item
`performClose:`, não aparece.

## Ato intermediário (só `#120`, commit `3e2bd8f`)

| Ação | Estado resultante | Status |
|---|---|---|
| `⌘W` com 2 abas (`Order.cs`, `Program.cs`) | `Program.cs` fechou, `Order.cs` ficou, app vivo | passa |
| `⌥⌘W` | nada abriu, app vivo | **falha** (defeito novo, virou #129) |
| `⌥⇧⌘T` (Search Symbol, também ⌥+letra) | nada abriu | falha |
| `⇧⌘A` (Find Action, sem Option) | diálogo `Search Everywhere` abriu | passa |
| Script isolado com o evento do webview (`key: "∑"`, `code: "KeyW"`, ⌘+⌥) | `matches` devolvia `false`; `shortcutFromEvent` gravava `{key: "∑"}` | falha |

Conclusão registrada no #129: com Option o macOS compõe outro caractere, então comparar `event.key`
nunca casa; a tecla física (`event.code`) passa a ser a fonte quando `alt` está pressionado.

## Aceitação final (bundle `113e62a`)

| # | Ação | Estado resultante | Prova fora da tela | Status |
|---|---|---|---|---|
| S1 | `⌥⌘W` | diálogo **Update project** (Branch `main`, Upstream `origin/main`, Behind 0, botões Update/Cancel), app vivo | `agent-desktop snapshot` do diálogo + captura `native-menu-keymap/s1-update-dialog.png` | passa |
| S2 | `⌘W` com `Order.cs` e `Program.cs` abertos | `Program.cs` fechou, `Order.cs` permaneceu ativo, app vivo | árvore de acessibilidade: só `radiobutton Order.cs` + `tabpanel Order.cs`; `pgrep -x openridr` com PID | passa |
| S3 | `⌘W` com `Order.cs` modificado (1 buffer sujo) | diálogo **Close modified file?** (Save / Keep editing / Discard and close), app vivo; `Keep editing` manteve a aba | captura `native-menu-keymap/s3-dirty-tab-close.png`; árvore: `radiobutton "Order.cs, modified"` | passa |
| S4 | `⌘Q` com o mesmo buffer sujo | diálogo **Close OpenRidr with unsaved changes?** (Save all / Keep editing / Discard and close app), app vivo até a decisão | captura `native-menu-keymap/s4-quit-dirty.png` | passa |
| S5 | Botão vermelho da janela (AX `Close`) com o mesmo buffer sujo | mesmo diálogo de saída, app vivo; `Keep editing` manteve tudo | árvore de acessibilidade antes/depois | passa |
| S6 | Menu File/Window e o item **Close Window** | File tem só `Close Window`, **sem** acelerador (`AXMenuItemCmdChar` = *missing value*); não existe `Close All`; Window não tem `Close Window` duplicado; clicar em File > Close Window disparou o diálogo de saída | dump da árvore do menu (`agent-desktop snapshot --surface menubar`) + `System Events` para o key equivalent; `Discard and close app` encerrou o app (PID morto, `desktop.sh down` = `stopped`) | passa |

## Correções da revisão independente (rodada 1)

Dois achados médios, corrigidos e provados no terceiro bundle (mesmo `0.3.0-alpha.11`, com
`typesACharacter` e o matcher compatível), fixture e `OPENRIDR_HOME` isolados, `VERIFY_OPENRIDR_DESKTOP_STATE`
próprio para não dividir state com outra sessão.

| # | Ação | Estado resultante | Prova fora da tela | Status |
|---|---|---|---|---|
| S7 | `⌥⇧U` com foco no editor (`Order.cs`) | o caractere entrou no buffer (U+2191 neste layout, tecla composta) e **não** abriu a Test Quick List; a aba virou "Order.cs, modified"; `⌘Z` devolveu `Order.cs` limpa | árvore de acessibilidade antes/depois + valor do `textfield Editor` | passa |
| S8 | `⌥⌘W` depois do gate de digitação | diálogo **Update project** abriu, app vivo | árvore do diálogo | passa |
| S9 | File > Close Window com buffer limpo | janela fechou e o app saiu; `desktop.sh down` = `stopped` | `pgrep -x openridr` vazio | passa |

O achado: com a tecla física sob Option, o default `tests.quickList` (⌥⇧U) passaria a engolir uma tecla
morta no editor; a correção faz um atalho só de Option ceder à digitação dentro do CodeMirror. O segundo
achado (override antigo gravado como `"∑"`) é de compatibilidade de `settings.json` e está coberto por
teste de unidade, não pelo app.

## O que não foi verificado

- **Windows, Linux e macOS Intel:** a cirurgia é só do ramo macOS; essas plataformas continuam com
  `Menu::default` e podem ter a mesma sombra (Ctrl+W no fechar janela). Não executado, não suportado.
- **Layout de teclado não-US:** com Option, o atalho passa a ser nomeado pela posição física da tecla
  (`KeyW` → `w`), o mesmo nome na captura e no matcher. Não testado em ABNT2/AZERTY.
- **Tecla composta no navegador (E2E):** o E2E roda em Chrome sem menu nativo e os testes de unidade
  usam eventos sintéticos; a composição real do Option só foi observada no app, não em spec Playwright.
- **Remapear `⌘Q`:** continua reservado ao menu nativo de propósito; um override do keymap para ⌘Q
  continuaria sombreado.

## Como repetir

```sh
sh scripts/git-remote-fixture.sh                       # imprime state, solution, peer
export VERIFY_OPENRIDR_DESKTOP_SOLUTION=<state>/user/OrderDesk.sln
PATH=$HOME/.bun/bin:$PATH bun run tauri build --debug --bundles app
sh .agents/skills/verify-openridr/scripts/desktop.sh up "$PWD/src-tauri/target/debug/bundle/macos/OpenRidr.app"
# confiar no prompt, abrir 2 arquivos, então: ⌥⌘W / ⌘W / ⌘W sujo / ⌘Q sujo / botão vermelho / menu File
sh .agents/skills/verify-openridr/scripts/desktop.sh down
```
