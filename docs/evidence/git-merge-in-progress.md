# Evidência — Merge em andamento: concluir ou abortar pela UI (issue #114)

Data: 2026-11-07. Alvo verificado: macOS 27.x arm64 (Apple Silicon). Windows e macOS Intel **não** foram executados.

Código sob teste: `8a6ad87` (#154, abort), `d4324fa` (#155, concluir + commit bloqueado), `04cfdac` (#156, Update deixa o merge aberto) e `371b88d` (texto da recusa por marcador), com `origin/main` já mergeada (`ccd5ce4`, #152). Versão `0.3.0-alpha.11`.

Bundle: `src-tauri/target/debug/bundle/macos/OpenRidr.app`, `CFBundleShortVersionString` `0.3.0-alpha.11`, construído nesta sessão do próprio checkout (`bun install` + `bun run tauri build --debug --bundles app`) **depois** de `371b88d`, então todas as capturas são do código final. Uma primeira rodada foi feita com o bundle anterior a `371b88d` e serviu só para achar o erro de concordância ("1 file still contain"); as capturas abaixo são da rodada final.

UI dirigida **somente** no app Tauri real via `agent-desktop` (sem navegador, sem Playwright de sessão), com `OPENRIDR_HOME` isolado, o `.sln` do clone em argv e state próprio (`VERIFY_OPENRIDR_DESKTOP_STATE=$TMPDIR/verify-openridr-issue114-desktop`). Uma instância por vez, fechada pelo PID gravado (`desktop.sh down` → `stopped`, `pgrep -x openridr` vazio).

## Precondições (não são passe da UI)

- **Fixture local**: `scripts/git-remote-fixture.sh $TMPDIR/verify-openridr-issue114-git` criou `origin.git` (bare), `user` e `peer` com o conteúdo público de `fixtures/OrderDesk` na raiz dos clones. Sem rede, sem login, sem `git config --global` (identidade só `--local` em cada clone). Baseline `08cb60c`.
- **Divergência criada por shell**: no clone `user`, `public decimal Total { get; }` → `{ get; private set; }` (commit `5d1c0c7`); no clone `peer`, a mesma linha → `{ get; init; }` (commit `b51317f`, publicado com `git push`). Resultado antes do Update: `## main...origin/main [ahead 1, behind 1]`.
- **O conflito foi criado pela própria UI**: o `Update…` do app fez o fetch e o merge, que conflitou. O shell só conferiu o efeito no disco.
- **Acesso ao menu de branches**: o `<summary>` do widget não tem `ref_id` na árvore de acessibilidade (`interactive: false`), então o menu foi aberto por clique físico nas coordenadas do disclosure (`agent-desktop mouse-click --xy 446,117 --headed`, bounds lidos de `agent-desktop find`). O resto foi por `ref` (`click`, `press`, `type` com `--headed` depois de `focus-window`).

## Quadro por cenário

| # | Cenário | Prova fora da UI (comando → resultado) | Status |
|---|---|---|---|
| S1 | Update conflitante deixa o merge aberto, com o ponteiro do menu | UI: diálogo `Update project` com a política nova ("… If the merge conflicts, it stays in progress: resolve the files in the editor, then finish or abort it from the branch menu. Nothing is pushed.") e o erro `Update stopped: 1 file changed on both sides (src/OrderDesk.Core/Order.cs). The merge is still in progress: resolve the conflicted files in the editor, then finish or abort it from the branch menu.` (vermelho, `role=alert`). Menu de branches com `Upstream origin/main · merge in progress`, o bloco `A merge is in progress: resolve the conflicts in the editor, then finish or abort it here.`, `Complete merge` e `Abort merge…`, e `Update…` desabilitado com "A merge is in progress; finish or abort it from the branch menu before checking out or updating." Disco: `git status --porcelain` → `UU src/OrderDesk.Core/Order.cs`; `.git/MERGE_HEAD` presente; `git log --oneline -2` → `5d1c0c7 local: private set` / `08cb60c Baseline…` (os dois commits intactos); marcadores nas linhas 13/15/17 do arquivo | **passou** |
| S2 | Complete merge sem resolver recusa e não escreve nada | UI: `Complete merge refused: 1 file still contains conflict markers (src/OrderDesk.Core/Order.cs). Resolve it in the editor first.` Disco: `git status --porcelain` continua `UU src/OrderDesk.Core/Order.cs`; `.git/MERGE_HEAD` presente; `git rev-parse HEAD` → `5d1c0c7` (nenhum commit criado); `git ls-files -u` continua com as três entradas unmerged | **passou** |
| S3 | Resolver no editor do app e concluir cria o merge commit | Editor do app: `cmd+a` + digitação do conteúdo resolvido + `cmd+s`; disco: arquivo sem marcador, `Total { get; private set; }`. UI depois de `Complete merge`: toast `Merge completed.`, bloco do merge desaparece, barra em `main ↑2`. Disco: `git log -1 --pretty='%h %s'` → `eebc2ea Merge remote-tracking branch 'origin/main'`; `git log -1 --format=%p` → `5d1c0c7… b51317f…` (commit local e commit da contraparte como parents); `.git/MERGE_HEAD` **ausente**; `git status --porcelain` → vazio; `git status -sb` → `## main...origin/main [ahead 2]` (o merge não foi empurrado) | **passou** |
| S4 | Commit tool durante o merge aponta o caminho e bloqueia | UI: nota `A merge is in progress; finish or abort it from the branch menu. A partial commit is not possible during a merge.`, `0 of 1 selected`, checkbox do `Order.cs` desabilitado, botão `Commit` desabilitado; o editor mostra `<<<<<<< HEAD` no arquivo | **passou** |
| S5 | Abort merge… pede confirmação e volta ao estado anterior | UI: modal `Abort merge` com "Undoes the merge: the branch, index and working tree go back to where they were before it. Conflict resolutions and anything staged during the merge are discarded; your commits stay." e `Abort merge`/`Cancel`. Depois de confirmar: toast `Merge aborted; the branch, index and working tree are back to where they were before the merge.`, `Update…` habilitado de novo. Disco: `.git/MERGE_HEAD` **ausente**; `git rev-parse HEAD` → `03a3f95` (igual ao de antes do Update); `git status --porcelain` → vazio; `grep Currency src/OrderDesk.Core/Order.cs` → `public string Currency { get; } = "BRL";` (conteúdo local, nenhum marcador) | **passou** |

Nenhuma mensagem mostrada na tela continha `hint:` nem conselho de editar `pull.rebase`/`pull.ff`.

## Onde o stderr cru do git ficou

O texto ao usuário é do app; a saída do git continua em `Operation.output`. Provas de host: `update_leaves_a_conflicting_merge_for_the_user_to_resolve` afirma `result.output.contains("CONFLICT")`. Para registrar o conteúdo no clone do state, os mesmos comandos do update foram rodados à mão **depois** dos cenários (declarado: é o mesmo comando no mesmo clone, não a captura do campo `output`):

```
$ git fetch --quiet
$ git merge --ff --no-edit '@{upstream}'
Auto-merging src/OrderDesk.Core/Order.cs
CONFLICT (content): Merge conflict in src/OrderDesk.Core/Order.cs
Automatic merge failed; fix conflicts and then commit the result.
```

Depois disso o merge foi abortado (`git merge --abort`) e o clone voltou a `03a3f95`, árvore limpa.

## Host tests (não é prova de UI)

- `cargo test -p openridr-host --lib -- git::tests` → **60 passed**. Os 7 casos novos: abort restaura `HEAD`/contagem/árvore/bytes locais e apaga `MERGE_HEAD`; abort recusa sem merge e com rebase em andamento; concluir stageia só os caminhos conflitados e cria merge de dois parents **sem** arrastar edição alheia (`git show HEAD:b.txt` mantém o conteúdo antigo e `status --porcelain` mostra `M b.txt`); concluir recusa com marcador remanescente (nada staged, `HEAD` igual, merge intacto); concluir recusa sem merge; `git.commit` recusa durante merge sem criar commit; Update conflitante deixa `MERGE_HEAD`/os dois commits/`status.operation == Some("merge")` e `merge_continue` depois fecha o merge; Update conflitante + `merge_abort` volta ao estado pré-merge.
- Bateria completa: `bun run check` ok; `bun test tests/unit` 89 passed; `cargo test -p openridr-host` **108 passed** no binário de integração e 394 na lib; `cargo clippy --workspace --all-targets -- -D warnings` ok; `cargo fmt --all --check` e `cargo fmt --manifest-path src-tauri/Cargo.toml --check` ok.
- Falha-antes: os métodos novos não existiam; o que prova a mudança de comportamento é o teste do Update, que reprova no código anterior (`MERGE_HEAD` ausente depois do pull e mensagem "aborted the merge"), e o de `git.commit`, que no código anterior devolvia o `git_failed` cru do Git em vez de `git_operation_in_progress`.

## Capturas (`docs/evidence/git-merge-in-progress/`)

| Arquivo | O que prova |
|---|---|
| [01-update-conflict-leaves-merge-open.png](git-merge-in-progress/01-update-conflict-leaves-merge-open.png) | Diálogo de Update com a política nova e a mensagem "The merge is still in progress… finish or abort it from the branch menu"; o bloco do merge no menu de branches por trás. |
| [02-complete-refused-markers.png](git-merge-in-progress/02-complete-refused-markers.png) | Recusa por marcador nomeando `src/OrderDesk.Core/Order.cs` ("1 file still contains conflict markers"). |
| [03-resolved-in-editor.png](git-merge-in-progress/03-resolved-in-editor.png) | O arquivo resolvido no editor do app, sem marcadores, antes de concluir. |
| [04-merge-completed.png](git-merge-in-progress/04-merge-completed.png) | `Merge completed.`, bloco do merge fora do menu, barra em `main ↑2`, árvore sem o conflito. |
| [05-commit-tool-during-merge.png](git-merge-in-progress/05-commit-tool-during-merge.png) | Commit tool com a nota do merge, `0 of 1 selected`, checkbox do conflitado e `Commit` desabilitados, marcadores no editor. |
| [06-abort-confirmation.png](git-merge-in-progress/06-abort-confirmation.png) | Modal de confirmação do abort com o que volta e o que é descartado. |
| [07-merge-aborted.png](git-merge-in-progress/07-merge-aborted.png) | `Merge aborted; the branch, index and working tree are back to where they were before the merge.`, `Update…` habilitado, árvore limpa. |

## Não verificado

- **Windows e macOS Intel**: não executados.
- **Rebase, cherry-pick e revert pela UI**: fora de escopo por decisão (o host recusa e a nota manda ao terminal); o teste de host cobre a recusa do rebase no abort.
- **Conflito em arquivo binário ou com `conflict-marker-size` customizado**: a guarda de marcador usa o critério do Git (`git diff --check`), não foi exercitada nesses casos.
- **Editar a mensagem do merge commit**: `--no-edit` por decisão.
- **Remoto autenticado/rede**: o remoto da rodada é um bare local.
- **`git.commit` bloqueado para rebase/cherry-pick/revert no app real**: provado só pelo teste de host do merge (o código é o mesmo `match`).
- **Teclado no menu de branches**: o widget não expõe `ref_id`; a abertura foi por clique físico, não por atalho.
