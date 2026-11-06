# Debug: `configurationDone` e dotnet ausente do PATH

## Diagnóstico — 2026-11-05

O `/Applications/OpenRidr.app` anuncia `0.3.0-alpha.8` no Info.plist e contém o runtime privado `Contents/Resources/dotnet/shared/Microsoft.NETCore.App/10.0.14`. A versão não identifica univocamente a branch. A comparação com `origin/main` (`eb3fe6f`) e a branch `feat/bundled-analysis-runtime` (`3696d89`) mostra o mesmo caminho de inicialização do DAP nas duas: o `DOTNET_ROOT` do launch é a raiz do SDK do projeto, mas o processo netcoredbg herda o PATH do app sem incluir essa raiz. A alteração da branch na descoberta do adaptador não muda esse caminho.

Prova isolada com o netcoredbg 3.2.0 empacotado no app instalado e uma cópia compilada da fixture OrderDesk net8, sem abrir o app desktop:

- Com `PATH=/usr/bin:/bin:/usr/sbin:/sbin` e `DOTNET_ROOT` do launch apontando para `~/.dotnet`, o adaptador responde ao `configurationDone` com `0x80070002`.
- Acrescentando `~/.dotnet` ao início do PATH do adaptador, o mesmo launch conclui `configurationDone`, carrega `Microsoft.NETCore.App/8.0.15` e termina com código 0.
- Com o PATH amplo do shell, ele também conclui, mas usa o `dotnet` encontrado em `/usr/local/share/dotnet` e carrega o runtime 8.0.22 daquela raiz. Portanto o `DOTNET_ROOT` do request não garante a escolha do executável pelo adaptador.

Essa reprodução demonstra uma causa no código comum, não prova qual PATH ou configuração o processo do usuário tinha na captura. Não foi constatado vazamento do runtime privado de análise para o launch.

## Correção

O host valida `<sdkRoot>/dotnet` (ou `dotnet.exe`), informa o caminho quando está ausente e antepõe `sdkRoot` ao PATH apenas do processo do adaptador, preservando as demais entradas. O ambiente explícito da configuração de execução continua aplicado ao processo depurado. Os testes unitários verificam a precedência da raiz e a mensagem para o arquivo ausente; ambos falharam antes da correção e passaram depois.

## Limites

O teste DAP direto verifica o binário empacotado, não o app Tauri nem a experiência de interface. App desktop, Windows e macOS Intel não foram verificados.
