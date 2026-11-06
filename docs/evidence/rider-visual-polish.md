# Rider visual polish

## Superfície web isolada

- Capturas em 1470×880: [antes](rider-visual-polish/antes.png) e [depois](rider-visual-polish/depois.png).
- `web.sh up` com `OPENRIDR_BUNDLED_NETCOREDBG` apontando para os recursos do app instalado; fixture isolada OrderDesk, projeto confiado, Program.cs e Order.cs abertos.
- O estado de avaliação mudou entre as capturas: depois aparece o aviso de modelo desatualizado após o restore. As capturas não demonstram equivalência desse estado.
- A referência visual externa não foi copiada para o repositório; glifos de C#, projeto e pasta não foram alterados.
- App desktop, Windows e macOS Intel: não verificados nesta sessão.

## Validação

- Após integrar `origin/main` no merge `15b4b62`, `bun run check` passou e `bun test tests/unit` passou (33 testes).
- E2E solicitado não iniciou: as portas 1540 e 4437 já estavam ocupadas por processos de outra sessão. Não foram encerrados nem reutilizados; os quatro specs permanecem não verificados.
- As primeiras capturas foram feitas antes do merge; a comparação final está registrada abaixo.
- O detector Impeccable sinalizou Inter; fonte mantida por exigência explícita do ticket.

## Ajuste pós-merge

- Novas capturas em 1470×880: [antes](rider-visual-polish/antes-pos-merge.png) e [depois](rider-visual-polish/depois-pos-merge.png). Ambas mostram `Evaluated · Debug · TFM net8.0 · revision 2`, sem estado stale, com os mesmos arquivos abertos.
- Projeto selecionado deixa de competir visualmente com o arquivo atual; foco por teclado mantém contorno. A barra mantém o breadcrumb e a avaliação compacta; detalhes de SDK/runtime aparecem no popover de confiança e a avaliação completa permanece no título.
- As portas anteriormente consideradas de outra sessão eram resíduos deste worktree; processos encerrados após confirmação do orquestrador.
- Os quatro specs E2E rodaram nas portas 1540/4437: 21 passaram e 8 falharam na primeira execução (expectativas anteriores de layout/seleção e dois timeouts de avaliação). Após ajustar as expectativas para o contrato visual e permitir até 90 segundos, os 9 casos selecionados para rechecagem passaram, incluindo todos os 8 anteriores.
- App desktop permanece não verificado. As capturas anteriores ao merge são mantidas como histórico e não são a comparação final.
