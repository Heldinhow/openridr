# OpenRidr — sete telas e estados

**Proposta estática / controles ilustrativos.** Leia o [README](README.md) para decisões e prioridades; [HTML offline](index.html) mostra os cenários abaixo. UI em inglês desde v1 e docs PT são decisões aprovadas; aguarda-se aprovação das TELAS, não dessas decisões.

Referência desktop: 1440 × 900, composição mínima 1100 px com overflow local no documento. Árvore ~280 px, editor flexível, painel inferior ~200 px, gutter separado para breakpoint/linha. Fonte sans na UI (13–14 px), mono em código/log. No mobile, descrições e estados empilham; cada frame desktop rola horizontalmente sem alargar a página. Sem animação. Links têm foco; todos os controles de produto são ilustrativos/desabilitados.

Legenda: `●` breakpoint verificado; `○` pendente; `>` linha atual; `[ação]` controle ilustrativo; `!` aviso/erro acompanhado de texto. Não são menus funcionais. Mac e Windows pertencem ao P0; atalhos devem ter rótulos por SO na implementação, sem pressupor que Cmd vira Ctrl em todos os casos.

`Keymap: IntelliJ / Rider` é padrão, sem seletor novo: compatibilidade de atalhos, não backend/componente proprietário ou branding. Run Mac Ctrl+R / Windows Shift+F10; Debug Mac Ctrl+D / Windows Shift+F9; breakpoint Mac Cmd+F8 / Windows Ctrl+F8; step over F8 / step into F7. Marca própria OpenRidr.

Fixture única: OrderDesk.sln → console OrderDesk.Cli + biblioteca OrderDesk.Core + OrderDesk.Tests, net8.0; global.json 8.0.408, rollForward disable. Host análise SDK 10 separado. Exemplos de diagnóstico são snapshot falho; debug mostra revisão corrigida, não execução do fonte quebrado.

## 01 · Primeira execução / confiança e SDK

```text
+ OpenRidr / Toolchain ----------------------------------------------+
| OrderDesk.sln · workspace path: ~/src/OrderDesk                     |
| RESTRICTED MODE · files can be read; project code will not run      |
| [ ] I trust this repository to execute code                         |
| MSBuild evaluation, restore, csharp-ls and workspace terminal blocked|
| [Open text only] [Trust and inspect project]                        |
|-------------------------------------------------------------------|
| Analysis host    SDK 10 installed · csharp-ls 0.28 not started       |
| Project SDK      global.json requests 8.0.408 · not resolved yet     |
| Project target   net8.0 declared · not evaluated                    |
| Debug adapter    netcoredbg 3.2.0 · platform asset must be verified   |
| Apple Silicon debugger: experimental · disabled                    |
| Install/download requires separate consent                         |
+-------------------------------------------------------------------+
```

Estados alternativos concretos (também no HTML):

```text
[SDK MISSING] Analysis host requires SDK 10.
  Text editing available. [View official SDK instructions]
[PROJECT SDK MISSING · trusted] global.json pins 8.0.408; only 10 found.
  Build blocked. Host SDK 10 does not replace project SDK 8.0.408.
  [View official SDK instructions] [Open global.json]
[TRUSTED / READY] Host SDK 10 · project SDK 8.0.408 · net8.0.
  csharp-ls 0.28 ready after project load. [Open workspace]
```

```text
[INSTALL PLAN · confirmation required]
  Tool: csharp-ls 0.28 (package 0.28.0) · Source: NuGet.org
  Destination: <app-tools>/csharp-ls/0.28 (resolve absolute path first)
  Command: dotnet tool install csharp-ls --version 0.28.0
           --tool-path <app-tools>/csharp-ls/0.28
  Permissions: network + write app tool directory; no repo/global change
  [Confirm installation] [Cancel] · not authorized by repository trust
  netcoredbg 3.2.0: download official Samsung/netcoredbg release asset;
  show exact URL, OS/architecture, destination and extract operation first.
  [ASSET UNAVAILABLE] No verified compatible asset. Installation blocked.
[DOWNLOADING] Download log visible · no percentage unless reported [Cancel]
[INSTALLING] Command / stdout / stderr visible [Cancel]
[VERIFYING] Check executable/version · not DAP compatibility [Cancel]
[CANCELLED] Partial state retained in log; no success claim [Retry]
[FAILED] Verification failed · raw log preserved [View log] [Retry]
```

