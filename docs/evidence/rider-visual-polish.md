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
- As capturas foram feitas antes do merge; a aparência após a integração não foi recapturada.
- O detector Impeccable sinalizou Inter; fonte mantida por exigência explícita do ticket.
