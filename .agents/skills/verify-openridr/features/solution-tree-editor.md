# 2. Árvore da solution + editor

## Sub-features

- Solution Explorer estilo Rider: raiz com contagem ("3 projects"), solution folders, projetos com TFM, `Dependencies`, fontes; colapsar/expandir tudo, filtro, localizar, "show all files" (bin/obj/dotfiles sem índice), menu de contexto (bloqueia execução em Restricted).
- Editor CodeMirror: abas, sujo (`dirty-marker`), salvar (`save-file`, Cmd+S), conflito com disco, barra de status (`Ln 1, Col 1`).
- Avaliação MSBuild automática após Trust (`eval-state` "Evaluated" **na barra de status**).
- Visual por tipo: projeto `data-project-kind` (console/library/test/web), arquivo `data-file-kind`, `.cs` com `data-symbol-kind` (class/interface/enum/record/struct) ou glifo C# quando não há evidência; `no index` esmaecido.

## How to get to it (user POV)

Abrir e decidir o Trust (feature 3) → painel "Solution" à esquerda → clicar num arquivo abre a aba no editor.

## Driving it

- **A**: `bunx playwright test tests/e2e/solution-explorer.spec.ts` (+ `medium-editor-state.spec.ts` se mexeu em abas/salvar).
- **B** (provado em 2026-11-03): depois de abrir e confiar, `browser.click` em `[data-testid="tree-file-src/OrderDesk.Cli/Program.cs"]`; `browser.snapshot` com `selector: [data-testid="editor"]` deve conter o texto do arquivo (`using OrderDesk.Core;` … `Console.WriteLine`) e `Ln 1, Col 1`. Seletores: `tree`, `tree-root`, `tree-project-OrderDesk.Cli`, `tree-file-<caminho relativo>`, `editor`, `tab-*`.
- Para provar **salvar**: edite pelo teclado (`browser.type` no editor), Cmd+S, depois leia o arquivo **da cópia** em `<state>/work/...` (o `<state>` é impresso pelo `web.sh up`) com `cat` e confira a alteração em disco; `app-log` mostra `Saved <caminho>`.
- `Project details` agora abre pelo ⋮ (`tree-more` → `tree-more-details`); não há mais botão no cabeçalho. `eval-run` é o ícone de refresh ao lado de `status-evaluation`.
- `Dependencies` agora tem grupos `Frameworks` / `Packages` / `Projects` (`tree-deps-group-<projeto>-<grupo>`), fechados por padrão e omitidos quando vazios; não há mais os sufixos `· project`/`· package`.
- Para provar a diferenciação: numa cópia da fixture, crie `IThing.cs` (interface), `Kind.cs` (enum), `Point.cs` (record) e `Money.cs` (struct), aguarde a avaliação e confira `data-symbol-kind`/o badge; um `.cs` só com top-level statements deve manter o glifo C# (sem `data-symbol-kind`).
- **C**: `agent-desktop snapshot --app OpenRidr -i --compact`; a árvore aparece como itens de lista/botões com o nome visível ("Program.cs").

Prova de que funciona: a árvore lista os 3 projetos (`OrderDesk.Cli`, `OrderDesk.Core`, `OrderDesk.Tests`) com 3 de 3 "Evaluated", o arquivo abre com o conteúdo real do disco, e salvar muda o arquivo no disco. Visual provado no app real em 2026-11-04 (capturas em `docs/evidence/solution-explorer-visual/`): grupos de Dependencies, badges I/E/R/S/C e `no index` atenuado.

## Gotchas

- Logo após o Trust há restore automático ("Restoring packages…"); a árvore pode mostrar "Partial workspace · incomplete context" até ele terminar. Espere `eval-state`/`restore-state` em vez de dormir.
- `workspace.spec.ts` e `run.spec.ts` gravam capturas em `<tmpdir>/openridr-e2e-screenshots-<porta da UI>` por default, nunca no repositório; `docs/evidence/screenshots` só recebe captura com `OPENRIDR_SCREENSHOT_DIR` apontando para lá.
- Viewport estreito esconde painéis; para a árvore completa use ≥ 1440×900.
