# Falso positivo: item recente do Welcome "não abre" no .app nativo (2026-10-31)

Status: **fechado como falso positivo** (não é defeito do produto). Causa: um `NSOpenPanel` modal, aberto por um clique anterior em **Open**, ficou na frente da janela e bloqueou a webview.

## Ambiente
- Bundle testado: HEAD `7063510` (produto igual a `a6e16a6`), `OpenRidr.app` 0.2.0, cópia em `/tmp`. Binário SHA-256 `34a87b3811ef47df2d781e5584fd2748cc06dd748e2c5b6b98954f826d684904`.
- Home isolado com `OPENRIDR_HOME` (confirmado em `ps eww`), `recent.json` com uma única solução: a cópia `OrderDesk.sln` em `/tmp`. Sem `trusted.json`.

## Sintoma inicial (observado, não reproduzia o defeito)
- Clique no item, Return e AXPress não abriam o workspace; a linha ficava selecionada.
- `screencapture -l <winid>` mostrava o Welcome: ele captura só a janela do app e **não mostra** o painel, que pertence ao `openAndSavePanelService`.

## Causa e resolução
- Diagnóstico com captura da **tela inteira** (só para diagnóstico, não publicada) mostrou o painel de abrir arquivo na frente. Cancel nele liberou a webview; um clique no item OrderDesk abriu o workspace (Restricted, 3 projetos).

## Lição de método (para os próximos smokes)
- Antes de concluir "clique não funciona", capture a **tela inteira** (com crop de privacidade antes de publicar) ou liste `AXSheet`/janelas do `openAndSavePanelService`.
- Não dispare **Open** sem necessidade: ele deixa um modal órfão que intercepta o foco.
- Revalide frontmost e a ausência de painel modal antes de cada tecla/clique.
