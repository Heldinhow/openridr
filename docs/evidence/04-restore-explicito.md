# Evidência — Ticket 04: restore explícito e recarga do contexto

Implementação local em 28/10/2026. Fonte normativa: [ticket 04](../tickets/04-restore-explicito.md), [SPEC](../SPEC.md) P0.1/P0.2, “Estado, erros e processos”, “Avaliação e ferramentas” e Testing Decisions, e gates G1–G5 do [índice de tickets](../tickets/README.md).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Node / Bun | v26.10.0 / 1.3.9 |
| Rust | cargo 1.99.0, rustc 1.99.0 |
| Chrome / Playwright | Google Chrome 155.0.8059.85 (canal `chrome`) / 1.64.1 |
| SDK do host | 10.0.100 (e 8.0.416) em `/usr/local/share/dotnet` |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` (fixado por `global.json`, `rollForward: disable`) |
| Fixtures | `fixtures/OrderDesk`; cópias temporárias para restore/falha/cancelamento; nenhum `fixtures/` foi restaurado/mutado |
| Base de código | `f7d92a1` (ticket 03) + esta fatia |

Log: `logs/04-environment.log` *(evidência removida do repositório público)*. Nenhum SDK/ferramenta foi instalado nesta tarefa.

## O que foi construído

- **Ação explícita e único caminho de restore** (`crates/host/src/restore.rs`): `restore.start`/`restore.retry` exigem trust e SDK do projeto resolvido; executam exatamente `[<sdk-root>/dotnet, "restore", <sln|csproj>, "-nologo", "-v:minimal"]` com argumentos separados, cwd na raiz do workspace, `DOTNET_ROOT` da raiz do SDK do projeto, `DOTNET_MULTILEVEL_LOOKUP=0` e grupo de processos próprio. Não existe restore dentro de `evaluation.run` nem escape `--no-restore`; abrir/avaliar nunca cria `obj/project.assets.json` (teste Rust + E2E).
- **Estado observável**: `restore` no snapshot com `state` `idle|running|ok|failed|cancelled`, `scope` (`solution|project`), `target`, `command` (argv), `cwd`, `sdkRoot`, `sdkVersion`, `revision`, `workspaceId`, `contextId`, `processId`, `exit` (`null` = desconhecido), `termination {requested, observed, exit, killedAfterGrace}`, `awaitingDecision`, `startedAt/finishedAt`, `sealed/staleReason` e `projects` com `restored|not_restored|unknown`.
- **Processo real e log**: o child entra no mesmo `Registry` dos demais processos (terminate/keep/awaiting iguais). Duas threads drenam stdout e stderr linha a linha para um log limitado às últimas **400 linhas / 60k caracteres**; quando trunca, `logTruncated: true` e o texto começa com “log truncated: N earlier line(s) not kept”. stderr aparece como `[stderr] …`; stdout fica cru. Ambiente nunca é logado e credenciais em URL de feed são mascaradas (`https://***@host/...`).
- **Estado por projeto**: derivado de `obj/project.assets.json` vs. mtime do `.csproj` **e** de `obj/project.nuget.cache` (`success: true`). Isso importa porque o NuGet reescreve o assets mesmo em falha (NU1100); sem o cache, a falha pareceria “restored”. Assets ausente/velho ou cache `success:false` = `not_restored`; cache ausente/ilegível = `unknown`.
- **Falha/parcial**: mantém editor, modelo anterior (rotulado stale) e log bruto; banner “Dependencies incomplete” com marcadores `restore-incomplete-<Project>` e `Restore incomplete — package dependencies and analysis are incomplete`; `Retry restore` explícito.
- **Sucesso**: marca a avaliação como `stale` com motivo “Restore completed; reload the evaluated model…”, mantém o TargetPath antigo apenas como “Previous evaluation (stale)”, e oferece **Reload evaluated model**, que chama `evaluation.run` com a configuração/TFM atuais. A resposta tardia de uma avaliação anterior continua rejeitada pelo token/revisão existentes.
- **Fronteiras de workspace**: revogar trust, fechar ou trocar workspace durante um restore chama `registry.terminate` (TERM → espera ~2 s → KILL), registra `termination`, e **sela** o registro; escritas tardias do processo antigo são descartadas. Se o processo continuasse vivo após o grace, o registro ficaria `sealed` + `awaitingDecision` e o painel de processos manteria a decisão pendente — sem alegação de limpeza.
- **Política para consumidores futuros**: `restore::require_restored` é chamada por `build.start`, `run.start`, `debug.start`, `tests.discover` e `tests.run` antes de qualquer ferramenta. Sem assets atuais/falhos devolve `restore_required` com `details.projects` e `details.action = {method:"restore.start", params:{}}`; nunca restaura. Os consumidores reais são tickets 11/12/16/17.
- **UI**: painel Restore sempre visível com estado textual, escopo (Solution/Project + arquivo), alvo, comando (argv completo), cwd, SDK/versão, log stdout/stderr, Cancel durante execução, Retry após falha/cancelamento, Reload após sucesso, marcador por projeto incompleto e motivo textual quando desabilitado. Os avisos ficam num contêiner rolável limitado a 46vh para o editor continuar utilizável.

