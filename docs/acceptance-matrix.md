# Matriz de aceite por feature — OpenRidr (MVP Rider + Fase 1)

Data: 2026-10-30. Branch `fase-1-fluxo-sem-atrito`, as execuções da seção 2 rodaram em commits diferentes, indicados por linha (`0587ee3` → `6c323ed` → `9bf0bec` → `96a0de7`…`1bc3d02`); a matriz foi revisada no commit `docs(acceptance)` final desta rodada. Os logs completos ficam em `/tmp/t01` e **não são versionados**; versionados ficam o resumo `docs/evidence/logs/matrix-2026-10-30-summary.md`, este resumo (comandos, exits e contagens) e os screenshots em `docs/evidence/`.
Alvo verificado: macOS ARM64 (M2), Chrome real + host real (`bun run test:e2e`), SDK do projeto 8.0.408 em `~/.dotnet`, host 10.0.100, netcoredbg e csharp-ls reais instalados no home do E2E.
**Windows e macOS Intel estão fora do escopo desta matriz: não verificado, sem promessa.** O histórico P0 de 28/10 (AC-01…AC-34, E01…E17, A18, L18) está preservado no **Apêndice A**, sem alteração de conteúdo.

**Decisão: release NÃO aprovado.** Esta matriz não transforma ausência de prova em aprovação. O smoke nativo do app empacotado foi executado em 2026-10-31 (seção 3b); G-LEGAL e G-PKG seguem abertos (seção 5).

## 1. Como ler

| Coluna | Significado |
|---|---|
| Esperado | Cláusula da SPEC/MVP/ticket da Fase 1 que define o comportamento. |
| Prova automatizada | `arquivo.spec.ts:linha` (teste nomeado) ou teste Rust. Só conta o que rodou nesta data (seção 2). |
| Prova NATIVA | Janela Tauri real do bundle. Observada só onde a seção 3b registra (smoke nativo de 2026-10-31, PID 55568, bundle `c7439ad`, numa cópia da fixture em `/tmp`); todo o resto é **não verificado**. |
| Defeitos/limites | Severidade A/M/B (alta/média/baixa). IDs `A1…`, `M1…`, `L1…` = `docs/UI-AUDIT.md` (28/10, **não reauditados** depois da Fase 1, salvo os fechados pelos tickets 03–13). IDs `D-xx` = achados novos desta matriz (seção 4). |
| Status | **verificado** = prova automatizada real passou nesta data, no ambiente web (Chrome + host real); a prova nativa continua em coluna própria. **parcial** = parte da prova falhou/timeout/falta ou há defeito aberto relevante. **não verificado** = sem prova executada. **bloqueado** = depende de gate/decisão aberta. |

Skip e timeout nunca contam como passe. Na suíte completa em `0efaf08` houve **1 skip**: `install-plan.spec.ts:56` (primeira instalação; o home do E2E já tem o csharp-ls). `find-usages.spec.ts` e `ui-fold.spec.ts:49` são skips condicionais, mas **não pularam** na suíte final (ver `docs/evidence/logs/matrix-2026-10-30-summary.md`); nas execuções por spec anteriores houve 0 skips. `verificado*` = só provas automatizadas; desvios M/L do UI-AUDIT abertos não foram reauditados.

## 2. Execuções reais desta matriz (2026-10-30, macOS ARM64)

**Suíte completa em `0efaf08`: 135 passed / 0 failed / 1 skipped (`bun run test:e2e`), `cargo test --workspace --no-fail-fast` exit 0, `bun run check` exit 0** (por spec no resumo versionado). As linhas abaixo são as execuções por spec desta rodada.

Comando base: `PATH=$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet DOTNET_CLI_UI_LANGUAGE=en bunx playwright test tests/e2e/<spec>.ts`, um spec por vez, portas 1420/4317 livres antes. Logs fora do repositório (`/tmp/t01`); números abaixo são os registrados.

| Spec(s) | Resultado | Observação |
|---|---|---|
| restore, evaluation, build, run, run-configs, workspace, debugger (juntos, em `0587ee3`) | **46 passed**, exit 0, 0 skips | debugger: 12 testes reais com netcoredbg, nenhum skip |
| gates, restore, run, install-plan, ui-layout (juntos, em `9bf0bec`) | **21 passed**, exit 0 | |
| csharp-ls (10 testes) — **histórico**, em `6c323ed` | 8 passed, 2 timeout (3 min cada) | Superado: após `7eb1cb2` o spec inteiro passou **10/10** (exit 0, 2026-10-30; ver linhas 8–9 e `docs/evidence/logs/matrix-2026-10-30-summary.md`) |
| git | 5 passed | specs abaixo (git … zz-acceptance) rodaram um por vez em `9bf0bec` |
| nuget | 4 passed | |
| settings | 5 passed | |
| actions | 6 passed | |
| search | 6 passed | |
| tests | 6 passed | |
| testsuite | 2 passed | |
| trust-sdk | 3 passed | |
| shell | 4 passed | |
| code | 3 passed | |
| editor-overflow | 2 passed | |
| disabled-states | 3 passed | |
| ui-fold | 4 passed | |
| ui-layout | 7 passed | |
| install-plan | 3 passed | o teste "first-install" usa home E2E novo; faz skip se o csharp-ls já está instalado no home usado |
| environment | 1 passed | |
| terminal | **4 passed com `TERM=xterm-256color`; 2 failed + 2 passed com `TERM=dumb`** | ver D-02 |
| zz-acceptance | **1 passed (UI audit), 1 failed (jornada P0)** | ver D-03 |
| `cargo test --workspace --no-fail-fast` (em `9bf0bec`; log `/tmp/t01/m/cargo2.log`, não versionado) | exit 101: lib 173 ok, debugger 9 ok, test_discovery 9 ok, test_run 5 ok, **host.rs 47 ok / 13 falhas** | ver D-01; após `96a0de7`, `cargo test -p openridr-host --test host`: 59 ok / 1 falha, exit 101 |
| `bun run check` | exit 0 | |

Contagens antigas (49/101/102 testes, 192 testes Rust, "E2E 49") são **histórico** (Apêndice A e `docs/implementation-notes.md`) e não valem como estado atual.

## 3. Matriz por feature

Legenda de tickets: `F1…F9` = `docs/tickets-mvp/`; `Fase1 NN` = `.scratch/fase-1-fluxo-sem-atrito/issues/NN`; `Mapa NN` = `.scratch/roadmap/issues/NN`. Prova NATIVA = **não verificado** em todas as linhas, salvo onde indicado.

