# 05: Árvore reflete mudanças externas

**What to build:** Criar, renomear ou apagar arquivos por fora atualiza a Solution Explorer sozinho, sem clicar em Re-evaluate. A seleção atual é preservada e falhas de avaliação continuam visíveis.

**Blocked by:** None (can start immediately).

**Status:** done

- [x] Os diretórios de fontes avaliados entram no watch do host.
- [x] Criar/renomear/apagar um `.cs` dispara re-avaliação com debounce (~500 ms), coalescendo uma rajada.
- [x] A árvore mostra o arquivo novo, o nome novo e some com o apagado.
- [x] A seleção da árvore não é perdida à toa.
- [x] Janela oculta não reavalia.
- [x] Falha de avaliação mantém o banner de stale e não inventa árvore.
- [x] Teste de host para os caminhos observados; e2e para criar/renomear/apagar por fora.
