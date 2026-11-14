# Microcopy de erro e de estado vazio (#336)

Data: 2026-11-09. Superfície: app em modo web (host-http 5717 + Vite 2720, `OPENRIDR_HOME`
isolado), Chromium via Playwright, 1440×900.

## O que mudou

| Onde | Antes | Depois |
|---|---|---|
| Welcome vazia (`ProjectsView.tsx`) | ensinava `↑ ↓ / Enter /` com a lista vazia | com a lista vazia, só o atalho que funciona (`Ctrl+O`/`⌘O` open project) |
| Editor vazio (`WorkspaceView.tsx`) | lista de atalhos começava em "Save file" antes de existir arquivo | ficam só Search Everywhere e Run saved build (Go to definition e Rename symbol também exigem arquivo/símbolo) |
| Settings (`SettingsDialog.tsx`) | "OpenRidr ships one approved dark token set" | "OpenRidr ships a dark theme only" |
| Git (`git.tsx`) | "Not a git repository" | texto do host + "Run git init in the workspace folder to use Git here."; `git_missing` ganhou "Install Git and reopen this panel."; `failed` ganhou "Check the workspace path and run git status in a terminal." |
| Caminho inválido (`SetupScreen.tsx`) | só o texto do host | texto do host + "Check the path and that the file exists, then try again." |
| Menu de contexto do editor (`EditorContextMenu.tsx`) | explicação do item desabilitado em parágrafo dentro do menu | motivo em `title` + `aria-description`; o parágrafo saiu (e o CSS morto junto) |

O texto real do host continua intacto em todos os erros; a linha de reparo é acrescentada depois dele.

## Verificado no app

- Welcome vazia: `welcome-keys` mostra só `Ctrl+O open project` (o spec `welcome.spec.ts` continua
  passando e asserta o atalho da plataforma).
- Caminho inválido: `workspace-error` = "Workspace path not found: /tmp/does-not-exist-328.sln
  Check the path and that the file exists, then try again."
- Editor vazio: a lista tem só "Search Everywhere" e "Run saved build"; não há "Save file", "Go to
  definition" nem "Rename symbol".
- Git: `git-log-disabled-reason` = "Not a git repository. Run git init in the workspace folder to
  use Git here."
- Menu de contexto: o motivo não é mais um `<span>` no menu; o botão leva `title` e
  `aria-description`.

## Specs

- `tests/e2e/welcome.spec.ts` — **9 passed** (inclui o atalho da welcome e o erro de caminho).
- `tests/e2e/git.spec.ts -g "restricted mode and a non-repository"` — **1 passed** com as novas
  strings (o spec foi atualizado junto, como o ticket pede).
- `tests/e2e/microcopy.spec.ts` — **3 passed**: welcome vazia sem atalho de lista, caminho inválido
  com linha de reparo, editor vazio sem atalhos que exigem arquivo, e menu de contexto sem
  `.context-menu-reason` com `aria-description` no item desabilitado.

## Correções da revisão

- A mensagem de "not a repository" virou uma constante (`NOT_A_REPOSITORY`) usada no toolbar e no
  Commit; o log preserva o texto recebido do host e acrescenta o reparo depois dele.
- A linha de reparo do caminho passou a depender da causa: "not found / does not exist / no such
  file" recebe a orientação de caminho; qualquer outro erro de abertura recebe "Fix the problem
  above and try again."
- O spec do Git foi atualizado para as strings novas (acessível, title e os dois `*-disabled-reason`).
- Erros de Git sem reparo (`git_missing`, `failed`, log) ganharam a linha de próximo passo.
- O editor vazio perdeu "Go to definition" e "Rename symbol", que também exigem arquivo/símbolo.

## Resposta a um achado do review (Standards)

O motivo do item desabilitado do menu fica em `title` + `aria-description`, como o ticket manda
("explicação de item desabilitado em `title`/`aria-description`, não em parágrafo dentro do menu").
Tornar o item focável exigiria trocar `disabled` por `aria-disabled` e impedir a ação no clique — um
modelo de interação novo, fora do que o ticket pede; fica registrado como alternativa se o leitor de
tela não anunciar o motivo.

## Não verificado

- App Tauri real (macOS ARM64).
- O menu de contexto é verificado por DOM no spec (sem `.context-menu-reason`, com
  `aria-description`), não por captura.
