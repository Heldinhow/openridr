# 06 — F6: Git

**What to build:** integração com o `git` do PATH: widget de branch (trocar, criar, update), tool window Commit com seleção por arquivo, diff, mensagem e amend, e tool window Git com log da branch. Push só com confirmação; nada roda sem trust.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] O widget de branch mostra a branch atual e abre popup com busca, nova branch, checkout de branch local/remota, Update (pull) e Commit; itens indisponíveis explicam o motivo.
- [ ] Commit marca arquivos por checkbox, mostra o diff do arquivo e commita **somente os marcados** com `git commit --only -- <caminhos>`; o index do usuário não é alterado e desmarcar não faz `restore --staged`.
- [ ] Amend do último commit funciona com a mensagem nova e mantém a seleção explícita.
- [ ] Git log mostra até 200 commits da branch atual com mensagem, autor, data e parents; selecionar um commit lista os arquivos alterados.
- [ ] Checkout e pull recusam árvore suja, merge/rebase/cherry-pick em andamento, com mensagem e sem auto-stash.
- [ ] Push abre diálogo com remote, branch, upstream e commits à frente, exige confirmação e nunca usa force.
- [ ] Em Restricted mode as superfícies dizem "Trust the workspace to use Git"; sem repositório dizem "Not a git repository"; nenhum comando de escrita roda sem confirmação.
- [ ] O app não guarda credenciais: usa o credential helper do git e não registra token/senha/chave em log.
- [ ] `bunx tsc --noEmit`, `cargo test -p openridr-host` e a suíte Playwright completa passam (git exercitado em repositório temporário); commit local `feat(F6): ...`.
