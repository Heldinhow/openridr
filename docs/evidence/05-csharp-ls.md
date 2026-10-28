# Evidência — Ticket 05: instalar csharp-ls com confirmação e carregar análise

Implementação local em 28/10/2026. Fonte normativa: [ticket 05](../tickets/05-csharp-ls.md), [SPEC](../SPEC.md) P0.1/P0.3, “Estado, erros e processos”, “Avaliação e ferramentas” e Testing Decisions, e gates G1–G5 do [índice de tickets](../tickets/README.md).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| SDK do host | 10.0.100 em `/usr/local/share/dotnet` (também 8.0.416) |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408`, fixado por `global.json` com `rollForward: disable` |
| csharp-ls | 0.28.0 via NuGet.org, `--tool-path` em `~/Library/Application Support/OpenRidr/tools/csharp-ls/0.28` |
| Fixtures | `fixtures/OrderDesk`; cópias temporárias para instalação/análise; nenhum `fixtures/` foi restaurado/mutado |
| Base de código | `50389fc` (ticket 04) + esta fatia |

Logs: [`logs/05-hostroot-environment.log`](logs/05-hostroot-environment.log), [`logs/05-drive.log`](logs/05-drive.log). O fluxo de evidência usou `OPENRIDR_HOME` temporário; o app home real foi verificado separadamente. Nenhum SDK foi instalado.

## O que foi construído

- **Plano de instalação** (`crates/host/src/tools.rs`): `tools.plan` devolve ferramenta, versão exata, pacote/fonte NuGet.org (URL efetiva), OS/arquitetura, destino absoluto fora do repo, staging, argv separado, cwd neutro, permissões, limitação de integridade e `planId` derivado de todos os campos materiais. O plano não executa nada: nem `--version` de uma instalação existente (a verificação acontece só depois da confirmação).
- **Instalação protegida**: `tools.install {planId, confirm:true}` exige trust, revalida o plano (mudança material → `confirmation_required`, nada baixado) e usa **somente** `dotnet tool install csharp-ls --version 0.28.0 --tool-path <staging>`, com HOME isolado (fonte default NuGet.org), `NUGET_PACKAGES`/cache dentro do app home e cwd neutro. Não é instalação global, não eleva, não altera SDK/global.json/repositório.
- **Ciclo real**: `downloading → installing (promoção staging→destino) → verifying (executável + `--version` + SHA-256 do launcher) → ready`. O staging nunca é ready; ferramenta válida existente é reutilizada sem download nem sobrescrita; falha/cancelamento preservam log e staging rotulado. `tools.cancel` mata o grupo TERM→KILL; `tools.retry` revalida o plano confirmado; progresso só aparece quando medido (não há percentual).
- **Origem/integridade honestas**: o processo roda com HOME isolado, cujo NuGet.Config default contém apenas `nuget.org`; o log real mostra `Skipping NuGet package signature verification.` e o host registra SHA-256 do launcher + saída exata de `--version`, sem alegar assinatura/hash de pacote oficial.
- **Host root composto** (`crates/host/src/hostroot.rs`): cópia do muxer do sistema, symlink de `host/fxr/10.0.0`, runtimes `10.0.0` + `8.0.15`, SDK do projeto `8.0.408`, `packs`, `metadata`, `sdk-manifests` e `templates` do root 8.0.408. Fica em `~/Library/Application Support/OpenRidr/hostroot`, é reconstruído por marker quando os inputs mudam e nunca escreve nos roots de SDK nem no `global.json`.
  - **Decisão verificada**: o SDK 10 **não** entra em `hostroot/sdk`. Ele entrou na primeira composição e o MSBuildLocator o registrou como default, produzindo 14 erros `CS0518` (“Predefined type 'System.Object' is not defined”) no projeto net8. Com apenas o SDK 8 em `sdk/` e o runtime 10 em `shared/`, o csharp-ls (net10.0) continua rodando no runtime 10 e o MSBuild registra 8.0.408; `--list-runtimes` prova o runtime 10. O SDK 10 do host é usado para instalar/rodar a ferramenta, não como SDK do projeto.
- **Cliente LSP stdio real** (`crates/host/src/lsp.rs`): framing `Content-Length` em bytes/UTF-8, leitor em thread (notificações antes da resposta de initialize), respostas a `workspace/configuration` (`[{}]`) e demais requests do servidor (`null`), `initialized` + `workspace/didChangeConfiguration`, ids/correlação, stderr drenado para log limitado, initialize com erro encerra o processo e expõe a mensagem de SDK, `shutdown`→`exit`→término observado, restart explícito e descarte de mensagens da sessão antiga.
- **Estados**: `unavailable/starting/loading/ready/partial/awaiting_diagnostics/failed/stopped`, capacidades negociadas resumidas, cobertura `full|partial`, erro, log, sessão/processo/exit/termination. Revogar trust/close/switch encerram o LSP e cancelam instalação em andamento; resposta tardia não muda a sessão atual. Editor nunca bloqueia.
- **Diagnósticos crus**: por URI, com `receivedAt`, `items` (range/severity/code/source/message/codeDescription), `clientRevision` (revisão do documento no cliente no momento da recepção) e `serverVersion: null` — o csharp-ls não envia versão. `didOpen` envia o buffer atual; `didChange` envia texto completo com versão monotônica. Problems UI/obsolescência ficam no ticket 06.
- **UI**: painel de instalação com estado, plano completo, Confirm/Cancel/Retry, SHA/versão verificada e log; linha de análise com estado, capacidades, cobertura, diagnósticos recebidos, log, Start/Restart/Stop/Verbose. Todos os controles são nativos, têm nome/motivo textual e `title`, e os painéis rolam sem esconder o editor. Strings em inglês.
- **Gates**: `tools.install`/`tools.retry`/`lsp.start`/`lsp.restart`/`lsp.didOpen`/`lsp.didChange` passam por `require_trust`; `tools.plan` é leitura, `tools.cancel`/`lsp.stop` são ações de segurança sem trust. `lsp.start` exige ferramenta ready + SDK do projeto resolvido + restore atual; sem restore devolve `restore_required` com ação, sem ferramenta `tool_missing`, sem SDK `sdk_missing`. Restore iniciado para a análise; salvar `.csproj/.sln/global.json` para a sessão (contexto novo).

## Comandos e saídas reais

Logs completos: [`05-install.log`](logs/05-install.log), [`05-install-state.json`](logs/05-install-state.json), [`05-plan-cancel.log`](logs/05-plan-cancel.log), [`05-lsp.log`](logs/05-lsp.log), [`05-diagnostics-broken.json`](logs/05-diagnostics-broken.json), [`05-diagnostics-fixed.json`](logs/05-diagnostics-fixed.json), [`05-restart.json`](logs/05-restart.json), [`05-revoke.json`](logs/05-revoke.json), [`05-hostroot-environment.log`](logs/05-hostroot-environment.log). O driver (HTTP bridge real) está descrito em [`05-drive.log`](logs/05-drive.log).

### Plano, cancelamento, confirmação e instalação

```text
workspace restricted · /…/Tool.csproj
tools.install before trust -> trust_required: no process was started.
tools.retry before trust -> trust_required
lsp.start before trust -> trust_required
lsp.didOpen before trust -> trust_required
plan state planned · id fnv1a64:eba3fffc53a81b8b
plan argv /usr/local/share/dotnet/dotnet tool install csharp-ls --version 0.28.0 \
  --tool-path /…/home/tools/csharp-ls/0.28.staging
