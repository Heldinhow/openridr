# Debug: `configurationDone` e dotnet ausente do PATH

## Diagnóstico — 2026-11-05

O `/Applications/OpenRidr.app` anuncia `0.3.0-alpha.8` no Info.plist e contém o runtime privado `Contents/Resources/dotnet/shared/Microsoft.NETCore.App/10.0.14`. A versão não identifica univocamente a branch. A comparação com `origin/main` (`eb3fe6f`) e a branch `feat/bundled-analysis-runtime` (`3696d89`) mostra o mesmo caminho de inicialização do DAP nas duas: o `DOTNET_ROOT` do launch é a raiz do SDK do projeto, mas o processo netcoredbg herda o PATH do app sem incluir essa raiz. A alteração da branch na descoberta do adaptador não muda esse caminho.

Prova isolada com o netcoredbg 3.2.0 empacotado no app instalado e uma cópia compilada da fixture OrderDesk net8, sem abrir o app desktop:

- Com `PATH=/usr/bin:/bin:/usr/sbin:/sbin` e `DOTNET_ROOT` do launch apontando para `~/.dotnet`, o adaptador responde ao `configurationDone` com `0x80070002`.
- Acrescentando `~/.dotnet` ao início do PATH do adaptador, o mesmo launch conclui `configurationDone`, carrega `Microsoft.NETCore.App/8.0.15` e termina com código 0.
- Com o PATH amplo do shell, ele também conclui, mas usa o `dotnet` encontrado em `/usr/local/share/dotnet` e carrega o runtime 8.0.22 daquela raiz. Portanto o `DOTNET_ROOT` do request não garante a escolha do executável pelo adaptador.
- Após o merge de `origin/main` (`3eae813`), com `DOTNET_ROOT` herdado apontando para o runtime privado 10 e o SDK do projeto primeiro no PATH do adaptador, o mesmo DAP conclui `configurationDone`, carrega o runtime 8.0.15 de `~/.dotnet` e sai com código 0. A inspeção do host após o merge confirma que esse PATH usa `project_sdk.root`, não `AnalysisRuntime.root`.

Essa reprodução demonstra uma causa no código comum, não prova qual PATH ou configuração o processo do usuário tinha na captura. Não foi constatado vazamento do runtime privado de análise para o launch.

## Correção

O host valida `<sdkRoot>/dotnet` (ou `dotnet.exe`), informa o caminho quando está ausente e antepõe `sdkRoot` ao PATH apenas do processo do adaptador, preservando as demais entradas. O ambiente explícito da configuração de execução continua aplicado ao processo depurado. Os testes unitários verificam a precedência da raiz e a mensagem para o arquivo ausente; ambos falharam antes da correção e passaram depois.

## Validação após o merge

- Unitários Rust: 266 passaram na execução inicial; dois testes Git falharam porque `TMPDIR` ficava dentro do repositório e o Git encontrou o `.git` ancestral. Reexecutados individualmente com `TMPDIR=/tmp`, os dois passaram. São 268/268 contabilizados em execuções separadas, não uma execução única verde.
- Integração Rust: 138/138 passaram nos seis alvos restantes, incluindo 8 de análise embutida e 14 de debugger, sequencialmente. O comando completo inicial parou nos dois unitários Git.
- E2E `debugger.spec.ts`, portas 1530/4427 e home isolado: 1 passou e 12 falharam. O teste do painel bundled passou; cenários legados tentam `debug.install` apesar de o harness fornecer adaptador bundled falso, e alguns pararam na avaliação ou por timeout antes de cobrir o DAP real. Não se conclui que todas as falhas sejam preexistentes.
- Comparação isolada do teste `host gates consent, trust and plan identity before any download` em cópia de `origin/main` `3eae813`: falhou igual, esperando `confirmation_required` e recebendo `trust_required` na linha 109. Essa falha específica é preexistente à correção.
- O hook de commit executou `bun run check` com sucesso após o merge.

## Limites

O teste DAP direto verifica o binário empacotado, não o app Tauri nem a experiência de interface. App desktop, Windows e macOS Intel não foram verificados.
