# Evidência — Ticket 11: compilar contexto e navegar erros no output

Implementação local em 28/10/2026, base `54ceddc` (ticket 10). Fonte normativa: [ticket 11](../tickets/11-build.md), [SPEC](../SPEC.md) AC-03/10/14/15/17/18/19/20/01/07/32/33, “Estado, erros e processos”, “Avaliação e ferramentas” e gates G1–G5 do [índice de tickets](../tickets/README.md).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| Chrome / Playwright | Google Chrome 155.0.8059.85 (canal `chrome`) / 1.64.1 |
| SDK do host | 10.0.100 em `/usr/local/share/dotnet` |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408`, `global.json` com `rollForward: disable` |
| Fixture | `fixtures/OrderDesk`, sempre copiada para diretório temporário; `Program.cs`/`.csproj` alterados só nas cópias |
| App home da evidência | temporário (`OPENRIDR_HOME=/tmp/openridr-11-home.*`); o E2E usa `$TMPDIR/openridr-e2e-home` |

Logs: `logs/11-environment.log` *(evidência removida do repositório público)*, `logs/11-build-restore-required.json` *(evidência removida do repositório público)*, `logs/11-build-ok.log` *(evidência removida do repositório público)* + `11-build-ok.json` *(evidência removida do repositório público)*, `logs/11-build-failed.log` *(evidência removida do repositório público)* + `11-build-problems.json` *(evidência removida do repositório público)*, `logs/11-build-library-rejected.json` *(evidência removida do repositório público)*, `logs/11-build-cancel.log` *(evidência removida do repositório público)*, `logs/11-build-revoke.json` *(evidência removida do repositório público)* + `11-build-revoke-late.json` *(evidência removida do repositório público)*, `logs/11-cargo-test.log` *(evidência removida do repositório público)*, `logs/11-playwright.log` *(evidência removida do repositório público)*, `logs/11-build.log` *(evidência removida do repositório público)*, `logs/11-tauri-build.log` *(evidência removida do repositório público)*. Nenhum SDK/ferramenta foi instalado; nenhum arquivo de `fixtures/` foi restaurado/alterado.

## O que foi construído

- **Host** (`crates/host/src/build.rs` + `lib.rs`): `build.start {project?, configuration?, targetFramework?, savedNote?}` e `build.cancel`, com `build` no snapshot (`state`, `project{name,path,kind,targetPath}`, configuração/TFM, `command`, `cwd`, `sdkRoot`/`sdkVersion`, `log`/`logTruncated`, `exit`, `termination`, `processId`, `awaitingDecision`, `revision`, `buildId`, `workspaceId`/`contextId`, tempos, `stale`/`staleReason`, `sealed`, `savedNote`, `problems`, `problemsTruncated`).
- **Gate e seleção**: trust → SDK do projeto → `restore::require_restored` (mesma resposta `restore_required` com `details.action = {method:"restore.start"}`) → avaliação atual (`ok|partial`, não stale, mesmo workspace/contexto) → `TargetPath` do projeto. `configuration`/TFM diferentes do avaliado são recusados com pedido explícito de `evaluation.run`. Startup só aceita `kind:"console"`: `library`/`test`/`unknown` recebem motivo; com mais de um console e sem `project`, a resposta é `invalid_state` com `details.reason:"ambiguous"` e `details.candidates`.
- **Processo**: argv separado `dotnet build <csproj> -c <Configuration> -nologo -v:minimal --no-restore` (+ `-p:TargetFramework=<tfm>`), cwd na raiz, `DOTNET_ROOT` do projeto e grupo próprio; o child entra no `Registry` compartilhado (`kind:"build:<Project>"`). Um build por vez; restore em andamento bloqueia. Duas threads drenam stdout/stderr para o `LogBuffer` (400 linhas/60k; stderr `[stderr]`; truncamento declarado) e o log é preservado após falha/cancelamento.
- **Exit e cancelamento**: `exit` só existe se observado (`null` ≠ 0; `ok` só com 0). `build.cancel` é ação de segurança sem trust, pede término do grupo e reporta `requested`/`observed`/`exit`; o registro do processo continua visível.
- **Fronteiras**: revogar/fechar/trocar termina o grupo e sela o registro, mantendo o processo no painel; revogar mostra o registro selado, fechar/trocar voltam a UI para `build.idle`. Linhas e exit tardios de um registro selado são descartados. Avaliação/restore/SDK que mudam o contexto marcam `build.stale` com motivo.
- **Diagnósticos**: parser textual conservador (`path(line,col): error|warning CODE: message [project]`, aceita 2 ou 4 números), posição 0-based só quando impressa, `path` resolvido/canonicalizado dentro do workspace (fora é rejeitado), deduplicação do resumo de falha, cap de 200 com aviso. `source:"dotnet"`, `category:"unknown"`, `severityRaw` 1/2, id `<buildId>:<index>`. Nada é ingerido no store LSP; a aba Problems tem a seção `Build · dotnet` separada de `LSP · csharp-ls`.
- **Save policy**: o build lê o disco. A UI pergunta “Save all and build”, “Build saved content (N unsaved buffers excluded)” ou “Cancel”; nunca salva sozinha. `savedNote` registra no estado o que foi usado; o host não grava buffer.
- **UI**: aba Build com startup/Configuration/TFM ligados ao contexto avaliado, Build + Cmd/Ctrl+F9, diálogo de buffers sujos, aviso com `Restore now` quando faltam assets, estado/exit/comando/cwd/SDK/conteúdo salvo/log bruto, cancelar e retry. Problems mostra os itens Build com severidade/código/mensagem/categoria/localização/documento/estado; item com posição abre no `line:character` real, sem posição abre o documento sem mover o cursor; editar depois do build marca o snapshot como **obsoleto**, não corrigido.

## Comandos e saídas reais

Driver real via ponte HTTP de desenvolvimento, cópias temporárias e SDK 8.0.408. Os caminhos abaixo foram encurtados só na prosa; os logs guardam os caminhos completos.

### Restore pendente bloqueia com a ação explícita

```text
$ build.start (antes de qualquer restore)
{"code":"restore_required",
 "message":"build.start needs current restore assets and OpenRidr never restores implicitly; run the explicit restore first.",
 "details":{"consumer":"build.start",
            "projects":[{"name":"OrderDesk.Cli","restored":"not_restored",...}, ...],
            "action":{"method":"restore.start","params":{}}}}