Fonte: leitura estática, inventário global em cwd neutro; resolução do SDK/propriedades avaliadas só depois de confiar. Confiança desmarcada por padrão; abrir texto não abre PTY nem inicia LSP/MSBuild. **App instala ferramentas** após plano e confirmação, download ou dotnet tool, com log e verificação. Não instala SDK de sistema: SDK 10 ausente recebe orientação oficial e nova detecção, nunca mudança em global.json. Não prometer asset Mac Intel 3.2 nem compatibilidade por arquitetura sem prova. Trust, instalação e habilitar experimental são consentimentos independentes. net6 best effort; Roslyn LS depois; #256 é risco não reproduzido.

## 02 · Janela principal / editor e Build

```text
+ OpenRidr · OrderDesk.sln ------------------------------------------+
| File  Edit  View  Navigate  Code  Run  Tools                        |
| Startup: OrderDesk.Cli | Debug | net8.0 | [Build] [Run] [Debug off]  |
| Apple Silicon debugger: experimental · disabled · Run available     |
|---------------------------+---------------------------------------|
| SOLUTION EXPLORER         | Program.cs · modified | Order.cs       |
| v OrderDesk.sln           | OrderDesk.Cli / Program.cs / net8.0     |
|   v src                   | bp ln                                  |
|     v OrderDesk.Cli *     |     1 using OrderDesk.Core;             |
|       Program.cs         |     2                                  |
|       Dependencies       |     3 var order = new Order("A-104", 42m);|
|     > OrderDesk.Core     |     4 var total = order.Total;          |
|   v tests                |  ○  5 Console.WriteLine($"Total: {total:C}");|
|     > OrderDesk.Tests    |                                       |
|---------------------------+---------------------------------------|
| Build (selected) | Problems | Debug | Terminal | Tests             |
| BUILD FAILED · OrderDesk.Cli · previous saved revision              |
| Program.cs(3,17): error CS0246: 'Order' could not be found           |
| Process exit: 1 · [Open source] [View raw output]                   |
| Editor changed since build. Rebuild to verify; no launch attempted. |
|-------------------------------------------------------------------|
| Host SDK 10 | Project SDK 8.0.408 | Keymap: IntelliJ / Rider         |
+-------------------------------------------------------------------+
```

```text
[ANALYSIS LOADING] csharp-ls is loading OrderDesk.sln.
  Editing available; completion not ready. [View language log]
[NO SOLUTION] Open a .sln or .csproj to start. [Choose solution]
[FILE CHANGED ON DISK] Program.cs differs from the open buffer.
  [Compare] [Reload from disk] · keep unsaved edits until explicit choice
```

Fonte: árvore MSBuild após trust, editor LSP, Build processo CLI; output bruto e revisão preservados. Breakpoint `○` ainda não verificado pelo DAP, não erro de código. `*` é startup, não modificação. Nenhum contador de progresso inventado. Startup, Build e runner básico VSTest são P0 (tela 07). Debug ARM64 está bloqueado pela flag padrão, além da necessidade de build válido; Run é independente.

## 03 · Solution Explorer / carga parcial

```text
+ OpenRidr · Solution Explorer --------------------------------------+
| v OrderDesk.sln              | PROJECT · OrderDesk.Cli              |
|   v src                      | Startup: yes · Debug / net8.0        |
|     v OrderDesk.Cli *        | Reference: OrderDesk.Core             |
|       v Dependencies        | TargetPath (evaluated):              |
|         Frameworks          | src/OrderDesk.Cli/bin/Debug/net8.0/   |
|         Project References  | OrderDesk.Cli.dll                     |
|       Program.cs            | [Build] [Set startup] [Open .csproj]  |
|     > OrderDesk.Core        |                                      |
|   v tests                    | ! RESTORE INCOMPLETE                |
|     > OrderDesk.Tests !     | Tests: NU1301 unable to load feed.    |
|                             | Dependencies/analysis incomplete.    |
|                             | [Restore] [View restore log]         |
|-----------------------------+--------------------------------------|
| Partial workspace · no clean-solution claim                        |
+-------------------------------------------------------------------+
```

```text
[PROJECT LOAD FAILED] OrderDesk.Core · MSBuild evaluation failed.
  Imported file build/Local.props was not found (MSB4019).
  Project stays visible. [Open project file] [View log] [Retry]
[RESTRICTED] File outline only · evaluated dependencies unavailable.
  [Review trust]
```

Fonte: contorno da solução é leitura; propriedades, itens e TargetPath são avaliação MSBuild após trust; pacotes restaurados dependem dos assets. Não derivar modelo de solução do LSP. Sem esconder projeto falho, sem afirmar TargetPath real antes de avaliação. Multi-target avançado fora de P0; nunca escolher silenciosamente quando ambíguo.

## 04 · Diagnósticos / origem e categoria

