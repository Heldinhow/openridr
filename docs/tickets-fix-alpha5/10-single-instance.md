# 10: Instância única ao abrir o binário direto

**What to build:** Rodar o binário direto com o OpenRidr já aberto reutiliza a instância existente e entrega o caminho/argv a ela, em vez de abrir uma segunda janela no mesmo `OPENRIDR_HOME`.

**Blocked by:** None (can start immediately).

**Status:** implementação concluída em código e testes de unidade; prova no app Tauri real pendente (`verify-openridr`).

- [x] `tauri-plugin-single-instance` é registrado como primeiro plugin (dependência aprovada pelo Helder).
- [x] A segunda execução encerra e encaminha o argv à instância existente.
- [x] Um `.sln` passado à segunda execução abre na instância existente pelo caminho normal de `workspace.open`.
- [ ] `open -a` e Finder continuam funcionando como hoje, sem instância dupla.
- [x] Buffer sujo na instância existente mantém o guard atual (não descarta edição).
- [ ] Prova no app real: duas execuções diretas → um processo, caminho encaminhado.

Os dois itens em aberto exigem o bundle rodando (LaunchServices/Finder e duas execuções diretas); o resto está coberto por `cargo test --bin openridr` e `cargo check`.
