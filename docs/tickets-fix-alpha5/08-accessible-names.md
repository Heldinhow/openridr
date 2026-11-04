# 08: Nomes acessíveis inequívocos

**What to build:** Leitores de tela e automação distinguem as três ações "Run", as duas "Save" e o menu de contexto do projeto. Os rótulos visíveis continuam os mesmos.

**Blocked by:** None (can start immediately).

**Status:** implementação concluída em código e e2e; prova com leitor de tela real não feita nesta rodada.

- [x] As três entradas "Run" do keymap têm nomes acessíveis distintos (tool window, run configuration, build/run action).
- [x] As duas "Save" (arquivo e settings) têm nomes acessíveis distintos.
- [x] O menu de contexto de projeto se chama "Project context menu" (ou equivalente inequívoco), não "Solution context menu".
- [x] O toolband usa o nome do grupo/ação, não só o título curto, quando há colisão.
- [x] Rótulos visíveis não mudam; `data-testid` existentes não mudam.
- [x] e2e de settings/keymap cobre a desambiguação.