| # | Feature / fluxo | Esperado | Prova automatizada | Defeitos / limites | Ticket corretivo | Status |
|---|---|---|---|---|---|---|
| 1 | Welcome, recentes, abrir solução, ambiguidade | SPEC P0 AC-08, AC-11; F1 | `workspace.spec.ts:35` welcome lists recent…; `:460` ambiguous and empty directories ask…; `:93` opens the fixture in restricted mode | Nativo: só a tela Welcome do bundle abriu (screenshot 14). Nome do recente sem `.sln` por desenho | Fase1 14 (smoke nativo) | verificado |
| 2 | Trust, Restricted, revogação, processos remanescentes | AC-01, AC-07, G1; Fase1 stories 8–9 | `trust-sdk.spec.ts:71` trust resolves…revoke blocks…; `:230` revoked workspace shows the remaining process…; `workspace.spec.ts:507` direct host calls fail before trust; `gates.spec.ts:275` Build and Run name Restricted mode; `gates.spec.ts:316` D-13 grant/geometry/keyboard; `gates.spec.ts:395` D-13 error/retry; `gates.spec.ts:435`/`:468`/`:529` D-13 ownership/retention; `disabled-states.spec.ts:43` | — | — | verificado |
| 3 | SDK do projeto vs host, `global.json`, toolchain | AC-02; AGENTS.md (qualquer raiz detectada vale) | `trust-sdk.spec.ts:175` missing project SDK is distinct and global.json is never rewritten; `environment.spec.ts:4`; Rust `host.rs` 60/60 após `e9d3804` (`cargo test -p openridr-host --test host`, 2026-10-30) | D-01 corrigido em `96a0de7` (asserts independentes da máquina). A premissa de `initialize_failure…` caducou com csharp-ls 0.28 (ele inicializa mesmo sem o SDK 8.0.408 no host root); o teste foi reescrito em `e9d3804` com um launcher stub que rejeita `initialize`, e o contrato (mensagem do SDK exposta + processo encerrado) segue provado sem o csharp-ls real | Roadmap 14 | verificado |
| 4 | Solution Explorer: árvore avaliada, TFM, detalhes do projeto, falha de avaliação | AC-08–AC-10; Fase1 story 2 | `evaluation.spec.ts:63` trust evaluates every project…; `:149` a broken import stays in the tree…; `:183` saving a project file…re-evaluates; `ui-layout.spec.ts:110` evaluated project details keep paths intact | **Operações de arquivo (criar/renomear/mover/apagar na árvore) estão FORA DA BASE**: a trilha existe só na raiz `debug-parity` e não foi incorporada a esta branch. M10 (path quebra no meio) coberto por `ui-layout.spec.ts:110` | — (fora da base); M10 sem dono | verificado |
| 5 | Editor: abrir, editar, salvar, conflito de disco, fechar com buffer sujo | AC-11 | `workspace.spec.ts:137`, `:161`, `:189`, `:227`, `:265`, `:285`, `:312`, `:335`, `:392`, `:446`; `editor-overflow.spec.ts:5`, `:43` | M13 (editor em 1100 vira ícones sem rótulo), L9 (breadcrumb mostra contadores internos), M12 (cor do botão destrutivo): não reauditados | M12/M13/L9 sem dono | verificado* |
| 6 | Menu de contexto e ações do editor | F7 | `code.spec.ts:27` editor context menu lists F7 actions…; `:69` Alt+Enter…; `:81` trusted but unanalyzed editor explains Find Usages and Go To Definition gates | M6 (menu abre longe do ponteiro) não reauditado | M6 sem dono | verificado* |
| 7 | csharp-ls: instalação, sessão, Problems, completion, definition | AC-12, AC-14, AC-16; Mapa 08 | `csharp-ls.spec.ts:134`, `:171`, `:292` problems navigate real diagnostics…, `:556` analysis loads…survives a crash…, `:676` completion and definition… (todos passed) | Instalação real roda no home temporário do E2E. Os 2 timeouts do csharp-ls estão nas linhas 8–9 (D-04). Rust: D-01 corrigido em `96a0de7`; `initialize_failure…` reescrito com stub em `e9d3804`. Gates fora da prova | D-01, D-04 | verificado* |
| 8 | Rename (prévia, cancelar inerte, aplicar a buffers) | AC-12; F7 | `csharp-ls.spec.ts:1177` rename previews real cross-project edits…, `:1311` a stale rename preview is rejected… e `:1788` D-12 (click/Esc com foco real); spec focado 16/16 (exit 0, 2026-10-31; teste de instalação real excluído) | M7a (preview docado, ticket 31) resolvido; **M7b (campo inline sobre a palavra com range) aberto**; A6 fechado pelo Fase1 11; D-12 cancela por click/Esc | M7b sem dono | verificado* |
| 9 | Code actions (listar, prévia, invalidar antes de escrever) | AC-13 | `csharp-ls.spec.ts:1434`, `:1593` code action lists and previews are invalidated…, `:1702` code actions forward the current diagnostic… e `:1788` D-12 — todos passed; spec focado 16/16 (exit 0, 2026-10-31) | M7a resolvido: painel docado não cobre o editor (ticket 31; editor ≥100 px a 1100×700); D-12 click/Esc mantidos | — | verificado* |
| 10 | Find Usages (painel, navegação) | F7 | `find-usages.spec.ts:18` Find Usages lists and opens real csharp-ls references (passed 2026-10-30, `1bc3d02`: 4 referências, Program.cs ×1 e OrderTests.cs ×3, abre a ocorrência); gate sem análise: `code.spec.ts:81` | Spec reutiliza o csharp-ls já instalado (`tools.install` só reutiliza o launcher, mas **muta o home do E2E**; skip com motivo se ausente). **macOS-only** (usa `Meta+ArrowUp`) e os totais (4 referências; Program.cs ×1, OrderTests.cs ×3) dependem da fixture OrderDesk. `data-testid` `find-usage-N` reinicia em 0 por arquivo (ids duplicados; contrato mantido) | — | verificado (skip condicional: sem csharp-ls no home E2E) |
| 11 | Restore automático após Trust (contrato novo) | Fase1 02: restore + avaliação sozinhos após Trust, progresso, cancelar, retry, nova avaliação ao mudar `.csproj`; Restricted nunca restaura; build/run/debug/testes **nunca** restauram por si (`restore::require_restored`, decisions 2026-10-28 ticket 04) | `restore.spec.ts:58` trust restores and evaluates by itself…; `:106` restricted mode never restores by itself; `:123` a failed restore keeps its log…retry; `:191` a project file changed on disk re-evaluates…; `:211` direct restore denied before trust; `build.spec.ts:152` build waits for the automatic restore…; `zy-restore-interrupt.spec.ts:187` Cancel restore during the automatic restore…; `:207` Revoke trust during the automatic restore… (restore atrasado de forma determinística por `Directory.Build.targets` na cópia; 5/5 em execução serial 2026-10-31). O delay grava PID + start time normalizado + token único em um diretório da cópia e executa um symlink `sleep-<token>`; as asserções e o cleanup PID-only exigem PID + start time + token em argv, sem `pgrep`/`pkill` e sem sinal de grupo não comprovado. Testes próprios provam cleanup positivo com delay próprio e sentinela simultânea, registro malformado/token errado ignorado e registro vazio com sentinela intocada. Limite residual: start time em segundos + PID não é prova absoluta contra reuso; o token único no argv reduz o falso positivo. Falha de rede real do restore: **não coberto** | O contrato **mudou**: não é mais "restore explícito" (AC-03 do Apêndice A descreve o contrato antigo). Gate "Dependencies are not restored" sem assert de texto. **Esclarecimento:** o clique de toolbar em Run/Debug (um clique) é o preparo explícito (restore → avaliação → build, Fase1 05; prova: `run.spec.ts:318` one click on Run restores, builds and shows the program output); atalhos, gates estritos, build, testes e `run.start`/`debug.start` diretos **nunca** restauram por si (`restore::require_restored`) | Fase1 01 (parcial) | verificado |
| 12 | Build (log cru, erro navegável, buffers sujos) | AC-17–AC-20 | `build.spec.ts:66` a real build reports its raw log…navigates to it; `:152`; `gates.spec.ts:30` Run is gated until built (inclui build obsoleto) | Gates: texto de todos afirmado em `execution-gates.spec.ts` (funções puras; D-07 fechado) | Fase1 01 / Mapa 07 | verificado* |
| 13 | Run e janela Run (saída, exit, parar, um clique) | Fase1 03, 05; AC-17–AC-20 | `run.spec.ts:90` a real run shows the saved build output…; `:241` stopping a real long-running program…; `:300` one click on Run on a prepared workspace…; `:318` one click on Run restores, builds and shows the output | A1 fechado (Fase1 03). O "um clique" é o preparo explícito de toolbar (ver linha 11); o atalho/gate estrito de Run não restaura nem builda | — | verificado |
| 14 | Debug (breakpoint, step, frames, locals, condicionais, exceções, um clique) | AC-21–AC-23; Fase1 05 | `debugger.spec.ts:55`, `:193`, `:262`, `:512`, `:623`, `:676`, `:732`, `:756`, `:777`, `:796`, `:823`, `:855` (12 passed, 0 skips) | macOS ARM64: netcoredbg é "community supported" e a flag experimental continua por desenho; Windows/Intel fora de escopo. Nativo: não verificado | Fase1 14 | verificado |
| 15 | Terminal xterm (prompt automático, Ctrl+C/D, tela cheia, resize, cores) | Fase1 06–07 (stories 17–23) | `terminal.spec.ts:34` restricted…starts no shell; `:74` a real shell runs…Ctrl+C…; `:223` Ctrl+D…; `:245` a full-screen program…q returns | **Limite conhecido:** o `dotnet` do shell de login do usuário pode não ser o SDK do projeto (nesta máquina o login shell põe `/usr/local/share/dotnet`, sem 8.0.408, na frente; `dotnet --version` no terminal dá "sdk-not-found" dentro de um projeto com pin 8.0.408) — **sem asserção** após `b74b63d` (a jornada usa `echo`); a resolução do SDK é provada por restore/build/run/testes, não pelo terminal. D-02 corrigido em `8e45eda` (PTY troca TERM ausente/vazio/`dumb` por `xterm-256color`): `terminal.spec.ts` 4/4 com `TERM=dumb` (antes 2 falhas) + teste Rust da regra | — | verificado |
| 16 | Testes (descoberta VSTest, run, TRX, cancelar, MTP, árvore agrupada) | AC-27–AC-31; F5 | `tests.spec.ts:92`, `:190`, `:229`, `:263`, `:413`, `:512`; `testsuite.spec.ts:54`, `:188` | A2 fechado (Fase1 04) | — | verificado |
| 17 | Git (commit, diff, log, branch, checkout/pull/push) | F6 | `git.spec.ts:94` commit tool commits only the checked files…; `:162` Git review keeps long diffs…readable (diff/log); `:216` log lists commits…checkout and pull explain refusals; push needs a remote…; `:286` restricted mode and a non-repository…; (`:77` só prova layout do nome longo de branch) | A3 fechado (Fase1 08); M8/L3/L5 não reauditados. `git.rs` com dívida (Mapa 07) | Mapa 07 | verificado* |
| 18 | NuGet (instalados/desatualizados, sem pacotes, CPM) | F9 | `nuget.spec.ts:38`, `:46`, `:75`, `:88` | A8 fechado (Fase1 13); M9 (tabela abaixo da dobra) não reauditado | M9 sem dono | verificado* |
| 19 | Settings, Keymap, Find Action | F2, F8 | `settings.spec.ts:62`, `:131`, `:181`, `:232`, `:271`; `actions.spec.ts:36`, `:91`, `:117`, `:165`, `:197`, `:232` | M4 (keymap não é tabela), M11 (caminho absoluto de settings.json), L8 (tema com uma opção): não reauditados | M4/M11/L8 sem dono | verificado* |
| 20 | Search Everywhere (Files, Classes, Symbols, Text, Actions) | F3 | `search.spec.ts:19`, `:51`, `:104`, `:134`, `:162`, `:216` | L4 (sem contagem de resultados) não reauditado | L4 sem dono | verificado* |
| 21 | Run configurations | F4 | `run-configs.spec.ts:116`, `:142`, `:174`, `:226`; `disabled-states.spec.ts:62` | M3 (rodapé Save/Duplicate/Remove/Cancel) parcialmente tratado pelos testes de dialog Run (`:116`–`:174`) | M3 sem dono | verificado* |
| 22 | Casca/layout 1440×900 e 1100×700, tool windows, barra superior, primeira dobra | F1; Fase1 04, 12 | `shell.spec.ts:18`, `:34`, `:71`, `:103`; `ui-layout.spec.ts:17` (×2 larguras), `:110`, `:150` (×2); `ui-fold.spec.ts` (4); `editor-overflow.spec.ts` | A4/A7 fechados (Fase1 09, 12); M1, M2, M5, L1–L2, L6–L7 não reauditados | M/L sem dono | verificado* |
| 23 | Plano de instalação de ferramentas (csharp-ls/netcoredbg) | Fase1 10 (A5) | `install-plan.spec.ts:9` (1440×900 e 1100×700), `:56` trusted first-install shows an active Confirm and install; `csharp-ls.spec.ts:134`, `:171` | Em 1100×700 o plano não cabe sem rolar (por desenho; drawer rola). A prova de "Confirm and install" ativo vem de execução anterior em home E2E novo (`9bf0bec`, 3/3 passed, screenshot `docs/evidence/screenshots/10-install-confirm-active-1440x900.png`); na suíte completa de `0efaf08` esse teste **pulou** (home já tem csharp-ls) e o skip **não conta como prova** | — | verificado |
| 24 | Estados desabilitados com razão visível | Fase1 09 (A4) | `disabled-states.spec.ts:43`, `:53`, `:62`; `gates.spec.ts` | — | — | verificado |
| 24a | Problems (painel, severidade, origem, navegação; Problems do Build separados do LSP) | AC-14, AC-15; F1 | `csharp-ls.spec.ts:292` problems navigate real diagnostics by revision and keep stale items; `build.spec.ts:66` (erro de build navegável, origem Build) | M2 (coluna Message trunca, linha selecionada sem realce) não reauditado | M2 sem dono | verificado* |
| 24b | Gutter Run/Test e Alt+Enter (ações de contexto) | F7 | `zy-gutter.spec.ts:16` the test gutter runs one test from the editor and reports its result (passed após `0f00ae7`: clique real no gutter roda 1 teste e mostra "1 result · 1 passed"; screenshot `d09-gutter-test.png`); `code.spec.ts:69` Alt+Enter stays the context action popup… (só o gate). Gutter **Run** (Main): **não verificado** — OrderDesk.Cli usa top-level statements, sem símbolo `Main` (o matching já aceita `Main()`, sem prova) | Limites do matching (`codeNavigation.ts`): o gutter **Run** casa a ÚNICA `Main` do arquivo do startup project (não só `Program`; duas `Main` desligam o ícone); sobrecargas dividem a mesma identidade VSTest (FQN não distingue) e o gutter marca a primeira; `record`/`struct` não casam (falso negativo seguro); o dono do método, sem `containerName`, é a menor classe (kind 5) que o contém (`gutter-matching.spec.ts`, 5 testes sem host, `4b9a454`/`8562d95`); o `first()` do `zy-gutter.spec.ts` prova que há gutter e que 1 teste passa, não qual teste. D-11 corrigido em `0f00ae7`. Spec macOS-only e depende do csharp-ls já instalado (skip com motivo) | — | verificado* (o gutter **Run** não verificado) |
| 24c | Debug de teste (Debug selected) desabilitado por decisão | `docs/decisions.md` (2026-10-28: debug de teste é P1; `debug.start` lança o assembly, não um teste) | `testsuite.spec.ts:188` the quick list dims unavailable actions with their reason | Limite de produto, não defeito | — | verificado |
| 24d | Terminal automático com prompt (decisão xterm.js; Fase1 07 separado do 06) | `docs/decisions.md` 2026-10-30 (xterm.js aprovado); Fase1 06 = host em bytes, Fase1 07 = terminal interativo | `terminal.spec.ts:74`, `:223`, `:245` (prompt sem Start, teclas, histórico, cores, tela cheia); host: testes Rust do `terminal.rs` (lib 173 passed) | Dependência `@xterm/xterm` + `@xterm/addon-fit` aprovada e registrada. `TERM`: ver D-02 (corrigido) | — | verificado* |
| 24e | Project details e Trust→SDK no drawer de tools | AC-02, AC-08; Fase1 10 | `ui-layout.spec.ts:110` evaluated project details keep paths intact and Close with the heading; `trust-sdk.spec.ts:71` trust resolves the project SDK…; `:175` missing project SDK is distinct… | M10 coberto pelo spec de layout; M1 (drawer mistura restore, debug adapter, installer e LSP) não reauditado | M1 sem dono | verificado* |
| 25 | App local `.app` (gerar, instalar, atualizar) | Fase1 14; `docs/install-app.md` | Build do bundle: `bun run tauri build --bundles app` exit 0 (2026-10-30, 15 MiB, ad-hoc). Abriu como processo próprio; **smoke nativo 2026-10-31** (seção 3b): abrir → Trust → restore/avaliação automáticos → Run exit 0 → Debug até o breakpoint → terminal `echo ok` | Bundle gerado com `bun run tauri build --bundles app` e validado no target (abrir→Trust→restore→Run→Debug→terminal; `docs/evidence/logs/native-smoke-2026-10-31.md`, `native-*.png`). Instalação em `/Applications` via `bun run app:install` verificada em 2026-10-30 (mtime 16:52) e **não repetida** nesta rodada por restrição; a cópia instalada **não foi exercitada com o HEAD atual**. O smoke usou **cópia da fixture**, não o projeto real de aceite (decisão do Helder em aberto) Sem assinatura/notarização por desenho. `app:install` não foi rodado nesta rodada | Fase1 14; Mapa 13 | verificado* |
| 26 | Jornada P0 de aceitação (restricted→trust→SDKs→restore→LSP→build→run→testes→PTY→DAP) | Apêndice A, AC-01…AC-34 | `zz-acceptance.spec.ts` UI audit passed; jornada P0 passou após `702761b` (2/2, 2026-10-30, `TERM=xterm-256color`): restricted→trust→SDKs→restore→LSP→build→run→testes→PTY→DAP real. A jornada precisou de 3 ajustes ao layout da Fase 1 (drawer de tools fechado, terminal abre sozinho, popover de Trust cobre a aba após Revoke) | D-10 corrigido em `3bbc30e` (workaround removido; jornada 2/2). `dotnet --version` no terminal só prova que o pin 8.0.408 é satisfeito por alguma raiz detectada, não que o host usou o SDK resolvido | — | verificado* |
| 27 | Fronteiras do host (Rust) | decisions.md; testes de fronteira | `cargo test --workspace --no-fail-fast` (28/10→30/10): lib 173 (+1 do D-02), debugger 9, test_discovery 9, test_run 5 passed; `host.rs` 60 passed / 0 failed após `e9d3804` | — | — | verificado |
| 28 | Gates de release (G-LEGAL, G-PKG, G-NATIVE) | `docs/release-decision.md` | Nenhuma prova possível por spec | Sem ticket de código; decisão do Helder (Mapa 10) | Mapa 10 | bloqueado |

