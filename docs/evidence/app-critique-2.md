# app-critique-2 — evidência da implementação (frente #364)

Data: 2026-11-10. Tickets: #365, #366, #367, #368, #369, #370, #371, #372, #373.

> **DEGRADED: o modelo desta sessão não recebe imagens.** Toda a prova abaixo é **medição**
> (`getBoundingClientRect`, `scrollWidth`/`clientWidth`, `getComputedStyle`, `grep`, teste) e os
> testes automatizados. As capturas (`*.webp`) estão aqui para **revisão humana**; nenhuma afirmação
> deste arquivo depende de o autor ter olhado para elas.

## Como foi medido

| Item | Valor |
|---|---|
| Superfície | web (Vite + `openridr-host-http`), a mesma árvore de componentes e o mesmo host do app |
| Portas | UI `3220`, host `6217` (par da sessão, ver `AGENTS.md`) |
| App home | `/tmp/openridr-critique-2/web/home` (descartável) |
| Workspace | cópia de `fixtures/OrderDesk` em `/tmp/openridr-critique-2/web/work` |
| Navegador | Chromium (Playwright), 1440×900, `deviceScaleFactor: 1` |
| Ferramenta | `openridr-host-http` com o par `csharp-ls`/`netcoredbg` semeado como no `globalSetup` do E2E |
| Medição | `before-measure.json` (merge-base) e `after-measure.json` (esta branch), mesmo script |

O script de medição é descartável e vive fora do repositório; o que ele produziu está nos dois JSON
acima. Cada linha abaixo cita o campo que a prova.

## Antes × depois, por ticket

### #365 — a barra de status não corta a prontidão

