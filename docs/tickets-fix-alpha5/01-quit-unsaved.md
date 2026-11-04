# 01: Quit pergunta antes de fechar

**What to build:** Cmd+Q, o item Quit do menu e o quit pelo Dock passam pelo mesmo diálogo de alterações não salvas que o botão Close da janela já usa. "Save all" grava e encerra; "Keep editing" mantém o app aberto e o buffer intacto; "Discard and close app" encerra sem gravar.

**Blocked by:** None (can start immediately).

**Status:** implementação concluída — prova no app Tauri real pendente (`verify-openridr`); Dock Quit é `terminate:` nativo, limitação em `docs/decisions.md`.

- [x] O menu do app tem um item Quit próprio com Cmd+Q que pede a decisão à UI em vez de encerrar direto.
- [x] `ExitRequested` do Tauri impede a saída, pergunta à webview e só encerra após uma resposta autorizada.
- [x] O diálogo existente ganha o caso "quit" reutilizando os três botões e o mesmo texto de fechamento.
- [x] "Keep editing" cancela a saída e o app continua com o buffer sujo e o foco no editor.
- [x] "Save all" grava todos os buffers sujos antes de encerrar.
- [x] App sem buffers sujos encerra imediatamente, sem diálogo.
- [x] Fechar a janela pelo botão Close continua funcionando como hoje (sem diálogo duplicado).
- [ ] Prova no app Tauri real de Cmd+Q com buffer sujo, com antes/depois em `docs/evidence/fix-alpha5/`.
- [x] AppleScript `quit` registrado como limitação conhecida (fora do hook do Tauri).
