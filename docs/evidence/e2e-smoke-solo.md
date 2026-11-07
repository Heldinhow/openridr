# Evidência — spec do smoke passa sozinho e a falha carrega o log cru (#251)

Data: 2026-11-07. Máquina local: macOS Apple Silicon, SDK do fixture em `~/.dotnet` (8.0.408 e 10.0.100).
Runner: `macos-15` do GitHub Actions, `PLAYWRIGHT_CHANNEL=chromium`, SDK 8.0.408 do `setup-dotnet`.
Windows e macOS Intel não foram executados.

## O que foi medido

| Run | Ref | Lista do smoke | Resultado |
|---|---|---|---|
| [42406578543](https://github.com/Heldinhow/openridr/actions/runs/42406578543) | `ci/issue-215-proof` | `build.spec.ts` | 1 failed, 1 passed — primeiro build em `Build failed · exit 1`, **sem log** (o achado da #251) |
| [42411400876](https://github.com/Heldinhow/openridr/actions/runs/42411400876) | `ci/issue-251-diagnosis` (só o instrumento do #255) | `build.spec.ts` | 2 passed |
| [42411789835](https://github.com/Heldinhow/openridr/actions/runs/42411789835) | idem | `build.spec.ts` | 1 failed, 1 passed — **log cru capturado** |
| [42411799383](https://github.com/Heldinhow/openridr/actions/runs/42411799383) | idem | `build.spec.ts` | 2 passed |
| [42412204140](https://github.com/Heldinhow/openridr/actions/runs/42412204140) | idem | `zz-251-diagnose.spec.ts` (descartável) | build **sucesso**; o spec falha de propósito e imprime a amostra de processos |
| [42412704045](https://github.com/Heldinhow/openridr/actions/runs/42412704045) | idem | idem | build **sucesso** |
| [42413304494](https://github.com/Heldinhow/openridr/actions/runs/42413304494) | idem | idem, com as mesmas asserções pré-build do `build.spec.ts` | build **sucesso** |
| [42414237554](https://github.com/Heldinhow/openridr/actions/runs/42414237554) | idem, **depois do fix** | `build.spec.ts` | **2 passed (1,9 min)** — teste 1 em 35,5 s |

Local, no par de portas 1925/4817: `build.spec.ts` 2/2 verde quente (1,3 min) e 2/2 verde com
`NUGET_PACKAGES` e `DOTNET_CLI_HOME` frios (1,1 min). **Cache NuGet frio não reproduz**; depois do fix,
2/2 verde em 42,9 s.

## A causa medida

O log cru que o instrumento do #255 passou a carregar (run 42411789835):

```
build-command: /Users/runner/.dotnet/dotnet build …/Order Desk build/src/OrderDesk.Cli/OrderDesk.Cli.csproj -c Debug -nologo -v:minimal --no-restore
build-cwd: /private/var/folders/…/openridr-build-8CRYfV/Order Desk build
build-log:
…/Microsoft.NET.Sdk.targets(284,5): error MSB4018: The "GenerateDepsFile" task failed unexpectedly. [OrderDesk.Core.csproj]
…/Microsoft.NET.Sdk.targets(284,5): error MSB4018: System.IO.IOException: The process cannot access the file
  '…/Order Desk build/src/OrderDesk.Core/bin/Debug/net8.0/OrderDesk.Core.deps.json' because it is being used by another process.
… Build FAILED. 1 Error(s)  Time Elapsed 00:00:03.22
```

Não é ordem de specs nem build a frio do restore. A amostragem de processos durante o primeiro build
(três runs, `ps` + `lsof` a cada 100 ms) mostra o que o app roda sozinho depois do trust, no mesmo
período em que o build explícito roda:

```
10260 7175 00:01 /Users/runner/.dotnet/dotnet test …/OrderDesk.Tests/OrderDesk.Tests.csproj -c Debug --no-restore --list-tests --nologo -v:minimal -f net8.0
10305 7175 00:00 /Users/runner/.dotnet/dotnet build …/OrderDesk.Cli/OrderDesk.Cli.csproj -c Debug -nologo -v:minimal --no-restore
10350 10250 00:01 /Users/runner/.dotnet/dotnet --roll-forward LatestMajor …/csharp-ls/0.28/…/BuildHost-netcore/Microsoft.CodeAnalysis.Workspaces.MSBuild.BuildHost.dll
```

- `dotnet test --list-tests` é a **descoberta automática de testes** (`WorkspaceView.tsx`, "one automatic
  discovery per workspace/context/restore boundary"): ela **constrói** o projeto de teste e as
  referências — inclusive `OrderDesk.Core`, escrevendo o mesmo `deps.json`.
- O `BuildHost` do MSBuild é da **análise** (csharp-ls), que o app também inicia sozinho depois do trust.
- O build explícito constrói o mesmo `OrderDesk.Core`. Duas construções do mesmo projeto ao mesmo tempo:
  uma delas perde em `GenerateDepsFile`.

Por que parecia ordem — **inferência, não medição**: rodar `workspace.spec.ts` antes paga restore, cache
NuGet, descoberta e análise, então o primeiro build da execução seguinte encontra a máquina quente e não
acha a janela. O que está **medido** é o efeito: o segundo teste do mesmo arquivo passa na mesma run, e
`build.spec.ts` sozinho falhou 2 de 5 vezes, sempre no primeiro build. A explicação mecânica do efeito de
ordem (máquina quente) é consistente com isso, mas não foi isolada por experimento.

**Limite da medição:** o `lsof` do arquivo não pegou o dono em nenhuma das três runs de amostragem (o
build passou nas três), então a atribuição vem do log + da lista de processos, não de um `lsof` no
momento exato da falha. Os dois candidatos (descoberta e análise) constroem o mesmo projeto; o fix do
spec fecha a janela do primeiro, e o bug do produto cobre os dois.

## O fix

- **#255 (`b3c6172`)**: helper `expectBuildState` no `build.spec.ts` — toda espera de `build-state` que
  falha passa a carregar `build-command`, `build-cwd` e o `build-log` cru. Falha-antes medida com um spec
  descartável e uma cópia do fixture que não compila.
- **#257 (`1bd7dc1`)**: `settleDiscovery` em `tests/e2e/automatic.ts` (exportado) e o `build.spec.ts`
  esperando a descoberta automática assentar antes do primeiro build, em `restoreAndReload`. Isso fecha a
  janela do primeiro candidato (a descoberta); a análise continua fora da sincronização e é o que o bug do
  produto (#275) cobre.

## Prova

- Runner, `build.spec.ts` sozinho, depois do fix: **2 passed (1,9 min)**, teste 1 em 35,5 s —
  [42414237554](https://github.com/Heldinhow/openridr/actions/runs/42414237554).
- Local: 2/2 verde (42,9 s) com o fix.
- Antes do fix, 2 de 5 runs sozinhas na runner falharam no primeiro build.

Uma run verde depois do fix não prova **qual** dos dois candidatos segurava o arquivo: prova que o spec
deixa de falhar no primeiro build com a descoberta sincronizada. A atribuição exata segue em aberto (ver
limite acima) e é o que a #275 precisa fechar.

## Efeito colateral registrado

A run verde de prova fechou a issue de rastreio do smoke **#266**, que estava com o corpo da run vermelha
da branch da #245 (`fix/issue-245-smoke-symlink-spec`) — o desenho da #215 fecha em qualquer verde. A
sessão da #245 já havia comentado ali que os corpos vermelhos vinham de `workflow_dispatch`, não da noite
em `main`.

## O que NÃO foi verificado

- O bug do produto (build explícito concorrente com a automação do app) — issue filha; o spec espera, o
  produto continua exposto.
- Qual dos dois processos de automação segurava o arquivo na run vermelha (ver "limite da medição").
- Windows e macOS Intel.
- A noturna agendada em `main` depois do merge (a primeira noite é a primeira execução real do `schedule`).