Contagem (33 linhas, 1–28 mais 24a–24e): **verificado/verificado\* 32**, **parcial 0**, **não verificado 0**, **bloqueado 1** (28). Prova NATIVA observada em 2026-10-31: 14 das 33 linhas, parcial ou totalmente (seção 3b); as demais, não verificado. Gutter Run (Main): não verificado.

## 3b. Prova NATIVA observada (smoke de 2026-10-31)

Bundle `src-tauri/target/release/bundle/macos/OpenRidr.app` do HEAD `c7439ad` (SHA-256 do binário `ecf0c103…dc1de`), PID 55568, janela do próprio processo capturada com `screencapture -l`; ações por System Events (AX) e, onde o AX não alcança (NSOpenPanel, canvas do xterm), eventos HID. **Numa cópia da fixture em `/tmp/openridr-smoke-20261031/OrderDesk`** (o netcoredbg do usuário usava a fixture do repositório). Log com ações/AX/horários: `docs/evidence/logs/native-smoke-2026-10-31.md`; capturas: `docs/evidence/screenshots/native-*.png`. Windows/Intel não verificados.

| Linha | Observado na janela nativa | Capturas |
|---|---|---|
| 1 Welcome/abrir | Welcome com recentes e Toolchain; botão Open → NSOpenPanel nativo → pasta da cópia → workspace aberto em Restricted | native-00, 02, 03, 04 |
| 2 Trust | Welcome → workspace aberto em Restricted (PID 50964, `OPENRIDR_HOME` isolado, cópia `/tmp/ponto`) → toolbar `Restricted mode` → popover → checkbox → "Trust and inspect project" → `Trusted mode`, popover **fecha sozinho** (D-13 pós-grant, native-19) e árvore vira `Evaluated · revision 2` → toolbar `Trusted mode` → popover → **Revoke trust** → `Restricted mode`, árvore volta a `Text outline · not evaluated`, shell parado com mensagem e `trusted.json` isolado `[]` (native-20). Capturas recortadas (sem prompt). | native-19, native-20 |
| 3 / 24e SDK | Barra de status `Host SDK 10.0.100 · Project SDK 8.0.408`; aba NuGet `Project SDK 8.0.408 · /Users/helder/.dotnet` | native-07 |
| 4 Solution Explorer | Árvore avaliada (`src/`, `tests/`, projetos console/library/test net8.0, Dependencies), `Evaluated · revision 2` | native-07 |
| 11 Restore automático | `Restore succeeded · exit 0` sem clique em restore (drawer Workspace tools) | native-08 |
| 12 Build | Build acionado pelo clique em Run: `Build succeeded · exit 0`, saída em `/private/tmp/openridr-smoke-…/bin/Debug/net8.0` | native-10 |
| 13 Run | Janela Run: `Run exited · exit 0`, `Order A-104: R$ 42,00`, `Process finished with exit code 0.` | native-09, 10, 11 |
| 14 Debug | Diálogo de consentimento experimental do produto → "Enable and debug" → `Paused` no breakpoint da linha 3 com Threads, Call frames (`Program.<Main>$()  …/Program.cs:3`) e Locals (`args`, `order = null`); Resume → `Terminated` com a saída do debuggee. Adaptador existente reutilizado, nada instalado | native-13, 14, 15, 16 |
| 5 Editor (parcial) | Abrir `Program.cs` pela árvore; breakpoint por **Cmd+F8** (ponto vermelho na linha 3). Salvar/conflito/fechar com buffer sujo: não verificado | native-12, 13 |
| 15 / 24d Terminal | Shell `/bin/zsh` já rodando após o Trust, cwd da cópia, prompt; `echo ok` → `ok` (canvas xterm fora do AX; prova por captura) | native-17, 18 |
| 17 Git (parcial) | Branch `fase-1-fluxo-…`, log, painel Commit com `src/OrderDesk.Core/Order.cs` Modified; diff do arquivo (Index × Working tree, dialog Close diff); commit pela UI do arquivo marcado (`47b4753`, árvore limpa); log atualizado por Refresh, commit com changed files (`M src/OrderDesk.Core/Order.cs`; captura não publicada, mostra autor git). Faltam: checkout, pull, push | native-21, 22, 23 |
| 18 NuGet (parcial) | Aba NuGet: "No packages are referenced by this project.", fonte nuget.org | native-07 |
| 25 App local | Todos os fluxos acima rodaram no `.app` gerado, como processo próprio | todas |

