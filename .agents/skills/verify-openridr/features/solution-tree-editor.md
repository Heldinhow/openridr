# 2. Árvore da solution + editor

## Sub-features

- Solution Explorer estilo Rider: raiz com contagem ("3 projects"), solution folders, projetos com TFM, `Dependencies`, fontes; colapsar/expandir tudo, filtro, localizar, "show all files" (bin/obj/dotfiles sem índice), menu de contexto (bloqueia execução em Restricted).
- Editor CodeMirror: abas, sujo (`dirty-marker`), salvar (`save-file`, Cmd+S), conflito com disco, barra de status (`Ln 1, Col 1`).
- Avaliação MSBuild automática após Trust (`eval-state` "Evaluated").

## How to get to it (user POV)

Abrir e decidir o Trust (feature 3) → painel "Solution" à esquerda → clicar num arquivo abre a aba no editor.

## Driving it

- **A**: `bunx playwright test tests/e2e/solution-explorer.spec.ts` (+ `medium-editor-state.spec.ts` se mexeu em abas/salvar).
- **B** (provado em 2026-11-03): depois de abrir e confiar, `browser.click` em `[data-testid="tree-file-src/OrderDesk.Cli/Program.cs"]`; `browser.snapshot` com `selector: [data-testid="editor"]` deve conter o texto do arquivo (`using OrderDesk.Core;` … `Console.WriteLine`) e `Ln 1, Col 1`. Seletores: `tree`, `tree-root`, `tree-project-OrderDesk.Cli`, `tree-file-<caminho relativo>`, `editor`, `tab-*`.
- Para provar **salvar**: edite pelo teclado (`browser.type` no editor), Cmd+S, depois leia o arquivo **da cópia** em `/tmp/verify-openridr/work/...` com `cat` e confira a alteração em disco; `app-log` mostra `Saved <caminho>`.
- **C**: `agent-desktop snapshot --app OpenRidr -i --compact`; a árvore aparece como itens de lista/botões com o nome visível ("Program.cs").

Prova de que funciona: a árvore lista os 3 projetos (`OrderDesk.Cli`, `OrderDesk.Core`, `OrderDesk.Tests`) com 3 de 3 "Evaluated", o arquivo abre com o conteúdo real do disco, e salvar muda o arquivo no disco.

## Gotchas

- Logo após o Trust há restore automático ("Restoring packages…"); a árvore pode mostrar "Partial workspace · incomplete context" até ele terminar. Espere `eval-state`/`restore-state` em vez de dormir.
- `workspace.spec.ts` e `run.spec.ts` gravam capturas em `docs/evidence/screenshots` (ignorado pelo git); aponte `OPENRIDR_SCREENSHOT_DIR` para `<tmp>` para não sujar a worktree.
- Viewport estreito esconde painéis; para a árvore completa use ≥ 1440×900.
