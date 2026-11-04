# 09: Criar, renomear e apagar arquivos pela árvore

**What to build:** A Solution Explorer ganha criar, renomear e apagar arquivo/pasta, confinado ao workspace, com diálogo de nome e confirmação no delete. A árvore atualiza depois da operação.

**Blocked by:** 05 (o refresh usa o mesmo caminho de frescor da árvore).

**Status:** ready-for-agent

- [ ] O menu de contexto de projeto/pasta oferece "New file…", "Rename…" e "Delete".
- [ ] Criar grava um arquivo vazio (ou com o nome digitado) dentro da pasta escolhida.
- [ ] Renomear move o arquivo/pasta e mantém o conteúdo.
- [ ] Apagar pede confirmação e não age fora do workspace.
- [ ] Caminhos com `..`, absolutos fora do workspace e pais inexistentes são recusados com erro claro.
- [ ] A operação marca a avaliação stale e a árvore reflete o resultado.
- [ ] Aba aberta do arquivo renomeado/apagado não corrompe o buffer; o conflito existente cuida do caso.
- [ ] Testes de host para confinamento e operações; e2e para criar/renomear/apagar pela UI.
- [ ] Fora: drag-drop, templates, multi-select, operações fora do workspace.
