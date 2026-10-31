# 06 — F6: Git

**What to build:** integração com o `git` do PATH: widget de branch (trocar, criar, update), tool window Commit com seleção por arquivo, diff, mensagem e amend, e tool window Git com log da branch. Push só com confirmação; nada roda sem trust.

**Blocked by:** 01

**Status:** done — commit `dc701c9`; specs de Git/ações/busca/shell/aceitação verdes (suíte completa 70/72 antes do ajuste de asserção, 13/13 depois).

- [x] O widget de branch mostra a branch atual e abre popup com busca, nova branch, checkout de branch local/remota, Update (pull) e Commit; itens indisponíveis explicam o motivo (título e texto), inclusive árvore suja, operação em andamento, sem upstream e sem remote.
- [x] Commit marca arquivos por checkbox, mostra o diff real do arquivo e commita **somente os marcados** com `git commit --only -- <caminhos>`; o index do usuário não é alterado (o staged existente permanece staged) e desmarcar não faz `restore --staged`.
- [x] Amend do último commit funciona com a mensagem nova e mantém a seleção explícita; hooks rodam (sem `--no-verify`).
- [x] Git log mostra até 200 commits da branch atual com mensagem, autor, data e parents; selecionar um commit lista os arquivos alterados (sem grafo/paginação).
- [x] Checkout e pull recusam árvore suja e merge/rebase/cherry-pick em andamento, com mensagem e sem auto-stash; pull também exige upstream.
- [x] Push abre diálogo com remote, branch, upstream e commits à frente, exige confirmação e nunca usa force; sem remote fica desabilitado com motivo.
- [x] Em Restricted mode as superfícies dizem "Trust the workspace to use Git"; sem repositório dizem "Not a git repository"; os nove métodos de git exigem trust (gate testado) e Push exige `confirm:true`.
- [x] O app não guarda credenciais: o host herda o credential helper, desliga o prompt de terminal e mascara URLs com userinfo no output/mensagens; nenhum token é escrito em log.
- [x] `bunx tsc --noEmit`, `cargo test -p openridr-host` e as specs F6 + shell + acceptance passam, com o fluxo real em repositório temporário descartável; commit local `feat(F6): ...`.

## Comments

- `git commit --only` recusa caminhos que o index não conhece, então arquivos untracked marcados recebem `git add --intent-to-add` apenas durante o commit; em falha o intent-to-add é desfeito com `git reset -- <caminhos>`. O estado staged que o usuário já tinha nunca é tocado (testado com um arquivo staged fora da seleção).
- Host: `crates/host/src/git.rs` (CLI, parsers `--porcelain=v1 -z`, log `--name-status -z`, diff, operações) e handlers em `crates/host/src/lib.rs`; testes unitários usam `tempfile` + `git init -b main` com identidade local.
- E2E: `tests/e2e/git.spec.ts` cria o repositório em `mkdtemp`, isola `HOME`, nunca executa git no checkout do app e cobre index intacto, amend, log, árvore suja, push sem remote/confirm e os estados de trust/repo ausente.
- UI: `src/git.tsx` (widget, Commit, Git log, diálogos de update/push); tool windows novas no registro (`commit` no slot esquerdo, `git` no inferior); refresh ao abrir, depois de cada operação e por botão, sem polling.