| Medida | Antes | Depois |
|---|---|---|
| `footer.status` altura | 28 px (conteúdo 30 px, 3 px cortados) | **32 px** |
| `status-evaluation` largura / `scrollWidth` | 68 / 242 (truncado: "Evaluated · De…") | **242 / 242** (íntegro) |
| `status-analysis` | estado + botão + **motivo** (até 170 px) | estado + botão; o motivo fica no `title` do botão |
| `eval-run` | 18×27 | **32×32** (ver #370) |

A altura de 32 px é a decisão que fecha o conflito entre este ticket ("28 px reais") e o #370
("alvo ≥ 32×32 no Re-evaluate"): a barra não pode ter 28 px de altura e um controle de 32 px dentro.
O separador virou linha `inset`, então o conteúdo da barra é o próprio alvo. `DESIGN.md` foi
atualizado na mesma entrega (tabela de tokens, linha "status bar").

Capturas: `before-status-1440.webp`, `after-status-1440.webp`.

### #366 — TFM implícito quando o projeto tem um só

| Medida | Antes | Depois |
|---|---|---|
| `build-tfm` valor / texto | `""` / "Not selected" | **`net8.0` / `net8.0`** |
| `tests-tfm` valor | `net8.0` (já resolvia pelo projeto de teste) | `net8.0` (agora pela mesma função) |
| Plano do NuGet (Install) — TFM | "not selected" | **`net8.0`** |

O host não muda: `evaluation.context.targetFramework` continua `null` e o gate continua sendo o do
host (`test_framework` resolve o TFM único). A função `effectiveTfm` (`src/tfm.ts`) é a única regra de
exibição; `testPanelFramework` e a barra de status passam a delegar nela. Teste:
`tests/unit/tfm.test.ts` (4 casos, incluindo multi-TFM e lista suja).

Capturas: `before-build-panel.webp`, `after-build-panel.webp`, `after-nuget-plan.webp`.

### #367 — Run/Debug com saída durante a descoberta automática

Opção (a) do ticket: o controle vai para a mensagem, chamando o mesmo `tests.cancel`.

| Medida | Antes | Depois |
|---|---|---|
| `build-disabled-reason` | só o texto | texto + **"Cancel discovery"** (`build-cancel-discovery`, 125×32) |
| `run-disabled-reason` | só o texto | texto + **`run-cancel-discovery`** |
| `dap-disabled-reason` | só o texto | texto + **`dap-cancel-discovery`** |
| Ação | inexistente ali | `tests.cancel` do host; o gate limpa sozinho |

Prova automatizada: `tests/e2e/build.spec.ts` › "the discovery gate offers the cancel action next to
its reason" (verde, 8,0 s) — clica o botão, o gate some e o estado da descoberta vira "cancelled".
Unidade: `isDiscoveryGateReason` em `tests/unit/executionGates.test.ts`.

Captura: `after-discovery-cancel.webp`.

### #368 — painel Git lê os tokens

| Medida | Antes | Depois |
|---|---|---|
| `grep -n '#[0-9a-fA-F]\{6\}' src/git-panel.css` | 16 linhas | **0** |
| Botão Commit (default) | `#386fda` + `white` (3,3:1 sobre o azul) | `--accent` `#548af7` + `--on-accent` `#101114` (**5,71:1**) |
| Botão Commit (hover) | herdava `--accent-surface-strong` (perdia o azul) | `--accent-hover` `#6d9bf9` (**6,93:1**) |
| Nome de arquivo `modified` / `untracked` | `#78a9d1` / `#d99578` | `--git-modified` `#82c0f0` / `--git-untracked` `#eda773` |

Tokens novos no `:root` (e em `DESIGN.md`): `--diff-mod-surface`, `--diff-empty-surface`,
`--diff-intraline-surface`, `--diff-connector-removed`, `--diff-connector-added`,
`--diff-connector-modified`, `--git-deleted`. As linhas de diff passam a usar
`--diff-add-surface`/`--diff-del-surface`, os mesmos do split: a mesma edição lê igual nas duas
vistas. O botão virou `className="primary"` (o CSS deixa de inventar cor).

Captura: `after-git-commit.webp` (medido com o repositório da fixture com 1 modificado + 1 novo).

### #369 — copy que ensina o errado

| Onde | Antes | Depois |
|---|---|---|
| Editor vazio (Trusted) | "Text editing is always available. Analysis and execution require trust." | **"This project is trusted: analysis and execution are available."** |
| Editor vazio (Restricted) | idem | mantém a frase de trust |
| NuGet, aviso de update | "from **unknown** to 3.0.10" | "from **not referenced** to 3.0.10" |
| NuGet, linha do batch | "unknown → 3.0.10" | "not referenced → 3.0.10" |
| `status-keymap` | "Keymap: Rider (macOS)" | **"Keymap: Rider"** |

O id do perfil (`rider-macos`) continua o mesmo: é valor persistido em `settings.json`.

Capturas: `before-empty-editor.webp`, `after-empty-editor.webp`.

### #370 — alvos de 32 px nos cabeçalhos

| Controle | Antes | Depois |
|---|---|---|
| `tree-show-all`, `tree-search`, `tree-locate`, `tree-expand-all`, `tree-collapse-all`, `tree-more`, `tree-minimize` | 24×24 | **32×32** (ícone continua 16 px) |
| `eval-run` (Re-evaluate) | 18×27 | **32×32** |
| `workspace-tools-toggle` (Workspace actions) | 32×32 | 32×32 (já estava) |
| Cabeçalho da árvore (`.panel-title`) | 38 px de altura | 40 px (+2 px, não +8: padding vertical 7→4) |

Os 7 botões de 32 px + o título precisam de 232 px; a sidebar padrão (280 px) cabe numa linha e só o
mínimo (220 px) quebra a faixa para a segunda linha — o título encolhe antes.

### #371 — os três botões M/A/D saem do rail direito

| Medida | Antes | Depois |
|---|---|---|
| Botões em `tool-windows-right` | 4 ("Workspace tools" + Monitoring + AI Assistant + Database) | **1** ("Workspace tools") |
| `data-testid` removido | — | nenhum (os três não tinham) |

Decisão registrada em `docs/decisions.md` (entrada de 2026-11-10, citando a #371), substituindo o
bullet G22 de 2026-11-04. `tests/e2e/shell.spec.ts` passa a provar a ausência pelo nome acessível.

### #372 — densidade do painel Tests

Medido a 1440×900 com o painel inferior no tamanho padrão (320 px):

| Medida | Antes | Depois |
|---|---|---|
| Linhas de chrome antes da árvore | 2 (toolbar 47 px + controles 32 px) | **1** (linha primária 32 px) |
| Topo da árvore (`tests-items`) | y = 789 | **y = 696** |
| **Lista visível** | **75 px** (≈2 linhas) | **164 px** (≈5 linhas) |
| Contadores, filtro, Retry, Expand/Collapse/Export | soltos na toolbar | disclosure rotulado "More" (`tests-overflow`) |
| Linha primária | projeto + TFM + Discover + estado | projeto + TFM + Discover + **Run** + estado + More |

O disclosure do Tests **expande inline** (não flutua): a linha primária fica logo acima da faixa de
abas, e um popover para baixo cobriria as abas e as primeiras linhas da lista enquanto aberto (foi o
que a primeira versão fez e o `tests.spec.ts` pegou). Com ele aberto a árvore desce ~125 px; ao
fechar, o espaço volta. Regra geral nova: `.panel-disclosure:not([open]) > .panel-disclosure-body {
display: none }` — um disclosure fechado não pode interceptar clique (bug latente do padrão #331).

`role="tab"` e todos os `data-testid` preservados; `tests-counts` foi para dentro do disclosure (o
estado "Discovered 4 tests · exit 0" continua na linha primária).

Capturas: `before-tests-panel.webp`, `after-tests-panel.webp`, `after-tests-disclosure.webp`.

### #373 — abrir um `.cs` pelo CLI no workspace atual

| Caso | Antes | Depois |
|---|---|---|
| Arquivo de código **dentro** do workspace aberto | erro do host ("Open a .sln, …") | abre a **aba** do arquivo |
| Arquivo **fora** do workspace | erro sem ação | mesmo erro + ação **"Open the containing solution"** |
| Solução que contém o arquivo | — | método novo `workspace.containingSolution` (read-only, sem gate de trust) |

`workspace.containingSolution` sobe do diretório do arquivo até a solução mais próxima (`.sln`/`.slnx`)
e, se não houver, até o `.csproj` mais próximo; devolve `{ path, kind }` ou `{ path: null }`. Prova no
host real (porta 6217):

```sh
curl -s -X POST http://127.0.0.1:6217/call -H 'content-type: application/json' \
  -d '{"method":"workspace.containingSolution","params":{"path":"…/OrderDesk/src/OrderDesk.Cli/Program.cs"}}'
# {"data":{"kind":"solution","path":"…/OrderDesk/OrderDesk.sln"},"ok":true}
```

Unidade: `tests/unit/externalOpen.test.ts` (5 casos, incl. prefixo compartilhado e `..`) e
`crates/host/src/lib.rs` › `containing_solution_walks_up_to_the_nearest_workspace`.

## Testes

| Comando | Resultado |
|---|---|
| `bun test tests/unit` | **181 pass, 0 fail** (25 arquivos) |
| `cargo test -p openridr-host --lib containing_solution` | **1 pass** |
| `bunx tsc --noEmit` | limpo |
| `bunx playwright test tests/e2e/panel-disclosures.spec.ts` | **1 pass** |
| `bunx playwright test tests/e2e/acceptance-fixes.spec.ts` | **5 pass** |
| `bunx playwright test tests/e2e/ui-layout.spec.ts` | **7 pass** (inclui 1440×900) |
| `bunx playwright test tests/e2e/gates.spec.ts` | 9 pass, 1 fail — **falha pré-existente no merge-base** (ver abaixo) |
| `bunx playwright test tests/e2e/build.spec.ts -g "discovery gate offers the cancel"` | **1 pass** |
| `bunx playwright test tests/e2e/shell.spec.ts` | 5 pass, 1 fail — **pré-existente no merge-base** |
| `bunx playwright test tests/e2e/tests.spec.ts` | 6 pass, 1 fail — o teste quarentenado (`tests.spec.ts:317`) |
| `bunx playwright test tests/e2e/testsuite.spec.ts` | 1 pass, 1 fail — o quarentenado (`tests-filter-count`) |
| `bunx playwright test tests/e2e/workspace.spec.ts` | 7 pass, 9 fail — todos por `⌘` (Meta) no Linux |

## Não verificado

- **App Tauri real (macOS ARM64)** — o alvo verificado do produto não foi executado nesta sessão:
  esta máquina é Linux ARM64 e não tem `agent-desktop` (`doctor.sh`: "agent-desktop not installed").
  Nada de menu nativo, atalhos do sistema, updater, seletor nativo, árvore de acessibilidade do
  WebKit, single-instance ou DMG.
- **`#373` no app desktop** — o caminho de "abrir com" (argv/Finder) só existe no shell Tauri. A
  unidade, o método do host (contra o host real) e o helper de UI estão provados; a ponta
  `App.openExternal → WorkspaceView.openSolution` não foi dirigida num app real.
- **E2E que aperta `⌘`** — no Linux o modificador primário é Ctrl, então `Meta+…` não aciona o
  keymap: `workspace.spec.ts` (9), `actions.spec.ts` (5), `settings.spec.ts`, `search.spec.ts`,
  `zz-acceptance.spec.ts`, `tests.spec.ts` (partes) e `build.spec.ts` (2) falham por isso, no
  merge-base e nesta branch. A suíte E2E é do macOS (`AGENTS.md`).
- **Falhas pré-existentes confirmadas no merge-base** (`origin/main` 66d4805, worktree separada):
  `gates.spec.ts` › D-13 a 1440×900 (o popover de trust de 380 px não cruza os pontos amostrados em
  x≥652; a 1100×700 passa) e `shell.spec.ts` › "the single bar keeps placeholders disabled…"
  (espera `Cmd+,` no `title`). Nenhuma das duas é desta frente e nenhuma foi "consertada" aqui.
- **Tema claro, leitor de tela, zoom e reflow** — não testados (o tema claro não é distribuído).
- **Windows e macOS Intel** — não executados.
