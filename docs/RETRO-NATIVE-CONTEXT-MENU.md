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