Não verificado nativamente (estado r10, após as evidências das seções 3b): completion (item 7), fechar aba suja por teclado (item 5), Rerun failed / cancelar / TRX / MTP (item 16), Working dir/.env/Environment e troca de Configuration/TFM (item 21), layout e Find Usages em 1440 (item 10) e o resto dos itens listados como parciais na 3b. Settings/Keymap, Search Everywhere, Problems, rename/code actions/find usages e Revoke com processo vivo têm evidência nativa parcial (ver 3b).

**Atualização 2026-10-31 (ponto/Trust).** A suspeita de defeito do Welcome "recent" foi **falso positivo**: um `NSOpenPanel` órfão, aberto por um Open anterior, bloqueava a webview (`docs/evidence/logs/native-welcome-recent-open-defect-2026-10-31.md`). A linha 2 (Trust) ganha sub-itens nativos (grant, D-13 pós-grant, Revoke) no .app de `7063510` (SHA-256 `34a87b38…684904`), mas segue **parcial**: o item também cobre processos remanescentes, não verificados. Continua não verificado nativamente: Git, Settings/Keymap, Search Everywhere, run configurations, Problems, rename/code actions/find usages (1440×900 e 1100×700, M7a), testes, buffers sujos/conflitos.

### Denominador explícito (33 itens da seção 3)

**Contagem: 0 provados / 23 parciais / 10 não verificados (total 33).** (Revisão r10: item 11 rebaixado de provado para parcial, pois o escopo da linha 71 é maior que `Restore succeeded`. Checkpoint r8: itens 16 e 21 saíram de não verificado para parcial, com evidência do SHA `34a87b38…`; itens 8, 9, 12 e 20 ganharam evidência do SHA `35038ee6…` (bundle com tickets 32 e 33). Checkpoint r6: itens 8, 9 e 10 saíram de não verificado para parcial. Antes, no checkpoint r4: itens 7, 19 e 20 saíram de não verificado para parcial.) Critério: provado só quando o escopo do item inteiro foi observado no .app; sub-itens observados contam como parcial. O número antigo "14/33" somava parciais e provados (14 rótulos distintos da 3b), por isso não era prova.

| # | Item (seção 3) | Status nativo | Evidência / lacuna |
|---|---|---|---|
| 1 | Welcome, recentes, abrir solução, ambiguidade | parcial | Open via NSOpenPanel provado (c7439ad, native-00–04); recentes e ambiguidade não exercidos nesta rodada |
| 2 | Trust, Restricted, revogação, processos remanescentes | parcial | grant, D-13 pós-grant e Revoke provados (`7063510`, native-19/20); processos remanescentes: SHA `35038ee6…` (r9, PID 80158): Revoke com `sleep 600` vivo no terminal → banner "Trust was revoked while the shell was running; it was stopped", "Termination requested: SIGTERM then SIGKILL · Observed: exited", `zsh: terminated sleep 600`, `ps` sem o processo (native-49). Faltam: Revoke com processo de build/Run ativo (cenário de árvore de processos filhos) |
| 3 | SDK do projeto vs host, `global.json`, toolchain | parcial | barra de status Host/Project SDK (native-07); toolchain/`global.json` não exercidos |
| 4 | Solution Explorer: árvore avaliada, TFM, detalhes, falha | parcial | árvore avaliada e TFM (native-07, native-19); falha de avaliação e detalhes não |
| 5 | Editor: abrir, editar, salvar, conflito, fechar com buffer sujo | parcial | abrir, editar e breakpoint (native-12/13); fechar com buffer sujo: diálogo "Close OpenRidr with unsaved changes?" (native-30), descarte por "Discard and close app" e buffers limpos ao reabrir; **salvar ⌘S** gravou em disco e "Saved …" (native-37); **conflito**: alteração externa por shell → "changed on disk. Your buffer was kept", disco preservado, "Reload from disk" descartou edições (native-38). Lacuna: fechar uma **aba** suja (× / atalho de aba) não exercitado; clique no × caiu no menu do macOS |
| 6 | Menu de contexto e ações do editor | não verificado | — |
| 7 | csharp-ls: instalação, sessão, Problems, completion, definition | parcial | Análise nativa ativa (checkpoint r5): Workspace tools (ícone de escudo) → Analysis tool 0.28.0 "Not installed" → Review install plan → Confirm: "Reused verified installation. Nothing was downloaded" (SHA-256 do launcher `0bcd…`) → Start analysis → "Ready · session 1". Problems: "coverage full", 22 erros / 1 aviso com diagnósticos reais (CS1729, CS1002, CS0117…), duplo clique navega para `Program.cs` 3:17 (native-28). Go to definition (⌘B) em `Order` abre `Order.cs` (native-29). Completion (Ctrl+Space) não observada: o popup não abriu com o helper de teclado (possível interceptação do atalho pelo macOS, não confirmado). Restore já atual (3 projetos, exit 0) e Build `OrderDesk.Cli` com exit 0 antes da análise | native-27, 28, 29 |
| 8 | Rename (prévia, cancelar inerte, aplicar a buffers) | parcial | Shift+F6 em `Order` (3:17) → campo inline; Enter → prévia docada abaixo do editor, "3 files · 6 edits · nothing has been applied", Apply to buffers / Cancel visíveis em 1440×900 (native-32) e 1100×700 (native-33); Cancel inerte nas duas ("rename cancelled; no buffer or file changed"); Apply to buffers em 1440 → "Rename applied to 3 files; 3 buffers are modified and unsaved. Nothing was written to disk" (sem salvar). Lacunas: M7b (campo inline) segue aberto. **SHA `35038ee6…` (ticket 32, r8):** com Problems aberto, 1100×700 mostra as 3 linhas da prévia (Program.cs, Order.cs, OrderTests.cs) com Apply/Cancel visíveis (native-41); Apply to buffers em 1100 → "Rename applied to 3 files; 3 buffers are modified and unsaved. Nothing was written to disk" (native-42); 1440×900 regressão ok (native-44). |
| 9 | Code actions (listar, prévia, invalidar) | parcial | Option+Enter → "Code actions: N actions from csharp-ls … nothing has been applied" com Close e 1ª ação; em 1440×900 "Convert to 'Program.Main' style program" → prévia "1 file · 1 edit … nothing has been applied" com Apply/Cancel visíveis (native-34); em 1100×700 "Wrap every argument" → prévia com Apply/Cancel (native-35), Cancel inerte nas duas ("code action cancelled; no buffer or file changed"). Lacunas: invalidação antes de escrever não exercitada. **SHA `35038ee6…` (ticket 32, r8):** com Problems aberto, 1100×700 lista as primeiras ações (Wrap every argument…, Unwrap…, Convert to 'Program.Main'…) com Close visível e a prévia de "Wrap every argument" com Apply/Cancel (native-40). A 1ª ação deixou de ser cortada. |
| 10 | Find Usages (painel, navegação) | parcial | Shift+F12 em `Order` → "4 references from csharp-ls; the list is read-only" com Close (native-36, 1440×900). Navegação: clique na referência → caret em Program.cs 3:17 (observado em 1100×700; não repetido em 1440). Lacuna: lista de 4 referências cortada pelo painel Problems (só 1ª linha visível sem rolagem) |
| 11 | Restore automático após Trust | parcial | Observado no SHA `34a87b38…` (native-08): `Restore succeeded · exit 0` sem clique após Trust. Lacunas do escopo (linha 71): cancelar, retry, reavaliação ao mudar `.csproj`, e prova de que Restricted nunca restaura (não exercitados nativamente) |
| 12 | Build (log cru, erro navegável, buffers sujos) | parcial | `Build succeeded · exit 0` por Run (native-10). **SHA `35038ee6…` (r8):** com buffers sujos, Build pede "Build with unsaved buffers?" (Save all and build / Build saved content / Cancel); "Build saved content" → "Running the saved build; 3 unsaved buffers are not included" + `Build succeeded · exit 0` (native-45). Erro navegável e log cru não reobservados neste SHA |
| 13 | Run e janela Run (saída, exit, parar, um clique) | parcial | saída e exit 0 (native-09–11); parar não |
| 14 | Debug (breakpoint, step, frames, locals, condicionais, exceções) | parcial | breakpoint, Paused, Resume (native-13–16); step, condicionais e exceções não |
| 15 | Terminal xterm (prompt, Ctrl+C/D, tela cheia, resize) | parcial | prompt e `echo ok` (native-17/18); Ctrl+C/D, tela cheia e resize não |
| 16 | Testes (descoberta VSTest, run, TRX, cancelar, MTP) | parcial | SHA `34a87b38…` (r7): Tests → Discover tests → "Discovered 4 tests · exit 0" (árvore OrderDesk.Tests → OrderTests); Run all → "Run completed · 4 results · exit 1" com 2 passed, 1 failed (`Deliberate_failure_for_runner_evidence`, falha intencional do fixture), 1 skipped (`Skipped_until_fixture_changes`, skip intencional) (native-46). Faltam: cancelar, MTP, TRX (painel mostra `Test run details` com caminho TRX, não expandido), Rerun failed, Run selected/Debug selected, detalhe de falha. |
| 17 | Git (commit, diff, log, branch, checkout/pull/push) | parcial | Trusted no home isolado; branch, log e diff do `Order.cs` observados após reabrir pelo Welcome; commit pela UI (`47b4753`, árvore limpa) e changed files do commit. Faltam: checkout, pull, push. Defeito de r4 (popup de branches com refs do worktree `openridr-fase1`) **não reproduzido** em 2026-10-31 no PID 57170: o popup lista `feature-b`, `origin/fase-1-…`, `origin/feature-b`, com upstream `origin/fase-1-…` (native-31); o host roda git em `repo.top` da cópia e o disco confirma as mesmas refs. Causa da observação de r4 não confirmada (hipótese não verificada: captura de outra janela). Faltam: checkout, pull, push (repo remoto bare local existe, não exercitado) | native-21, 22, 23, 31 |
| 18 | NuGet (instalados/desatualizados, sem pacotes, CPM) | parcial | aba NuGet com sem pacotes e fonte (native-07); desatualizados e CPM não |
| 19 | Settings, Keymap, Find Action | parcial | Settings abre (Appearance/Keymap/Editor/Build/Version Control) e a página Keymap lista perfil Rider (macOS) com Change/Reset (native-24); ⇧⌘A abre Find Action com atalho e `id: view.findAction` (native-25). Faltam: alterar atalho, conflito, Reset, salvar |  native-24, 25 |
| 20 | Search Everywhere | parcial | ⇧⌘A abre no tab Actions; tab Files filtra (`order` → 7 resultados com prévia); tab Text (`Order` → 34 resultados destacados, native-26); Symbols mostra "Analysis not ready. Start analysis." (sem análise, não prova). **SHA `35038ee6…` (ticket 33, r8):** Classes com `Order` → Order.cs e OrderTests (r6); Symbols com análise Ready e `Program` → "No results", **sem** a mensagem enganosa "truncated by the host" (native-43; defeito de r6 corrigido). Faltam: Enter abrir resultado de Symbols/Classes com análise, Actions | native-25, 26, 43 |
| 21 | Run configurations | parcial | SHA `34a87b38…` (r7): toolbar → "Edit run configurations…" → configuração derivada somente leitura ("Duplicate it to edit a named copy"); Duplicate → cópia editável; Arguments `Hello` aplicado e persistido após OK; Run direto → recusado ("Build the startup project first; Run never builds or restores implicitly"); Build (Cmd+F9) exit 0 → Run (Ctrl+R) → `Order A-104: R$ 42,00`, "Process finished with exit code 0" (native-47, native-48); Remove → lista volta só à derivada; Cancel fecha (sem captura própria). Faltam: edição de Working dir/.env/Environment, linha .env inválida, troca de Configuration/TFM. |
| 22 | Layout 1440×900 e 1100×700, tool windows, barra | não verificado | 1440×900 visto (native-19); 1100×700 não |
| 23 | Plano de instalação de ferramentas | não verificado | — |
| 24 | Estados desabilitados com razão visível | não verificado | — |
| 24a | Problems (painel, severidade, origem, navegação) | não verificado | — |
| 24b | Gutter Run/Test e Alt+Enter | não verificado | — |
| 24c | Debug de teste desabilitado por decisão | não verificado | — |
| 24d | Terminal automático com prompt | parcial | shell iniciado após Trust com prompt (native-17); parada/revoke do shell visto em native-20 |
| 24e | Project details e Trust→SDK no drawer | parcial | barra de status SDK (native-07); drawer de project details não |
| 25 | App local `.app` (gerar, instalar, atualizar) | parcial | geração e execução do bundle; instalar em /Applications e atualizar não |
| 26 | Jornada P0 de aceitação | não verificado | — |
| 27 | Fronteiras do host (Rust) | não verificado | nativo não aplicável; prova via cargo test, não conta aqui |
| 28 | Gates de release (G-LEGAL, G-PKG, G-NATIVE) | não verificado | — |