tool root exists before cancel? False
processes before cancel: []
cancel state cancelled · Installation plan cancelled; no download and no process were started.
tool root exists after cancel? False
processes after cancel: []
install without confirm -> confirmation_required
install with stale plan id -> confirmation_required: The install plan changed since it was shown…; Nothing was downloaded or executed.
```

Instalação real (4,08 s, cópia com espaços no caminho):

```text
$ /usr/local/share/dotnet/dotnet tool install csharp-ls --version 0.28.0 \
  --tool-path /…/home/tools/csharp-ls/0.28.staging
cwd /var/folders/.../openridr-neutral
source https://api.nuget.org/v3/index.json · destination /…/0.28 · staging /…/0.28.staging
[stderr] An issue was encountered verifying workloads. For more information, run "dotnet workload update".
Skipping NuGet package signature verification.
You can invoke the tool using the following command: csharp-ls
Tool 'csharp-ls' (version '0.28.0') was successfully installed.
Install process exited 0; promoting the staging directory.
Verifying the launcher and its --version output.
$ /…/tools/csharp-ls/0.28/csharp-ls --version
csharp-ls, 0.28.0 (Klebiškis)+72671fc7fa810c158864e72485db68e8b9b25c3d
sha256 0bcd0dbc79b426667961f35bc5d319325e7a6added5d221b30c37cdc3f3e882c
```

Estados observados na mesma instalação: `["downloading", "installing", "verifying", "ready"]`. O `install-state.json` contém `reused: false`, `exit: 0`, `destinationPresent: true`, `staging` ausente, `sha256` acima, `verifiedVersion: "0.28.0"`.

### Host root: SDK do projeto 8 + runtime 10

```text
$ DOTNET_ROOT=hostroot hostroot/dotnet --list-sdks
8.0.408 [/…/hostroot/sdk]
exit=0
$ DOTNET_ROOT=hostroot hostroot/dotnet --list-runtimes
Microsoft.NETCore.App 8.0.15 [/…/hostroot/shared/Microsoft.NETCore.App]
Microsoft.NETCore.App 10.0.0 [/…/hostroot/shared/Microsoft.NETCore.App]
exit=0
$ cd "Order Desk evidence" && DOTNET_ROOT=hostroot hostroot/dotnet --version
8.0.408
exit=0
```

Layout real (symlinks): `sdk/8.0.408 → …/sdks/8.0.408/sdk/8.0.408`, `host/fxr/10.0.0 → /usr/local/share/dotnet/host/fxr/10.0.0`, `shared/Microsoft.NETCore.App/{8.0.15,10.0.0}`, `packs`, `metadata`, `sdk-manifests`, `templates` do root 8. `global.json` foi lido como texto e não mudou. O log de ambiente também mostra o SDK 10.0.100 do sistema e o 8.0.408 do projeto separados.

### Inicialização e diagnósticos reais

```text
[stderr] SDK instances found, as reported by MSBuild:
[stderr] - SDK=".NET Core SDK", Version=8.0.408, MSBuildPath="/…/hostroot/sdk/8.0.408"
[stderr] MSBuildLocator: will register ".NET Core SDK", Version=8.0.408 as default instance
[client] initialize accepted · completion · definition · rename · code actions ·
         diagnostics (pull advertised) · semantic tokens · push diagnostics
