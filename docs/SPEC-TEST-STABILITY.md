# SPEC — Suite de testes confiável (issue #43)

Status: ready-for-agent
Fonte: issue GitHub #43 (Heldinhow/openridr) + grill de 2026-11-06 com o orquestrador (Q1–Q6).
Tickets: issues #57 (harness hermético), #59 (guard de env, bloqueada por #57), #62 (launch profile pelo SDK do projeto), #64 (medição final, bloqueada por #57/#59/#62) e #67 (quarentena do E2E, bloqueada pela decisão do Helder sobre Playwright).
Alvo: macOS Apple Silicon. Windows e macOS Intel continuam fora do que foi executado.

## Problem Statement

Quem abre um PR no OpenRidr não consegue distinguir regressão de ruído: o relatório de testes carrega "failed, but also fails on main". `cargo test -p openridr-host` aborta no primeiro binário que falha (hoje o binário `lib`, em um teste que chama o `dotnet` do PATH), então uma falha esconde todos os binários seguintes. Testes do host dependem de variáveis de ambiente herdadas do shell de quem roda (um `OPENRIDR_BUNDLED_NETCOREDBG` exportado derruba 10 testes de instalação do adaptador), e testes que mexem em env de processo correm entre si. A suíte E2E acumula specs escritos para fluxos que o produto já não tem (download do netcoredbg antes do bundle), e o `AGENTS.md` lista um conjunto de "falhas conhecidas" que não bate com a realidade medida.

## Solution

Tornar o resultado de `cargo test -p openridr-host` reprodutível: a mesma árvore, na mesma máquina, dá verde 3 vezes seguidas com o paralelismo default, sem variável de ambiente especial e sem `--test-threads=1`.

1. O teste que lança o fixture por launch profile passa a usar o muxer do SDK resolvido do projeto — o mesmo caminho da produção — em vez do `dotnet` do PATH; quando nenhuma raiz detectada satisfizer o pin do `global.json`, ele faz skip dizendo isso, e skip não conta como passe.
2. O harness limpa as variáveis de ambiente que decide sozinho (`OPENRIDR_HOME`, `OPENRIDR_BUNDLED_*`, `OPENRIDR_FORCE_HOSTROOT`, `OPENRIDR_TRUST_STORE`, `OPENRIDR_SESSION_STORE`, `OPENRIDR_DEBUG_*`) antes de abrir um host, de modo que um shell contaminado não mude o resultado.
3. Os testes que mutam env de processo no mesmo binário compartilham um único guard, no padrão que `tests/debugger.rs` já usa; uma corrida entre dois testes deixa de existir, em vez de aparecer só no run completo.
4. O que a issue lista como falha e não reproduz no HEAD vira linha nominal de evidência com "não reproduzido nesta máquina", não conserto inventado.
5. O E2E fica **quarentenado com motivo** e explicitamente bloqueado pela decisão do Helder sobre Playwright: sem poder rodar a suíte, conserto de E2E não tem falha-antes/passa-depois e não entra aqui.

## User Stories