## 4. Defeitos e limites novos desta rodada (D-01, D-02, D-03, D-04, D-05, D-09, D-10, D-11, D-12 e D-13 corrigidos; batch D12/D13 aprovado pela auditoria independente terminal — não é aceite global)

| ID | Sev. | Achado | Evidência | Ticket proposto |
|---|---|---|---|---|
| D-01 | M | **Corrigido** em `96a0de7`; a falha restante (`initialize_failure…`) foi reescrita com stub em `e9d3804`. 13 testes de `crates/host/tests/host.rs` falham nesta máquina: `sdk_inventory…` espera SDK `8.0.416` e a máquina tem `8.0.408`; `restore_reports…` exige `sdkRoot` terminando em `OpenRidr/sdks/8.0.408`, contra a regra do `AGENTS.md:47` da base (seção ".NET SDK 8.0.408") ("qualquer raiz detectada pode satisfazer o pin… o contrato é a versão resolvida e o `sdkRoot` absoluto"). Os outros 11 falham em cascata por `PoisonError` do `ENV_LOCK`. | exit 101 em `9bf0bec`; as 13 falhas foram contadas no log não versionado `/tmp/t01/m/cargo2.log` (13 linhas `FAILED`) | Tornar os dois asserts independentes do caminho/versão da máquina (contrato = versão resolvida) |
| D-02 | B | **Corrigido** em `8e45eda`. O PTY só define `TERM=xterm-256color` quando `TERM` não existe (`terminal.rs`); herdando `TERM=dumb` (ex.: app aberto a partir de shell/ferramenta) `less` avisa "terminal is not fully functional" e as cores ANSI não aparecem. Os specs de terminal dependem do `TERM` do chamador. | `terminal.spec.ts` 2 failed com `TERM=dumb`, 4/4 com `TERM=xterm-256color` | Forçar `TERM=xterm-256color` no PTY (o emulador é o xterm.js) e fixar no spec |
| D-03 | M | **Corrigido** em `702761b`. A jornada `zz-acceptance.spec.ts:166` quebra na etapa csharp-ls: `install-plan-open` não existe naquele estado da UI (drawer/menu). Sem ela não há prova de ponta a ponta atual (restore→LSP→build→run→testes→PTY→DAP). | 1 failed, 1 passed (UI audit), 2 execuções | Atualizar a jornada ao layout da Fase 1 (abrir o drawer antes do plano) e rodá-la verde |
| D-04 | M | **Specs corrigidos** em `7eb1cb2` (foco por teclado sob o popover; csharp-ls 10/10, exit 0). A causa não foi só drift de spec: o popover de rename/code actions **cobre o centro do editor**, então um usuário real também não consegue clicar no editor com a prévia aberta (ligado a M7 do UI-AUDIT). | `docs/evidence/logs/matrix-2026-10-30-summary.md` (resumo versionado; logs completos em `/tmp/t01/m`, não versionados). Issues 12 e 14 do roadmap: `/Users/helder/Projetos/openridr/.scratch/roadmap/issues/` (não rastreados) | Ver D-12 |
| D-12 | B/M | **Corrigido e aprovado pela auditoria independente terminal** no intervalo `2a054ca..7d5b62e` (leitura + logs; não reexecutado nesta rodada). **Correção:** cancelamento central em `cancelEditPreview` bloqueia consistentemente Cancel/Esc do popup/Esc e clique no editor durante `editPreview.applying`; Apply verifica sequência + identidade **antes de qualquer mutação de sucesso ou falha** após os `file.read`, então leitura antiga não marca prévia nova como stale, não sobrescreve status novo e não chega a `setTabs`. Clique em texto alcançável e Esc com foco real fecham rename preview, lista e code-action preview sem aplicar; cliques internos e Cancel continuam. O popup **continua sobrepondo** o centro do editor (M7 não resolvido); não se afirma acesso ao texto coberto. | `csharp-ls.spec.ts:1788` + `:2070`: red stale-error antes da correção (exit 1; leitura antiga abortada sobrescreveu `Rename preview:` com erro), red sem guard central (exit 1, prévia sumiu no Esc durante Apply) → green 7/7. Após **cada** saída: conteúdo/revisão/abas/dirty + hash imediato; hit-testing/click+Esc dos três popups em 1440×900 e 1100×700; Apply+Esc no popup, Apply+clique no editor, troca de arquivo durante Apply; 6 cenários independentes de foco+digitação/undo com dirty explícito; foco já ativo assertado imediatamente após o dismissal, antes de qualquer refocus, e reexecutados **6/6** após o endurecimento. Spec focado 16/16 anterior (instalação real excluída); `check`/`build` exit 0. Logs em `/tmp/openridr-final-review/` e `/tmp/openridr-focus-proof/` (não versionados). Nativo Tauri: não verificado | — |
| D-05 | M | **Corrigido** em `1bc3d02`. Find Usages real não tem prova automatizada; só o gate "sem análise" (`code.spec.ts:81`). | grep de `tests/e2e` | Spec real de `lsp.references` com navegação |
| D-06 | A | **Fechado** em 2026-10-31: smoke nativo executado (seção 3b; log e capturas em `docs/evidence/`). A automação precisou de Acessibilidade para o System Events (concedida pelo Helder) e de eventos HID (`CGEventPost`) para o NSOpenPanel, que não aparece no AX. | `docs/evidence/logs/native-smoke-2026-10-31.md` | — |
| D-07 | M | **Fechado:** `execution-gates.spec.ts` (4 testes puros, import direto de `src/executionGates.ts`, sem host) afirma o texto de **todos os ramos contados no código**: Build 11, Run 17 e Debug 20 (nenhum ramo "não afirmado"), incluindo sealed, another startup project, not a Debug build, Apple Silicon (e a ausência dele fora de macOS arm64), not restored, More than one console project, program already running e debug session already active. Limite: prova as funções puras; a fiação UI→gate segue provada pelos specs E2E (`gates.spec.ts`, `run.spec.ts`, `debugger.spec.ts`) | `execution-gates.spec.ts` 4 passed (2026-10-31) | — |
| D-09 | M | **Fechado** em `0f00ae7`: `zy-gutter.spec.ts` prova o gutter de teste; gutter Run segue não verificado (fixture sem `Main`). | `zy-gutter.spec.ts:16` | — |
| D-11 | M | **Corrigido** em `0f00ae7` (`codeNavigation.ts`: nome sem a lista de parâmetros e, sem `containerName`, a classe é achada pela faixa de linhas). Host, gates, Trust e testids intocados. | `zy-gutter.spec.ts`, `code.spec.ts` 3/3, `csharp-ls.spec.ts` 10/10 | — |
| D-10 | M | **Corrigido** em `3bbc30e`: Revoke fecha o popover de Trust (`WorkspaceView.tsx`; gate de Trust intocado). Spec de geometria `gates.spec.ts:68` Revoke closes the Trust popover and leaves the tabs clickable (red→green); workaround removido da jornada. Asserts `mode-restricted` pós-Revoke passaram a `mode-badge`. | `zz-acceptance` 2/2, csharp-ls 10/10, tests 6/6, terminal 4/4, debugger 12/12, gates/workspace/trust-sdk/shell 25/25 | — |
| D-13 | B/M | **Aprovado pela auditoria independente terminal** no intervalo `6de4bea..1c2eb25` (Standards e Spec, 0 findings acionáveis; leitura + evidência registrada, sem novas suítes). D13 produto foi aprovado em `6de4bea`; os P2 de cleanup partial-open/unknown-state e hook contemporâneo foram fechados em `1c2eb25`. **Correção:** o `TrustControls.onChange` do toolbar só é chamado depois do `trust.grant`/`trust.revoke` real resolver; nesse boundary o popover fecha e devolve foco ao summary. Pending e `trust-error` permanecem visíveis; retry usa o host real e só fecha no sucesso. Cleanup usa `workspace.state` + `realpath`, fecha só a cópia própria e, em estado/close/posse desconhecido, **retém** todas as cópias potencialmente possuídas e reporta o motivo. **Limite aceito:** host E2E exclusivo/serial, workers=1, sem outro ator; snapshot->close não é transação nem enforcement atômico de posse. | `gates.spec.ts:316` (loop 1440/1100), `:395` (erro/retry), `:435` (partial-open), `:468` (retenção com workspace de outra cópia do teste) e `:529` (snapshot indisponível). Seleção final gates **8/8**, `restore.spec.ts` **2/2**; `trust-sdk` 3/3 e `zy-restore-interrupt` 5/5 já verdes no mesmo produto. Red legado 2/2 → ownership green 3/3; check standalone exit 0; build não repetido. Erro injetado é boundary de UI, **não** falha real de host/rede. Hook contemporâneo fechado (child_exit 0/config inalterado); caveat histórico: **check standalone verificado; hook histórico não comprovado**. **Receipts da auditoria:** `/tmp/openridr-review-1c2eb25-v2qjn/` (`final-head.log`, `final-status.log`, `commits.log`, `diff.log`, `scope.log`, `whitespace.log`, `product.log`, `hook-mode.log`, `hook-config.log`). Nativo Tauri/Windows/Intel/full suite: não verificados. M7/overlay continua **não resolvido**: não é aceite global. | — |
| D-08 | B | **Corrigido** em `ecfc0b9` (+ borda do `staleReason` vazio, que usa `||`): o gate de build obsoleto tira o ponto final da razão do host antes de acrescentar o seu (`executionGates.ts`); `execution-gates.spec.ts` afirma o texto com ponto único (red→green: `…build again..` → `…build again.`). | `execution-gates.spec.ts` | — |