[client] server csharp-ls 0.28.0.0
[server] csharp-ls: Loading solution "/…/Order Desk evidence/OrderDesk.sln"...
[stderr] Will use MSBuild props: map [(TargetFramework, net8.0)]
[server] csharp-ls: Finished loading solution "..."
[client] didOpen src/OrderDesk.Cli/Program.cs version 1
[server] publishDiagnostics 1 item(s) · file:///…/Program%20…cs · server version null · client revision 1
```

Publicação real (arquivo com `new OrderReport()`):

```json
{"code":"CS0246","severity":1,"source":"lsp",
 "message":"The type or namespace name 'OrderReport' could not be found …",
 "range":{"start":{"line":5,"character":17},"end":{"line":5,"character":28}},
 "clientRevision":1,"serverVersion":null,"receivedAt":1793160253081}
```

`didChange` com o texto completo corrigido (versão 2) republicou `items: []` com `clientRevision: 2`; a republicação levou ~0,24 s. O estado voltou a `ready`. Nenhuma versão do servidor é inventada: `serverVersion` permanece `null`.

### Restart, shutdown e revogação

```text
restart: state ready · session 2 · logLevel debug
revoke:  state stopped · exit 0 · termination null
[client] shutdown id 2 did not answer within 3000 ms; exiting anyway
[client] stopped: Trust was revoked while analysis or an installation was running…
[client] termination observed; exit 0
```

O `shutdown` não foi respondido dentro de 3 s, mas o `exit` foi aceito e o processo terminou com **exit 0** observado; por isso não houve SIGTERM/SIGKILL (`termination: null`). O processo aparece `exited` no painel. Reiniciar cria sessão 2, descarta as publicações da sessão 1 (nova sessão) e reenvia `didOpen` para os buffers abertos via UI.

### UI real

Screenshots do mesmo build servido ao Chrome (Playwright, Chrome 155): [`screenshots/05-csharp-ls-install.png`](screenshots/05-csharp-ls-install.png) (plano/estado/log de instalação) e [`screenshots/05-csharp-ls.png`](screenshots/05-csharp-ls.png) (LSP ready, capacidades, diagnósticos e editor com buffer). O teste E2E também edita e salva depois de matar o servidor e depois de revogar trust; o editor permanece funcional.

## Testes automatizados

```text
$ cargo test -p openridr-host
running 28 tests   (unit: framing split/merged/UTF-8, hostroot runtime pick, plan id,
                    log redaction/truncation, SDK e restore existentes)
test result: ok. 28 passed; 0 failed; 0 ignored

running 40 tests   (boundary: gate/trust, plano/cancelamento inerte, mudança material,
                    instalação real + verify + reuso, cancelamento real de instalação,
                    hostroot idempotente, restore_required,
                    SDK ausente/ferramenta ausente, LSP real + diagnostics + didChange +
                    restart + revogação, falha de initialize com kill, testes 01–04)