```text
+ OpenRidr · Problems · faulty saved snapshot ------------------------+
| Scope: reported items | Errors 1 | Warnings 2 | Info 0               |
| ! OrderDesk.Tests restore incomplete · [View restore log]           |
| Sev   Code    Message                    Origin        Category     |
| Error CS0246  'Order' could not be found  LSP/csharp-ls  Compiler     |
|               Program.cs:3:17                                      |
| Warn  CS8618  'Name' must contain value   Build/dotnet   Compiler     |
|               Order.cs:5:19                                        |
| Warn  CS0168  'ex' declared but unused    Build/dotnet   Compiler     |
|               Order.cs:14:26                                       |
|-------------------------------------------------------------------|
| LSP selection: Program.cs:3:17 · request contextual code actions     |
| Example if returned: [Add using OrderDesk.Core] [Preview edit]      |
| Build-only selection: no LSP quick fix implied                     |
+-------------------------------------------------------------------+
```

```text
[AWAITING DIAGNOSTICS] Program.cs changed after LSP publication.
  Previous report retained, marked stale; not crossed out as resolved.
[VERIFIED DOCUMENT EMPTY] No problems reported for Program.cs.
  LSP published an empty list for the current document revision.
  Not a solution-wide guarantee; analyzers coverage shown separately.
[BUILD VERIFIED] Build succeeded · 0 errors, 0 warnings · exit 0.
  Scope: last saved OrderDesk.sln build; unsaved buffers not included.
```

Fonte: publishDiagnostics e saída CLI de uma execução identificada. Categoria Analyzer somente quando identificada pela fonte, com severidade recebida/configurada; caso contrário Unknown. Não presumir analyzer habilitado. Não agregar LSP e build apagando sua origem. Sem JSON universal via dotnet build; parser conservador e saída bruta. Quick fix só oferecido no contexto pelo LSP; não prometer correção antes da republicação.

## 05 · Debug / revisão corrigida

Pausa abaixo é cenário ilustrativo Windows x64 com adaptador a validar, não prontidão Mac ARM64. Mac/Windows saem juntos; run/análise são independentes de debug e da disponibilidade de asset por arquitetura.

```text
+ OpenRidr · Debug · OrderDesk.Cli · corrected revision ---------------+
| PAUSED · Program.cs:5 | [Continue] [Step over] [Step into] [Stop]     |
| Build succeeded → netcoredbg 3.2.0 → breakpoint verified            |
|---------------------+--------------------------------+------------|
| FRAMES              | Program.cs                     | LOCALS     |
| Main thread paused  | bp ln                          | order Order|
| Program.<Main>$ :5  |     3 var order = new Order(    | Id A-104   |
|                     |         "A-104", 42m);         | total 42   |
| BREAKPOINTS         |     4 var total = order.Total; | args []    |
| ● Program.cs:5      | ● > 5 Console.WriteLine(       |            |
|                     |         $"Total: {total:C}"); |            |
|---------------------+--------------------------------+------------|
| DEBUG OUTPUT · No application output yet (paused before WriteLine)  |
| Windows x64 · illustrative DAP session · hot reload not supported   |
+-------------------------------------------------------------------+
```

```text
[BUILD FAILED] Debug not started. [Open Build] [Open source]
[BREAKPOINT PENDING ○] Program.cs:5 · symbols not loaded.
  [View adapter log] [Rebuild Debug] · no verified claim
[ADAPTER MISSING] netcoredbg unavailable for selected OS/architecture.
  [Review tool installation] · block install if no verified asset
  Run remains separate from Debug; no promise of a Mac Intel 3.2 asset.
[APPLE SILICON DEFAULT] Apple Silicon debugger: experimental · disabled
  Debug unavailable while experimental feature is off. Run remains available.
  [Review experimental feature] · independent consent, not installation
[OPTIONAL OPT-IN] Apple Silicon debugger: experimental · enabled
  Compatibility unverified; adapter installation has separate confirmation.
  [View DAP log] · enabled does not mean ready or proven
[STOPPED / EXIT UNKNOWN] Debug session ended; adapter sent no exit code.
  [View DAP log] · do not report exit 0 or process cleanup without proof
```

Fonte: DAP initialize → launch → setBreakpoints → configurationDone, respeitando ordem/capacidades do adaptador; DLL vem de TargetPath e build Debug bem-sucedido. Rodando: sem locals antigos como se fossem atuais; step só em pausa. Encerramento natural só mostra exit se recebido. Risco #256 não reproduzido aqui. Não há attach/conditional/log/watch/evaluate em P0; sem hot reload, vsdbg ou Dev Kit.

Flag ARM64 OFF por padrão, experimental e não bloqueante de release. Só retirar após evidência Mac arm64 em `netcoredbg --interpreter=vscode` (DAP), async e multi-thread, launch/breakpoint/step/stop e logs de encerramento. Presença/versão do binário não é prova DAP. Mac Intel depende de asset compatível verificado; ausência deixa Debug indisponível, não run/análise.