## 5. Defeitos e limites já conhecidos (herdados)

| Origem | Sev. | Item | Dono |
|---|---|---|---|
| UI-AUDIT A1–A8 | A | Fechados pelos tickets Fase1 03, 04, 08, 09, 10, 11, 12, 13 (todos `resolved`; A5 fechado no ticket 10 com prova de geometria) | — |
| UI-AUDIT M1–M13 | M | Abertos, **não reauditados** pós-Fase 1 (M1 tools numa faixa, M2 Problems, M3 run configs, M4 keymap, M5 terminal, M6 menu de contexto, M7 rename, M8 git log, M9 NuGet, M10 detalhes do projeto, M11 settings, M12 botão destrutivo, M13 editor 1100) | **sem dono** |
| UI-AUDIT L1–L9 | B | Abertos, não reauditados | **sem dono** |
| Dívida | M | `WorkspaceView.tsx` 4672 linhas / 95 ocorrências de hooks em `cdfa541` (base `b8bd154` = 5126 / 113; comando: `grep -c "use\(State\|Ref\|Effect\|Callback\|Memo\)" src/WorkspaceView.tsx`); restore/avaliação, Build/Run/Debug e o store de run configs estão em hooks próprios; fora do critério: fluxo de testes e `git.rs` | Fase1 01 (partial até o auditor reconfirmar), Mapa 07 |
| G-LEGAL | A | Sem `LICENSE`/`NOTICE`/`THIRD_PARTY_NOTICES`, ícone provisório (`docs/release-decision.md`) | **sem ticket de código** |
| G-PKG | A | Sem assinatura/notarização, sem instalador público | **sem ticket de código** |
| G-NATIVE | A | Smoke macOS ARM64 feito em 2026-10-31 (seção 3b), numa cópia da fixture; fora dele: atalhos, fechamento com buffers sujos e diálogos de conflito/rename na janela nativa não verificados; Windows não verificado | Mapa 13 (parcialmente fechado) |
| csharp-ls | B | Verificação de assinatura NuGet pulada pelo `dotnet tool install`; netcoredbg sem assinatura/notarização (Apêndice A, G3) | aceito/documentado |
| Plataformas | — | Windows, macOS Intel, Linux: **não verificado**, sem promessa | fora de escopo |

## 6. Decisões que dependem do Helder

1. ~~Smoke nativo~~ — **feito** em 2026-10-31 (seção 3b); falta decidir se o bundle instalado em `/Applications` também precisa do smoke e se a cobertura nativa adicional (atalhos, buffers sujos, rename, Git) entra no MVP.
2. G-LEGAL e G-PKG entram no MVP aberto ou ficam para depois? (Mapa 10; sem ticket de código hoje.)
3. Os 22 desvios M/L do UI-AUDIT entram na estabilização do MVP ou ficam em backlog? (Mapa 10 d.)
4. csharp-ls rename/code actions: aceitar o timeout como limite do ambiente ou investigar a causa agora (Mapa 12)?
5. Aprovar abrir tickets para D-01, D-02, D-03 e D-05, que são correções pequenas de teste/host sem mudar produto (exceto D-02, que muda o `TERM` do PTY).

---

# Apêndice A — Histórico P0 (2026-10-28), preservado

> Estado histórico. Os números de testes, as células AC × ambiente e o contrato de restore "explícito" deste apêndice refletem a base `4b8885c` de 28/10 e **foram superados** pela Fase 1 (restore automático após Trust). Mantidos como registro de evidência (E01…E17, A18, L18).

## A.0 Matriz de aceitação P0 — OpenRidr

Data: 28/10/2026. Base de código: `4b8885c` + o commit `feat(acceptance)` desta entrega.
Escopo: fatia P0 (tickets 01–17) e consolidação do ticket 18. Documento em PT-BR; UI em inglês.

**Decisão desta matriz: release NÃO aprovado.** A matriz registra o que foi realmente
executado, o que está bloqueado e o que não foi verificado. Ela não transforma ausência de
prova em aprovação, não cobre Linux/outras arquiteturas e não é parecer jurídico.

### Como ler os estados

| Estado | Significado |
|---|---|
| `approved` | A cláusula foi exercitada naquele ambiente e a evidência observada está registrada. Não vale para outro ambiente. |
| `blocked` | A cláusula não pode ser satisfeita com a evidência atual, ou um gate obrigatório de release está aberto. |
| `not_verified` | Não foi executado naquele ambiente. Ausência de prova; nunca “passou”. |

Evidência: `E01`…`E17` = `docs/evidence/NN-*.md`; `A18` = `docs/evidence/18-aceitacao-e-release.md`;
`L18-*` = `docs/evidence/logs/18-*`; `S18-*` = `docs/evidence/screenshots/18-*`.
Os logs de cada ticket (comandos, saídas e exits) estão em `docs/evidence/logs/`.

### Ambientes