```

Nenhum `obj/project.assets.json` foi criado por avaliação/build.

### Build ok (exit 0, log bruto)

```text
$ build.start  →  workspace.state
state: ok
exit: 0
buildId: build:1:1
project: OrderDesk.Cli (console) src/OrderDesk.Cli/OrderDesk.Cli.csproj
targetPath: /…/Order Desk build evidence/src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll
configuration: Debug · targetFramework: not selected
command: /…/OpenRidr/sdks/8.0.408/dotnet build /…/src/OrderDesk.Cli/OrderDesk.Cli.csproj -c Debug -nologo -v:minimal --no-restore
cwd: /…/Order Desk build evidence
sdkVersion: 8.0.408 · sdkRoot: /…/OpenRidr/sdks/8.0.408
savedNote: Built saved content; no unsaved buffers were open.
problems: 0
--- raw log ---
  OrderDesk.Core -> /…/OrderDesk.Core.dll
  OrderDesk.Cli -> /…/OrderDesk.Cli.dll
Build succeeded.
    0 Warning(s)
    0 Error(s)
Time Elapsed 00:00:00.81
```

Log: `logs/11-build-ok.log` *(evidência removida do repositório público)*.

### Build falho real (CS0246, exit 1, problema navegável)

`Program.cs` recebeu `new Missing();` no disco antes do build:

```text
state: failed
exit: 1
buildId: build:1:2
problems: 1
— raw log —
…/src/OrderDesk.Cli/Program.cs(6,5): error CS0246: The type or namespace name 'Missing' could not be found (are you missing a using directive or an assembly reference?) [/…/OrderDesk.Cli.csproj]
Build FAILED.
```

O item publicado (`logs/11-build-problems.json`) tem `severity:"error"`, `severityRaw:1`, `code:"CS0246"`, `source:"dotnet"`, `category:"unknown"`, `path:"src/OrderDesk.Cli/Program.cs"`, `position:{"line":5,"character":4}` (0-based), `range:null`, `buildId:"build:1:2"`, `revision:2` e `raw` com a linha MSBuild completa. A posição 1-based impressa (6,5) corresponde ao erro real na cópia. O `raw` é a única forma de recuperar a linha não interpretada.

Biblioteca como startup é recusada:

```text
$ build.start project=src/OrderDesk.Core/OrderDesk.Core.csproj
{"code":"invalid_state",
 "message":"OrderDesk.Core is a library project; only a console executable project can be the build startup project."}
