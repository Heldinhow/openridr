# Retro — Diff do Git: arquivo inteiro, unificado e hash do commit (#48)

Sessão filha do orquestrador, worktree `feat/issue-48-git-diff`. Escopo entregue: spec `docs/SPEC-GIT-DIFF.md`, host `git.fileDiff` (#98), painel split com conectores + hash (#99), modo unificado (#100), evidência `docs/evidence/git-diff.md`. Três rodadas de revisão independente (r1, r2, re-check r3). Follow-up #101 (diff a partir do Git log) nasceu fora do escopo, como issue independente.

## O que funcionou e deve continuar

- **Escrever a base do diff na spec antes do código.** A issue pedia "o hash do commit", mas o painel compara `Index → Worktree` no unstaged. Descobrir isso no grill e registrar (hash do commit é linha de contexto, não rótulo do lado esquerdo) evitou um rótulo falso que o review teria derrubado. A regra "quem é o lado esquerdo" agora está escrita na spec e na decisão.
- **Host dono dos dados, React dono da projeção pura.** `git.fileDiff` devolve lados completos, ranges e commit; o React só monta linhas (`buildCommitRows`, `projectUnified`). Isolou o que quebrou nas rodadas de revisão: os achados de symlink/captura/UTF-16/blob ficaram todos num arquivo (`crates/host/src/git.rs`) e provados sem UI.
- **Algoritmo de diff no Git, não em TypeScript.** Os ranges vêm de `git diff -U0`; para UTF-16, do próprio Git sobre o texto decodificado. Nenhuma dependência nova e um contrato que o review aceitou.
- **Falha-antes por reversão de ramo, sem stash.** Nas três rodadas a prova foi reverter só o trecho corrigido (guard de symlink, ramo staged do `diff_output`, validação de contexto) e rodar o mesmo teste. Barato, determinístico e não mexe no index.
- **App real pegou um bug invisível ao E2E.** A linha do commit não era pintada porque `overflow: hidden` zera o `min-height: auto` do flex item; a árvore de acessibilidade via o texto, o DOM chamava `toBeVisible()` falso. `agent-desktop` + screenshot foi o que mostrou a faixa vazia.
- **Revisões com achado pequeno e concreto.** As duas altas (symlink e captura sem limite) e a terceira (staged pulando confinamento) foram corrigidas com testes de matriz, e o re-check pediu exatamente a matriz de falha-antes/passa-depois — um formato que vale padronizar para frentes de host.

## Achados de ambiente (ordenados por custo)

1. **`bun run tauri dev` não abre em worktree nova.** O build falha em `resource path 'resources/csharp-ls' doesn't exist`; mesmo copiando os três vendors do app instalado para `src-tauri/resources/`, o app em dev não acha o par embutido (`resources_root()` resolve `<exe>/../Resources`) e o SetupGate termina em "Retry setup" com o adaptador desabilitado. O caminho que funcionou foi `bunx tauri build --debug --bundles app --config '{"build":{"beforeBuildCommand":"bun run build"}}'` (~1 min, reusa `target/debug`) e abrir o `.app` do bundle com `OPENRIDR_HOME` isolado.
   *Melhoria*: um `scripts/dev-app.sh` que monta o bundle de debug com os vendors já presentes (ou uma linha no `AGENTS.md`/skill de verificação com o comando e o motivo). É o mesmo achado de `RETRO-DEBUG-NO-STOP`, ainda sem script — segunda sessão pagando o custo.
2. **Instância única disputa entre sessões, e eu matei processo por nome.** `openridr` do bundle de debug e o instalado têm o mesmo `CFBundleIdentifier`. Parei a minha instância com `pkill -x openridr` — errado: o nome não distingue sessões, então isso pode matar o app de outra pessoa — e abri o bundle "à mão", sem a checagem de PID que o `desktop.sh` faz. Quando outra sessão abriu o app entre o meu kill e o meu launch, o meu processo saiu e a janela dela continuou (o meu path não vazou; o launch virou no-op).
   *Melhoria*: **nunca matar processo por nome**; parar só o PID que o próprio launcher gravou (como o `desktop.sh` faz) e repetir o `pgrep` imediatamente antes de abrir. Registrar em `test-app-agent-desktop` que o single-instance transforma um launch concorrente em "nada aconteceu", e que `pkill` por nome é a armadilha que mata a sessão vizinha.
3. **`cargo test -p openridr-host --lib` depende do PATH do dotnet.** Sem `~/.dotnet` no `PATH`/`DOTNET_ROOT`, `run_configs::tests::launch_profile_runs_fixture...` falha com "A compatible .NET SDK was not found" (8.0.408 ausente do SDK do sistema). A suíte inteira parece vermelha por ambiente.
   *Melhoria*: o `AGENTS.md` já manda exportar `PATH`/`DOTNET_ROOT` para E2E; incluir a mesma linha para `cargo test` (ou um `.cargo/config.toml` com o env) economiza um diagnóstico de 10 min por sessão.
4. **Delegar o review travou por modelo/transporte.** `delegate_task` respondeu `fetch failed` (2×) e `openai-codex/gpt-6.1-terra` não existia mais no provider Pi (só no Codex). O certo foi não trocar modelo sozinho e avisar o orquestrador; mas o custo foi um round-trip.
   *Melhoria*: checar `orchestrator_capabilities` **antes** de prometer um modelo no briefing, e tratar `fetch failed` como "provedor indisponível, peça instrução" no fluxo da sessão filha.
5. **`src-tauri/resources` é gitignored e pode ficar velho.** Copiar os vendors de `/Applications/OpenRidr.app` fez o bundle de debug rodar, mas a versão é a do app instalado, não a do checkout. Funcionou porque o setup gate só precisa do par; para uma feature que dependa da versão exata, isso vira prova enganosa.
   *Melhoria*: o script do item 1 deveria vendorizar a partir do próprio repo (`scripts/vendor-*.sh`) e imprimir as versões usadas junto do bundle.

## As três rodadas de revisão

| Rodada | Achados | Correção/prova | Limite que ficou |
|---|---|---|---|
| r1 (f062fb7) | 2 altos: symlink fora do repo; captura sem limite. 3 médios: contexto não validado; UTF-16 sem ranges; erro de blob virando ausência | confinamento do diretório real + symlink como objeto link; `spawn_bounded`/`cat-file -s`/teto no worktree; comparação par a par do contexto; ranges do Git sobre texto decodificado em temporários 0600; `rev-parse --verify --quiet` separa ausência de erro | TOCTOU não demonstrada nem fechada; SIGKILL pode deixar temporários; sem reabertura do app nessa rodada |
| r2 (3023349) | 1 alto: `staged=true` + caminho não rastreado pulava o confinamento e usava `--no-index` | staged sempre `HEAD → Index` (nunca toca a worktree; conserta deleção staged); `is_tracked` separa "sem entrada no index" de index corrompido; matriz `staged × API` com marcador secreto e falha-antes por reversão | idem; corrupção de árvore/index agora coberta; caminhos excepcionais de captura (travamento/erro) seguem sem prova |
| r3 | — | re-check fechou a alta; Git unit 27/27 | Intel/Windows sem prova; sem reabertura do app nas correções |

## O que ficou fora e por quê

- **Diff a partir do Git log** (arquivos de um commit): exige outra base (`parent:<path>`) e outro contrato; virou a issue #101, relacionada à #48.
- **Fallbacks no app real**: binário e arquivo grande têm prova de host (unit) **e** de E2E (`git.spec.ts`); o `unreadable` (blob/árvore/index corrompidos) tem prova **só de host** — não existe fixture de ponta a ponta e o ramo nunca foi exercitado por E2E ou app. Nenhum dos dois foi aberto na UI do app real (a correção não mudava a tela).
- **Evidência visual preservada é parcial**: só recortes de moldura do painel (toolbar, rótulos, linha do commit, nota e o gutter de conectores) foram publicados em `docs/evidence/git-diff/`; as linhas do arquivo inteiro, o corpo unificado e as telas de adicionado/deletado/binário ficaram sem captura publicável porque o app rodou sobre cópia de projeto privado. O recorte está declarado na evidência; nada foi recapturado depois.
- **TOCTOU residual do confinamento**: declarada como limite; não foi demonstrada nem fechada, e não se afirma que só um sandbox a resolveria.
- **Temporários do diff decodificado sob SIGKILL**: nome único e 0600, limpeza por `Drop` nos caminhos normais e de erro; um kill duro pode deixar dois arquivos, e isso está na evidência.
- **Windows e macOS Intel**: não executados, como no resto do repo.
