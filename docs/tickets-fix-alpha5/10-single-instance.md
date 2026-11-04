# 10: Instância única ao abrir o binário direto

**What to build:** Rodar o binário direto com o OpenRidr já aberto reutiliza a instância existente e entrega o caminho/argv a ela, em vez de abrir uma segunda janela no mesmo `OPENRIDR_HOME`.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] `tauri-plugin-single-instance` é registrado como primeiro plugin (dependência aprovada pelo Helder).
- [ ] A segunda execução encerra e encaminha o argv à instância existente.
- [ ] Um `.sln` passado à segunda execução abre na instância existente pelo caminho normal de `workspace.open`.
- [ ] `open -a` e Finder continuam funcionando como hoje, sem instância dupla.
- [ ] Buffer sujo na instância existente mantém o guard atual (não descarta edição).
- [ ] Prova no app real: duas execuções diretas → um processo, caminho encaminhado.
