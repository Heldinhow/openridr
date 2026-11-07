# Evidência — os três menus da toolbar como botões acessíveis (issue #177, tickets #184, #185, #186, #187)

Rodada de 2026-11-07, macOS arm64. Duas sessões: a primeira medida no commit `9510124` e a segunda no `3b47368` (o commit dos consertos do code review; só os dois últimos critérios da tabela dependem dele). Spec: [SPEC-TOOLBAR-MENUS-A11Y](../SPEC-TOOLBAR-MENUS-A11Y.md). Bundle debug `0.3.0-alpha.11` construído desta branch (`bun run tauri build --debug --bundles app`), copiado para `~/Applications-test/OpenRidr-issue177/OpenRidr.app`, `OPENRIDR_HOME` isolado e workspace próprio (cópia do `fixtures/OrderDesk` feita pelo `desktop.sh` da skill de verificação; a segunda sessão usou uma cópia isolada do `fixtures/MissingSdk`).

## O que foi provado, e como

Árvore de acessibilidade do WKWebView por `agent-desktop` (app real, superfície C do `verify-openridr`). Nada de Playwright.

| Critério | Medição |
|---|---|
| Os três gatilhos são ação, não disclosure | `button: 'Project: OrderDesk.sln. Open recent solutions.'` (`@…:e3`), `button: 'Workspace actions'` (`@…:e10`), `button: 'Run configuration: Run OrderDesk.Cli. Run and Debug use it.'` (`@…:e6`), **todos com `ref_id`**. Antes: `disclosure`, `interactive: false`, sem `ref_id` — medido no mesmo tipo de snapshot na evidência da #168. |
| `aria-expanded` | Aberto: `states: ['expanded']` no gatilho dos três. Fechado: sem `expanded` (medido no gatilho do projeto depois do Escape, `states: ['focused']`, e no toggle). |
| O toggle do gatilho ainda funciona | Clique 1 no projeto: `states: ['expanded']` e `OrderDesk.sln (current)` na árvore. Clique 2: sem `expanded` e o item sai. |
| Popover alcançável por papel e nome | Projeto: `button: 'OrderDesk.sln (current)'`. Workspace actions: `combobox: 'Configuration'`, `combobox: 'TFM'`, `button: 'Build Solution ⇧⌘F9'`, `'Find Action ⇧⌘A'`, `'Keymap: change shortcuts'`, `'Redetect SDKs'`, `'Open global.json'` (disabled), `'Commit…'` (disabled), `'Close Workspace'`. Run config: `button: 'Run OrderDesk.Cli Derived from the evaluation; read only.'` e `'Edit run configurations…'`. |
| Escape fecha e devolve o foco | Com o foco dentro do popover (`OrderDesk.sln (current)`, `Build Solution`, `Edit run configurations…`), `press escape`: popover fora da árvore e o gatilho com `states: ['focused']`. Testado nos três. |
| Clique fora fecha | Workspace actions aberto, clique físico (`mouse-click --xy 900,500 --headed`) no editor: `Build Solution` e `Close Workspace` saem da árvore, só o gatilho fica. Clique semântico (`AXPress`) não serve para esta prova — ele não gera `pointerdown`, como o retro da #168 registrou. |
| Sem mudança visual | Comparação direta com `evidence/branch-menu-a11y/menu-open-with-merge.png` (mesma janela de 1440×900, mesma fixture): os três gatilhos aparecem na mesma posição e com o mesmo tamanho, e o destaque de "aberto" é o mesmo fundo de antes. |
| Fechar por item devolve o foco (2ª sessão) | Run configuration aberto, clique em `Run OrderDesk.Cli`: o popover sai da árvore e o gatilho fica `states: ['focused']` — o foco volta para o gatilho, não para o `body`. |
| Abrir `global.json` fecha o menu (2ª sessão) | Workspace `MissingSdk` (cópia isolada, `global.json` com SDK `8.0.999`), onde a ação fica habilitada: clique em `Open global.json` fecha o popover, deixa o gatilho `states: ['focused']` e abre o arquivo no editor (aba `Close global.json`). |

## O que o app pegou que o `tsc` não pegaria

Nada quebrou nesta rodada, mas a armadilha do #168 foi evitada por construção: o estado de aberto/fechado mora no componente `ToolbarMenu`, e não no `RunConfigWidget`, que tem `return` antecipado no caso sem configuração disponível. Um `useState`/`useEffect` depois daquele `return` repetiria a tela em branco da #168.

## Teste automatizado

- `bun run check` (`tsc --noEmit` + `cargo check --workspace --all-targets`) limpo; `bun run build` (tsc + vite) limpo.
- `bun test tests/unit` → 95 pass, 0 fail. Nenhum teste novo: não há seam puro nesta mudança (os nomes acessíveis não mudaram) e o repo não tem infraestrutura de DOM — decisão registrada na spec e em `decisions.md`.
- `cargo fmt --all --check` e `cargo fmt --manifest-path src-tauri/Cargo.toml --check` limpos; `cargo clippy --workspace --all-targets -- -D warnings` limpo; `cargo test -p openridr-host` → 580 passed, 0 failed, 54 ignored (as suítes que exigem rede/ferramentas continuam `ignored`, como antes da frente).
- Os 13 locators de E2E que apontavam para o `<summary>` do menu de workspace actions passaram a `getByRole("button", { name: "Workspace actions" })`. **Nenhum Playwright foi executado nesta rodada** (decisão da issue #67: Playwright é smoke noturno), então essa atualização está **não verificada**.

## Capturas

- `toolbar-menus-a11y/project-menu-open.png` — menu de projeto aberto com "Recent solutions" e o gatilho destacado.
- `toolbar-menus-a11y/workspace-actions-open.png` — workspace actions aberto (configuração, TFM, Build Solution, Find Action, Keymap, SDK, Close Workspace).
- `toolbar-menus-a11y/run-config-open.png` — run configuration aberto com a configuração derivada e "Edit run configurations…".

## Não verificado

- Ordem de tabulação: o `agent-desktop` só vê `webarea` no conteúdo web; os gatilhos são `<button>` nativos, mas a sequência de Tab não foi medida.
- Leitor de tela real (VoiceOver): não executado.
- Suíte Playwright: não executada (smoke noturno); os locators atualizados não foram rodados.
- Windows e macOS Intel.
- O caso desabilitado do run configuration (`.toolbar-disabled` com `<button disabled>`) não foi exercitado no app real nesta rodada; ele já era um botão antes da frente e não mudou.
