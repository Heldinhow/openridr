# Microcopy de erro e de estado vazio (#336)

Data: 2026-11-09. Superfície: app em modo web (host-http 5717 + Vite 2720, `OPENRIDR_HOME`
isolado), Chromium via Playwright, 1440×900.

## O que mudou

| Onde | Antes | Depois |
|---|---|---|
| Welcome vazia (`ProjectsView.tsx`) | ensinava `↑ ↓ / Enter /` com a lista vazia | com a lista vazia, só o atalho que funciona (`Ctrl+O`/`⌘O` open project) |
| Editor vazio (`WorkspaceView.tsx`) | lista de atalhos começava em "Save file" antes de existir arquivo | "Save file" saiu; ficam Search Everywhere, Go to definition, Rename symbol, Run saved build |
| Settings (`SettingsDialog.tsx`) | "OpenRidr ships one approved dark token set" | "OpenRidr ships a dark theme only" |
| Git (`git.tsx`) | "Not a git repository" | "Not a git repository. Run git init in the workspace folder to use Git here." |
| Caminho inválido (`SetupScreen.tsx`) | só o texto do host | texto do host + "Check the path and that the file exists, then try again." |
| Menu de contexto do editor (`EditorContextMenu.tsx`) | explicação do item desabilitado em parágrafo dentro do menu | motivo em `title` + `aria-description`; o parágrafo saiu (e o CSS morto junto) |

O texto real do host continua intacto em todos os erros; a linha de reparo é acrescentada depois dele.

## Verificado no app

- Welcome vazia: `welcome-keys` mostra só `Ctrl+O open project` (o spec `welcome.spec.ts` continua
  passando e asserta o atalho da plataforma).
- Caminho inválido: `workspace-error` = "Workspace path not found: /tmp/does-not-exist-328.sln
  Check the path and that the file exists, then try again."
- Editor vazio: a lista começa em "Search Everywhere"; não há "Save file".
- Git: `git-log-disabled-reason` = "Not a git repository. Run git init in the workspace folder to
  use Git here."
- Menu de contexto: o motivo não é mais um `<span>` no menu; o botão leva `title` e
  `aria-description`.

## Specs

- `tests/e2e/welcome.spec.ts` — **9 passed** (inclui o atalho da welcome e o erro de caminho).

## Não verificado

- App Tauri real (macOS ARM64).
- Menu de contexto verificado por código e DOM, não por captura (o menu exige clique direito no
  editor com um arquivo aberto).
