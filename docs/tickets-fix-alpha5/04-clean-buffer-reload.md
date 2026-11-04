# 04: Buffer limpo recarrega sozinho

**What to build:** Uma aba ativa sem alterações passa a refletir o arquivo do disco sem o usuário trocar de aba ou dar foco. Aba suja continua mostrando o diálogo de conflito atual.

**Blocked by:** 03 (compartilha a leitura/escrita de arquivo e o hash de conflito).

**Status:** ready-for-agent

- [ ] Aba limpa e ativa recarrega em até ~2 s depois de uma escrita externa, com a janela visível.
- [ ] O cursor/scroll não salta para o início de forma perceptível numa recarga simples.
- [ ] Aba suja com disco mudado mantém "changed on disk … Reload from disk / Overwrite disk".
- [ ] Janela oculta não fica relendo em loop.
- [ ] Troca de aba e foco continuam recarregando como hoje.
- [ ] e2e cobre escrita externa em aba limpa e em aba suja.
