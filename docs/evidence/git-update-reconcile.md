# Evidência — Update reconcilia branch divergente (issue #112)

Data: 2026-11-06. Alvo verificado: macOS 27.x arm64 (Apple Silicon). Windows e macOS Intel **não** foram executados.

Código sob teste: `bb600fc` (host, #127), `591a790` (UI, #128) e `cd23b43` (correções da revisão independente), com `origin/main` já mergeada (`721ae53`, #111; `e467fc8`, #49). Versão `0.3.0-alpha.11`.

Bundle: `src-tauri/target/debug/bundle/macos/OpenRidr.app`, `CFBundleShortVersionString` `0.3.0-alpha.11`, construído nesta sessão do próprio checkout (`bun install` + `bun run tauri build --debug --bundles app`) às 20:02 sobre o conteúdo de `src/git.tsx` e `crates/host/src/git.rs` que virou `cd23b43`, e refundido às 20:25 depois do merge de `origin/main` para a rodada S5. Cada captura diz de qual rodada veio.

UI dirigida **somente** no app Tauri real via `agent-desktop` (sem navegador, sem Playwright de sessão), com `OPENRIDR_HOME` isolado, o `.sln` do clone em argv e state próprio (`VERIFY_OPENRIDR_DESKTOP_STATE=$TMPDIR/verify-openridr-112-desktop`). Uma instância por vez, fechada pelo PID gravado.

## Precondições (não são passe da UI)

- **Dois states do fixture local**: `scripts/git-remote-fixture.sh` criou `origin.git` (bare), `user` e `peer` com o conteúdo público de `fixtures/OrderDesk` na raiz dos clones. Sem rede, sem login, sem `git config --global` (identidade só `--local` em cada clone). Os diretórios ficaram no disco; nada foi apagado.
  - **State A** (`openridr-git-remote.j4kuAo`): o clone `user` estava em `d423be7` (o merge da primeira rodada) e commitou `88f13ba` em `tests/OrderDesk.Tests/OrderTests.cs`; `peer` (em `decbe33`) publicou `d0600cf` em `src/OrderDesk.Cli/Program.cs`. Resultado antes do Update: `ahead 3, behind 1`.
  - **State B** (`openridr-git-remote.RNKWta`): `user` em `9c9d6f2` e `peer` em `9fe75c1`, os dois alterando **a mesma linha** de `src/OrderDesk.Core/Order.cs` (`namespace OrderDesk.Core;` → `// local-112-conflict` / `// peer-112-conflict`), resultado `ahead 3, behind 1`.
- **`fetch` manual no clone do usuário** antes de cada Update, para os contadores da tela refletirem a contraparte (o app não tem ação de fetch — #113). O `Update…` funciona sem esse fetch; o número é que ficaria defasado.
- **Estado de conflito criado pela própria UI**: aqui o merge divergente e o conflito foram produzidos pelo `Update…` do app, e o abort também. O shell só conferiu o efeito.
- **Atalho**: os dois primeiros bundles desta sessão foram construídos antes do merge do #111, quando `⌥⌘W` ainda era do Close All e derrubava o app; por isso S1–S4 abriram o diálogo por **Find Action** (`⇧⌘A` → "Update Project…"). Depois do merge de `origin/main`, o bundle refeito abriu o mesmo diálogo com `⌥⌘W` e o app sobreviveu (S5).

## Quadro por cenário

| # | Cenário | Prova fora da UI (comando → resultado) | Status |
|---|---|---|---|
| S1 | Diálogo declara a política e mostra os dois contadores | `git status -sb` no clone → `## main...origin/main [ahead 3, behind 1]`. UI: `Update project` com `Branch main`, `Upstream origin/main`, `Ahead 3`, `Behind 1`, a frase "When the branch and its upstream both moved, Update fetches and merges the upstream into a merge commit (`git fetch`, then `git merge --ff --no-edit`); a fast-forward stays a fast-forward. The repository's `pull.rebase`, `pull.ff` and `merge.ff` settings do not change that. If the merge conflicts, OpenRidr aborts it and leaves your branch, commits and working tree unchanged; if it cannot abort, the dialog says so.", e `Update`/`Cancel` | **passou** (state A) |
| S2 | Update divergente termina em merge commit com os dois lados | UI: `Update completed.`; barra de branch passa a `main ↑4` (sem `↓`). `git log -1 --pretty='%h %s'` → `930e328 Merge remote-tracking branch 'origin/main'`; `git log -1 --format=%p` → `88f13ba d0600cf` (commit local e commit da contraparte como parents); `git status --porcelain` → vazio; `git status -sb` → `## main...origin/main [ahead 4]`; a última linha de `src/OrderDesk.Cli/Program.cs` é `// Peer change for issue 112 (round 2 merge)` e a de `tests/OrderDesk.Tests/OrderTests.cs` é `// Local change for issue 112 (round 2 merge)` | **passou** (state A) |
| S3 | O merge commit aparece no histórico | UI: `Git log` lista `930e328 Merge remote-tracking branch 'origin/main'` com o selo `2 parents`, acima de `88f13ba` e `d0600cf`, igual a `git log --oneline -4` | **passou** (state A), com a observação abaixo |
| S5 | Update sem nada novo não inventa merge | `⌥⌘W` no bundle com #111: diálogo abre (`Ahead 4`, `Behind 0`) e o app continua vivo. Depois de `Update`: `Update completed.`; `git log -1` continua `930e328` (nenhum merge commit novo), `git status --porcelain` vazio, `git status -sb` → `[ahead 4]` | **passou** (state A, já atualizado) |
| S4 | Conflito aborta, em palavras do app, sem perder nada | UI: o diálogo continua aberto com `Update stopped: 1 file changed on both sides (src/OrderDesk.Core/Order.cs). OpenRidr aborted the merge; your branch and working tree are unchanged.` (vermelho, `role=alert`), `Ahead 3`, `Behind 1`. Depois do clique: `git rev-parse --short HEAD` → `9c9d6f2` (inalterado), `git log -1` → `Local same-line change for issue 112 conflict`, `.git/MERGE_HEAD` **ausente**, `git status --porcelain` → vazio, primeira linha de `Order.cs` → `namespace OrderDesk.Core; // local-112-conflict` (nenhum marcador `<<<<<<<`), `git status -sb` → `[ahead 3, behind 1]` | **passou** (state B) |

Nenhuma mensagem de erro mostrada na tela continha `hint:` nem o conselho de editar `pull.rebase`/`pull.ff`.

## Onde o stderr cru do git ficou

O texto mostrado ao usuário é do app; a saída do git não foi descartada, continua no `output` da operação. Provas de host: `update_aborts_a_conflicting_merge_and_keeps_both_commits` afirma `result.output.contains("CONFLICT")`. Para registrar o conteúdo exato no clone do state B, os mesmos comandos do update foram rodados à mão **depois** do cenário (declarado: não é a captura do campo `output`, é o mesmo comando no mesmo clone):

```
$ git fetch --quiet
$ git merge --ff --no-edit '@{upstream}'
Auto-merging src/OrderDesk.Core/Order.cs
CONFLICT (content): Merge conflict in src/OrderDesk.Core/Order.cs
Automatic merge failed; fix conflicts and then commit the result.
```

Depois disso o merge foi abortado de novo (`git merge --abort`) e o clone voltou a `9c9d6f2`, árvore limpa.

## Observação sem ticket

- **O `Git log` não se recarrega sozinho depois do Update.** Em S2/S3, a lista continuou mostrando o histórico antigo depois do merge; só o botão `Refresh` do próprio Git log trouxe `930e328`. O `Refresh` do menu de branches (que atualiza o status) não recarrega o log. É comportamento pré-existente do painel (o log recarrega ao abrir/focar/botão), não regressão desta frente; fica como achado para triagem, sem ticket aberto aqui.

## Host tests (não é prova de UI)

`cargo test -p openridr-host -- git::tests::update` → 6 passed, com bare remoto real e clone `peer` real: fast-forward continua fast-forward (sem merge commit); divergência gera dois parents e preserva os dois arquivos (`ahead 2, behind 0` depois do merge, porque o merge commit local não foi empurrado); `pull.rebase=true` + `pull.ff=only` + `merge.ff=only` no repositório ainda resultam em merge; um merge em andamento de outro commit **não** é abortado (o `MERGE_HEAD` estranho continua lá); conflito aborta (`MERGE_HEAD` ausente, mesmo `HEAD`, árvore limpa, bytes locais preservados, `CONFLICT` no `output`); identidade ausente vira mensagem do app (`user.useConfigOnly=true` + `user.name`/`user.email` vazios, determinístico mesmo com config global). As árvores de teste fixam `commit.gpgsign=false` local, para uma exigência de assinatura da máquina não decidir o resultado. Falha-antes: restaurando o `pull` antigo, os testes de divergência, config e conflito falham (o de fast-forward passa nas duas versões — é guarda de regressão).

## Capturas (`docs/evidence/git-update-reconcile/`)

| Arquivo | O que prova |
|---|---|
| [s1-update-dialog-policy.png](git-update-reconcile/s1-update-dialog-policy.png) | Diálogo de Update com `Ahead 3`, `Behind 1` e a frase de política completa; `main ↑3 ↓1` na barra por trás. |
| [s2-update-completed-merge.png](git-update-reconcile/s2-update-completed-merge.png) | `Update completed.` e a barra em `main ↑4` sem `↓`: o merge foi feito. |
| [s3-history-merge-commit.png](git-update-reconcile/s3-history-merge-commit.png) | `Git log` com `930e328 Merge remote-tracking branch 'origin/main'` e o selo `2 parents`, acima do commit local e do commit da contraparte. |
| [s4-conflict-aborted.png](git-update-reconcile/s4-conflict-aborted.png) | Conflito abortado: mensagem do app nomeando `src/OrderDesk.Core/Order.cs`, diálogo aberto, `Ahead 3`/`Behind 1`, sem hint do git. |
| [s5-already-up-to-date.png](git-update-reconcile/s5-already-up-to-date.png) | `Update completed.` num workspace já atualizado (`main ↑4`, sem `↓`), aberto por `⌥⌘W` no bundle com o #111 mergeado: nenhum merge commit novo. |

## Não verificado

- **Windows e macOS Intel**: não executados.
- **Remoto autenticado/rede** (HTTPS/SSH, credential helper): o remoto da rodada é um bare local.
- **Escolha de política (merge/rebase)**: fora de escopo (decisão em `docs/decisions.md`); o rebase e o abort de rebase dependem da #114.
- **Update fast-forward e as configs `pull.ff=only`/`merge.ff=only` no app real**: provados só nos testes de host.
- **Merge em andamento de outro processo durante um Update real**: a posse do abort é provada por teste de host (`update_leaves_a_merge_it_did_not_start_alone`), não por uma corrida real entre dois processos no app.
- **Concluir ou abortar um merge pela UI** (#114): esta frente aborta o conflito automaticamente; não há resolução de conflito no app.
- **Contador `↓` atualizando sozinho** (#113): continua precisando de fetch manual.
