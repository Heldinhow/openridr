# Retro — menu de contexto nativo (2026-11-05)

## O que funcionou

- **Reproduzir antes de teorizar.** O primeiro palpite (o `AXShowMenu` não despacha `contextmenu` no
  WKWebView) estava errado: raiz, projeto e arquivo abriam o menu do app pelo mesmo caminho. O que
  separava os casos era **linha com handler** × **linha sem handler**. Comparar raiz × pasta de solução
  no app real custou dois `right-click` e matou a hipótese errada.
- Ler o contrato da ferramenta: o `right-click` do agent-desktop **sem** `--headed` é a ação de
  acessibilidade, com `--headed` é clique físico. Sem isso, a evidência da aceitação ("pasta não abre,
  projeto abre") parecia uma diferença entre linhas, quando parte dela era entre *gestos*.
- Um único dono do `preventDefault` (`TreeRow`) elimina a classe do bug: esquecer o handler numa linha
  nova deixa de significar "menu do navegador", no máximo "nenhum menu".

## O que quase passou batido

- **Regressão de foco.** O `EditorContextMenu` rouba o foco para o primeiro item. Sem devolver o foco à
  linha no `closeTreeMenu`, o menu por `Shift+F10` deixaria o usuário de teclado no `<body>`. O spec
  cobre isso com `toBeFocused()`.
- **Duas armadilhas da ferramenta**, ambas registradas para a próxima sessão: um clique físico na janela
  de fundo só ativa a janela e não chega à página (o menu React não fecha — precisa de
  `focus-window` antes); e `press escape --headed` vira `AXCancel`, que não fecha o menu React, então
  `Escape` do app só foi provado no Chromium (E2E). No app real, fecharam-se os menus com clique físico.
- Clicar "Reload" do menu nativo do WebKit **recarrega o webview** e derruba o workspace aberto: foi o
  que aconteceu ao usar o primeiro script de "fechar menu". Fechar menu nativo é clique fora, nunca
  clicar num item.

## Revisão (o que a primeira rodada errou)

- **"Nenhuma área fora de campo de texto" era regra errada.** O terminal não é área de texto: o xterm
  trata o `contextmenu` no próprio elemento só para mover o foco para o `textarea` escondido, e deixa o
  menu nativo fazer Copiar/Colar. O guard cancelava o único jeito de copiar de um shell. A exceção agora
  se chama "onde o menu do sistema **é** a funcionalidade" (campo de texto, `.xterm`) e tem teste.
  Generalizar demais uma defesa é tão errado quanto não tê-la — a pergunta certa não é "é texto?", é
  "quem implementa este menu?".
- **`[contenteditable]` não é `contenteditable`.** O atributo com `="false"` casava com o seletor e
  deixava passar o menu do navegador; o teste unit com DOM falso fixou a semântica
  (`:not([contenteditable="false"])`).
- **O keymap global roda em capture.** Minha linha interceptava `Shift+F10`/tecla de menu, mas o
  handler do `WorkspaceView` roda **antes** (capture) e, no perfil IntelliJ, `Shift+F10` é Run All
  Tests. Regra que ficou: com o foco numa `.tree-row` o evento volta para a linha *antes* de casar
  atalho; fora da árvore o keymap manda. Teste cobre os dois lados, e no app real o `Shift+F10` no
  editor rodou a suíte de verdade.
- **Foco no mouse não é o foco do teclado.** Guardar `document.activeElement` funciona para
  `Shift+F10` e falha no clique direito, porque o WKWebView não move o foco ao clicar. Agora quem
  desenha a linha é dono do foco: `onMenu(position, row)` e o menu devolve o foco a `row`, nos dois
  caminhos.
- **"Add Project…" na solution folder.** A ação adiciona na raiz da solução; oferecê-la numa pasta é
  prometer o que ela não faz. Menos é mais: o menu da pasta tem só o que é dela.

## Incidente operacional (vale mais que o código)

`git stash` é **global ao repositório**, e este repo tem worktrees de vários agentes rodando em
paralelo. Um `git stash push` meu virou `stash@{1}` porque outra sessão empurrou o dela para
`stash@{0}` no meio do caminho; o `git stash pop` seguinte puxou o stash **dela** e deixou quatro
arquivos Rust em conflito (`UU`) no meu worktree. Saída: `git checkout HEAD --` nos arquivos alheios,
recuperar o meu WIP por `git stash show -p` + `git apply` (sem `pop`), conferir e dropar só o meu.
Lição: neste repositório, provar "vermelho antes" com `cp` de arquivo para `/tmp` e restauração, nunca
com `stash`.

## Custo/benefício

- O guard global é 12 linhas e uma condição (`import.meta.env.PROD`). A alternativa nativa (desligar o
  `WKWebView` context menu) exigiria swizzle de Objective-C ou dependência nova — fora das regras.
- Três menus novos de linha reaproveitam builders existentes; nenhuma ação nova de host, nenhum
  `data-testid` novo.

## Dívida deixada

- O nome acessível dos menus de arquivo/pasta era `Solution context menu` por default; agora diz o tipo
  (`File`/`Folder context menu`). Continua sem `README` em
  `.agents/skills/verify-openridr/features/` para "menus da árvore" — a receita está no spec E2E.
- Windows/Intel e o DMG desta correção não rodaram (ver "não verificado" na evidência).