## Comandos e saídas reais

Logs completos: `logs/04-restore-success.log` *(evidência removida do repositório público)*, `logs/04-restore-failure.log` *(evidência removida do repositório público)*, `logs/04-restore-cancel.log` *(evidência removida do repositório público)*. Todos usam o host real via ponte HTTP de desenvolvimento, cópia temporária da fixture e o SDK 8.0.408.

### Sucesso (exit 0, log real, assets criados)

```text
$ workspace.open path=<tmp>/Order Desk restore/OrderDesk.sln
{"ok":true,"mode":"restricted","restore":"idle"}
$ trust.grant
{"ok":true,"projectSdk":"8.0.408","root":"/…/OpenRidr/sdks/8.0.408"}
$ evaluation.run (opening/evaluating never restores)
{"ok":true,"state":"ok","revision":1,"restored":["not_restored","not_restored","not_restored"],"restoreState":"idle","restoreRevision":0}
$ find $copy -name project.assets.json   # before clicking Restore
(no output)
$ restore.start
{"state":"running","scope":"solution","revision":1,
 "command":["/…/OpenRidr/sdks/8.0.408/dotnet","restore","/…/Order Desk restore/OrderDesk.sln","-nologo","-v:minimal"],
 "cwd":"/…/Order Desk restore","sdkVersion":"8.0.408","sdkRoot":"/…/OpenRidr/sdks/8.0.408","processId":1}
```

Estado terminal e log (trecho real; caminhos encurtados só aqui):

```text
state: ok · exit: 0 · logTruncated: false
  Determining projects to restore...
  Restored /…/Order Desk restore/src/OrderDesk.Core/OrderDesk.Core.csproj (in 36 ms).
  Restored /…/Order Desk restore/src/OrderDesk.Cli/OrderDesk.Cli.csproj (in 39 ms).
  Restored /…/Order Desk restore/tests/OrderDesk.Tests/OrderDesk.Tests.csproj (in 153 ms).
projects: Cli restored, Core restored, Tests restored
$ stat -f '%Sm %N' …
2026-10-28 00:12:38 /…/src/OrderDesk.Cli/obj/project.assets.json
2026-10-28 00:12:38 /…/src/OrderDesk.Core/obj/project.assets.json
2026-10-28 00:12:38 /…/tests/OrderDesk.Tests/obj/project.assets.json

$ evaluation.run   # explicit reload
{"ok":true,"state":"ok","revision":2,"stale":false,
 "cliTargetPath":"/…/src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll",
 "restored":[{"name":"OrderDesk.Cli","restored":"restored"}, …]}
$ build.start   # policy passes, consumer itself is ticket 11
{"ok":false,"error":{"code":"invalid_state","message":"build.start is not implemented in this build."}}
```

### Falha controlada (exit 1), política e retry (exit 0)

Cópia com `NuGet.config` sem fontes e `xunit` `9.9.999` (versão inexistente). O restore falha em ~0,1 s, sem rede:

```text
$ restore.start → state running, exit null
$ workspace.state
state: failed · exit: 1
log:   Determining projects to restore...
/…/tests/OrderDesk.Tests/OrderDesk.Tests.csproj : error NU1100: Unable to resolve 'xunit (>= 9.9.999)' for 'net8.0'. [/…/OrderDesk.sln]
  Restored /…/src/OrderDesk.Core/OrderDesk.Core.csproj (in 35 ms).
  Restored /…/src/OrderDesk.Cli/OrderDesk.Cli.csproj (in 38 ms).
  Failed to restore /…/tests/OrderDesk.Tests/OrderDesk.Tests.csproj (in 77 ms).
projects: Cli restored, Core restored,
          Tests not_restored — "The last restore of this project failed (obj/project.nuget.cache reports success: false); dependencies are incomplete."
evaluationStale: true
staleReason: "Restore failed; package dependencies and analysis may be incomplete."

$ build.start
{"ok":false,"error":{"code":"restore_required",
 "message":"build.start needs current restore assets and OpenRidr never restores implicitly; run the explicit restore first.",
 "details":{"consumer":"build.start",
            "projects":[{"name":"OrderDesk.Tests","restored":"not_restored",...}],
            "action":{"method":"restore.start","params":{}}}}}

$ fix the copy (xunit back to 2.9.2) and restore.retry
state: ok · exit: 0 · revision: 3 · projects: Cli/Core/Tests restored
2026-10-28 00:12:41 /…/tests/OrderDesk.Tests/obj/project.assets.json
```