1. Como mantenedor abrindo um PR, quero que `cargo test -p openridr-host` rode todos os binários e diga de uma vez o que está quebrado, para não descobrir falha em cascata.
2. Como mantenedor, quero rodar `cargo test -p openridr-host` três vezes seguidas e ver verde, para confiar no relatório do PR.
3. Como mantenedor em outra máquina, quero que o resultado não dependa de variáveis `OPENRIDR_*` exportadas no meu shell, para não acusar o colega de regressão.
4. Como mantenedor, quero que um teste que depende de ferramenta externa ausente faça skip com o motivo escrito, e não passe silenciosamente.
5. Como mantenedor com o SDK 8.0.408 só em `~/Library/Application Support/OpenRidr/sdks`, quero que o teste de launch profile rode, para não pular cobertura por causa do `dotnet` do PATH.
6. Como mantenedor, quero que o teste de launch profile exercite o mesmo caminho de SDK que o Run usa, para que ele não prove algo diferente do produto.
7. Como mantenedor, quero que os testes de processo (`keep_leaves`, `switching_workspaces`, `revoke_*`) não disputem a variável de probe do probe de processo, para não falharem por ordem de execução.
8. Como mantenedor, quero conseguir reproduzir a corrida de env de propósito (com uma janela alargada), para provar que o guard é o que conserta.
9. Como mantenedor, quero uma tabela única de "teste → sintoma → causa → status → ambiente", para responder "é o meu PR?" numa consulta só.
10. Como mantenedor, quero que o `AGENTS.md` liste exatamente as falhas conhecidas medidas, para ele parar de ser uma lista de rumores.
11. Como mantenedor, quero que os specs E2E que testam o fluxo antigo de `debug.install` fiquem marcados como quarentena com o motivo (netcoredbg agora é embutido), para ninguém gastar um dia investigando um spec obsoleto.
12. Como mantenedor, quero que a quarentena do E2E esteja explicitamente bloqueada por uma decisão do Helder, para não ser retomada por engano como trabalho pronto.
13. Como mantenedor, quero que as falhas relatadas na issue que não reproduzem no HEAD fiquem registradas como tal, com o comando usado, para que a próxima sessão não recomece do zero.
14. Como mantenedor, quero que a spec, os tickets e a evidência usem o mesmo vocabulário do repositório, para a tabela de `docs/README.md` continuar coerente.
15. Como mantenedor de uma sessão futura, quero uma decisão em `docs/decisions.md` explicando por que o harness limpa env e por que a quarentena E2E existe, para não desfazer por engano.

## Implementation Decisions

- **Escopo Rust agora; E2E quarentenado.** A decisão de manter/migrar/quarentenar o Playwright está pendente com o Helder. Nesta rodada: `cargo test -p openridr-host` confiável, e os specs E2E relatados entram em tabela nominal de quarentena com motivo e bloqueio explícito. Nenhuma execução de E2E é feita nesta rodada.
- **Harness hermético, não produto.** O host continua lendo `OPENRIDR_*` do ambiente (é contrato de dev/E2E). O que muda é o harness de teste: cada fixture de teste decide explicitamente quais variáveis valem para o host que ela cria, e limpa as demais. A limpeza fica num único ponto do harness, não espalhada por teste.
- **Launch profile espelha a produção.** O teste de integração do launch profile resolve o SDK do projeto pelo mesmo mecanismo do host (inventário de raízes + `global.json`) e lança o fixture pelo muxer da raiz resolvida, com `DOTNET_ROOT` apontando para ela. Skip só quando nenhuma raiz detectada satisfizer o pin, com mensagem nomeando a versão pedida e as raízes vistas (`OPENRIDR_REQUIRE_DOTNET=1` troca o skip por falha); muxer resolvido que não existe ou não inicia é falha. `dotnet` do PATH deixa de ser pré-requisito do teste.
- **Ambiente do processo imutável; env diferente só em processo-filho.** `support::prepare()` define uma vez por binário, antes do primeiro teste, a home temporária, o probe ligado e a purga das `OPENRIDR_*` herdadas. Nenhum teste escreve env depois disso: um teste que precisa de valores diferentes roda o corpo numa folha `#[ignore]` executada por um processo-filho dedicado (`support::run_isolated_leaf`), e um teste-guard lê os fontes e falha se aparecer `set_var`/`remove_var` fora de uma folha isolada.
- **Quarentena é `#[ignore]` com motivo, ou linha de tabela.** Teste Rust que precise ficar fora entra com `#[ignore = "<motivo>"]` e aparece na tabela; spec E2E entra só na tabela, apontando o motivo e a decisão pendente. Nada é apagado para "deixar verde".
- **Medição antes de conserto.** Falha relatada que não reproduz em 3–5 rodadas completas é registrada como "não reproduzida nesta máquina", com comando, ambiente e data; não se infla timeout nem se marca teste como lento sem causa observada.
- **Fonte única de falhas conhecidas.** `docs/evidence/test-stability.md` é a tabela normativa (teste, sintoma, causa, status, ambiente); a seção "Falhas conhecidas" do `AGENTS.md` passa a ser um resumo que aponta para ela.
- **Sem dependência nova, sem mudança de contrato de UI.** Nenhum `data-testid`, nenhum comando de host e nenhuma dependência mudam.

