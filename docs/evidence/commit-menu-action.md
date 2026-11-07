# Evidência — o item Commit… do menu de workspace actions abre o painel de Commit (issue #190, ticket #192)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-COMMIT-MENU-ACTION](../SPEC-COMMIT-MENU-ACTION.md). Bundle debug `0.3.0-alpha.11` construído desta branch (`bun run tauri build --debug --bundles app`), copiado para `~/Applications-test/OpenRidr-issue190/OpenRidr.app`, `OPENRIDR_HOME` isolado (`$TMPDIR/verify-openridr-190-desktop/home`) e workspace próprio: cópia do `fixtures/OrderDesk` com `git init` + commit inicial (`$TMPDIR/verify-openridr-190-work/orderdesk`), porque sem repositório Git o `git.commit` fica desabilitado e o caso habilitado não existiria.

## O que foi provado, e como

Árvore de acessibilidade do WKWebView por `agent-desktop` (app real, superfície C do `verify-openridr`). Nada de Playwright.

| Critério | Medição |
|---|---|
| Desabilitado em workspace restrito (critério 2) | Aberto com **Open in Restricted mode**, menu de workspace actions aberto: `button: 'Commit…'` com `states: ['disabled']`. O painel de Commit e a aba Git mostram o mesmo motivo, `Trust the workspace to use Git`, e o gatilho do branch carrega a mesma string na árvore — é o `TRUST_MESSAGE` que `gitBlockedReason` devolve. Captura: `commit-menu-action/commit-disabled-restricted.png`. |
| Habilitado com Git e trust (critério 1, antes) | Trust concedido pelo popover do status bar (`I trust this repository to execute code` → `Trust and inspect project`), status `Trusted mode`, gatilho do branch `Current branch: main · no upstream`. Com o painel de Commit **fechado** (`switch: 'Commit tool window'` sem `checked`; `navigation: 'Commit'` ausente da árvore), menu aberto: `button: 'Commit…'` **sem** `states: ['disabled']`. |
| Clicar em Commit… abre o painel e fecha o popover (critério 1, depois) | Clique em `Commit…`: `Close Workspace` e `Commit…` saem da árvore (o popover desmonta), o gatilho fica `button: 'Workspace actions'` com `states: ['focused']` e **sem** `expanded` (o `close()` devolve o foco), e o painel volta: `switch: 'Commit tool window'` com `states: ['checked']`, `navigation: 'Commit'` e `textfield: 'Commit message'` presentes. Captura: `commit-menu-action/commit-opened.png`. |
| O mesmo painel do atalho e da Find Action | O `onClick` chama `commitAction.execute()`, a mesma ação da registry que serve ⌥⇧⌘K e a Find Action — nenhum caminho novo, nenhuma lógica duplicada. |

## O que o app pegou que o `tsc` não pegaria

Nada quebrou. O `title` do estado desabilitado não aparece na árvore de acessibilidade (o nome vem do texto do botão), então o critério "mesmo `title` de hoje" se apoia em duas coisas medidas: o botão continua `disabled` no workspace restrito, e o diff **não toca** as linhas de `disabled`/`title` — só acrescenta o `onClick`.

## Teste automatizado

- `bun run check` (`tsc --noEmit` + `cargo check --workspace --all-targets`) limpo.
- `bun test tests/unit` → 95 pass, 0 fail (nenhum teste novo: não há seam puro nesta mudança).
- Nenhum Playwright executado e nenhum cenário E2E novo — decisão do Helder no grill desta issue (Playwright é smoke noturno desde a #67). O critério "um teste cobre o caso habilitado" da #190 é substituído pela prova no app real acima.

## Não verificado

- **Falha-antes no app**: não medida. Exigiria construir um bundle sem o conserto e repetir a sessão; o defeito é a ausência do handler, visível no diff (`git-commit-action` não tinha `onClick`).
- Leitor de tela real (VoiceOver) e ordem de tabulação: o `agent-desktop` só vê `webarea` no conteúdo web.
- Windows e macOS Intel.
- A suíte Playwright inteira e o smoke noturno.
