# Evidência — aceitação Git com remoto (issue #50)

Data: 2026-11-06. Alvo verificado: macOS 27.x arm64 (Apple Silicon). Windows e macOS Intel **não** foram executados.

Código sob teste: `5faaff7` (`feat(git): add full-file split and unified diffs with commit context (#103)`), versão `0.3.0-alpha.11`.
Bundle: `src-tauri/target/debug/bundle/macos/OpenRidr.app`, `CFBundleShortVersionString` `0.3.0-alpha.11`, construído nesta sessão a partir do próprio checkout (`bun install` + `bun run tauri build --debug --bundles app`).
Artefatos pinados usados pelo build (pins inalterados): `netcoredbg-osx-arm64-3.2.0-1092.zip` com SHA-256 `f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea`, runtime .NET `10.0.14` e `csharp-ls` `0.28.0` baixados do pin dos próprios scripts de vendor.

UI dirigida **somente** no app Tauri real via `agent-desktop` (sem navegador, sem Playwright de sessão), com `OPENRIDR_HOME` isolado e o `.sln` do clone em argv (`desktop.sh up` + `VERIFY_OPENRIDR_DESKTOP_SOLUTION`).

## Precondições (não são passe da UI)

- **Remoto local**: `scripts/git-remote-fixture.sh /tmp/openridr-git-remote-50` criou `origin.git` (bare), `user` e `peer`, com o conteúdo público de `fixtures/OrderDesk` na raiz dos clones. Baseline `0bfb695`. Sem rede, sem login, sem `git config --global` (identidade só `--local` em cada clone). Esses diretórios ficaram no disco; nada foi apagado.
- **Correção de infraestrutura depois da revisão**: o script usado na rodada aceitava um caminho arbitrário e recriava o diretório, o que podia apagar um caminho válido. A versão atual é **não destrutiva**: sem argumento cria um state novo com `mktemp`; um argumento só vale se for caminho absoluto novo dentro da raiz temporária real; caminho existente, `/`, home, checkout, relativo/`..` ou symlink para fora são recusados sem apagar. Provado por `sh tests/shell/git-remote-fixture.test.sh` (9 casos). O comportamento da rodada em si (comandos e estados abaixo) não mudou.
- **Contraparte**: `peer` foi dirigida por shell (commit, `pull --rebase`, `push`, branch), como qualquer colega faria.
- **Fetch manual no clone do usuário** (`git -C /tmp/openridr-git-remote-50/user fetch origin`), duas vezes, porque o app não tem ação de fetch. O `↓` que aparece depois do fetch prova a **renderização** do contador, não uma atualização feita pelo app (issue #113).
- **Estados de merge criados fora do app**: o pull divergente do app falha antes do merge (issue #112), então o estado de conflito foi produzido por `git pull --no-rebase --no-edit` no clone descartável, e a recuperação por `git merge --abort`. A UI só foi observada **no estado** resultante; ela não criou nem resolveu o merge.
- **Recuperação da divergência**: `git merge --no-edit origin/main` no clone do usuário entre a rejeição de push e o push seguinte. Sem `reset --hard`, `clean` ou `force` em nenhum momento.

## Quadro por cenário

| # | Cenário | Prova fora da UI (comando → resultado) | Status |
|---|---|---|---|
| S1 | Status inicial após trust | lido da árvore de acessibilidade no launch: `Current branch: main · upstream origin/main`, Commit tool `0 of 0 selected`, "The working tree is clean." | **evidência inicial não preservada**: a leitura foi feita, mas nenhuma captura do estado inicial foi guardada; a captura posterior (`s3-push-rejected.png`) mostra um estado limpo semelhante e **não** comprova o estado inicial |
| S2 | Commit → Push | UI: `Committed 78d6589 …` e `Pushed main to origin.`; `git -C origin.git log --oneline -2` → `78d6589` no topo; `git status -sb` → `## main...origin/main` (0 à frente) | **passou** |
| S3 | Push rejeitado (non-fast-forward) | UI: diálogo com `! [rejected] main -> main (fetch first)`; `git -C user log -1` → `c8777eb` preservado; `git -C origin.git log -1` → `8ee41ac` (inalterado) | **passou** |
| S4 | Pull fast-forward | UI: `↓1` após fetch, diálogo `Behind 1`, depois `Update completed.`; `git -C user log --oneline -1` → `619a6ab`; a linha `// Peer change for issue 50 (S4 pull)` existe no arquivo | **passou** |
| S5 | Pull divergente / conflito | UI: `Update…` → falha, diálogo aberto com o hint cru do git (`fatal: Need to specify how to reconcile divergent branches`), `ahead 1, behind 1`, nenhum merge commit criado. Estado de conflito produzido por CLI: `UU src/OrderDesk.Core/Order.cs`, depois a UI mostra `A merge is in progress…`, `0 of 1 selected`, checkbox do arquivo desabilitado, `Commit` desabilitado | **falhou na reconciliação** (bug #112) e **parcial na leitura do conflito** (o estado foi criado por CLI, não pela UI; sem abort na UI, #114) |
| S6 | Árvore suja | UI: `Update…` desabilitado com "The working tree has uncommitted changes; commit them first. OpenRidr does not auto-stash.", entradas de branch desabilitadas, `Push…` aberto mostrando `Commits ahead 1`; `git stash list` → 0 linhas; `git status --short` → a alteração intacta | **passou** |
| S7 | Branch nova e volta | UI: `New branch name` `accept-conflict` → `Current branch: accept-conflict · no upstream`; `rev-parse --abbrev-ref HEAD` → `accept-conflict`, `rev-parse HEAD` → `4d5ca4a`; volta pelo item `main` da lista → `rev-parse --abbrev-ref HEAD` → `main`, `rev-parse HEAD` → `4d5ca4a` | **passou** |
| S8 | Checkout de branch remota (tracking) | peer publica `peer-feature` (`6827350`); após fetch manual, a lista mostra `origin/peer-feature remote`; UI → `Current branch: peer-feature · upstream origin/peer-feature`; `branch --show-current` → `peer-feature`, `rev-parse HEAD` == `rev-parse origin/peer-feature` == `6827350…`, linha da contraparte presente no arquivo | **passou** (com fetch manual como precondição, #113) |
| S9 | Histórico | Lista da aba Git: `db8a0e9`, `4d5ca4a` (selo `2 parents`), `8ee41ac`, `c8777eb`, `619a6ab`, `78d6589`, `0bfb695` (selo `root`) — igual a `git log --oneline main` em ordem, hash e assunto; selecionar `8ee41ac` mostra `parents 619a6ab` e `src/OrderDesk.Core/Order.cs`, igual a `git show --name-status`; filtro `619a6ab` reduz a lista a um commit | **passou** |
| S10 | Branch sem upstream | UI: `Current branch: accept-conflict · no upstream` e `Update…` desabilitado com "No upstream is configured; nothing to pull." | **passou** |

S1 é o único cenário com prova não preservada; S5 está registrado como **falhou/parcial** acima (com o estado de merge criado fora do app). O que não foi executado está na seção **Não verificado**.

## Fetch e pull são coisas diferentes

- O `↓` do painel é a distância entre `HEAD` e a ref de tracking **local**. Medido no S4: antes do fetch manual a linha era `main · upstream origin/main` (0 atrás) mesmo com a contraparte já publicada; depois de `git fetch origin` no clone e de Refresh, `0 ahead, 1 behind`.
- O `Update…` roda `git pull`, que faz fetch. No S3, o diálogo mostrou `Behind 0` (contador defasado, recém-rejeitado) e o pull ainda assim incorporou o commit da contraparte — o `pull` funciona com o contador velho; o contador é que não se atualiza sozinho (#113).
- A mesma defasagem esconde branch remota nova: `origin/peer-feature` só apareceu na lista depois do fetch manual (S8).

## Limitações de superfície encontradas na sessão

- **Atalho que encerra o app (bug #111).** `⌥⌘W` é o atalho anunciado de `Update Project…` e também do item nativo **Close All**. Durante a rodada, pressionar `⌥⌘W` foi roteado para o item de menu e o app **saiu** (PID morto, `pgrep -x openridr` vazio). O app foi relançado uma vez, com o mesmo bundle e o mesmo workspace; a Trust decision teve de ser refeita porque o `desktop.sh up` rotaciona o `OPENRIDR_HOME`. Os cenários seguintes usaram a Find Action (`Update Project…`) para abrir o diálogo, nunca o atalho.
- **Summary de branch sem ação de acessibilidade (observação, sem ticket).** A árvore expõe o widget de branch como `disclosure` com `RightClick`/`ScrollTo` e **sem** `Click`/ref, enquanto os botões ao redor expõem `Click`. Por isso o menu de branches foi aberto pelas ações de Git (Find Action → `New Branch…`/`Push…`), não pelo widget. Não foi confirmado se teclado/VoiceOver abre o menu; fica como achado para triagem.

## Capturas (`docs/evidence/git-remote-acceptance/`)

| Arquivo | O que prova |
|---|---|
| [s2-push-dialog.png](git-remote-acceptance/s2-push-dialog.png) | Diálogo de Push: Remote `origin`, Branch `main`, Upstream `origin/main`, Commits ahead `1`, com o aviso "never `--force`, never tags". |
| [s2-pushed-branch-menu.png](git-remote-acceptance/s2-pushed-branch-menu.png) | `Pushed main to origin.` e a lista de branches com `main (current)`. |
| [s3-push-rejected.png](git-remote-acceptance/s3-push-rejected.png) | Rejeição non-fast-forward com o stderr do git; `Changes 0 files` e árvore limpa (o commit local está preservado). |
| [s3-update-behind-0.png](git-remote-acceptance/s3-update-behind-0.png) | Diálogo de Update com `Behind 0` e `↑1` na barra: contador defasado antes do pull. |
| [s3-merged-pushed.png](git-remote-acceptance/s3-merged-pushed.png) | `↑2` e `Pushed main to origin.` depois do merge feito fora do app. |
| [s4-behind-1.png](git-remote-acceptance/s4-behind-1.png) | Barra com `main` e `↓1` depois do fetch manual: o contador renderiza o atraso. |
| [s4-update-dialog.png](git-remote-acceptance/s4-update-dialog.png) | Diálogo de Update com `Upstream origin/main` e `Behind 1`. |
| [s4-pull-done.png](git-remote-acceptance/s4-pull-done.png) | `Update completed.` e a barra sem contadores: pull fast-forward incorporado. |
| [s5-divergent-pull-refused.png](git-remote-acceptance/s5-divergent-pull-refused.png) | Diálogo de Update com o hint cru do git sobre branches divergentes e `❯ Upstream origin/main`; `↑1 ↓1` na barra. |
| [s5-conflict-in-progress.png](git-remote-acceptance/s5-conflict-in-progress.png) | `Upstream origin/main · merge in progress`, branches e `Update…` desabilitados com o motivo, `Changes 1 file`, arquivo `Conflicted` e checkbox desabilitado. Estado criado por CLI (declarado acima). |
| [s6-dirty-tree-push-allowed.png](git-remote-acceptance/s6-dirty-tree-push-allowed.png) | Árvore suja: `Update…` desabilitado com o motivo "uncommitted changes … does not auto-stash" e `Push…` disponível. |
| [s7-branch-created.png](git-remote-acceptance/s7-branch-created.png) | `Current branch: accept-conflict · no upstream` e o formulário de criação de branch. |
| [s7-switched-back-main.png](git-remote-acceptance/s7-switched-back-main.png) | Lista de branches com `main` disponível para checkout depois da branch criada. |
| [s8-remote-branch-checkout.png](git-remote-acceptance/s8-remote-branch-checkout.png) | `Current branch: peer-feature · upstream origin/peer-feature` e a entrada `origin/peer-feature remote` na lista. |
| [s9-history.png](git-remote-acceptance/s9-history.png) | Aba Git log com os sete commits, selos `2 parents` e `root`, e o detalhe do commit selecionado com `parents` e o arquivo alterado. |
| [s9-history-filter.png](git-remote-acceptance/s9-history-filter.png) | Filtro por hash reduzindo a lista a `619a6ab`. |
| [s10-no-upstream.png](git-remote-acceptance/s10-no-upstream.png) | Branch `accept-conflict` sem upstream com `Update…` desabilitado e o motivo "No upstream is configured; nothing to pull.". |

## Bugs abertos nesta rodada

- #111 — atalho `Update Project…` (`⌥⌘W`) sombreado por **Close All**: pressionar o atalho encerra o app.
- #112 — `Update…` não reconcilia branch divergente e mostra o hint cru do git.
- #113 — sem ação de fetch: `↓` e a lista de branches remotas ficam defasados até um push/pull.
- #114 — merge em andamento não pode ser concluído nem abortado pelo app.

## Não verificado

- **Autenticação** (credential helper, SSH, token) e **transporte de rede** (HTTPS/SSH): o remoto da rodada é um bare local. Nenhuma prova de que um remoto autenticado funciona.
- **`push --force`, tags, submodules**: não exercitados (o app não os oferece).
- **Workspace sem remoto**: não houve segundo workspace; o estado "sem upstream" foi coberto por branch local.
- **Push com árvore suja efetivamente executado**: provado que a ação está disponível e o diálogo abre com `Commits ahead 1`; o push em si não foi concluído nesse estado (seria rejeitado por non-fast-forward e o cenário já tinha prova de rejeição).
- **Amend** e **diff de commit pelo histórico (#101)**: fora do escopo aprovado.
- **Fechar/reabrir sobrevivendo a estado de merge**: não testado.
- **Windows e macOS Intel**: sem build nem execução.

## Como reproduzir a infraestrutura

Numa máquina limpa, este é o caminho completo; o state e a solution vêm da execução **atual**, não de
um caminho fixo:

```sh
out=$(sh scripts/git-remote-fixture.sh)          # state novo via mktemp; nada existente é apagado
printf '%s\n' "$out"
state=$(printf '%s\n' "$out" | sed -n 's/^state //p')
solution=$(printf '%s\n' "$out" | sed -n 's/^solution //p')
[ -f "$solution" ] || { echo "solution missing: $solution" >&2; exit 1; }

VERIFY_OPENRIDR_DESKTOP_STATE="${TMPDIR:-/tmp}/openridr-git-remote-desktop" \
VERIFY_OPENRIDR_DESKTOP_SOLUTION="$solution" \
  sh .agents/skills/verify-openridr/scripts/desktop.sh up src-tauri/target/debug/bundle/macos/OpenRidr.app
# ... cenários do quadro acima ...
VERIFY_OPENRIDR_DESKTOP_STATE="${TMPDIR:-/tmp}/openridr-git-remote-desktop" \
  sh .agents/skills/verify-openridr/scripts/desktop.sh down
```

Recusa esperada (nenhum caminho existente é apagado ou reutilizado): passar `/`, `$HOME`, o próprio
checkout, caminho relativo, `..`, symlink para fora, ou um `TMPDIR` que resolva para dentro do
checkout. Um caminho explícito só é aceito se for absoluto, novo e estiver sob a raiz temporária real:

```sh
sh scripts/git-remote-fixture.sh "${TMPDIR:-/tmp}/meu-state-novo"
sh tests/shell/git-remote-fixture.test.sh   # prova das recusas, do escape por symlink e do state novo
```

**Dados históricos da rodada (não reprodução):** os diretórios `/tmp/openridr-git-remote-50`
(fixture da rodada) e `/tmp/openridr-issue50-desktop` (state do app) foram criados pela versão
anterior do script, que aceitava um caminho explícito e o recriava. Eles ficaram no disco e não são
necessários para repetir a aceitação: use o bloco acima.