## Testing Decisions

- **Bom teste aqui é o que falha por comportamento observável no seam do host**, não por detalhe interno: os binários `tests/host.rs` e `tests/debugger.rs` chamam `Host::dispatch` e leem o snapshot; o teste de launch profile chama `run_configs::launch_options` e executa o processo real. Nenhum teste novo deve espionar estrutura interna.
- **Seams usados (existentes, nenhum novo):**
  - fronteira `Host::dispatch` / snapshot — onde vive a maior parte da suíte do host e do debugger;
  - `run_configs::launch_options` + processo real do fixture — para o caso do `global.json`;
  - fronteira HTTP de dev do host + Playwright — existe, mas fica fora desta rodada por decisão pendente.
- **Falha-antes/passa-depois obrigatório** por ticket:
  - env herdado: rodar o binário do debugger com `OPENRIDR_BUNDLED_NETCOREDBG` exportado (hoje derruba 10 testes; depois, verde);
  - corrida de env: os testes que escreviam ambiente no processo compartilhado falham ali (`tool_missing`, probe desligado) e passam com o corpo isolado; o teste-guard impede a reintrodução;
  - launch profile: com o `dotnet` do PATH sem o SDK 8.0.408 (falha hoje, passa depois).
- **Bateria final:** `cargo test -p openridr-host` completo (os 7 binários), 3 rodadas seguidas, paralelismo default, sem variável de ambiente especial. Os números entram em `docs/evidence/test-stability.md`.
- **Prior art:** `tests/debugger.rs` (guard de env tolerante a envenenamento) e `tests/host.rs` (`EnvVar`, `wait_for_*`) são o padrão; não se cria harness paralelo.

## Out of Scope

- E2E/Playwright: execução, conserto e migração. Ficam quarentenados com motivo até o Helder decidir.
- `docs/DESIGN.md` e `docs/PRODUCT.md`.
- Mudar produto para acomodar teste (o host continua lendo env; a produção continua usando o muxer do SDK do projeto).
- Windows e macOS Intel.
- Inflar timeouts ou marcar testes como lentos sem causa medida.
- Mandar PR/push (o orquestrador faz).

## Further Notes

- Medições desta sessão (2026-11-06, macOS ARM64, `dotnet` do PATH = 8.0.416 + 10.0.100, pin 8.0.408 só em `~/Library/Application Support/OpenRidr/sdks`):
  - `--test debugger`: 14/14 verdes; `--test host`: 98/98 verdes (63–70 s); `bundled_analysis` 11, `test_discovery` 10, `test_run` 7, `trust_model` 1; binário `lib`: 285 verdes + a falha determinística do launch profile.
  - `OPENRIDR_BUNDLED_NETCOREDBG` herdado: 4 passam, 10 falham no binário do debugger — a dependência de ambiente é real e reproduzível.
  - Os testes de instalação do debugger que a issue cita não reproduzem no HEAD `1921508` nem no base citado (`196e3b3`), com `~/.dotnet` no PATH e `--test-threads=1`; entram na tabela como "não reproduzido" até o ambiente da sessão que relatou ser caracterizado.
- O bug de produto que a issue insinua ("run usa dotnet do PATH") não existe: `run.start`/`debug.start` já usam o muxer do SDK resolvido. O que estava errado era só o teste.
- Tickets vivem como issues do GitHub (decisão do Helder de 2026-11-06), não em `docs/tickets-<tema>/`; esta spec, o RETRO e a evidência continuam em `docs/`.
