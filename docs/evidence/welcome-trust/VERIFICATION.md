# Welcome + Trust — verificação real (ticket 06)

Data: 2026-11-01. Commit verificado: 9dbbb89 (tickets 01–05).

## Como foi rodado

- Chrome instalado, **visível** (`channel: "chrome"`, `headless: false`, `slowMo: 150`), script Playwright standalone (sem `playwright test`; o `.mjs` ad-hoc não é versionado, o log está em `verify-visible.log`).
- Host: `bun run host:http` (4317) com `OPENRIDR_HOME` temporário novo e `OPENRIDR_TRUST_STORE=on`; UI: `bun run dev` (1420). O app home real não foi tocado.
- Projetos: três cópias temporárias de `fixtures/OrderDesk` (Alpha, Bravo, Charlie). Saída do script: `verify-visible.log`.

## Roteiro e resultado

| # | Passo | Resultado | Captura |
|---|-------|-----------|---------|
| 1 | Welcome vazia (primeira execução): `welcome-empty`, sem lista | ok | `v01-welcome-empty.png` |
| 2 | Abrir Alpha (novo): modal Trust prompt, `aria-modal=true`, nenhuma árvore aberta antes da decisão | ok | `v02-trust-prompt.png` |
| 3 | Trust project: IDE abre; barra de cima sem "Trusted"/"Restricted" (assert); status bar "Trusted mode"; popover aberto | ok | `v03-ide-trusted.png`, `v03b-status-popover.png` |
| 4 | Recarregar à welcome e reabrir Alpha: modal NÃO aparece, status "Trusted mode" | ok | `v04-reopen-no-prompt.png` |
| 5 | Bravo (novo) → Open in Restricted mode; status "Restricted mode"; reabrir não pergunta | ok | `v05-ide-restricted.png` |
| 6 | Charlie: Cancel e depois Esc não abrem nada (sem árvore, modal some) | ok | `v06a-after-cancel.png`, `v06b-after-esc.png` |
| 7 | Welcome com recentes: contagem, selos (Not decided/Restricted/Trusted), "just now"; ⋯ → Trust troca Bravo para Trusted sem abrir; ⋯ → Remove from list tira a linha | ok | `v07a-welcome-recents.png`, `v07b-row-menu.png`, `v07c-after-trust-change.png`, `v07d-after-remove.png` |

### Observação (não é falha do roteiro)

Observado na rodada visível (antes da correção): Cancel deixava o projeto em **Recent projects** como "Not decided". Corrigido depois (decisions.md, "Cancel não deixa rastro"): Cancel/Esc não gravam nada e `workspace.open` só registra recentes de projetos já decididos; coberto por `trust-prompt.spec.ts` e `undecided_open_leaves_no_trace_until_decided`. As capturas `v02`/`v07a` são da rodada anterior à correção.

## Checks automáticos (HEAD 9dbbb89)

- `bun run check`: verde (tsc + cargo check --workspace --all-targets).
- `cargo test -p openridr-host`: unit 182 ok, 9 ok, integração 61 ok / 1 falha (`file_access_is_confined_to_the_workspace`). A outra falha citada (`evaluation_returns_the_real_msbuild_model_for_every_project`) **passou** em todas as execuções deste HEAD (suíte completa 2x e isolada 3x, com `PATH`/`DOTNET_ROOT` do `~/.dotnet`).
- E2E, um spec por vez com `PATH=$HOME/.dotnet:... DOTNET_ROOT=$HOME/.dotnet DOTNET_CLI_UI_LANGUAGE=en`: trust-prompt 4, status-mode 3, welcome 6, gates 9, ui-layout 7, workspace 15, trust-sdk 3. Todos passaram, 47 testes, 0 skip/falha.

## As 2 falhas de cargo: baseline?

- `file_access_is_confined_to_the_workspace`: depende do caminho do checkout. O teste usa `../../../../../../etc/hosts` a partir do workspace; no worktree profundo (`~/.local/share/opencode/worktree/...`) falha, e no mesmo HEAD em `/tmp/openridr-baseline` (caminho raso) **passa**. O diff de `crates/host/tests/host.rs` desde 6588b6b só adiciona linhas (+78), então o teste não foi alterado. Não consegui rodar 6588b6b no mesmo caminho profundo (a branch ocupa o worktree), então "falha igual no baseline, mesmo caminho" **não foi provado**; a prova é a dependência de caminho.
- `evaluation_returns_the_real_msbuild_model_for_every_project`: no worktree limpo em 6588b6b falhou 3/3 (também com dotnet no PATH), provavelmente por ambiente de worktree novo (fixtures não restauradas); no HEAD passou sempre. A "intermitência" **não foi reproduzida**, e a causa da falha no baseline não foi investigada.
- Worktree `/tmp/openridr-baseline` removido.

## Não verificado

- Tauri real (janela nativa, ⌘O nativo); só a ponte HTTP + Vite no Chrome.
- Rider.app (referência não reaberta para não mexer na config do usuário).
- Leitor de tela (VoiceOver/NVDA); foco/ARIA só por asserts de atributos e specs de teclado.
- Windows e macOS Intel.