```

### Cancelamento de um build realmente pendurado

Cópia com `<Target Name="Hang" AfterTargets="Build"><Exec Command="sleep 30" /></Target>`. O build chegou a imprimir o `.dll` e ficou no target; `build.cancel` pediu término:

```text
state: cancelled
exit: 143
buildId: build:2:3
termination: {"requested":"SIGTERM","observed":"exited","exit":143,"killedAfterGrace":false}
problems: 0
— raw log (preservado) —
  OrderDesk.Core -> …/OrderDesk.Core.dll
  OrderDesk.Cli -> …/OrderDesk.Cli.dll
— registry process —
{"id":5,"kind":"build:OrderDesk.Cli","state":"exited","exit":143,
 "termination":{"requested":"SIGTERM","observed":"exited","exit":143,"killedAfterGrace":false}}
```

`exit 143` é o que o registro observou; o log não foi apagado. Log: `logs/11-build-cancel.log` *(evidência removida do repositório público)*.

### Revogação sela o build

Segundo build pendurado, revogado durante a execução:

```text
build: state cancelled · sealed true · stale true
staleReason: Trust was revoked while this build was running; it was stopped and its result is not current.
termination: SIGTERM → observed exited · exit 143
processo: exited, exit 143

0,7 s depois: state cancelled · sealed true · finishedAt igual · logHasLateLines false · trust.granted false
```

Logs: `logs/11-build-revoke.json` *(evidência removida do repositório público)* e `logs/11-build-revoke-late.json` *(evidência removida do repositório público)*.

## UI real (Playwright/Chrome 155, Vite + host-http)

Dois testes novos com cópias temporárias e dotnet real:

1. **“a real build reports its raw log, fails on a compile error and navigates to it”** — abre solução → trust → avalia → restore → recarrega modelo; o select de startup oferece só `OrderDesk.Cli` (a biblioteca não aparece); Build → `Build succeeded · exit 0`, comando/cwd/SDK/TargetPath/nota salvos e log real; salva `new Missing();` pelo editor → Rebuild → `Build failed · exit 1`, `error CS` no log e 1 build problem; Problems mostra `Build · dotnet`/`unknown`/CS e o clique em `build-problem-row-0` abre `Program.cs` no `data-cursor` publicado; corrigir+salvar mostra **Obsolete** (não “fixed”); Rebuild volta a `ok · exit 0` com 0 problemas.
2. **“build waits for the explicit restore and asks before using unsaved buffers”** — antes do restore, Build desabilitado com motivo e `Restore now`; restore real + reload liberam o Build; buffer sujo → diálogo com Save all/Build saved content/Cancel; “Build saved content” executa e registra “1 unsaved buffer excluded”; editar e `Cmd+F9` mostra o mesmo diálogo (sem autosave).

Capturas: `screenshots/11-build-ok.png` *(evidência removida do repositório público)*, `11-build-failed.png` *(evidência removida do repositório público)*, `11-problems-build.png` *(evidência removida do repositório público)*, `11-build-fixed.png` *(evidência removida do repositório público)*, `11-build-dirty.png` *(evidência removida do repositório público)*.

## Testes automatizados

```text
$ cargo test -p openridr-host
running 69 tests  (unit: parser com espaços/absoluto/relativo/CRLF e linhas não parseáveis,
                   posição não inventada, path fora do workspace, cap/dedup, exit unknown,
                   cancelamento observado, seal + linha tardia, seleção de startup)
test result: ok. 69 passed; 0 failed; 0 ignored
running 46 tests  (boundary: gate/trust, restore_required, library, ambiguidade, comando real,
                   log/exit/problemas reais, avaliação stale, cancelamento real pendurado,
                   revogação selada e exit tardio)
