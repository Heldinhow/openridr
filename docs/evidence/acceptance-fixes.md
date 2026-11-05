# Correções da aceitação do `main` (`60e784f`) — D1…D6

Branch `fix/main-acceptance-findings`, criada de `origin/main` `60e784f` e mesclada com `origin/main` `731c3d8`
(PR #28, ícones de arquivo) antes da verificação final. Origem dos defeitos:
[main-60e784f-acceptance.md](https://github.com/Heldinhow/openridr/blob/main/docs/evidence/main-60e784f-acceptance.md)
(sessão de aceitação no app instalado, commit `32738dd` do worktree `test-main-60e784f-acceptance`).

Data: 2026-11-05. Máquina: macOS ARM64, `~/.dotnet` com 8.0.408 e 10.0.100.

## Resumo

| D | Defeito relatado | Causa | Correção | Prova |
|---|---|---|---|---|
| D1 | Export grava `run` vazio | `export_tests` lia `Tests::view()`, cujo `run` é sempre `idle_view()` | um único ponto de composição, `Inner::tests_view()`, serve snapshot e export | teste Rust (falha antes), arquivo real no host, E2E |
| D2 | Progresso sem total e com 102.956.650 bytes para um pacote de 21.643.814 | dois escritores de `progress`: o leitor medido (Content-Length) e uma observação de disco que somava staging + HOME do NuGet + tmp (o mesmo `.nupkg` três vezes) | o download medido é dono do número; a observação de disco só roda quando não há medição e deduplica raízes aninhadas | unit + roteiro no app (19→44→68→82→100 % de 21.643.814) |
| D3 | TFM "Not selected" em toda a aba Tests | a descoberta usava `evaluation.context.targetFramework`, nulo até alguém escolher um TFM | projeto de um TFM é descoberto no TFM avaliado; multi-TFM continua exigindo escolha explícita | testes Rust (falha antes), E2E, app |
| D4 | Menu de folha de `OrderDesk.Tests` com Run/Debug de `OrderDesk.Cli` | **não reproduz**: o menu já é do projeto dono (o screenshot 25 é da linha de `OrderDesk.Cli`) | o defeito real era a seleção não seguir a linha; abrir o menu agora seleciona o dono | app (árvore AX: `Run Unit Test`, sem Run/Debug), E2E |
| D5 | Barra "N selected for update" duplicada | dois `<div>` idênticos | um só | E2E (`toHaveCount(1)`), app |
| D6 | "Discovery details", painel de detalhe e barra de status fora da árvore AX | **não reproduz como relatado**: os textos estão na árvore AX do app instalado; o que faltava era nome/landmark (o `<footer>` dentro de `<main>` não é `contentinfo`; o painel de detalhe não tinha nome) | `role="contentinfo"` + `aria-label` na barra, região nomeada com `aria-live` no detalhe, `aria-label` nas seções recolhíveis | árvore AX do app instalado (antes) e do app compilado com a correção (depois), E2E |

## D1 — o export é a mesma vista do painel

Sessão no app instalado (`60e784f`), home isolado, depois de um run real (`2 passed · 1 failed · 1 skipped`):

```
$OPENRIDR_HOME/exports/tests-discovery-1-1.json
  run.revision 0 · run.runId null · counts { failed: 0, passed: 0, skipped: 0, total: 0 } · command []
```

O nome do arquivo era a identidade da **descoberta** porque `run.run_id` era nulo. Depois da correção, no host
real (mesmo fluxo, `tests.run` + `tests.export`):

```
$OPENRIDR_HOME/exports/tests-testrun-1-1.json   identity testrun:1:1   runRevision 1
  discovery items 4 · targetFramework net8.0
  run state completed · counts { total: 4, passed: 2, failed: 1, skipped: 1 } · results 4
```

`crates/host/src/lib.rs`: `export_tests` e o snapshot usam `Inner::tests_view()`; nenhum leitor do estado de
testes precisa lembrar de injetar o run.

## D2 — progresso medido, com total e percentual

Código: `crates/host/src/tools.rs` (`progress_measured`, `observes_disk_bytes`, `observed_bytes`).
O `Sync` de `downloading` somava `staging + toolhome + tmp`: o `dotnet tool install` grava o **mesmo** `.nupkg`
no feed verificado, em `NUGET_PACKAGES` e no cache do HOME isolado, então o total era ~4,8× o arquivo e
`total` ficava `None`.

Roteiro na UI real (host isolado, home vazio, download de verdade; captura
[`setup-download-with-total.png`](acceptance-fixes/setup-download-with-total.png)):

```
Downloaded 4284878  of 21643814 bytes (19%)
Downloaded 9578478  of 21643814 bytes (44%)
Downloaded 14811190 of 21643814 bytes (68%)
Downloaded 17901838 of 21643814 bytes (82%)
Downloaded 21643814 of 21643814 bytes (100%)
```

21.643.814 é o tamanho real do `.nupkg` (`tools/csharp-ls/0.28/.store/…/csharp-ls.0.28.0.nupkg`). Sem total
conhecido (o instalador de probe, só em debug) o texto continua "(measured; no percentage)" — é o caminho que
`tools-install-ux.spec.ts` exercita.

## D3 — TFM da descoberta

`resolve_test_scope` agora resolve `test_framework(&project, requested)`: seleção explícita quando existe,
senão o único TFM que o projeto declara; multi-TFM continua recusando sem escolha (`tfm_required`, teste que já
existia). Host real, sem `targetFramework` no pedido:

```
tests.discover · project=tests/OrderDesk.Tests/OrderDesk.Tests.csproj
  state completed · targetFramework net8.0 · items 4 · itemTfm net8.0
  command … dotnet test … --no-restore --list-tests --nologo -v:minimal -f net8.0
```

Na UI: `tests-tfm` já vem com `net8.0` selecionado, cada folha mostra "· net8.0", o detalhe mostra `TFM
net8.0` e a barra de status lê `Evaluated · Debug · TFM net8.0 · revision 5` (antes: "TFM not selected").

## D4 — o dono da linha (relido)

O relatório dizia que o menu de `xunit 2.9.3` (OrderDesk.Tests) abria Run/Debug de `OrderDesk.Cli`. No app
instalado, com `OrderDesk.Cli` selecionado antes, o clique direito em `xunit 2.9.2` de `OrderDesk.Tests` abre
`Project context menu` com: Manage NuGet Packages, Reload Project, Build Selected Projects, **Run Unit Test**,
Edit, New File…, Copy Path, Show in Finder, Properties… — sem Run/Debug, que é o certo para um projeto de
teste. As capturas `24`/`25` do relatório mostram a linha `Microsoft.NETCore.App (net8.0)` **de
OrderDesk.Cli**, onde Run 'OrderDesk.Cli' está correto.

O que existia de verdade: abrir o menu de uma folha não movia a seleção, então o inspetor de projeto continuava
em `OrderDesk.Cli` enquanto o menu agia em `OrderDesk.Tests`. `SolutionTree` agora seleciona o dono antes de
abrir o menu, como a linha de projeto já fazia. E2E: menu da folha sem Run/Debug, menu do framework de
`OrderDesk.Cli` com `Run 'OrderDesk.Cli'` e o inspetor seguindo a linha.

## D5 — barra única

`NuGetPanel` renderizava o mesmo `<div class="build-controls nuget-batch-bar">` duas vezes. App instalado
(antes): duas ocorrências de `0 selected for update` em y=819 e y=846. Depois: uma.

## D6 — árvore de acessibilidade (relido)

No app **instalado** (`60e784f`), com o painel Tests aberto, `agent-desktop find` devolve:

```
statictext 'Analysis runtime 10.0.14 · Project SDK 8.0.408'
statictext 'Evaluated · Debug · TFM not selected · revision 2'
statictext 'Analysis: Partial coverage'   statictext 'UTF-8'   statictext '4 spaces'
disclosure 'Discovery details'
heading 'OrderDesk.Tests' + statictext 'Project: …' + statictext '4 tests · 0 passed …'
combobox 'Trusted mode'
```

Ou seja: os textos **estavam** na árvore (o relatório leu snapshots `-i`, que só mostram nós acionáveis). O que
faltava era estrutura: a barra de status é um `<footer>` dentro de `<main>`, logo sem papel `contentinfo`,
com os filhos soltos sob `main>group`; o painel de detalhe era um `<aside>` sem nome; as seções recolhíveis não
tinham nome próprio (o nome vindo do `<summary>` não chega ao grupo). Correção em `shell/StatusBar.tsx`,
`TestsPanel.tsx` e `BuildPanel.tsx`. Verificação web (Chromium, E2E): `contentinfo "Status bar"` com "Trusted
mode" dentro, `complementary > region "Test details"` e `group "Discovery details"`.

## Testes

| Tipo | Comando | Resultado |
|---|---|---|
| Rust (workspace host) | `cargo test -p openridr-host` | 391 passed, 0 failed (lib 263, host 97, debugger 14, discovery 10, run 6, trust 1) |
| Unit TS | `bun test tests/unit` | 27 passed |
| Check | `bun run check` | ok (pre-commit, a cada commit) |
| E2E (portas 1500/4397, home próprio) | `bunx playwright test acceptance-fixes solution-explorer solution-file-icons nuget` | 24 passed |
| E2E | `bunx playwright test tools-install-ux setup-gate-layout ui-layout evaluation` | 18 passed |
| E2E | `bunx playwright test tests testsuite tests-auto-discovery` | 11 passed, 1 falha **pré-existente** |

`testsuite.spec.ts:51` procura `tests-filter-count`, um `data-testid` que não existe em `src/` nem em
`60e784f` (verificado no worktree da aceitação): falha do HEAD, não desta branch.

## Verificação no app Tauri real (agent-desktop, WKWebView)

Bundle **de debug desta branch** (`bun run tauri build --debug --bundles app`), `OPENRIDR_HOME` isolado e cópia
da fixture em argv (`scripts/desktop.sh up src-tauri/target/debug/bundle/macos/OpenRidr.app`), uma instância,
fechada ao fim (`desktop.sh down` → `stopped`, `pgrep` vazio).

| Item | Resultado | Prova |
|---|---|---|
| D1 | aprovado | `Run all tests` → `Run completed · 4 results · exit 1`; `Export` gravou `~/exports/tests-testrun-1-1.json` (10046 bytes) com `run.revision 1`, `runId testrun:1:1`, `counts {total 4, passed 2, failed 1, skipped 1}`, 4 resultados e a descoberta de 4 itens com `targetFramework net8.0` — captura `tests-panel-tfm-and-export.png` |
| D2 | aprovado (UI real, host isolado) | download de verdade: 19 % → 44 % → 68 % → 82 % → 100 % de 21.643.814 bytes — captura `setup-download-with-total.png` |
| D3 | aprovado | painel: `Test project OrderDesk.Tests`, `TFM net8.0`, `Discovered 4 tests`; detalhe `TFM net8.0`; barra `Evaluated · Debug · TFM net8.0 · revision 2`; cada folha "… · net8.0" — mesma captura |
| D4 | aprovado | clique direito em `xunit 2.9.2` (de OrderDesk.Tests) com `OrderDesk.Cli` selecionado antes: menu `Project context menu` sem Run/Debug e com `Run Unit Test`; depois do menu, o inspetor passou a `tests/OrderDesk.Tests/OrderDesk.Tests.csproj` — captura `menu-owner-and-inspector.png` |
| D5 | aprovado | `agent-desktop find "selected for update"` → **1** ocorrência (antes: 2, y=819 e y=846) |
| D6 | aprovado | árvore AX: `main > contentinfo:Status bar` contendo `Trusted mode`, `Analysis: Partial coverage` e `Evaluated · Debug · TFM net8.0`; `group "Discovery details"` nomeado; `group "Test details"` presente no painel Tests |

## Não verificado (com motivo)

| Item | Motivo |
|---|---|
| Windows, macOS Intel, DMG notarizado | não executados nesta máquina |
| WKWebView lendo `aria-live` do painel de detalhe | a região aparece nomeada na árvore; a *anúncio* do leitor de tela não foi observado (nenhum leitor instalado) |
| Reinstall de ferramentas pela UI | baixaria os 21,6 MB de novo; o caminho de download foi exercitado com home vazio |
| Percentual com servidor sem `Content-Length` | não exercitado; cai no texto "(measured; no percentage)" |

## Revisão 2 (GPT-6.1 Terra) — quatro achados

Revisão bloqueante sobre o commit `0a75249`. Cada achado com o "falha antes" medido.

### R1 (alta) — a avaliação atual manda no seletor de TFM

O seletor lia `discovery.targetFramework` antes da avaliação: descobrir em `net8.0` e reavaliar em `net9.0`
deixava o seletor em `net8.0` enquanto o host já rodava no TFM novo. A decisão saiu do componente para
`src/testScope.ts` (`testPanelFramework`): contexto avaliado primeiro, e só então o **único** TFM do próprio
projeto selecionado. E2E `D3b` (projeto de dois TFMs): descobre em `net8.0`, troca para `netstandard2.0` —
antes `Received: "net8.0"`, depois `netstandard2.0`. Unit `tests/unit/testScope.test.ts` fixa as quatro regras.

### R3 (média) — export observa o processo antes de gravar

`export_tests` lia o estado sem `sync_test_run()`: um run que já saiu depois do último snapshot ia para o JSON
como `running`. Agora sincroniza antes de compor a vista. Teste
`export_observes_a_finished_run_without_an_intervening_snapshot`: espera o run ficar `running`, deixa o
processo terminar **sem** nenhum snapshot e exporta — antes o arquivo dizia `running`, depois `completed` com
`counts.total 4`, 4 resultados e `exit 1`.

### R2 (média) — download medido é dono do progresso mesmo sem `Content-Length`

Só `total.is_some()` marcava a medição como dona; sem o cabeçalho o instalador voltava a somar as cópias do
`.nupkg` em disco. Agora qualquer byte lido do socket é medição (`progress_measured = true`), com `total`
nulo quando o servidor não informa tamanho. Para exercitar o fluxo real do instalador sem NuGet.org, builds de
debug aceitam `OPENRIDR_TOOLS_PACKAGE_URL` + `OPENRIDR_TOOLS_PACKAGE_SHA512` (mesmo formato de seam do
`OPENRIDR_TOOLS_MUXER`; release continua no pin compilado) e o download verificado roda mesmo com o instalador
de probe. Teste de fronteira `a_download_without_content_length_keeps_the_measured_bytes`: servidor TCP local
sem `Content-Length`, instalador falso que grava 400 KiB de cópias antes de falhar. Antes: `downloaded`
409.600 (as cópias); depois: 102.400 (o pacote), `total: null`.

### R4 (baixa) — fallback só do TFM do próprio projeto

O fallback usava `evaluation.availableTfms` (o TFM da solução inteira) mesmo quando o projeto selecionado não
declara TFM nenhum. `testPanelFramework` recebe `projectTfms` do projeto selecionado; projeto sem TFM devolve
`null` (o seletor mostra "Not selected", honestamente). A expressão antiga reprovava as duas regras:

```
$ bun run /tmp/old-rule.test.mts
D1 expected net9.0, got: net8.0 FAIL
D4 expected null, got: net8.0 FAIL
```

## Testes (revisão 2)

| Tipo | Comando | Resultado |
|---|---|---|
| Rust host | `cargo test -p openridr-host` | 393 passed (lib 263, host 98, debugger 14, discovery 10, run 7, trust 1) |
| Unit TS | `bun test tests/unit` | 33 passed |
| E2E | `acceptance-fixes` (5 specs, inclui D3b) | 5 passed |
| E2E | `tests testsuite tests-auto-discovery nuget` | 15 passed, 1 falha pré-existente (`tests-filter-count`) |
| E2E | `solution-explorer solution-file-icons ui-layout evaluation tools-install-ux setup-gate-layout` | 34 passed |
