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
| Host lendo `OPENRIDR_HOME` enquanto outro teste escreve sob guard | `--test host` | Antes: só o escritor era serializado; o host de um teste sem guard podia ler a home de outro teste no meio do corpo | Falta de coordenação do lado leitor | **Corrigido** (#59, re-check): o lock virou `RwLock`, o host de teste segura o lado compartilhado durante criação *e* uso, e o escritor toma o lado exclusivo |
| O mesmo ambiente hostil no binário do host | `--test host` com `OPENRIDR_BUNDLED_NETCOREDBG`/`OPENRIDR_HOME`/`OPENRIDR_FORCE_HOSTROOT`/`OPENRIDR_TRUST_STORE` exportados | Antes: `96 passed; 2 failed` (`debug_start_gates_…`, `trust_grant_needs_an_open_workspace_…`). Depois: `100 passed; 0 failed` | Idem | **Corrigido** (#57) |
| Skip do launch profile virando passe silencioso (achado do re-check) | `--lib` | Skip só quando `sdk::resolve` não acha raiz para o pin, com versão e raízes na mensagem; muxer resolvido ausente ou que não inicia **falha** o teste; `OPENRIDR_REQUIRE_DOTNET=1` troca o skip por falha | Skip era aceito também para uma instalação detectada e quebrada | **Corrigido** (#62, re-check) |
| `run_configs::tests::launch_profile_runs_fixture_with_effective_environment_and_arguments` falha quando o muxer do PATH não tem o SDK do fixture | `--lib` | Antes: `A compatible .NET SDK was not found. Requested SDK version: 8.0.408` (`dotnet` do PATH = 8.0.416 + 10.0.100). Como era a única falha do binário `lib`, `cargo test -p openridr-host` abortava e **não rodava os outros seis binários** | O teste chamava o `dotnet` do PATH; Run/Debug de produção já usam o muxer do SDK resolvido (`project_muxer` + `dotnet_command`) | **Corrigido** (#62, `7a662dc`) — passa resolvendo 8.0.408 de `~/.dotnet`, com o PATH sem esse SDK |
| `keep_leaves…` "falha no run completo a menos que `OPENRIDR_ENABLE_PROBE=1` esteja exportado" | `--test host` | Mecanismo demonstrado (patch temporário não commitado que solta o binding do probe no meio do teste): `processes.probe is a development-only probe; set OPENRIDR_ENABLE_PROBE=1 to enable it.` — exatamente o sintoma relatado | O probe era ligado/desligado por escopo; o `Drop` de um teste restaurava "unset" | **Corrigido** (#59, `24ca412`): probe é uma decisão do binário e os testes de processo tomam o guard |
| Restore/LSP do host falham "ocasionalmente" no run completo e passam isolados | `--test host` | Não reproduzido: 6 rodadas completas do pacote (3 antes e 3 depois das mudanças) sem nenhuma falha em `--test host` | Não determinada; o harness agora isola ambiente, home por processo e nomes de `/tmp` | **Não reproduzido** (classe fechada por #57) |
| Debug E2E no fluxo legado `debug.install` (1/13) | `tests/e2e/debugger.spec.ts` | Não executado (decisão do Helder sobre Playwright pendente) | Spec escrito para o download do netcoredbg; o adaptador agora vem no bundle | **Quarentenado** (#67) |
| `csharp-ls.spec.ts:150`, `testsuite.spec.ts` (`tests-filter-count`), `tests.spec.ts:317`, `ui-layout` em 1440 px | `tests/e2e/*` | Não executado | Não investigado (E2E fora de escopo nesta rodada) | **Quarentenado** (#67) |

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

### Falha-antes/passa-depois por ticket

| Ticket | Comando | Antes | Depois |
|---|---|---|---|
| #57 | `OPENRIDR_BUNDLED_NETCOREDBG=/tmp/fakebundle cargo test -p openridr-host --test debugger` | `4 passed; 10 failed` | `15 passed; 0 failed` |
| #57 | idem, `--test host` (+ `OPENRIDR_HOME`, `OPENRIDR_FORCE_HOSTROOT`, `OPENRIDR_TRUST_STORE`) | `96 passed; 2 failed` | `100 passed; 0 failed` |
| #57 | novo teste `an_ambient_bundled_override_never_reaches_the_host` no base `196e3b3` (`--exact trust_grant_needs_an_open_workspace_and_gates_every_direct_call` com o override) | `FAILED` (`debug.install must fail without starting a process: left "invalid_state", right "confirmation_required"`) | verde |
| #57 | novo teste `an_ambient_bundled_override_never_reaches_the_controlled_installer` no base | `FAILED` (`left "invalid_state", right "confirmation_required"`) | verde |
| #59 | `a_concurrent_env_scope_cannot_disable_the_probe_for_another_test` com o `env_guard()` do lado escritor removido (patch temporário, 3 execuções) | `FAILED` nas 3, sempre com `processes.probe is a development-only probe; set OPENRIDR_ENABLE_PROBE=1 to enable it.` | com o guard: `1 passed` em 0,18–0,19 s; o escritor segura o ambiente e a leitura do host espera por ele |
| #59 | patch temporário soltando o binding do probe no `keep_leaves…` (padrão antigo por escopo) | mesma mensagem de probe desligado | `support::enable_probe()`: o switch não é restaurado por escopo; 100/100 |
| #62 | `cargo test -p openridr-host --lib launch_profile` | `A compatible .NET SDK was not found. Requested SDK version: 8.0.408` (binário `lib` inteiro abortado) | `287 passed`, com `launch profile fixture ran on SDK 8.0.408 from /Users/helder/.dotnet` |

## O que não foi verificado

- **E2E/Playwright:** nenhuma execução nesta rodada, por decisão explícita do orquestrador enquanto
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