### Cancelamento de um restore realmente pendurado

Servidor TCP local que aceita a conexão e nunca responde; `NuGet.config` aponta só para ele. O restore fica em `running` por >2 s e é cancelado:

```text
$ workspace.state (running, real stdout)
"Determining projects to restore..."
"warning NU1803: You are running the 'restore' operation with an 'HTTP' source, 'http://127.0.0.1:59601/v3/index.json'…"

$ restore.cancel   (00:12:43)
state: "cancelled" · exit: 143 · awaitingDecision: false
termination: {"requested":"SIGTERM","observed":"exited","exit":143,"killedAfterGrace":false}
log: preserved exactly as above
projects: Hang not_restored
processes: [{"id":4,"state":"exited","exit":143,
             "termination":{"requested":"SIGTERM","observed":"exited","killedAfterGrace":false}}]

$ restore.cancel again
{"ok":false,"error":{"code":"invalid_state","message":"No restore is running."}}
```

`exit 143` é o que o host **observou** do dotnet neste OS; o campo continua `null` quando o registro não recebe código (não há zero presumido). `killedAfterGrace: false` porque o TERM bastou.

## Cobertura do ticket e dos gates

| Item do ticket / gate | Resultado | Evidência |
|---|---|---|
| Restore só após trust e SDK resolvido; comando/escopo/cwd/SDK visíveis; abrir/avaliar não restaura | Gate único; argv exato `dotnet restore <sln> -nologo -v:minimal`; `evaluation.run` não contém restore; nenhum assets antes do clique | `cargo-test` (19+33), `04-restore-success.log`, E2E 1 |
| Andamento real, stdout/stderr, resultado, erro, cancelamento, retry; falha preserva edição e sinaliza parcial | Log bruto (400 linhas/60k, truncamento declarado); `running/ok/failed/cancelled`; `exit` observado; retry real exit 0; editor salva após falha; marcadores por projeto | `04-restore-*.log`, E2E 1/2, `cargo-test` |
| Sucesso invalida avaliação/TargetPath e recarga é explícita; resposta tardia não reativa outro contexto | `evaluation.stale` + motivo; TargetPath “Previous evaluation (stale)”; Reload chama `evaluation.run` (revisão 2, `restored`); selo em revoke/close/switch | `04-restore-success.log`, `cargo-test`, E2E 1 |
| Política de build/run/test na boundary, sem restore implícito | `restore_required` com ação explícita e lista de projetos; nenhuma ferramenta roda/restaura | `04-restore-failure.log`, `cargo-test` |
| Cancelamento/revogação distinguem pedido, processo observado e exit conhecido/unknown; logs e remanescentes visíveis; sem credenciais de feed/upload | TERM → observed exited, exit 143; registro sealed; processo visível; URL userinfo mascarada; sem env no log; sem upload | `04-restore-cancel.log`, `cargo-test` |
| G1 — trust/fronteira | `trust_required` antes de trust (UI/HTTP/teste); revoke/troca/fechamento encerram e reportam; nenhum restore implícito | HTTP, `cargo-test`, E2E 3 |
| G2 — contexto/integridade | Token/workspace/contexto no registro; selo descarta resultado tardio; `exit` desconhecido não vira zero; argv separado, cwd/env explícitos, grupo de processos | `cargo-test`, logs |
| G3 — consentimento/instaladores | Nenhum download/instalação/elevação nesta fatia | ambiente |
| G4 — UI/privacidade | UI inglesa, escopo/comando/estado/log textuais, foco nativo, cancelar/retry/reload com motivo; editor utilizável; sem telemetria | E2E, capturas |
| G5 — validação/proveniência | OS/arch/versões, SDK host/projeto, TFM, comandos, logs, exit, revisões e limitações por ambiente | esta página + logs |

ACs tocados: **AC-03** (restore explícito, falha/log/retry e política), **AC-09** (restore parcial sem “solução limpa”), **AC-10** (invalidação/recarga/TargetPath), **AC-15 parcial** (restore incompleto não vira garantia de análise global), **AC-01/07** (gate e revogação deste consumidor), **AC-32/33** (superfície acessível e evidência desta fatia).

