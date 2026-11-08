# SPEC — o build explícito nunca corre com a automação do app

Issue: [#275](https://github.com/Heldinhow/openridr/issues/275). Origem medida na
[#251](https://github.com/Heldinhow/openridr/issues/251) — evidência em
[evidence/e2e-smoke-solo.md](evidence/e2e-smoke-solo.md), retro em
[RETRO-E2E-SMOKE-SOLO.md](RETRO-E2E-SMOKE-SOLO.md).

## Problem Statement

Abrir uma solução e clicar Build logo em seguida pode terminar em `Build failed · exit 1` com
`error MSB4018: The "GenerateDepsFile" task failed unexpectedly` e
`System.IO.IOException: The process cannot access the file '.../OrderDesk.Core.deps.json' because it
is being used by another process`. O stack trace não tem relação nenhuma com o código do usuário: quem
está escrevendo o mesmo arquivo é a **automação do próprio app**, que o host inicia sozinho depois do
trust.

O que corre com o build explícito, medido na #251 (run
[42411789835](https://github.com/Heldinhow/openridr/actions/runs/42411789835)):

- a **descoberta automática de testes**, que roda `dotnet test <tests>.csproj --no-restore
  --list-tests` — sem `--no-build`, então é um build real do projeto de teste e das referências,
  incluindo o `OrderDesk.Core` que o build do usuário também constrói;
- a **análise** (csharp-ls), cujo `BuildHost` do MSBuild carrega a solução e escreve em `obj/` e `bin/`
  dos mesmos projetos.

Hoje o host não sabe que essas duas coisas existem: `build.start` recusa build concorrente e restore
concorrente, mas não a descoberta nem a análise. O usuário paga o erro no lugar da automação, e o
`build.spec.ts` paga o mesmo preço de forma intermitente (2 de 5 runs sozinho na runner, antes do
`settleDiscovery` da #257).

## Solution

Um contrato só: **o host nunca roda duas construções do mesmo projeto ao mesmo tempo**. Quem já está
construindo é nomeado, e o build explícito recusa com um motivo acionável em vez de correr e perder o
arquivo.

1. Enquanto a descoberta automática de testes roda, `build.start`, `build.solution` e o build implícito
   de Run/Debug recusam com `invalid_state` e o motivo: a descoberta está construindo o projeto de
   teste; espere ou cancele a descoberta.
2. Enquanto a análise está **carregando** a solução, o build recusa com o motivo e a saída "Stop
   analysis" (o botão já existe). Depois de carregada, a análise não bloqueia mais.
3. Simetria: enquanto um build roda, `tests.discover` recusa com o motivo — senão o usuário inicia um
   `dotnet test` que constrói durante o build, a mesma colisão na direção oposta.
4. A UI mostra o motivo **antes** do clique: os botões de Build/Build Solution/Run/Debug ficam
   desabilitados com o motivo no `title` e no texto de motivo do painel, e voltam sozinhos quando a
   automação termina (o snapshot manda). Nada de stack trace do MSBuild.

## User Stories

1. Como usuário, quero clicar Build logo depois de abrir a solução e receber o build do meu projeto,
   para não ver um stack trace do MSBuild sem relação com o meu código.
2. Como usuário, quero que o botão de Build me diga que a descoberta de testes está construindo o
   projeto, para entender por que ele está desabilitado em vez de achar que o app travou.
3. Como usuário, quero que o motivo ofereça a saída (cancelar a descoberta, parar a análise), para não
   ficar esperando sem saber quanto tempo falta.
4. Como usuário, quero que o botão volte a ficar habilitado sozinho quando a automação termina, para não
   precisar reabrir o painel ou reavaliar a solução.
5. Como usuário, quero que o Build Solution e o build implícito de Run/Debug respeitem o mesmo contrato,
   para a colisão não voltar por outra porta.
6. Como usuário, quero que o Run/Debug me diga o mesmo motivo, para não descobrir a recusa só no clique.
7. Como usuário, quero que a descoberta de testes recuse enquanto um build roda, para não iniciar um
   `dotnet test` que constrói no meio do meu build.
8. Como usuário, quero que a análise bloqueie o build só enquanto ela carrega a solução, para não
   perder o Build por causa de uma análise já pronta.
9. Como mantenedor, quero um teste de host que falha antes (build durante a descoberta) e passa depois,
   para a garantia não depender de timing de E2E.
10. Como mantenedor, quero a tabela de motivos numa função pura testada por unidade, para a ordem das
    recusas não virar conhecimento tribal.
11. Como mantenedor, quero que o E2E do cenário reusa o `settleDiscovery` da #251, para não inventar
    uma segunda forma de esperar a mesma automação.
12. Como mantenedor, quero a medição da janela da análise registrada em evidência, para o custo de UX
    ser um número e não uma impressão.
13. Como sessão de agente, quero que a recusa seja um `invalid_state` com motivo nomeado, para o log e a
    mensagem de falha dizerem quem está construindo.

## Implementation Decisions

- **Recusar, não enfileirar.** O contrato é o mesmo do restore ("A restore is running; wait for it to
  finish before building."). Enfileirar exigiria um estado novo no build, gatilho no fim da descoberta,
  cancelamento e invalidação por workspace/contexto/trust/restore — superfície nova para o mesmo ganho.
  O motivo nomeia a automação e a saída.
- **Um chokepoint no host.** O gate entra em `start_build_locked`, que é a função compartilhada por
  `build.start`, `build.solution` e pelo build implícito de Run/Debug. Um gate ali cobre os três
  chamadores; nenhuma outra porta de build existe.
- **A tabela de motivos é uma função pura.** A decisão sai do corpo de `start_build_locked` para uma
  função que recebe o que já está no `Inner` (build rodando, restore rodando, descoberta rodando, estado
  da análise) e devolve o motivo. Ordem: build > restore > descoberta > análise carregando. Assim o
  contrato é testável por unidade, sem csharp-ls real e sem workspace.
- **A análise bloqueia só enquanto carrega.** O estado da análise no host vai a `loading` quando o
  `initialize` é aceito e sai quando o csharp-ls loga `Finished loading solution`. Depois disso a análise
  não bloqueia: o `BuildHost` continua vivo, mas não está construindo. O motivo nomeia "Stop analysis"
  como saída.
- **A recusa é `invalid_state`, nunca um erro novo.** O código é o que a UI já trata para os gates
  existentes; o motivo é o texto que a UI mostra.
- **A UI desabilita com motivo, não reporta no clique.** `buildDisabledReason`/`runDisabledReason`/
  `debugDisabledReason` em `executionGates.ts` já alimentam `build-disabled-reason`,
  `run-disabled-reason`, `dap-disabled-reason`, os `title` e o menu `build-solution-menu`. As razões
  novas entram ali; nenhum `data-testid` novo, nenhum estado novo de UI.
- **A simetria mora na descoberta.** `tests.discover` passa a recusar com build rodando, no molde do que
  já faz com descoberta concorrente e com test run.
- **O E2E reusa o que a #251 deixou.** `settleDiscovery` (`tests/e2e/automatic.ts`) é a espera da
  descoberta automática; os specs que afirmam "enabled" logo depois de restaurar passam a esperar a
  descoberta antes da asserção.
- **Registro.** Decisão em `docs/decisions.md`; medição e prova em
  `docs/evidence/concurrent-builds.md`; a linha da #275 em `docs/evidence/test-stability.md` e o
  "o que fica" do `docs/RETRO-E2E-SMOKE-SOLO.md` passam a apontar para o fix.

## Testing Decisions

- **Bom teste aqui é comportamento externo**: o `invalid_state` com o motivo no dispatch do host, e o
  texto de motivo que a UI mostra. Nenhum teste olha o corpo de `start_build_locked`.
- **Seam 1 (host, o mais alto, já existe)**: `crates/host/tests/host.rs`, no molde de
  `crates/host/tests/test_discovery.rs` — abre o fixture OrderDesk, confia, restaura, avalia, dispara
  `tests.discover` e, com a descoberta rodando, chama `build.start`: espera `invalid_state` com o motivo.
  Depois que a descoberta assenta, o mesmo `build.start` funciona. Falha-antes: hoje o build começa.
- **Seam 2 (host, a simetria)**: com um build rodando (o fixture de build pendurado que `host.rs` já
  usa), `tests.discover` devolve `invalid_state` com o motivo. Falha-antes: hoje a descoberta começa.
- **Seam 3 (unidade, a tabela de motivos)**: teste Rust da função pura com as quatro entradas, incluindo
  a ordem (build rodando ganha da descoberta, descoberta ganha da análise) e o caso "nada rodando".
- **Seam 4 (unidade, a UI)**: `tests/unit/executionGates.test.ts` — `buildDisabledReason` devolve o
  motivo da descoberta e o da análise carregando, e `null` quando nada roda. `tests/unit` roda no CI;
  o E2E não. Prior art: `tests/unit/evaluationStatus.test.ts` (fixture de `Snapshot` com overrides).
- **Seam 5 (E2E, o cenário)**: `tests/e2e/build.spec.ts` — clicar Discover e, com a descoberta rodando,
  Build/Build Solution desabilitados com o motivo; depois de assentar, o Build conclui. Reusa
  `settleDiscovery`; ajusta as asserções de "enabled" que quebram (`gates.spec.ts:255` e
  `run.spec.ts:69/197/314`).
- **Limite aceito do seam 5**: a asserção "desabilitado durante a descoberta" tem janela (a descoberta
  do OrderDesk leva ~10-20 s a frio e ~3-5 s quente). Se ela se mostrar instável, a prova fica no teste
  de host e o E2E mantém só "o build conclui depois da descoberta".

## Out of Scope

- **Enfileirar o build** (o clique não se perde). Fica registrado como alternativa recusada.
- **Build durante test run**: troca as DLLs que o vstest está executando. É outra colisão, com outro
  arquivo e outro dono; vai para a retro, não para esta spec.
- **Recarga da análise depois de carregada**: o csharp-ls recarrega o projeto sozinho quando um arquivo
  muda, e o host não observa isso (o estado não volta a `loading`). Fica como risco residual escrito na
  evidência.
- **Tirar o build da descoberta** (`--no-build`): deixaria o painel Tests vazio até o primeiro build.
- Windows e macOS Intel continuam não verificados.

## Further Notes

- **Medição da análise** (csharp-ls 0.28 direto no fixture OrderDesk, sem app, macOS arm64): carregar a
  solução escreve 19 arquivos em `obj/Debug/net8.0/` (AssemblyInfo.cs, GlobalUsings.g.cs,
  GeneratedMSBuildEditorConfig.editorconfig, `*.assets.cache`, AssemblyReference.cache, `ref/`, `refint/`)
  e 1 em `bin/Debug/net8.0/` (`.msCoverageSourceRootsMapping_OrderDesk.Tests`) — **não** escreve
  `deps.json` nem `.dll`. A janela medida do início do processo ao `Finished loading solution` é ~5 s.
  Ou seja: a análise concorre na mesma árvore, mas não é a produtora do `MSB4018` medido; o produtor
  comprovado é a descoberta, que roda um build real.
- **Custo de UX da janela da análise**: ~5 s de Build desabilitado depois de cada restore, numa solução
  de 3 projetos nesta máquina; maior em solução grande e a frio. É o preço de cobrir o segundo
  candidato.
- **O `build.spec.ts` já espera a descoberta** desde a #257 (`settleDiscovery` em `restoreAndReload`).
  Isso fechou a janela do spec; esta issue fecha a do produto.
