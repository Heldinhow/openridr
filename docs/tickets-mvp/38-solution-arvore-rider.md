# 38 — Solution: árvore fiel ao Rider

Fonte: [SPEC-SOLUTION-EXPLORER](../SPEC-SOLUTION-EXPLORER.md).

- [x] Linha única (`TreeRow`): chevron, ícone, nome e sufixo cinza (`· console · net8.0`); avisos de erro/restore continuam coloridos.
- [x] Raiz recolhível com `· N projects`; pastas de solução com chevron, ícone e contagem.
- [x] Dependencies como nó expansível com filhos (projetos e pacotes reais).
- [x] Fontes aninhadas por pasta (pastas antes dos arquivos); outline textual usa as mesmas linhas.
- [x] "Scratches and Consoles" fica fora (sem backend).

Aceite: `solution-explorer.spec.ts` (linhas, contagens, raiz recolhível); suíte existente verde com os `data-testid` antigos.