## 06 · Terminal / teste CLI complementar

```text
+ OpenRidr · Terminal -----------------------------------------------+
| Terminal · zsh · ~/src/OrderDesk · trusted workspace                |
|-------------------------------------------------------------------|
| ~/src/OrderDesk $ dotnet test tests/OrderDesk.Tests                  |
| ...                                                               |
| Passed! - Failed: 0, Passed: 2, Skipped: 0, Total: 2                |
| ~/src/OrderDesk $                                                  |
|-------------------------------------------------------------------|
| Shell running · CLI output only · basic Test Runner: screen 07      |
+-------------------------------------------------------------------+
```

```text
[WINDOWS] PowerShell · C:\src\OrderDesk
  PS C:\src\OrderDesk> dotnet test tests/OrderDesk.Tests
  Same manual workflow; shell discovered, not assumed installed.
[RESTRICTED] Workspace terminal blocked. [Review trust]
[CONFIRM CLOSE] Terminal shell is still running.
  [Keep terminal] [Terminate shell and close]
[SHELL EXITED] PTY process exited with code 0. [Open new terminal]
  This is the shell exit, not the previous dotnet test command exit.
```

Fonte: PTY do host. Saída do comando é texto, não modelo estruturado de testes. Não mostrar shell exit enquanto o prompt está ativo; código por comando só com integração comprovada. Ctrl+C é entrada real no terminal; encerramento e árvore de processos exigem validação nos dois SOs. Abas/splits ficam no P2.

## 07 · Test Runner básico / VSTest · P0

```text
+ OpenRidr / Test Runner · OrderDesk.sln -----------------------------+
| Project: [OrderDesk.Tests / net8.0] · Adapter: project VSTest/xUnit   |
| [Discover] [Run all] [Run selected] [Cancel] · selection: 1 test      |
|-------------------------------+-----------------------------------|
| TESTS                         | RESULT · Total_returns_order_total |
| v OrderDesk.Tests             | Failed · expected 42, actual 0     |
|   [x] Total_returns_order_total| Stack: OrderTests.cs:18            |
|       Failed                  | [Open source] (position supplied)  |
|   [ ] Id_preserves_value      | stdout: order A-104                |
|       Passed                  | stderr: (empty)                    |
|   [ ] Legacy_discount        | [View raw output] [View result log]|
|       Skipped                 |                                   |
|-------------------------------+-----------------------------------|
| Completed · Passed 1 · Failed 1 · Skipped 1 · VSTest run snapshot    |
+-------------------------------------------------------------------+
[DISCOVERING] Discovering tests via project adapter. [Cancel] [View log]
[NONE] No tests discovered for OrderDesk.Tests. [Discover] [View log]
[ADAPTER ABSENT] No compatible VSTest adapter detected in project.
  Configure the project's adapter explicitly; no silent package changes.
  [View adapter guidance] [View log]
[FAILED] Discovery or execution failed. Results may be incomplete.
  stdout/stderr retained. [View log] [Retry]
[CANCELLED] Run cancelled; partial results retained, no success claim.
  [View log] [Run selected]
[MTP] Microsoft.Testing.Platform is not supported in this version. Use the terminal.
```

Fonte proposta: descoberta/execução VSTest com adapter já instalado no projeto, resultados VSTest/TRX a validar; não assumir que TRX por si fornece descoberta. xUnit é suportado por esse caminho, não é filtro exclusivo de framework; outros adapters exigem validação, sem promessa universal. Trust e SDK do projeto são pré-condições; não adicionar ou alterar packages silenciosamente. Detalhes preservam stack e stdout/stderr; navegação só quando o resultado fornecer posição, senão log como fallback. Cancelamento não transforma testes sem resultado em passed/skipped. Descoberta e execução têm estados próprios; execução todos/selecionados preserva escopo e revisão. Nomes de fixture: `Total_returns_order_total`, `Id_preserves_value`, `Legacy_discount`; o terminal mostra execução separada de dois testes aprovados, não o snapshot desta tela.

**Runner básico P0 aprovado:** impacto pequeno/médio frente ao CLI, mas novo trabalho de descoberta, resultados e navegação, não capacidade herdada comprovada. Fecha editar/build/testar. Cobertura/gutters/debug test em P1 e MTP a avaliar no P2, fora de P0; nunca tentar comando VSTest em projeto MTP. Não confundir fallback de logs com resultado estruturado disponível.

## Encerramento

As sete telas cobrem P0 e falhas; P1 testes avançados/perfis e P2 MTP/debug avançado não são funcionalidades escondidas no mockup. **Parar aqui e aguardar Helder aprovar as TELAS.** Nenhuma implementação ou instalação executada.