| Ambiente | SO / arquitetura | Versões observadas | SDK host | SDK do projeto | TFM | Ferramentas/assets | Executado |
|---|---|---|---|---|---|---|---|
| macOS ARM64 (esta máquina) | macOS 27.2 (26B118), Apple M2, arm64; Xcode 27.1 | Bun 1.3.9; Node 26.10.0; Rust/cargo 1.99.0; Chrome 155.0.8059.85; Playwright 1.64.1 | .NET SDK 10.0.100 (`/usr/local/share/dotnet`) | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408`, `global.json` `rollForward: disable` | net8.0 | csharp-ls 0.28.0; netcoredbg 3.2.0-1092 (`osx-arm64`, community supported) | Sim, jornada real + suítes por ticket |
| macOS Intel | Não executado | — | — | — | — | netcoredbg 3.2.0-1092 não publica `osx-x64`; nenhuma versão substituta foi baixada | Não: sem host Intel nesta sessão |
| Windows x64 | Não executado | — | — | — | — | Asset `netcoredbg-win64.zip` existe no metadata, mas não foi instalado/executado | Não: sem host/CI Windows; UI desktop e DAP exigem smoke próprio |

O repositório é local (sem remote); não há CI Windows configurado. CI que só exercite CLI/contratos
não comprova a UI desktop e não fecharia o gate. Nenhum mínimo de macOS/Windows foi inventado;
as versões acima são as reais desta máquina.

### Matriz AC-01…AC-34 × ambiente

| AC | Cláusula (resumo) | macOS ARM64 | macOS Intel | Windows x64 |
|---|---|---|---|---|
| AC-01 | Abrir não confiável: caminho completo, trust desmarcado, edição textual e ausência de MSBuild/restore/LSP/tarefas/build/run/debug/runner/PTY | **approved** — E01/E02 + A18: restrito, 12 chamadas diretas `trust_required`, edição/save, sem `bin/obj`; L18-acceptance-summary | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows; UI desktop não executada |
| AC-02 | Inventário fora do workspace; SDK do projeto separado do host 10 e do TFM; SDK faltante não reescreve `global.json` | **approved** — E02/E05 + A18: host 10.0.100, projeto 8.0.408, TFM net8.0, `global.json` intacto | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-03 | Restore explícito; build/run/test não restauram; falha mantém contexto parcial, log e retry | **approved** — E04/E11/E12/E16/E17 + A18: restore exit 0, build/test bloquearam antes do restore, falha/cancel/retry registrados | **not_verified** — sem host Intel | **not_verified** — restore/build/test Windows não executados |
| AC-04 | Cancelar plano não baixa/executa; confirmar mostra versão/fonte/asset/arquitetura/destino/operação/permissões; trust e opt-in não substituem confirmação | **approved** — E05/E13 + A18: planos reais csharp-ls e netcoredbg; flag/trust separados | **not_verified** — instalação Intel não executada; asset osx-x64 ausente | **not_verified** — instalação/plano Windows não executados |
| AC-05 | downloading/installing/verifying, sucesso, cancelamento, falha, retry; log sobrevive; parcial não vira ready; executável/versão verificados; sem SDK de sistema | **approved** — E05/E13 + A18: csharp-ls 0.28.0 e netcoredbg 3.2.0-1092 reais; versões verificadas; nenhum SDK instalado pelo app | **not_verified** — sem host Intel | **not_verified** — caminho Windows existe no código, não executado |
| AC-06 | Asset incompatível/ausente, origem não validada ou verificação falha bloqueiam; sem downgrade silencioso/global; percentual só medido | **approved** — E05/E13 + testes de fronteira: hash/redirect/zip/traversal/versão; limitação: NuGet imprime “Skipping NuGet package signature verification”; SHA do netcoredbg vem do digest do GitHub, não de assinatura/notarização | **not_verified** — ausência de `osx-x64` é bloqueio documental, não execução em Intel | **not_verified** — asset win64 não instalado/validado |
| AC-07 | Revogar trust bloqueia novas execuções, encerra serviços e solicita término; remanescentes visíveis; sem alegação falsa de limpeza | **approved** — E02/E04/E05/E06/E11/E12/E15/E16/E17 + A18: revogação com buffer preservado, chamada direta `trust_required`, LSP parado, processos observados | **not_verified** — sem host Intel | **not_verified** — término Windows não executado |
| AC-08 | Abrir sln/projeto; ambiguidade pede escolha; console/biblioteca/testes com referências/TFM/pacotes; bin/obj ocultos; restrito só contorno textual | **approved** — E01/E03 + A18: três projetos avaliados, TargetPath, árvore sem bin/obj | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-09 | Projeto com falha de avaliação permanece visível com abrir/log/retry; restore incompleto explícito | **approved** — E03/E04: MSB4019, restore parcial, retry | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-10 | Mudança de contexto invalida avaliação; TargetPath corresponde ao contexto MSBuild, nunca caminho inferido/obsoleto | **approved** — E03/E04/E11/E12/E14 + A18: revisões 1→2→3, TargetPath Debug/net8.0 | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-11 | Sem solução/host/LSP: editar/salvar; abas/breadcrumb/modificado; fechar/conflito com decisão explícita | **approved** — E01/E07 + A18: edição restrita/save, fechamento com descarte, conflito coberto por E07 | **not_verified** — sem host Intel | **not_verified** — CRLF/paths/permissões Windows não executados |
| AC-12 | Completion/definição reais; rename console↔biblioteca com prévia; cancelar não altera; mudança concorrente invalida | **approved** — E08/E09 + A18: completion `Id`, definição `Order.cs:4:11`, rename 3 arquivos/6 edições com cancelamento inerte | **not_verified** — sem host Intel | **not_verified** — UTF-16/CRLF Windows não executados |
| AC-13 | Code action só se retornada no contexto; aplicar não marca diagnóstico corrigido; ausência não fabrica quick fix | **approved** — E10 + A18: 6 ações reais listadas, prévia cancelada; L18-code-actions | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-14 | Problems mantém severidade/código/mensagem/posição/origem/categoria/revisão; LSP e Build não se fundem | **approved** — E06/E11 + A18: `CS0246` em LSP e Build separados, origem/categoria/revisão | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-15 | Editar buffer torna diagnóstico/build anterior obsoleto; publicação vazia só cobre documento/revisão; restore/analyzers não viram garantia global | **approved** — E06/E11 + A18; limitação: `analyzerCoverage: not_verified`; publicação vazia é document-scoped | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-16 | LSP: carga, falha, log, reinício; coexistência host 10 + projeto 8; falha mantém edição e log | **approved** — E05/E06 + A18: csharp-ls 0.28.0.0, hostroot runtime 10 + SDK 8; reavaliação posterior deixou cobertura `partial` explicitamente | **not_verified** — sem host Intel | **not_verified** — symlink/hostroot/CRLF Windows não executados |
| AC-17 | Startup/config/TFM explícitos; biblioteca/ambiguidade não inicia; comando/contexto/stdout/stderr/estado visíveis | **approved** — E11/E12 + A18: startup console, Debug/net8.0, comando/cwd/SDK/log | **not_verified** — sem host Intel | **not_verified** — Shift+F10/processos Windows não executados |
| AC-18 | Build falho bloqueia Run/Debug; erro com posição abre fonte; texto não reconhecido fica no log; corrigir/salvar/recompilar permite executar | **approved** — E11/E12/E14 + A18: falha CS0246 exit 1, navegação, rebuild exit 0, Run OK | **not_verified** — sem host Intel | **not_verified** — build/erro/cancelamento Windows não executados |
| AC-19 | Buffer salvo depois do build não é fonte verificada; execução identifica revisão salva e exige decisão | **approved** — E11/E12: `savedNote`, hashes de fontes, aviso “Sources were saved after this build”; A18 executou o build salvo | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-20 | Cancelar build/run pede término real da árvore; exit conhecido preservado, ausente continua desconhecido, falha explícita | **approved** — E11/E12: cancelamentos reais, requested/observed separados; limitação: `exit:null` foi provado por unidade/boundary, o cancelamento real observou 143 | **not_verified** — sem host Intel | **not_verified** — TerminateProcess/árvore Windows não executados |
| AC-21 | Onde asset/plataforma habilitada: build Debug, launch da saída avaliada, breakpoint pendente→verificado, pausa, continue, step over/into, frames, locals, stop e logs | **approved (experimental opt-in)** — E14 + A18: netcoredbg 3.2.0-1092 `--interpreter=vscode`, breakpoint verificado, pausa, F8, locals, continue exit 0, stop observado; async/multi-thread em E14 | **not_verified** — debug indisponível: sem asset osx-x64; núcleo Intel não executado | **not_verified** — DAP Windows exige smoke próprio; asset win64 não executado |
| AC-22 | Build falho/adaptador ausente bloqueiam launch; breakpoint sem símbolos fica pendente; locals antigos não são atuais; exit ausente não vira zero nem prova limpeza | **approved** — E14/E13/E11 + A18: bloqueio antes do build, breakpoint pendente, locals limpos no continue, exit/termination; limitações: `runInTerminal` real não emitido, variáveis aninhadas não expandidas, `exit:null` por boundary | **not_verified** — sem host Intel | **not_verified** — DAP Windows não executado |
| AC-23 | Mac ARM64 inicia flag OFF/aviso experimental, Debug bloqueado, Run independente; opt-in não instala/prova; retirada exige gate DAP/async/multi-thread | **approved** — E13/E14 + A18: OFF por padrão, opt-in de sessão, instalação/launch separados; prova DAP real existe, mas a flag foi mantida; #256 segue risco não reproduzido | **not_verified** — N/A fora de macOS ARM64 | **not_verified** — N/A fora de macOS ARM64 |
| AC-24 | Mac Intel sem asset 3.2 mostra indisponibilidade e não baixa versão divergente; Windows x64 exige prova DAP própria; limitações não bloqueiam análise/run/testes | **blocked** — E13/E14 + A18: a parte Mac ARM64 (limitações não bloqueiam análise/Run/testes) está provada, mas a cláusula exige Intel e Windows; Intel sem `osx-x64` e Windows sem smoke DAP | **not_verified** — sem hardware Intel; indisponibilidade é decisão documental, não execução | **not_verified** — DAP Windows obrigatório e não executado |
| AC-25 | Após trust, um PTY com shell/cwd reais; entrada, output e Ctrl+C nos dois SOs; antes de trust não abre shell | **approved** — E15 + A18: `/bin/zsh`, cwd com espaço, `pwd`, `echo`, Ctrl+C em `sleep 30`, `dotnet --version` 8.0.408 | **not_verified** — sem host Intel | **not_verified** — ConPTY/input/Ctrl+C Windows não executados |
| AC-26 | Fechar shell ativo oferece manter/encerrar; saída/código são do shell; prompt não é shell encerrado; término e remanescentes observados | **approved** — E15 + A18: diálogo Keep/Terminate, `SIGTERM then SIGKILL`, `killedAfterGrace:true`, exit null; processo observado | **not_verified** — sem host Intel | **not_verified** — término/ChildKiller Windows não executados |
| AC-27 | Descoberta VSTest real da fixture xUnit; projeto/TFM/seleção explícitos; Run all/selected respeitam escopo | **approved** — E16/E17 + A18: 4 identidades, TFM net8.0, selected com filtro, all com 4 resultados | **not_verified** — sem host Intel | **not_verified** — VSTest/CRLF/cancelamento Windows não executados |
| AC-28 | passed/failed/skipped reais; mensagem/stack/output; navegar só com posição fornecida, senão log | **approved** — E17 + A18: 2 passed/1 failed/1 skipped, detalhes/stack, posição só quando fornecida | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-29 | Cancelar descoberta/execução preserva parciais e marca cancelled; ausentes não viram passed/skipped; infra ≠ asserção, com logs/retry | **approved** — E16/E17 + A18: cancelamento real de `Task.Delay(60000)`, 0 results parciais, exit 143, sem passed inventado | **not_verified** — sem host Intel | **not_verified** — cancelamento Windows não executado |
| AC-30 | Vazio, adapter ausente e descoberta falha distintos; nenhuma ação instala/edita packages; contagens pertencem à execução/revisão | **approved** — E16/E17 + A18: 4/5 identidades por descoberta, contagens da execução, sem alteração de packages | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-31 | MTP recebe mensagem exata e fallback de terminal; nenhum VSTest/conversão; TRX não descobre | **approved** — E16/E17: mensagem exata, comando vazio, sem processo; TRX só resultados | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-32 | Sete telas/tema/strings inglesas, foco visível, nomes acessíveis, teclado e estados textuais; atalhos por SO respeitam gates | **approved com limitações** — E01–E17 + A18: S18-screen-01…07 a 1440×900, auditoria heurística de nomes/foco/contraste/overflow, layout 1100×700; limitações: sem leitor de tela, janela Tauri nativa não dirigida, teclado Windows só por código | **not_verified** — sem host Intel | **not_verified** — UI desktop/teclado Windows não executados |
| AC-33 | Registrar matriz macOS Intel/ARM64/Windows x64 do núcleo; publicar Mac e Windows juntos com debug separado e limitações | **blocked** — matriz e decisão registradas (este documento + `docs/release-decision.md`), mas a publicação conjunta está retida: Windows/Intel não verificados; Mac sozinho não é entrega reduzida | **not_verified** — ambiente não executado | **not_verified** — ambiente não executado |
| AC-34 | Auditar licenças/avisos de componentes/assets incorporados ou baixados, preservar obrigações e identidade; auditoria não é liberação jurídica | **blocked** — auditoria feita (A18), mas faltam `LICENSE`/`NOTICE`/`THIRD_PARTY_NOTICES`, o ícone é o default do Tauri e não há revisão jurídica; MPL-2.0 transitivo exige avisos | **not_verified** — grafo Intel específico não executado | **not_verified** — grafo Windows específico não auditado em runtime |

**Contagem das 102 células AC × ambiente:** 31 `approved`, 3 `blocked`, 68 `not_verified`.
Os 3 bloqueios Mac são AC-24, AC-33 e AC-34; todas as células Intel/Windows são `not_verified`.
Não há percentual de cobertura nem contagem de arquivos inventada.

### Gates compartilhados G1–G5

| Gate | macOS ARM64 | macOS Intel | Windows x64 |
|---|---|---|---|
| G1 — trust e fronteira | **approved** — chamadas diretas negativas em 12 métodos, consumidores reais, revogação e processos observados (E02/E04/E11/E12/E15/E16/E17 + A18) | **not_verified** | **not_verified** |
| G2 — contexto e integridade | **approved** — `contextId`, respostas tardias, revisões, paths canônicos, exit `null` distinto de 0 (E03–E07/E11/E12/E14/E16/E17 + A18) | **not_verified** | **not_verified** |
| G3 — consentimento e instaladores | **approved com limitações** — planos/confirmação/cancelamento/hash/extração reais; sem assinatura NuGet e sem notarização do netcoredbg (E05/E13 + A18) | **not_verified** — sem asset osx-x64 | **not_verified** — asset win64 não executado |
| G4 — UI e privacidade | **approved com limitações** — UI inglesa, tema, foco, nomes, teclado e logs locais; sem leitor de tela, janela nativa não dirigida, teclado Windows só por código (A18) | **not_verified** | **not_verified** |
| G5 — validação e proveniência | **approved** — matriz, logs, comandos, exits e limitações por ambiente; release bloqueado pelos gates abertos | **not_verified** | **not_verified** |

### Matriz de Debug (separada do núcleo)

| Plataforma | Asset da versão aprovada | Flag / opt-in | Prova DAP real | Decisão |
|---|---|---|---|---|
| macOS ARM64 | `netcoredbg-osx-arm64.zip` 3.2.0-1092, community supported; SHA-256 do digest oficial | Flag experimental **OFF por padrão**; opt-in de sessão explícito; ligar não instala nem prova DAP | Sim: E14 (launch, breakpoint, step, stop, async/multi-thread) + A18 (launch, breakpoint verificado, F8, locals, continue exit 0, stop com término) | **Experimental, flag mantida.** Não é suporte de release; #256 segue risco não reproduzido. A retirada da flag exigiria nova prova no build corrente; nenhuma remoção foi feita. |
| macOS Intel | Nenhum asset `osx-x64` na release 3.2.0-1092 | Sem substituição de versão; `unsupported_platform` | Não executado | **Indisponível/não verificado.** Debug bloqueado; não há promessa de cobertura universal Mac. Núcleo Intel também não foi executado. |
| Windows x64 | `netcoredbg-win64.zip` existe no metadata | Asset/instalação não executados | Não executado | **Gate aberto.** Exige smoke DAP próprio (launch, breakpoint, step, frames, locals, stop, processos) antes de qualquer aprovação Windows. |

Linux e outras arquiteturas Windows estão fora do P0; não há promessa.

### .NET: o que está comprovado

| Alegação | Estado | Evidência |
|---|---|---|
| Fixture `OrderDesk` net8.0 com console, biblioteca e testes xUnit | **comprovado** | E01/E03 + A18; TFM net8.0 nos três projetos |
| SDK do projeto 8.0.408 fixado por `global.json` com `rollForward: disable` | **comprovado** | E02/E03/E05 + L18-environment; nenhum arquivo reescrito |
| Host de análise SDK 10.0.100, separado do SDK do projeto | **comprovado** | E05/E06 + A18 (hostroot runtime 10, SDK 8 em `sdk/`) |
| Build/Run/Debug/VSTest no TFM net8.0 | **comprovado neste ambiente** | A18 (exit 1→0, Run exit 0, 4 resultados, DAP exit 0) |
| .NET 8+ em todos os TFMs/alvos | **não comprovado** | Só net8.0 foi exercitado; não herdar aprovação |
| net6 | **best effort, não verificado** | Nenhuma execução; sem garantia equivalente |
| Windows .NET / macOS Intel .NET | **não verificado** | Sem execução nesses ambientes |
| Versões mínimas de SO | **não declaradas** | Registradas apenas as versões reais desta máquina; sem mínimo inventado |
| Linux/outras arquiteturas | **fora do escopo** | Sem promessa |

### Comandos, contexto e exits observados

Contexto real: cópia temporária de `fixtures/OrderDesk` em diretório com espaço no nome; host
`openridr-host-http` em `127.0.0.1:4317`; Vite em `1420`; Chrome real; ferramentas reais.
Logs: `L18-acceptance-summary.json`, `L18-acceptance-pre-revoke.json`, `L18-acceptance-final.json`,
`L18-restore.json`, `L18-build-failed.json`, `L18-build-fixed.json`, `L18-run.json`,
`L18-tests-run-all.json`, `L18-tests-cancel.json`, `L18-terminal.json`, `L18-dap.json`,
`L18-debug-install.json`, `L18-lsp-session.json`, `L18-ui-audit.json`, `L18-playwright-*.log`.

| Operação | Comando/contexto | Resultado observado | Exit |
|---|---|---|---|
| Build frontend | `bun run build` | tsc + Vite; 62 módulos; aviso de chunk >500 kB | 0 |
| Host | `cargo test -p openridr-host` | 192 testes passaram, 0 falharam (somatório dos binários; doc-tests 0) | 0 |
| E2E completo | `bun run test:e2e` | 49 testes passaram | 0 |
| E2E aceitação | `bun run test:e2e tests/e2e/zz-acceptance.spec.ts` | 2 testes passaram | 0 |
| Tauri debug | `bun run tauri build --debug --no-bundle` | app em `src-tauri/target/debug/openridr` | 0 |
| Restore | `dotnet restore OrderDesk.sln -nologo -v:minimal` (argv separado) | três projetos `restored` | 0 |
| Build falho | `dotnet build OrderDesk.Cli.csproj -c Debug ... --no-restore` | CS0246, um problema navegável | 1 |
| Build corrigido | idem, fonte original salva | sucesso | 0 |
| Run | `dotnet <TargetPath>` do build Debug/net8.0 | stdout `Order A-104` | 0 |
| Testes selecionados | `dotnet test ... --filter FullyQualifiedName=...` | 1 passed | 0 |
| Testes todos | `dotnet test ...` (4 identidades) | 2 passed, 1 failed, 1 skipped | 1 (falha de asserção) |
| Teste longo cancelado | `dotnet test ... --filter ...Waits_for_cancellation` | `cancelled`, 0 results parciais, TRX missing; sem passed inventado | 143 observado |
| PTY | `/bin/zsh` no cwd do workspace; `pwd`, `echo`, `sleep 30` + Ctrl+C, `dotnet --version` 8.0.408 | shell/cwd reais; `^C`; prompt; `8.0.408` | shell `null` (unknown) após SIGTERM→SIGKILL; `killedAfterGrace:true` |
| DAP | `netcoredbg --interpreter=vscode`; launch do assembly Debug; breakpoint; F8; continue; stop | breakpoint verificado; pausa; locals; exit 0; stop com `termination` observada | debuggee 0; adapter 0 na primeira sessão; segunda sessão cancelada com término observado |

`exit:null` significa desconhecido; não foi convertido em 0. Falha de término continua visível.

### Limitações e lacunas que esta matriz não esconde

- **Windows x64**: nenhum núcleo, UI desktop, PTY, VSTest ou DAP executado; CI local não existe.
- **macOS Intel**: nenhum núcleo executado; sem asset osx-x64 do netcoredbg 3.2; sem substituição.
- **Janela Tauri nativa**: o fluxo foi dirigido no Chrome real contra Vite + host-http (mesmo
  `dispatch`); TCC bloqueou captura/automação da janela nativa no macOS. Smoke humano da janela
  nativa continua recomendado.
- **Acessibilidade**: auditoria heurística por Playwright (nomes, foco, contraste, overflow,
  strings), não WCAG formal; sem leitor de tela.
- **Analyzers/pull diagnostics**: `analyzerCoverage` permanece `not_verified`; só push diagnostics
  foi consumido; MTP é explicitamente não suportado.
- **Rede/segurança de suprimentos**: csharp-ls via NuGet com verificação de assinatura pulada pelo
  próprio `dotnet tool install`; netcoredbg usa SHA-256 do digest do GitHub, sem assinatura/
  notarização; o host não inventa hash/assinatura.
- **Sessões/contadores globais**: os testes por ticket assumem sessão/sequência inicial; o spec de
  aceitação roda por último (`zz-acceptance.spec.ts`) para não alterar essas premissas. Se a ordem
  for alterada, os testes de ticket precisam ser revistos.
- **Package/legal**: ver `docs/release-decision.md`; a auditoria é técnica, não parecer jurídico.
