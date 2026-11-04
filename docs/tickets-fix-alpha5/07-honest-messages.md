# 07: Mensagens honestas de abertura e avaliação

**What to build:** Abrir `.cs`/`.txt` por argv explica que é preciso `.sln`, `.slnx`, `.csproj` ou pasta; um `.sln` sem projetos diz que o arquivo não é uma solução válida em vez de falar de SDK; a primeira avaliação com TFM inválido não diz "valores da avaliação anterior" quando não houve avaliação anterior.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] `scripts/openridr` recusa extensão não suportada com a mesma orientação da tela de pasta vazia e imprime qual bundle abriu.
- [ ] O app já trata `.cs`/`.txt` com mensagem no HEAD; e2e cobre argv para os dois.
- [ ] `.sln` que não lista projetos mostra "não lista projetos; não é uma solução válida" (sem mencionar SDK).
- [ ] `revision == 1` com erro de TFM não diz "previous evaluation"; a frase aparece só a partir de `revision > 1`.
- [ ] O reload reason (`evaluation.reloadReason`) segue a mesma regra.
- [ ] e2e cobre `.sln` inválido e TFM inválido na primeira avaliação.
