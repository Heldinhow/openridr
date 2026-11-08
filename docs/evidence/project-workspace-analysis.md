# Evidência — a análise de um workspace de projeto sai de `loading` (#299)

Issue: [#299](https://github.com/Heldinhow/openridr/issues/299) (bug, encontrado na
[#275](https://github.com/Heldinhow/openridr/issues/275)). Decisão:
[decisions.md](../decisions.md) (2026-11-08, issue #299). Contexto que restringiu o gate:
[concurrent-builds.md](concurrent-builds.md).

## O defeito, medido

Workspace de **projeto** (`.csproj` aberto direto), csharp-ls 0.28.0 do bundle, macOS arm64, fixture
CrashApp. Host de verdade (`openridr-host-http`), sequência `workspace.open` → `setup.start` →
`trust.grant` → `restore.start` → `evaluation.run` → `lsp.start`:

| Medida | Resultado antes do fix |
|---|---|
| `lsp.state` | `starting` → `loading` em ~2 s e **parado em `loading`** |
| Log do servidor | termina em `loading project "<...>/CrashApp.csproj"..`, sem linha nova por 100 s |
| CPU do processo | 0,4% |
| Filho `dotnet`/BuildHost | nenhum (o mesmo host carregando `OrderDesk.sln` mantém um `BuildHost` a ~84%) |
| `lsp.stop`/`lsp.restart` | não mudam o quadro: o servidor não emite `Finished loading solution` |

O mesmo travamento acontece **fora do host**: um cliente LSP mínimo (initialize + initialized +
`workspace/didChangeConfiguration`, sem `--solution`) reproduz o log parado em `loading project` por 90 s.
Não é ambiente do host, é o servidor.

## A causa no upstream

`src/CSharpLanguageServer/Roslyn/Solution.fs` (tag `0.28.0`, e igual na `master` de 2026-11-08):

- `solutionFindAndLoadOnDir` não acha `.sln`/`.slnx` no diretório do workspace e cai em
  `solutionTryLoadFromProjectFiles`;
- essa função chama `MSBuildWorkspace.OpenProjectAsync` por projeto e **trava lá**: nenhum log, nenhum
  processo filho, nenhum retorno;
- ela também **nunca** emite `Finished loading solution` — o único sinal de "carregou" que o host conhece.

`--solution <arquivo>` usa `solutionTryLoadOnPath` (`OpenSolutionAsync`), que funciona e emite
`Finished loading solution`.

## O fix

O host gera `<app home>/analysis/<nome>-<16 hex do sha256 do caminho>.sln` com o projeto aberto e passa
`--solution` nos dois tipos de workspace (`start_analysis` em `crates/host/src/lib.rs`, função
`project_solution`). O `.sln` nomeia o projeto por caminho absoluto, então não importa que ele viva fora
do workspace; o digest do caminho separa dois projetos de mesmo nome em pastas diferentes, e um caminho
com aspa ou quebra de linha é recusado com `operation_failed` em vez de gerar uma solução inválida.

| Prova | Onde | Resultado |
|---|---|---|
| O projeto sai de `loading` e chega a `ready` | host de verdade, mesma sequência acima | `starting` → `loading` (~2 s) → **`ready` (~4 s)**; log com `Loading solution ".../analysis/CrashApp.sln"` e `Finished loading solution ".../analysis/CrashApp.sln"` |
| O gate da #275 vale para projeto | `crates/host/tests/host.rs` › `build_is_refused_while_the_analysis_is_loading_the_solution_isolated` | durante `loading` do projeto: `build.start` → `invalid_state` com "Analysis is loading the solution (csharp-ls); wait for it to finish, or stop analysis to build now."; depois de carregado, o mesmo build roda (`ok`, exit 0) |
| O gate da UI não olha mais o tipo do workspace | `tests/unit/executionGates.test.ts` | `lsp.state == "loading"` desabilita Build/Run/Debug com o motivo, sem exceção de projeto |
| A UI nomeia a análise de um projeto antes do clique | `tests/e2e/build.spec.ts` › `the build gate names the analysis loading a project workspace` | com o CrashApp aberto, `lsp.restart` → `build-disabled-reason` com o texto da análise, `build-action` desabilitado com o motivo no `title`, `run-action` desabilitado; depois de carregar, o gate libera sozinho e o Build conclui (`Build succeeded`) |
| `lsp.restart` recarrega o projeto | host de verdade, `lsp.start` → `lsp.restart` (verbose) | volta a `loading` e chega a `ready`; a solução gerada é reescrita com o mesmo caminho |
| Fechar e reabrir o workspace recarrega | host de verdade, `lsp.stop` → `workspace.close` → `workspace.open` → `setup.start` → `trust.grant` → `lsp.start` | chega a `partial` (a avaliação fica stale no contexto novo, então a cobertura é parcial) com `Finished loading solution` no log; nunca fica em `loading` |

## Falha-antes medida

Com o teste final e o fix em stash, o leaf acima falha na primeira asserção do bloco de projeto:

```
build.start started while the project analysis was loading: "running"
```

(21,65 s de rodada; a análise do projeto continuava em `loading`.) Com o fix, o mesmo leaf passa em
15,56 s.

## O que não foi verificado

- **App Tauri real**: é o mesmo host; a reprodução e a prova foram pelo host HTTP (o E2E em modo web usa a
  mesma ponte). Não abri o app desktop para este ticket.
- **E2E em PR**: nenhum spec roda em PR (o smoke é noturno). Rodado à mão, em modo web
  (`PLAYWRIGHT_CHANNEL=chromium`, par 2320/5317): `build.spec.ts` **4 passed** em duas rodadas seguidas
  (1,4 min e 1,3 min), incluindo o cenário novo do projeto; `run.spec.ts` 5 passed (1,6 min);
  `execution-gates.spec.ts` 4 passed. O cenário novo depende da janela de `loading` (~2 s medidos) contra
  o poll de 400 ms da UI: duas rodadas não provam ausência de flake, e é por isso que o smoke noturno é
  quem vai dizer.
- **Windows e macOS Intel**: não executados.
- **Projeto com `ProjectReference`**: o `.sln` gerado nomeia só o projeto aberto. Pelo código do
  csharp-ls os projetos referenciados entram como metadados (`LoadMetadataForReferencedProjects`), o que
  significa diagnóstico do projeto aberto sem navegação para o código dos referenciados — **isso não foi
  medido** numa fixture com referência cruzada, é leitura do upstream.