## Testes automatizados

```text
$ cargo test -p openridr-host
running 19 tests   (unit: argv/redação de credenciais, limite e truncamento do log,
                    assets/cache tri-estado, política restore_required)
test result: ok. 19 passed; 0 failed; 0 ignored
running 33 tests   (boundary: gate, comando real, assets reais, sem restore implícito,
                    política, cancelamento real pendurado, revogação/troca, falha+retry,
                    projeto específico da solução)
test result: ok. 33 passed; 0 failed; 0 ignored
```

Log: `logs/04-cargo-test.log` *(evidência removida do repositório público)*. Os testes reais usam SDK 8.0.408 e cópias temporárias; o cancelamento usa um servidor TCP local que aceita e atrasa (sem mock do host).

```text
$ bun run test:e2e
  ✓ restore is explicit, streams a real log and reloads the evaluated model
  ✓ a failed restore keeps its log, the editor and a working retry
  ✓ a direct restore call is denied before trust
  ✓ … 14 testes dos tickets 01–03 continuam verdes
  17 passed (28.8s)
```

Log: `logs/04-playwright.log` *(evidência removida do repositório público)*. Capturas: `screenshots/04-restore.png` *(evidência removida do repositório público)* (sucesso + comando + log stdout real + reload disponível) e `screenshots/04-restore-failure.png` *(evidência removida do repositório público)* (exit 1, NU1100 no log, projeto incompleto, retry). Ambas usam cópias em diretório temporário; `fixtures/` não foi restaurada nem alterada.

### Frontend

```text
$ bun run build
✓ built in 737ms
```

Log: `logs/04-build.log` *(evidência removida do repositório público)*. O aviso de chunk >500 kB é do CodeMirror.

## Checklist por projeto (o que é derivado, não inventado)

| Projeto | Depois do sucesso | Depois da falha | Após retry |
|---|---|---|---|
| OrderDesk.Cli | `restored` (assets 00:12:38) | `restored` (assets escritos antes da falha) | `restored` |
| OrderDesk.Core | `restored` (assets 00:12:38) | `restored` | `restored` |
| OrderDesk.Tests | `restored` (assets 00:12:38) | `not_restored` (cache `success:false`, NU1100) | `restored` (00:12:41) |

Não há percentual/progresso numérico: o estado é `running` + linhas reais do NuGet + resultado observado.

## Limitações e pendências honestas

- **Windows x64 não foi executado**. `dotnet.exe`, criação de grupo/TerminateProcess do Windows, caminhos CRLF e o comportamento de cancelamento não têm evidência aqui; o ticket 18 deve repetir restore real, falha e cancelamento em Windows.
- **macOS Intel não foi exercitado**; a matriz P0 continua pendente (ticket 18).
- **Cancelamento observado**: neste macOS o dotnet terminou com `exit 143` após SIGTERM. O caminho `exit null` (sem código) está implementado e testado no registro de processos do ticket 02, mas não foi observado neste restore; a UI mostra `exit unknown` quando ele ocorrer.
- **Restore não garante análise global**: assets atualizados + `nuget.cache success` é a declaração do próprio NuGet; pacotes corrompidos/analyzers ausentes só aparecem em build/LSP (tickets 05/11).
- **Cancelamento pode deixar artifacts parciais** (`obj/*.nuget.*`, assets incompleto). O host não os apaga; o estado por projeto deriva `not_restored`/`unknown` e nunca declara sucesso por ausência de erro.
- **Log limitado**: últimas 400 linhas/60k caracteres; o restante é descartado com aviso. Não é transcrição completa.
- **Troca/fechamento de workspace** descarta o registro selado da UI (o log daquele restore não fica visível no workspace novo), mas o processo permanece no painel de processos com o término observado. Revogar trust mantém o registro visível e selado.
- **UI só oferece restore do workspace inteiro** (solution ou project). `restore.start {project}` existe no host e tem teste Rust; botão por projeto fica para quando houver demanda.
- **Consumidores reais** (build/run/debug/testes) são stubs: a política `restore_required` é provada na boundary; a execução real e a chamada da mesma política ficam nos tickets 11/12/16/17.
- **Playwright usa Chrome real contra Vite + host-http**, não a janela Tauri nativa (limitação de captura TCC já registrada nos tickets 01–03). O mesmo `dispatch` é chamado.
- **Acessibilidade**: controles nativos, `aria-label`/título e estado textual; não houve teste com leitor de tela nem auditoria WCAG completa.
- **Credenciais de feed**: o host não loga ambiente e mascara userinfo em URLs; não foi testado um feed real com credenciais embutidas.
