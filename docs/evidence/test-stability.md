# Evidência — suite de testes confiável (issue #43)

Data: 2026-11-06. Máquina: macOS Apple Silicon (macOS 26), `cargo` stable, `dotnet` do PATH =
`/usr/local/share/dotnet` (SDKs 8.0.416 e 10.0.100, runtime 10 presente). O pin dos fixtures
(`8.0.408`, `rollForward: disable`) existe em `~/.dotnet` e em
`~/Library/Application Support/OpenRidr/sdks/8.0.408`. `~/.dotnet` **não** está no PATH.
Windows e macOS Intel não foram executados.

## Status por item da issue

| Item da issue #43 | Onde | Sintoma medido | Causa | Status |
|---|---|---|---|---|
| 3–4 testes de instalação do `crates/host/tests/debugger.rs` falham "mesmo com `--test-threads=1`" | `--test debugger` | Não reproduzido aqui: 14/14 verdes no HEAD `1921508` **e** no base `196e3b3`, com e sem `~/.dotnet` no PATH, com `--test-threads=1` e default. A sessão que relatou rodava em paralelo com outras (app aberto, `CARGO_TARGET_DIR` compartilhado, alpha.11 em `/Applications`) | Não determinada na origem; a classe "ambiente herdado decide o teste" foi reproduzida e fechada (linha abaixo) | **Não reproduzido** + classe fechada (#57, `55fbdd7`) |
| Adaptador embutido apontado pelo ambiente muda o resultado | `--test debugger` com `OPENRIDR_BUNDLED_NETCOREDBG=/tmp/fakebundle` | Antes: `4 passed; 10 failed` (`netcoredbg is bundled with OpenRidr; there is nothing to download or install`). Depois: `15 passed; 0 failed` | O harness não decidia o ambiente do host que cria | **Corrigido** (#57) |
| Testes lendo/escrevendo o ambiente do processo compartilhado | `--test host`, `--test debugger`, `--test bundled_analysis` | Ao converter, 3 testes que ainda escreviam env no processo compartilhado falharam de imediato (`tool_missing: the verified csharp-ls launcher disappeared`, probe desligado) por estado deixado por outro teste | O env do processo era escrito por vários testes e lido por todos, sem dono | **Corrigido** (#59, 2º re-check): `support::prepare()` define o ambiente uma única vez e **nenhum teste o escreve depois**; quem precisa de valores diferentes roda o corpo num processo-filho (`*_isolated` + `run_isolated_leaf`); teste-guard `env_writes_are_confined_to_isolated_leaves` lê os fontes e falha se voltar `set_var`/`remove_var` fora de folha isolada |
| O mesmo ambiente hostil no binário do host | `--test host` com `OPENRIDR_BUNDLED_NETCOREDBG`/`OPENRIDR_HOME`/`OPENRIDR_FORCE_HOSTROOT`/`OPENRIDR_TRUST_STORE` exportados | Antes: `96 passed; 2 failed` (`debug_start_gates_…`, `trust_grant_needs_an_open_workspace_…`). Depois: `100 passed; 0 failed` | Idem | **Corrigido** (#57) |
| Skip do launch profile virando passe silencioso (achado do re-check) | `--lib` | Skip só quando `sdk::resolve` não acha raiz para o pin, com versão e raízes na mensagem; muxer resolvido ausente ou que não inicia **falha** o teste; `OPENRIDR_REQUIRE_DOTNET=1` troca o skip por falha | Skip era aceito também para uma instalação detectada e quebrada | **Corrigido** (#62, re-check) |
| `run_configs::tests::launch_profile_runs_fixture_with_effective_environment_and_arguments` falha quando o muxer do PATH não tem o SDK do fixture | `--lib` | Antes: `A compatible .NET SDK was not found. Requested SDK version: 8.0.408` (`dotnet` do PATH = 8.0.416 + 10.0.100). Como era a única falha do binário `lib`, `cargo test -p openridr-host` abortava e **não rodava os outros seis binários** | O teste chamava o `dotnet` do PATH; Run/Debug de produção já usam o muxer do SDK resolvido (`project_muxer` + `dotnet_command`) | **Corrigido** (#62, `7a662dc`) — passa resolvendo 8.0.408 de `~/.dotnet`, com o PATH sem esse SDK |
| `keep_leaves…` "falha no run completo a menos que `OPENRIDR_ENABLE_PROBE=1` esteja exportado" | `--test host` | Mecanismo demonstrado (patch temporário não commitado que solta o binding do probe no meio do teste): `processes.probe is a development-only probe; set OPENRIDR_ENABLE_PROBE=1 to enable it.` — exatamente o sintoma relatado | O probe era ligado/desligado por escopo; o `Drop` de um teste restaurava "unset" | **Corrigido** (#59): o probe é definido uma vez por processo pelo `prepare()` e os testes de processo rodam em folha isolada |
| Restore/LSP do host falham "ocasionalmente" no run completo e passam isolados | `--test host` | Não reproduzido nas 6 rodadas da #43 (3 antes e 3 depois das mudanças); **reincidiu** em 2026-11-06 (sessão da #74): 1 de 3 runs completos, ver abaixo | Não determinada; o harness agora isola ambiente, home por processo e nomes de `/tmp` | **Não reproduzido na #43; reincidência aberta (2026-11-06)** |
| Debug E2E no fluxo legado `debug.install` (1/13) | `tests/e2e/debugger.spec.ts` | Não executado (decisão do Helder sobre Playwright pendente) | Spec escrito para o download do netcoredbg; o adaptador agora vem no bundle | **Quarentenado** (#67) |
| `csharp-ls.spec.ts:150`, `testsuite.spec.ts` (`tests-filter-count`), `tests.spec.ts:317`, `ui-layout` em 1440 px | `tests/e2e/*` | Não re-verificado nesta rodada | Não investigado | **Quarentenado com motivo** (#67) |
| `lsp_start_requires_tool_sdk_and_current_restore` falhou 1× no run completo e passou isolado | `--test host` | 1 falha em `cargo test -p openridr-host` na sessão da #113, com o pacote inteiro rodando em paralelo logo depois de um merge; isolado: `1 passed` em 22 s; `--test host` completo de novo: `100 passed` em 141 s | Não determinada; a mensagem da falha não foi preservada e o teste depende de `dotnet`/`csharp-ls` reais sob carga | **Observado 1×, não reproduzido** (2 execuções verdes em seguida) |
| `lsp_start_requires_tool_sdk_and_current_restore`, `lsp_loads_the_real_solution_republishes_diagnostics_and_stops_on_revoke` e `lsp_completion_and_definition_answer_from_the_real_session_and_reject_unknown_documents` falharam no run completo e passaram depois | `--test host` | 3 falhas em `cargo test -p openridr-host` na sessão da #101 (2026-11-07), com o bundle de debug do Tauri recém-construído e o app real rodando antes; `--test host` de novo, sozinho: `108 passed` em 284,78 s (o run verde anterior do mesmo binário levou 104 s, sinal de máquina carregada) | Carga: os três dependem de `csharp-ls`/`dotnet` reais e dos timeouts curtos do produto | **Ambiente/carga, não código** (mesma classe da linha acima; 1 execução verde em seguida) |
| 26 falhas simultâneas em `--test host` | `--test host` | Uma execução com `74 passed; 26 failed` na mesma sessão, logo antes de a worktree ser desmontada por uma limpeza externa (`target/` e `node_modules` apagados); nenhuma mensagem de falha foi preservada | Ambiente sendo removido durante a execução: os testes voltaram a `100 passed` na primeira execução depois do rebuild | **Ambiente, não código** (declarado, não é passe) |
| Os três testes que instalam/esperam o `csharp-ls` falham juntos no run completo e passam isolados | `--test host` (`tools_install_is_real_verified_and_reused`, `lsp_start_requires_tool_sdk_and_current_restore`, `lsp_completion_and_definition_…`) | `cargo test -p openridr-host` na sessão da #53: `105 passed; 3 failed` — os três com `state: failed` na instalação da ferramenta. Isolados: 88,6 s, 110,2 s e 119,3 s, todos verdes; no worktree do `main` o mesmo teste passou em 114,7 s | **Banda, não código**: cada teste baixa o mesmo `.nupkg` de 21,6 MB do nuget.org em paralelo (homes isolados, sem cache compartilhado) e o download do host tem prazo de 120 s (`tools.rs`); medido com `curl`: >120 s para o mesmo arquivo nesta rede. `tools.rs`/`http.rs`/`nuget.rs` estão idênticos ao `main` | **Ambiente/rede, declarado** (rodar isolado é verde; não é passe) |
| `build_start_needs_the_current_evaluation_and_accepts_an_explicit_library` falhou 1× no run completo e passou depois | `--test host` | 1 falha em `cargo test -p openridr-host` durante a sessão da #112, com o build do Tauri e o app real rodando em paralelo; isolado: ok; `--test host` completo de novo: `100 passed` em 171,77 s | Não determinada (a mensagem da falha não foi preservada) | **Observado 1×, não reproduzido** (2 execuções verdes em seguida) |
| `build_start_needs_the_current_evaluation…` e `initialize_failure_surfaces_the_sdk_message…` falharam 1× cada no run completo, em runs diferentes | `--test host` | Sessão da #161, com o `tauri dev` e um servidor local rodando em paralelo. `build_start…` devolveu `restore_required` em vez de `invalid_state` (a mesma assinatura da linha acima); `initialize_failure…` não preservou a mensagem. Isolados: 3/3 e 1/1 verdes; `--test host` completo de novo na máquina ociosa: `108 passed` em 80 s | Hipótese: carga — os dois dependem de `dotnet`/processo filho real e o run completo sobe muitos filhos isolados. Causa **não determinada** (uma das mensagens não foi preservada) | **Observado 1× cada, não reproduzidos** (registrado, não é passe) |
| Os três testes de install real (`tools_install_is_real_verified_and_reused`, `lsp_start_requires_tool_sdk_and_current_restore`, `lsp_loads_the_real_solution_republishes_diagnostics_and_stops_on_revoke`) falharam aos ~120 s | `--test host` | 3 falhas em `cargo test -p openridr-host` na sessão da #93 (2026-11-07); `curl` do mesmo pacote mediu **151,19 s** para 21,6 MB; com o `tools.rs` revertido para o `origin/main` a falha é idêntica (120,34 s); quando o download terminou em **108,79 s** o teste passou | Rede lenta: `prepare_local_package` baixa o `.nupkg` com prazo de 120 s e o download não cabe nele | **Ambiente/rede, não código** (medido dos dois lados); o prazo total de 120 s saiu na #161 |
| `run_start_reports_restore_required_when_assets_are_invalidated_after_a_build` falhou 1× no run completo e passou isolado | `--test host` | 1 falha em `cargo test -p openridr-host` na sessão da #55 (2026-11-07), com o build de release do Tauri e o app real rodando em paralelo; isolado: `1 passed`; o run completo seguinte teve 107 verdes e só essa falha | Não determinada; o teste depende de `dotnet`/restore reais sob carga (mesma classe das linhas acima) | **Observado 1×, não reproduzido** (isolado verde; nenhuma mudança da frente toca o host) |
| `real_run_reports_trx_outcomes_positions_and_only_the_selected_identity` falhou 1× no run completo e passou isolado 2× | `--test test_run` | 1 falha em `cargo test -p openridr-host` na sessão da #55 (2026-11-07), no mesmo run em que `--test host` fez 108 verdes; a asserção esperava `tests/OrderDesk.Tests/OrderTests.cs` e recebeu `Null`; isolado: `1 passed` em 5,94 s e de novo em 7,16 s | Não determinada; o teste roda `dotnet test` real e depende do TRX sob carga (mesma classe das linhas acima) | **Observado 1×, não reproduzido** (isolado verde 2×; nenhuma mudança da frente toca `crates/`) |

**Reincidência da classe "restore/LSP falham no run completo" (2026-11-06, sessão da issue #74, branch
`ci/issue-74-risk-tiers`).** O run completo de `cargo test -p openridr-host` falhou 1 de 3 vezes em
`lsp_loads_the_real_solution_republishes_diagnostics_and_stops_on_revoke`: a folha isolada parou em
`ensure_tool` — `tools.install` terminou em `failed` com **log vazio**
(`assert_eq!(done["state"], "ready", "{}", done["log"])` → `left: "failed"`, `right: "ready"`), 120 s de
espera. O mesmo teste rodado sozinho em seguida passa (75,6 s) e os outros dois runs completos ficaram
verdes. O diff do branch não toca `crates/` (é CI + docs), então não há evidência de relação com ele; a
hipótese é a instalação do csharp-ls (rede/NuGet) falhando sem registrar motivo. Não corrigido nesta
frente: registrado aqui, não mascarado com skip.

## Medições

### Antes das mudanças (3 rodadas, HEAD `1921508`)

`debugger` 14/14, `host` 98/98, `bundled_analysis` 11/11, `test_discovery` 10/10, `test_run` 7/7,
`trust_model` 1/1. O binário `lib` só podia ser medido com `--skip launch_profile_runs_fixture`
(285 verdes); com o teste, `FAILED. 285 passed; 1 failed` e o `cargo test` parava ali
(`error: test failed ... --lib`), sem rodar os binários seguintes.

| Rodada | lib | debugger | host | bundled | discovery | run | trust | tempo do host |
|---|---|---|---|---|---|---|---|---|
| 1 | 285 ok (1 skip) | 14 ok | 98 ok | 11 ok | 10 ok | 7 ok | 1 ok | 63,10 s |
| 2 | 285 ok (1 skip) | 14 ok | 98 ok | 11 ok | 10 ok | 7 ok | 1 ok | ~64 s |
| 3 | 285 ok (1 skip) | 14 ok | 98 ok | 11 ok | 10 ok | 7 ok | 1 ok | 64,23 s |

### Depois das mudanças (3 rodadas, paralelismo default, sem variável de ambiente especial)

`cargo test -p openridr-host` completo, 431 testes por rodada:

| Rodada | lib | debugger | host | bundled | discovery | run | trust | doc-tests | total |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 287 ok | 15 ok | 100 ok | 11 ok | 10 ok | 7 ok | 1 ok | 0 | 431, 146 s |
| 2 | 287 ok | 15 ok | 100 ok | 11 ok | 10 ok | 7 ok | 1 ok | 0 | 431, 124 s |
| 3 | 287 ok | 15 ok | 100 ok | 11 ok | 10 ok | 7 ok | 1 ok | 0 | 431, 115 s |

Tempos por binário nas três rodadas (segundos, ordem lib / bundled_analysis / debugger / host /
test_discovery / test_run / trust_model):

| Rodada | lib | bundled | debugger | host | discovery | run | trust | total |
|---|---|---|---|---|---|---|---|---|
| 1 | 2,44 | 1,20 | 5,42 | 89,86 | 6,81 | 25,27 | 2,73 | 146 |
| 2 | 2,44 | 1,32 | 5,64 | 80,23 | 5,14 | 24,44 | 2,97 | 124 |
| 3 | 2,25 | 1,36 | 5,65 | 72,24 | 5,50 | 24,87 | 2,80 | 115 |

### Modelo atual (3º re-check do #59): processo compartilhado sem escrita de env, lib incluída

O ambiente comum é definido uma vez por binário (`support::prepare()` na integração,
`test_harness::prepare()` na lib). Testes que precisam de outros valores rodam o corpo numa folha
`#[ignore]` (`*_isolated`) executada por um processo-filho com ambiente controlado
(`env_clear` + `PATH`/`HOME`/`TMPDIR`/`LANG`/`LC_ALL`). Folhas: 27 no binário do host, 14 no do
debugger, 11 no da análise embutida, 11 na lib e 1 no `trust_model`.

O guard `env_writes_are_confined_to_isolated_leaves` varre `crates/host/src/**` e
`crates/host/tests/**` (recursivo) e falha quando: `set_var`/`remove_var`/`EnvVar::` aparece fora de
folha isolada ou do módulo de harness; uma folha `*_isolated` não é `#[ignore]`; uma folha não tem
wrapper; ou um teste não isolado toca no host sem `prepare`/`host` antes. Ele já pegou, nesta rodada,
os helpers `with_home` de `trusted`/`recent`, o `ShellEnv` do PTY, o `Seam` do debugger e dois testes
que liam `app_home()` antes do `prepare`.

Cada folha `*_isolated` aparece como ignorada no processo compartilhado e é executada pelo wrapper
num processo-filho. 3 rodadas seguidas, paralelismo default:

| Rodada | lib | bundled (exec/ign) | debugger (exec/ign) | host (exec/ign) | discovery | run | trust | total do host |
|---|---|---|---|---|---|---|---|---|
| 1 | 287 ok / 11 ign | 11 ok / 11 ign | 15 ok / 14 ign | 100 ok / 27 ign | 10 ok | 7 ok | 1 ok / 1 ign | 131,77 s |
| 2 | 287 ok / 11 ign | 11 ok / 11 ign | 15 ok / 14 ign | 100 ok / 27 ign | 10 ok | 7 ok | 1 ok / 1 ign | 96,77 s |
| 3 | 287 ok / 11 ign | 11 ok / 11 ign | 15 ok / 14 ign | 100 ok / 27 ign | 10 ok | 7 ok | 1 ok / 1 ign | 116,30 s |

Duas falhas apareceram nas rodadas de validação, ambas de **carga** e ambas fechadas:

- **`lsp_completion_and_definition…`**: `csharp-ls did not answer within 5 seconds` (timeout de
  produto) com 27 filhos isolados disputando dotnet. Conserto: **1 processo-filho por vez** por
  binário (`PARALLEL_CHILDREN = 1`), com semáforo no harness; isolamento intacto, só a largura muda.
  Com 2 filhos a falha ainda apareceu (1 em 3 rodadas); com 1, não reapareceu em 3 rodadas.
- **`build_start_needs_the_current_evaluation…` / `run_start_reports_restore_required…`**:
  `restore_required` em vez de `invalid_state`
  porque a cópia do fixture mantinha mtimes do checkout e a comparação
  `project.assets.json < projeto` (ms) podia empatar sob carga. Conserto: `fixture_copy` normaliza
  todos os mtimes para 2020 (`age_tree`), então o que o restore escreve é sempre mais novo.

**Depois do merge de `origin/main`** (commit `987ef23`, que trouxe #44/#45/#79/#92): `bun run check`
verde; `cargo test -p openridr-host` com lib 298 (+11 ignoradas), bundled 11 (+11), debugger 15
(+14), host 100 (+27), discovery 10, run 7 e trust 1 (+1), tudo verde; smoke E2E do
`scripts/e2e-smoke.txt` com 25 passando em 1,8 min, headless. O guard de fontes continuou verde com
os fontes novos do #79/#92.

Antes desses consertos: 1 rodada com 1 falha (`test_run`, `real_run_reports_trx…`, passou
isolado), 1 rodada com 2 falhas (as duas acima), 2 rodadas verdes. Depois: **3 rodadas seguidas
verdes**, acima. Essas duas falhas são sensíveis a carga (timeout de 5 s do produto para o LSP e
restore real do NuGet); com a máquina carregada por outras sessões elas podem voltar, e a tabela
acima é a medição honesta desta rodada — não um conserto declarado. Com `OPENRIDR_BUNDLED_NETCOREDBG`, `OPENRIDR_HOME`, `OPENRIDR_FORCE_HOSTROOT`,
`OPENRIDR_TRUST_STORE` e `OPENRIDR_ENABLE_PROBE=0` exportados: `--test debugger` 15 ok + 14 ign,
`--test bundled_analysis` 11 ok + 11 ign — o ambiente herdado não chega ao host.

### Falha-antes/passa-depois por ticket

| #59 | testes que escreviam env no processo compartilhado (3 casos) ao rodarem juntos no mesmo binário | `FAILED` (`tool_missing`, probe desligado) | corpo em `*_isolated` + wrapper: verdes; guard de fontes verde |
| #59 | `env_writes_are_confined_to_isolated_leaves` (teste-guard) | — | passa; falharia se um `set_var` voltasse para um teste compartilhado |
| Ticket | Comando | Antes | Depois |
|---|---|---|---|
| #57 | `OPENRIDR_BUNDLED_NETCOREDBG=/tmp/fakebundle cargo test -p openridr-host --test debugger` | `4 passed; 10 failed` | `15 passed; 0 failed` |
| #57 | idem, `--test host` (+ `OPENRIDR_HOME`, `OPENRIDR_FORCE_HOSTROOT`, `OPENRIDR_TRUST_STORE`) | `96 passed; 2 failed` | `100 passed; 0 failed` |
| #57 | novo teste `an_ambient_bundled_override_never_reaches_the_host` no base `196e3b3` (`--exact trust_grant_needs_an_open_workspace_and_gates_every_direct_call` com o override) | `FAILED` (`debug.install must fail without starting a process: left "invalid_state", right "confirmation_required"`) | verde |
| #57 | novo teste `an_ambient_bundled_override_never_reaches_the_controlled_installer` no base | `FAILED` (`left "invalid_state", right "confirmation_required"`) | verde |
| #62 | `cargo test -p openridr-host --lib launch_profile` | `A compatible .NET SDK was not found. Requested SDK version: 8.0.408` (binário `lib` inteiro abortado) | `287 passed`, com `launch profile fixture ran on SDK 8.0.408 from /Users/helder/.dotnet` |

## O que não foi verificado

- **E2E/Playwright:** decisão do Helder (#67): só smoke noturno. Conjunto de smoke entregue em
  `scripts/e2e-smoke.txt` (4 specs, 25 testes, verde em 3 rodadas de ~2,8 min / ~1,6 min) e a
  quarentena com motivos em `docs/evidence/e2e-quarantine.md` — `debugger.spec.ts` medido (12 de 13
  falhando) e os outros quatro relatados, não re-verificados. Nenhuma execução de UI no app real
  (sessão usa `agent-desktop`, não Playwright).
- **E2E/Playwright (histórico desta rodada):** nenhuma execução, por decisão explícita do orquestrador enquanto
  o Helder decide manter/migrar/quarentenar. Os specs relatados ficam quarentenados (#67) e o
  bootstrap da suíte continua dependendo de rede (`curl` do csharp-ls no nuget.org + `dotnet tool
  install`) e de ~14,5 min com `workers=1`.
- **Falha original dos testes de instalação do debugger:** não reproduzida aqui, em nenhum dos dois
  commits. A hipótese levada à tabela é interferência entre processos (outra sessão com app aberto,
  `CARGO_TARGET_DIR` compartilhado, app instalado em `/Applications`); o ambiente/estado exato da
  sessão que relatou não foi preservado. O que foi provado é que a classe "ambiente herdado decide o
  teste" existia e deixou de existir.
- **Windows e macOS Intel.**
- **`verify-openridr`:** não aplicável nesta rodada — nenhuma mudança de UI, host-http, Tauri ou
  contrato de `data-testid`; a prova é a suíte medida acima. Nenhuma janela do app foi aberta.
- **Re-check do `/code-review` (2026-11-06):** os cinco achados foram corrigidos — coordenação
  leitor/escritor do ambiente (`RwLock` + host que segura o lado compartilhado), o teste de corrida
  com barreira (não tautológico: falha 3/3 sem o guard), o skip do launch profile, o alvo de escape
  em diretório temporário aleatório (não um nome previsível por pid) e a execução do teste-filho
  exigindo exatamente 1 teste executado.
- **`tools::neutral_cwd()`** usa `openridr-neutral` fixo em `std::env::temp_dir()`: inspecionado,
  serve só de diretório de trabalho para o `--version` do csharp-ls embutido e nunca é escrito;
  não foi alterado (código de produto, fora do escopo do harness).

## Arquivos tocados

- `crates/host/tests/support/mod.rs` (novo): harness compartilhado (purga de ambiente, `EnvVar`,
  `env_guard`, `enable_probe`, `host`).
- `crates/host/tests/{host,debugger,bundled_analysis,test_discovery,test_run,trust_model}.rs`.
- `crates/host/src/run_configs.rs` (teste de launch profile + resolução do SDK).
- `docs/SPEC-TEST-STABILITY.md`, `docs/decisions.md`, `docs/implementation-notes.md`,
  `docs/README.md`, `AGENTS.md`, este arquivo.
