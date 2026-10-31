# Defeito aberto: item recente do Welcome não abre o workspace no .app nativo (2026-10-31)

Status: **aberto**, não corrigido. Severidade: **a confirmar** (bloqueia a prova nativa dos itens que dependem de abrir workspace; o fluxo NSOpenPanel já foi provado em c7439ad).

## Ambiente
- Bundle testado: HEAD `7063510` (produto igual a `a6e16a6`), `OpenRidr.app` 0.2.0, copiado para `/tmp` e executado de lá. Binário SHA-256 `34a87b3811ef47df2d781e5584fd2748cc06dd748e2c5b6b98954f826d684904`.
- Home isolado com `OPENRIDR_HOME` (confirmado em `ps eww`), contendo `recent.json` com um único item: a solução `OrderDesk.sln` em uma cópia da fixture em `/tmp`. Sem `trusted.json`.
- Janela de 1440×900 pt, lançada via `open -n --env OPENRIDR_HOME=...`.

## Repro
1. Abrir o app com o home acima. O Welcome mostra o item `OrderDesk` com o caminho da cópia e o texto "Opens in Restricted mode".
2. Clicar no item (CGEvent, janela frontmost). Resultado: a linha fica selecionada (azul) e o workspace não abre.
3. Pressionar Return com a linha selecionada. Mesmo resultado.
4. AXPress no `AXDisclosureTriangle` e no `AXButton` de nome "OrderDesk". Mesmo resultado.

## Observações
- Nenhuma mensagem de erro visível no Welcome após os cliques; o log do processo (`/tmp/ponto/run2.log`) só registra `sandbox_extension_issue_file_to_process ... Operation not permitted`.
- A árvore AX, em uma leitura, passou a conter elementos de workspace (`OrderDesk.sln (current)`, editor, "Trusted mode"), enquanto os pixels da janela mostravam o Welcome. Não ficou claro se é uma camada oculta ou um estado stale.
- Os semáforos da janela alternam entre coloridos (key) e cinza após o clique, mesmo com o processo como frontmost.
- Não foi possível separar se o clique não chega à webview (foco) ou se `workspace.open` falha silenciosamente.

## Próximo passo para fechar
- Reproduzir com o log do host capturado (stderr do processo) e com captura da janela logo após o clique, no mesmo estado de foco.
- Verificar se `workspace.open` retorna erro ou fica em busy (ver `src/SetupScreen.tsx` e o handler `open(solution, true)`).
