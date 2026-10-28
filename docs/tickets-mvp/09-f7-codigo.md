# 09 — F7: Código — gutter, ⌥↩, menu, rename inline e usages

**What to build:** ações de código no editor: ícones de Run e de teste no gutter, popup ⌥↩ com quick fixes/intenções e prévia, menu de contexto completo, rename inline com prévia e revalidação, e Find Usages somente leitura.

**Blocked by:** 02, 05, 06

**Status:** ready-for-agent

- [ ] O gutter mostra ícone de Run no ponto de entrada do projeto de startup e ícone por teste, apenas quando a posição é determinável sem ambiguidade; caso contrário, nenhum ícone (nada inventado).
- [ ] Clicar no ícone de Run/teste executa o mesmo fluxo do widget, com os gates atuais.
- [ ] ⌥↩ abre o popup no cursor com as ações retornadas pelo LSP para a posição/revisão atuais, mostra o diff de prévia e aplica com revalidação de sessão, contexto, buffers e disco.
- [ ] Lista vazia ou ação não suportada aparece com motivo; nenhuma quick fix é fabricada a partir de código CS.
- [ ] O menu de contexto do editor tem Context Actions, Find Usages, Go To Definition, Rename, Run/Debug Test e Git; itens indisponíveis ficam desabilitados com motivo.
- [ ] Rename inline abre campo sobre o identificador, Enter aplica pelo fluxo de prévia existente, Esc cancela, e edição concorrente invalida a prévia.
- [ ] Find Usages (⇧F12) lista ocorrências por arquivo e navega; é somente leitura e não aplica edições.
- [ ] Refactor This não existe no MVP e o menu não aponta para uma ação inexistente.
- [ ] `bunx tsc --noEmit`, `cargo test -p openridr-host` e a suíte Playwright completa passam; commit local `feat(F7): ...`.
