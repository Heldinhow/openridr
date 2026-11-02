# 39 — Solution: barra e Show All Files

- [x] Barra: Show All Files, buscar (filtro por nome), locate, expand, collapse, ⋮ e minimizar.
- [x] Host `workspace.tree {dir}`: listagem de um nível, só leitura, sem trust, confinada ao workspace; `.git` nunca listado. Teste `workspace_tree_lists_one_level_inside_the_workspace`.
- [x] Show All Files: `bin`, `obj`, `.idea`, dotfiles e arquivos fora do modelo aparecem com `· no index` e fundo destacado; fontes continuam indexados.

Limite: "no index" é heurístico (pastas ignoradas do host, dotfiles e arquivos que não são fonte avaliada); `.gitignore` não é lido.