test result: ok. 46 passed; 0 failed; 0 ignored
```

Log: `logs/11-cargo-test.log` *(evidência removida do repositório público)*. Os testes reais usam SDK 8.0.408 e cópias temporárias; o cancelamento usa um target `Exec sleep 30` real, não mock do host.

```text
$ bun run test:e2e
  ✓ build reports a real log, fails on a compile error and navigates to it
  ✓ build waits for the explicit restore and asks before using unsaved buffers
  … 32 testes dos tickets 01–10 continuam verdes
  34 passed (3.9m)
```

Log: `logs/11-playwright.log` *(evidência removida do repositório público)*. O E2E usa Chrome real contra Vite + host-http; não é a janela Tauri nativa (limitação de captura TCC já registrada).

```text
$ bun run build
✓ built in 801ms

$ bun run tauri build --debug --no-bundle
Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.31s
Built application at: ~/Projetos/openridr/src-tauri/target/debug/openridr
```

Logs: `logs/11-build.log` *(evidência removida do repositório público)* e `logs/11-tauri-build.log` *(evidência removida do repositório público)*. O aviso de chunk >500 kB é do CodeMirror.

## Cobertura do ticket e dos gates

| Item do ticket / AC / gate | Resultado | Evidência |
|---|---|---|
| Startup/configuração/TFM explícitos; biblioteca não é startup; ambiguidade pede escolha | Só `kind:"console"`; `library`/`test`/`unknown` recusados com motivo; `invalid_state` + `details.reason:"ambiguous"`/`candidates`; selects de configuração/TFM chamam `evaluation.run` | `cargo-test` (unit `startup_selection…` + boundary), `11-build-library-rejected.json`, E2E 1/2 |
| Build identifica alvo/contexto/comando, SDK e avaliação válida; não reusa TargetPath invalidado | `buildId`, `workspaceId`/`contextId`, `savedNote`, comando/cwd/SDK/TargetPath no snapshot; avaliação `stale` ou `TargetPath` ausente recusam; config/TFM não avaliados pedem reload | `11-build-ok.log`, `11-build-ok.json`, `cargo-test` |
| Trust no host e nenhum restore implícito; dependências pendentes → ação de 04 | `require_trust` antes de qualquer processo; `restore_required` com `details.action` e lista de projetos; nenhum assets criado antes do clique | `11-build-restore-required.json`, `cargo-test`, E2E 2 |
| Não salvar/sobrescrever buffers silenciosamente; identificar conteúdo salvo | Diálogo Save all/Build saved content/Cancel; `savedNote` no estado; o host nunca grava buffer | E2E 2, `11-build-dirty.png`, `11-build-ok.log` |
| stdout/stderr, estados, cancelamento e exit conhecido/unknown; falha identificada; refazer verificável | Log bruto com `[stderr]` e truncamento; `running/ok/failed/cancelled`; `exit` observado (143 no cancelamento); rebuild após corrigir → `ok` | `11-build-ok.log`, `11-build-failed.log`, `11-build-cancel.log`, E2E 1, `cargo-test` (exit `None` → `failed`) |
| Posição conservadora, linhas não reconhecidas no log, sem assumir JSON | Parser textual com 2/4 números, 0-based; linhas fora do formato ficam no log; sem posição não há `position`/`range`; fora do workspace é rejeitado | unit tests do parser, `11-build-problems.json`, E2E 1 |
| Problems com origem `Build · dotnet`, severidade/código/mensagem/categoria Unknown, execução/revisão; não funde com LSP | Seção separada com `source:"dotnet"`, `category:"unknown"`, `buildId`/`revision`; store LSP intocado; edição marca **Obsolete** | `11-problems-build.png`, `11-build-problems.json`, E2E 1, `cargo-test` |
| Cancelar/revogar pede término real e informa observado; exit ausente não vira zero | TERM→KILL no grupo; `requested`/`observed`/`exit` separados; revogação sela e mantém o processo; linha/finishedAt tardios não mudam o registro | `11-build-cancel.log`, `11-build-revoke*.json`, `cargo-test`, E2E 2 |
| G1 — trust/fronteira | `build.start` nega antes de trust; nenhum processo/build antes do gate; restore pendente não inicia build | `cargo-test`, E2E |
| G2 — contexto/integridade | `workspaceId`/`contextId` no registro; avaliação stale/TargetPath antigo recusados; selo descarta resultado tardio; `exit:null` não vira `ok`; argv separado/cwd/env explícitos | `cargo-test`, logs |
| G3 — consentimento/instaladores | Nenhum download/instalação/elevação nesta fatia | ambiente |
| G4 — UI/privacidade | UI inglesa, controles nativos rotulados, motivo textual de bloqueio, foco/teclado (Cmd/Ctrl+F9, diálogo), sem telemetria/upload; logs locais | E2E 1/2, capturas |
| G5 — validação/proveniência | OS/arch/versões, SDK host/projeto, comandos, exit, revisões, logs e limitações por ambiente | esta página + logs |

ACs tocados: **AC-03** (restore explícito e bloqueio), **AC-10** (contexto/TargetPath), **AC-14/15 parcial** (origem/revisão/obsolescência Build), **AC-17 parcial** (startup/configuração/TFM/build), **AC-18 parcial** (falha/erro/log/rebuild; Run/Debug em 12/14), **AC-19 parcial** (snapshot do conteúdo salvo), **AC-20** (cancelamento Build), **AC-01/07** (gate e revogação), **AC-32/33** (superfície acessível e evidência).

## Limitações e pendências honestas

- **Windows x64 e macOS Intel não foram executados.** `dotnet.exe`, `TerminateProcess` no lugar de TERM/KILL, paths com `\`, CRLF e cancelamento não têm evidência aqui; o ticket 18 deve repetir build ok/falha/cancelamento nesses ambientes. O parser normaliza `\r` do fim de linha e aceita paths relativos/absolutos, mas CRLF real de MSBuild no Windows não foi observado.
- **Build é de um projeto de startup**, não da solução inteira: o comando mostra exatamente um `.csproj` e o `TargetPath` de um console. Build de solução com `Build.0` não foi implementado.
- **Política de restore é a do workspace** (a mesma `require_restored` do ticket 04), não o grafo do projeto escolhido: um build exige assets atuais de todos os projetos da avaliação. A UI sempre restaura o workspace inteiro; um `restore.start {project}` isolado via API não libera o build se outro projeto continuar sem assets.
- **Consumidores Run/Debug não existem** (tickets 12/14). O contrato `build.state`/`build.stale`/`build.buildId` é exposto para que eles não autorizem execução com falha ou resultado antigo; “impede Run/Debug” é verificável apenas quando esses consumidores chegarem.
- **`savedNote` é declaração da UI**, não hash de buffers: o host registra o texto informado e não inspeciona o estado de buffers (que vivem só na UI). O build lê o disco, e a UI mostra a decisão tomada.
- **Editar fonte depois do build** marca o snapshot como obsoleto pelo horário da edição na UI; o host só marca `stale` quando avaliação/restore/SDK/contexto mudam. Um save em `.cs` depois do build não muda `build.stale` no host (o `TargetPath` antigo permanece rotulado pelo estado da UI); ticket 12 deve decidir a política de Run com base em mtime/revisão.
- **Parser conservador**: só reconhece a forma textual `path(line,col): error|warning CODE: message [project]` (e 4 números). Formas localizadas/JSON/logs de outras ferramentas ficam no log bruto sem item; `error`/`warning` sem path ou sem posição não viram problema navegável. Builds com mais de 200 diagnósticos listam os 200 primeiros e marcam `problemsTruncated`.
- **Sem categoria de analyzer/compiler**: `source:"dotnet"` não prova a origem, então `category` é sempre `unknown`; nenhum prefixo `CS`/`CA` é interpretado.
- **Log limitado**: últimas 400 linhas/60k caracteres, com aviso de truncamento; não é transcrição completa. O build com `sleep` no target foi usado só para exercitar cancelamento/revogação.
- **Exit unknown** (`exit:null`) foi provado em teste unitário; neste macOS o dotnet observado terminou com 143 no cancelamento. A UI mostra `exit unknown` quando o registro não recebe código.
- **Playwright usa Chrome real contra Vite + host-http**, não a janela Tauri nativa; o mesmo `dispatch` é chamado.
- **Acessibilidade**: controles nativos, `aria-label`/título e motivo textual; não houve teste com leitor de tela nem auditoria WCAG.
- **`fixtures/`**: todas as execuções reais usaram cópias temporárias; nada foi restaurado/alterado na fixture versionada.
