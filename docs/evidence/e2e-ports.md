# E2E com portas por sessão e Playwright como verificação de UI (issue #210)

Medição em macOS Apple Silicon (arm64), 2026-11-07, Chrome real (`channel: "chrome"`), SDK 8.0.408 das
fixtures em `~/.dotnet`, `workers: 1`. Logs fora do repositório (`/tmp`); capturas em diretório
temporário (`OPENRIDR_SCREENSHOT_DIR`), nunca em `docs/evidence/screenshots/`.

## O que mudou

- O default do app home do E2E passou a ser derivado da porta da UI: `<tmpdir>/openridr-e2e-home-<uiPort>`.
  `OPENRIDR_E2E_HOME` continua vencendo como override. Antes era um caminho fixo
  (`<tmpdir>/openridr-e2e-home`), compartilhado por todos os worktrees.
- As portas já vinham de `OPENRIDR_E2E_UI_PORT`/`OPENRIDR_E2E_HOST_PORT` (defaults 1420/4317) desde a
  `5031a5a`; nada mudou ali. O Vite de teste fala com o host pelo proxy same-origin `/__host`, então o
  CORS fixo do `host-http` não participa do caminho de teste.
- Documentação: `AGENTS.md`, `.agents/skills/verify-openridr/SKILL.md`, `scripts/e2e-smoke.txt` e
  `docs/evidence/e2e-quarantine.md` descrevem a divisão de superfícies decidida em 2026-11-07 e a
  convenção de portas; a decisão está em `docs/decisions.md`.

## Prova: o mesmo spec em dois worktrees ao mesmo tempo

Dois checkouts do mesmo commit (`6a7cbbf`), cada um com `bun install` e `cargo build -p openridr-host-http`
feitos antes (o build não é parte da medição), rodando `tests/e2e/build.spec.ts` simultaneamente:

| Worktree | Portas (UI/host) | App home | Resultado |
|---|---|---|---|
| `chore-issue-210-e2e-session-ports` | 1531 / 4438 | `<tmpdir>/openridr-e2e-home-1531` | **2 passed, exit 0** (1,1 min) |
| checkout detached do mesmo commit, em `<tmp>` | 1532 / 4439 | `<tmpdir>/openridr-e2e-home-1532` | **2 passed, exit 0** (1,1 min) |

Janela medida: início `1794069106`, fim `1794069173` (epoch) — as duas execuções se sobrepuseram do
começo ao fim. Comando de cada lado:

```sh
PATH=$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet \
DOTNET_CLI_UI_LANGUAGE=en OPENRIDR_E2E_UI_PORT=1531 OPENRIDR_E2E_HOST_PORT=4438 \
OPENRIDR_SCREENSHOT_DIR=/tmp/210-shots-a bunx playwright test tests/e2e/build.spec.ts
```

Cada execução abriu o próprio home: os dois diretórios existem no disco, cada um com `tools/`,
`bundled/`, `hostroot/`, `toolhome/` e `recent.json` escritos pelo host daquela execução. Antes da
mudança os dois usariam o mesmo `<tmpdir>/openridr-e2e-home`, e o `globalSetup` de cada execução
apagaria e recriaria `tools/csharp-ls/0.28` e o cache `real-tools` embaixo da outra.

## Caminho do smoke noturno (par default, isolado)

`tests/e2e/build.spec.ts` sem nenhuma variável de porta (1420/4317), sozinho:

```sh
bunx playwright test tests/e2e/build.spec.ts
# 2 passed (30.4s), exit 0
```

O home usado foi `<tmpdir>/openridr-e2e-home-1420`. O workflow `e2e-smoke.yml` não seta porta nem home:
continua no default, com o `concurrency` que já existia.

## `workspace.spec.ts` vermelho na main desde a #177 (issue #213)

Medido no mesmo slot, no par default:

- **Falha-antes:** `bunx playwright test tests/e2e/workspace.spec.ts -g "welcome lists recent|closing a modified buffer|saves every modified"` → **3 failed (42 s)**, os três em
  `reveal(page.getByTestId("close-workspace"))` com `expect(locator).toBeAttached() failed — element(s) not found`.
- **Causa:** a #177 (squash `9abc5e4`) trocou o menu de workspace actions de `<details>` por um botão com
  popover montado condicionalmente; `close-workspace` passou a existir só com o menu aberto, e `reveal()`
  só abre `<details>` ancestrais. Os quatro sítios do spec não foram atualizados (a #177 atualizou só os
  `.locator("summary")`).
- **Passa-depois:** `bunx playwright test tests/e2e/workspace.spec.ts` → **16 passed (38.6 s)**, exit 0.

Não é colisão de portas nem de home: a falha é determinística e apareceu igual em duas execuções
simultâneas com portas e homes diferentes.

## Não verificado

- Suíte E2E completa (63 specs): só os specs da área rodaram, como o processo manda.
- App Tauri real (`agent-desktop`): nada nesta frente é superfície nativa do shell — a mudança é de
  harness e de documentação.
- Windows e macOS Intel: não executados.
- O smoke noturno automatizado (workflow `e2e-smoke.yml`) não foi disparado: o caminho dele foi medido
  localmente no par default, acima.