test result: ok. 40 passed; 0 failed; 0 ignored
```

Log: [`05-cargo-test.log`](logs/05-cargo-test.log). A instalação real usa a origem NuGet.org e um `OPENRIDR_HOME` temporário; o teste de LSP usa o app home real e reutiliza a ferramenta verificada. O teste de hostroot prova `--list-sdks` = 8.0.408, `--list-runtimes` contém 10.0.0 e `--version` na fixture = 8.0.408.

```text
$ bun run test:e2e
  ✓ install plan is exact, cancel is inert, and the real install reaches verifying
  ✓ analysis loads, republishes diagnostics, survives a crash and stops on revoke
  ✓ … 17 testes dos tickets 01–04 continuam verdes
  19 passed (50.1s)
```

Log: [`05-playwright.log`](logs/05-playwright.log). O host E2E usa `OPENRIDR_HOME=$TMPDIR/openridr-e2e-home`; o teste de instalação remove a ferramenta só nesse home isolado e observa `downloading → installing → verifying → ready`; o teste de análise restaura, carrega o LSP, vê CS0246, corrige com didChange, mata o processo, reinicia e revoga. Capturas em [`screenshots/05-csharp-ls-install.png`](screenshots/05-csharp-ls-install.png) e [`screenshots/05-csharp-ls.png`](screenshots/05-csharp-ls.png).

```text
$ bun run build
✓ built in 753ms
```

Log: [`05-build.log`](logs/05-build.log). O aviso de chunk >500 kB é do CodeMirror.

## Cobertura do ticket e dos gates

| Item do ticket / gate | Resultado | Evidência |
|---|---|---|
| Plano exato, Cancel inerte, mudança material pede nova confirmação, trust ≠ confirmação | Plano com versão/fonte/OS/arch/destino/argv/cwd/permissões; `planId` material; cancel sem processo/dir; `confirmation_required` sem download; install/retry/LSP atrás de `require_trust` | `05-plan-cancel.log`, `05-install-state.json`, `cargo-test`, E2E 1 |
| G3 instalador NuGet: origem/destino/extração/limites, sem global/elevação, sem sobrescrever ferramenta válida | HOME isolado com fonte default `nuget.org`; destino/staging no app home; argv separado; staging só promovido após exit 0; verificação antes de ready; reuso de instalação válida; assinatura NuGet declarada como não verificada | `05-install.log`, `05-install-state.json`, `cargo-test`, E2E 1 |
| Estados downloading/installing/verifying/ready/cancelled/failed, log persistente, retry, cancelamento real, sem percentual | Estados reais observáveis; log stdout/stderr limitado preservado; cancel TERM→KILL; retry revalida; staging parcial rotulado; sem percentual | `05-install.log`, `05-install-state.json`, `05-plan-cancel.log`, `cargo-test`, E2E 1 |
| Lançar csharp-ls com host 10 e projeto 8.0.408/rollForward disable, sem reescrever global.json; SDK ausente/incompatível com log/ação | Hostroot composto; `--list-runtimes` 10.0.0, `--list-sdks` 8.0.408, `--version` 8.0.408; `global.json` não muda; falha de initialize devolve a mensagem de `hostfxr_resolve_sdk2`/8.0.408 e mata o processo | `05-hostroot-environment.log`, `05-lsp.log`, `cargo-test` (`initialize_failure…`) |
| Estados de análise, capacidades/ordem, cobertura parcial, aguardando diagnósticos, log/reinício sem travar editor | `starting/loading/ready/partial/awaiting/failed/stopped`; capacidades negociadas; `didOpen`/`didChange`; restart sessão 2; editor funcional após falha/revogação | `05-lsp.log`, `05-restart.json`, `05-revoke.json`, E2E 2 |
| Revogar encerra serviço, impede reinício, ignora resposta tardia; G1–G5 e MIT/acessibilidade | Revoke → shutdown/exit, exit 0, `stopped`; `lsp.start` volta a `trust_required`; nova sessão substitui a antiga; csharp-ls MIT registrado; controles nativos com motivo textual | `05-revoke.json`, `05-revoke.log`, `cargo-test`, E2E 2 |
| AC-02 coexistência host 10/projeto 8 | Runtime 10.0.0 + SDK 8.0.408 no hostroot; MSBuild registra 8.0.408 | `05-hostroot-environment.log`, `05-lsp.log` |
| AC-04/05/06 instalação/consentimento/verificação/origem | Plano/confirmação/estados/log/retry/cancel/verify; sem global; integridade limitada declarada | E2E 1, `cargo-test` |
| AC-09/15 cobertura/diagnósticos sem falsa limpeza | Cobertura `partial` quando avaliação não está ok/stale; diagnósticos por URI+revisão do cliente; `serverVersion:null`; publicação vazia só do documento | `05-diagnostics-broken/fixed.json`, `cargo-test` |
| AC-11 falha preserva edição | Após matar o LSP e após revogar, buffers continuam editáveis/salváveis | E2E 2, editor na captura |
| AC-16 carga/falha/log/reinício/coexistência | Initialize real, log, restart, falha de SDK, hostroot 10/8 | `05-lsp.log`, `05-restart.json`, `cargo-test` |
| G1 — trust/fronteira | Todos os métodos executáveis validam trust; revoke para LSP/instalação; respostas tardias descartadas; editor não executa | `05-plan-cancel.log`, `cargo-test`, E2E 2 |
| G2 — contexto/integridade | Sessão/contexto no LSP; restart troca `Arc`/sessão; `exit 0` observado; argv/cwd/env explícitos; sem interpolação de shell | `05-restart.json`, `05-revoke.json`, `cargo-test` |
| G4 — UI/privacidade | Painéis em inglês, estado textual + foco nativo + motivo em `title`; log local; sem telemetria/upload; screenshot | E2E, capturas |
| G5 — validação/proveniência | OS/arch/versões, SDK host/projeto, TFM, comandos/logs/revisões/exits e limitações por ambiente | esta página + logs |
| AC-32/33/34 (parcial) | Superfície acessível e evidência desta fatia; licença MIT do csharp-ls registrada; auditoria de distribuição segue no ticket 18 | `05-lsp.log` (MIT), esta página |

## Limitações e pendências honestas

- **Windows x64 e macOS Intel não executados**. O hostroot usa symlinks e o LSP usa paths/URIs; a ordem de descoberta de SDK e CRLF/posições no Windows não têm prova. O código Windows existe (`symlink_dir` guardado por `cfg`, `TerminateProcess`) mas não foi validado; o ticket 18 deve repetir instalação/LSP/falha/revogação nesses ambientes.
- **SDK 10 fora de `hostroot/sdk`**: a receita inicial com os dois SDKs no mesmo diretório fez o MSBuildLocator escolher 10.0.100 e gerar `CS0518` no net8. Esta implementação mantém o **runtime** 10 no hostroot e apenas o SDK 8 em `sdk/`, o que é a separação necessária para o csharp-ls. O SDK 10 do sistema continua sendo o host de instalação/execução da ferramenta; a decisão está registrada em [decisions.md](../decisions.md).
- **Assinatura NuGet**: o `dotnet tool install` real imprime `Skipping NuGet package signature verification.`; o host verifica executável/versão e registra SHA-256 do launcher, não uma assinatura ou hash oficial do pacote. O HOME isolado fixa a fonte default `nuget.org`, mas não é um mirror verificado de conteúdo. Um `NuGet.Config` de escopo de máquina (`/etc`, `%ProgramData%`) pode acrescentar feeds; o host não o inspeciona nem o registra. Não há pin de hash do `.nupkg`.
- **`shutdown` do csharp-ls** não respondeu em 3 s nesta sessão; o `exit` foi aceito e o processo saiu com exit 0. O host registra o pedido, o exit observado e mantém o log; se o processo não sair, o grupo é terminado (TERM→KILL) e o painel mostra o resultado.
- **Diagnósticos**: apenas push (`publishDiagnostics`) é consumido; pull diagnostics e analyzers não foram exercitados. A versão do servidor não existe no payload; `clientRevision` é a revisão do cliente na recepção, portanto uma publicação atrasada pode ser atribuída à revisão mais nova — o limite é explícito. `didClose` não é enviado ao fechar aba; a próxima sessão/ticket 06 deve decidir a política.
- **Problems UI, obsolescência e navegação** ficam no ticket 06; esta fatia guarda as publicações cruas e não cria lista/contagens de arquivos.
- **E2E** usa Chrome real contra Vite + host-http, não a janela Tauri nativa (limitação de captura TCC já registrada); o host E2E usa `OPENRIDR_HOME` isolado e o app home real é verificado à parte. O teste de instalação só remove a ferramenta no home isolado; se o host reutilizar o app home real, ele apenas prova o reuso.
- **Acessibilidade**: controles nativos, `title`/texto de motivo, foco visível; não houve teste com leitor de tela nem auditoria WCAG completa.
- **Licença**: csharp-ls 0.28.0 é MIT; o próprio servidor loga “released under MIT license and is not affiliated with Microsoft Corp.”; nenhum código-fonte da ferramenta foi incorporado. A auditoria de distribuição de assets permanece no ticket 18.
