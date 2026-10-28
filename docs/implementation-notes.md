# Notas de implementação — arquitetura e contratos (fatia 01–06)

Documento de coordenação entre tickets. Fonte normativa continua sendo [SPEC](SPEC.md) e os tickets. Decisões pequenas ficam em [decisions.md](decisions.md).

## Forma do repositório

```text
crates/host/        # openridr-host: toda a lógica de host (Rust, sem Tauri)
crates/host-http/   # binário SÓ de desenvolvimento: expõe dispatch() em 127.0.0.1
src-tauri/          # janela Tauri 2; comandos finos que chamam o host
src/                # React + TypeScript + CodeMirror
tests/e2e/          # Playwright (UI real via Vite + host-http)
fixtures/OrderDesk/ # fixture .NET 8 real (console + lib + testes)
docs/evidence/      # evidência real por ticket (NN-slug.md)
```

A UI nunca executa processo. Toda operação passa por `dispatch(method, params)` no host, que valida trust/contexto/paths. Botão desabilitado é conveniência; o gate é no host.

## Transporte

`openridr_host::Host::dispatch(method, params) -> Result<Value, HostError>`.
- Tauri: comando `host_call { method, params }` com `Host` em `tauri::State`.
- HTTP dev: `POST /call` `{"method":"...","params":{...}}` → `{"ok":true,"data":...}` ou `{"ok":false,"error":{"code","message","details"}}`; `GET /health`.
- Frontend: `src/host/client.ts` escolhe Tauri (`window.__TAURI_INTERNALS__`) ou HTTP (`VITE_HOST_URL`/default `http://127.0.0.1:4317`).
- Erro: `{ code, message, details? }`. Códigos usados: `trust_required`, `ambiguous_workspace`, `not_found`, `outside_workspace`, `path_not_allowed`, `conflict`, `sdk_missing`, `tool_missing`, `restore_required`, `confirmation_required`, `invalid_state`, `build_stale`, `unsupported_platform`, `cancelled`, `operation_failed`, `stale_context`.
- Polling: `workspace.state` a cada 400 ms enquanto houver operação; toda tela lê o mesmo snapshot.

## Snapshot de estado (cresce por ticket, nunca quebra campos)

```json
{
  "app": { "home": "...", "appHome": "...", "platform": "macos", "arch": "arm64", "version": "0.1.0" },
  "workspace": { "id": 1, "path": "...", "name": "OrderDesk.sln", "kind": "solution|project",
                 "mode": "restricted|trusted", "candidates": [{"path","name","kind"}] },
  "outline": { "source": "text", "projects": [...], "globalJson": {...} },
  "trust": { "granted": false, "workspaceId": null, "decidedAt": null, "revokedAt": null },
  "sdk": { "host": {"state":"found|missing","version","root","sdkPath","instructionsUrl","message"},
           "project": {"state":"no_workspace|not_inspected|resolved|sdk_missing|no_global_json|not_pinned|invalid_global_json",
                       "globalJsonPath","requestedVersion","rollForward","allowPrerelease",
                       "resolvedVersion","root","sdkPath","instructionsUrl","message"},
           "roots": [{"root","source","versions":[{"version","path"}],"exit","error"}],
           "inspectedAt": 0 },
  "debug": { "adapterState": "not_checked|unavailable|not_installed|planned|downloading|installing|verifying|ready|cancelled|failed|experimental_disabled",
             "adapterMessage": "...", "appleSiliconExperimental": false, "version": "3.2.0-1092",
             "source": {"name","url"}, "os", "arch", "platform", "assets": [...], "asset": null|{...},
             "availability": "available|unverified|unsupported_archive|absent", "availabilityReason",
             "installable": false, "gateReason": "...", "dapProven": false, "destination", "staging",
             "plan": null|{...}, "log": "...", "logTruncated": false, "progress": null|{"downloaded","total"},
             "sha256": null, "verifiedVersion": null, "verifiedOutput": null, "error": null, "note": null,
             "reused": false, "revision": 0, "workspaceId": null, "contextId": null, "startedAt": null,
             "finishedAt": null, "sealed": false, "staleReason": null, "stagingPresent": false,
             "destinationPresent": false, "busy": false },
  "processes": [ { "id", "kind", "pid", "workspaceId", "state": "running|exited", "exit": null,
                   "awaitingDecision": false, "kept": false, "termination": null|{...} } ],
  "evaluation": { "state": "idle|running|ok|partial|failed",
                  "context": { "configuration": "Debug", "targetFramework": null },
                  "configurations": ["Debug","Release"], "availableTfms": ["net8.0"],
                  "projects": [{ "name", "path", "folder", "state": "ok|failed", "kind": "console|library|test|unknown",
                                 "tfms": [], "targetPath": null, "outputType": null,
                                 "projectRefs": [{"name","path"}], "packageRefs": [{"name","version"}],
                                 "sources": ["src/.../File.cs"], "error": null, "log": "...", "exit": 0,
                                 "restored": "restored|not_restored|unknown", "restoredReason": null,
                                 "note": null }],
                  "log": "...", "stale": false, "staleReason": null, "revision": 1,
                  "workspaceId": 1, "contextId": 2, "startedAt": 0, "finishedAt": 0 },
  "restore": { "state": "idle|running|ok|failed|cancelled", "scope": "solution|project",
               "target": "/abs/OrderDesk.sln", "command": ["/abs/sdk/8.0.408/dotnet","restore","<target>","-nologo","-v:minimal"],
               "cwd": "/abs/root", "sdkRoot": "/abs/sdk/8.0.408", "sdkVersion": "8.0.408",
               "log": "...", "logTruncated": false, "exit": null,
               "termination": null|{"requested","observed","exit","killedAfterGrace"},
               "processId": 1, "awaitingDecision": false,
               "projects": [{ "name","path","state":"restored|not_restored|unknown","reason",
                              "assetsPath","assetsModifiedAt","projectModifiedAt" }],
               "revision": 1, "workspaceId": 1, "contextId": 2,
               "startedAt": 0, "finishedAt": 0, "sealed": false, "staleReason": null },
  "build": { "state": "idle|running|ok|failed|cancelled",
             "project": {"name","path","kind":"console","targetPath"}|null,
             "configuration": "Debug", "targetFramework": null,
             "command": ["/abs/sdk/8.0.408/dotnet","build","/abs/src/App/App.csproj","-c","Debug","-nologo","-v:minimal","--no-restore"],
             "cwd": "/abs/root", "sdkRoot": "/abs/sdk/8.0.408", "sdkVersion": "8.0.408",
             "log": "...", "logTruncated": false, "exit": null,
             "termination": null|{"requested","observed","exit","killedAfterGrace"},
             "processId": 1, "awaitingDecision": false,
             "revision": 1, "buildId": "build:1:1", "workspaceId": 1, "contextId": 2,
             "startedAt": 0, "finishedAt": 0, "stale": false, "staleReason": null,
             "sealed": false, "savedNote": "Built saved content; 1 unsaved buffer excluded.",
             "problems": [{ "id","severity":"error|warning","severityLabel","severityRaw":1,
                            "code","source":"dotnet","category":"unknown","message","path",
                            "position":{"line","character"}|null,"range":null,
                            "buildId","revision","raw" }],
             "problemsTruncated": false },
  "run": { "state": "idle|running|exited|failed|cancelled",
           "project": {"name","path"}|null, "targetPath", "buildId",
           "configuration": "Debug", "targetFramework": null,
           "command": ["/abs/sdk/8.0.408/dotnet","/abs/bin/Debug/net8.0/App.dll"],
           "cwd": "/abs/src/App",
           "log": "...", "logTruncated": false, "exit": null,
           "termination": null|{"requested","observed","exit","killedAfterGrace"},
           "processId": 1, "awaitingDecision": false,
           "revision": 1, "runId": "run:1:1", "workspaceId": 1, "contextId": 2,
           "startedAt": 0, "finishedAt": 0, "sealed": false, "stale": false,
           "staleReason": null },
  "tools": { "csharp-ls": { "state": "absent|planned|downloading|installing|verifying|ready|cancelled|failed", "version": "0.28.0", "destination": "...", "staging": "...", "plan": {...}, "log": "...", "sha256": "...", "verifiedVersion": "...", "error": null, "reused": false, "revision": 0, "workspaceId": null, "contextId": null } },
  "lsp": { "state": "unavailable|starting|loading|ready|partial|awaiting_diagnostics|failed|stopped", "session": 0, "logLevel": "info|debug", "log": "...", "capabilities": {...}, "serverInfo": {...}, "hostRoot": "...", "hostRootCheck": "...", "coverage": "full|partial", "coverageNote": null, "publicationCount": 0, "diagnosticItems": 0, "awaitingDocuments": [], "processId": null, "exit": null, "termination": null, "error": null, "stoppedReason": null },
  "diagnostics": { "session": 0, "origin": {"id":"lsp","label":"LSP · csharp-ls"},
                   "snapshots": [ { "id","origin","session","workspaceId","contextId","uri","path",
                                    "receivedAt","clientRevision","currentRevision","revisionKnown",
                                    "serverVersion","state":"current|awaiting|stale","staleReason",
                                    "items": [{"id","severity","severityLabel","severityRaw","code","source",
                                               "category":"compiler|analyzer|unknown","message",
                                               "position":{"line","character"}|null,"range",
                                               "codeDescription","stale"}] } ],
                   "awaiting": [{"origin","session","path","revision"}],
                   "counts": {"error","warning","information","hint","unknown"},
                   "summaries": [{"origin","session","currentItems","staleItems","unavailable"}],
                   "currentItems": 0, "staleItems": 0,
                   "coverage": "full|partial", "coverageNote": null,
                   "analyzerCoverage": "not_verified", "analysisUnavailable": null }
}
```

## Invariantes de host

1. `dispatch` valida trust para: MSBuild/evaluation, restore, instalação, LSP, PTY, build/run/debug/testes. Antes de trust: só `workspace.*`, `file.read/write`, `outline`, inventário de SDK (`sdk.redetect`) e leitura de logs. O gate único é `require_trust`; chamadas diretas (HTTP/testes) recebem a mesma resposta. Operações que possam terminar depois capturam `ContextToken` e chamam `accept_result` antes de gravar (`stale_context`).
2. Paths de leitura/escrita sempre canonicalizados e contidos no workspace; recusar `..`, symlink para fora, caminho absoluto externo. Nome com espaços/`;`/`&` é válido; nada é interpolado em shell.
3. Processos: `Command` com args separados, cwd explícito e env explícito; no Unix `process_group(0)` + `killpg` (TERM, depois KILL após 2 s); exit ausente = `null` (UI mostra `exit unknown`). O registro expõe `awaitingDecision`/`kept` e não declara limpeza sem observação. `processes.probe` existe só para testes/desenvolvimento com `OPENRIDR_ENABLE_PROBE=1`; não é PTY nem UI.
4. Operação longa roda em thread própria e publica progresso no estado; cancelamento é pedido distinto de término observado. Restore é o primeiro consumidor assíncrono: o child fica no `Registry`, duas threads drenam stdout/stderr para um log limitado e o snapshot observa o exit (nunca presume código zero).
5. Resposta tardia carrega `workspaceId`/`contextId`; ao gravar no estado, descartar se não for o atual. O snapshot expõe `contextId` e a UI ignora snapshots com contexto menor.
6. Trust em memória; revogar/trocar/fechar workspace invalida tudo e encerra serviços (LSP, restore, evaluation). Processos ativos viram `awaiting decision` visíveis. SDK do projeto só é resolvido após trust; `sdk_missing` nunca cai para o SDK 10 nem reescreve `global.json`; host e projeto são linhas separadas.
7. Logs: manter stdout/stderr brutos, sem apagar em falha/cancelamento. Não logar env completo (segredos/feed credentials).
8. UI inglês; docs PT-BR. Tema escuro/cobre de `docs/index.html`. Todo controle com label acessível, foco visível, teclado e motivo de bloqueio textual.

## Ferramentas e ambiente (Mac ARM64 desta máquina)

- OS: macOS 27.2 (26B118), arm64. Xcode 27.
- SDKs do sistema: 8.0.416 e 10.0.100 em `/usr/local/share/dotnet`.
- SDK 8.0.408 instalado pelo operador em `~/Library/Application Support/OpenRidr/sdks/8.0.408` (não pelo app).
- Host de análise: SDK 10 (`dotnet` do PATH); csharp-ls 0.28.0 via NuGet.org para `~/Library/Application Support/OpenRidr/tools/csharp-ls/0.28`.
- Fixture: `fixtures/OrderDesk`, net8.0, `global.json` fixa 8.0.408 com `rollForward: disable`, console referencia biblioteca, testes xUnit com adapter VSTest do pacote. `bin/obj` nunca versionados.
- Raízes de SDK detectadas: `dotnet` do PATH, `~/Library/Application Support/OpenRidr/sdks/*`, `~/.dotnet` (se existir). Inventário sempre com cwd temporário neutro.

## Comandos

```bash
bun install
bun run dev                 # Vite em 1420
bun run tauri dev           # app desktop real (Tauri + Vite)
bun run build               # tsc + vite build
cargo test -p openridr-host # testes do host (unit + boundary)
cargo run -p openridr-host-http -- --port 4317   # host para Playwright
bun run test:e2e            # Playwright (webServer sobe Vite + host-http)
```

Fixture (somente autoria; o app nunca roda dotnet nesta fatia):

```bash
cd fixtures/OrderDesk
DOTNET_ROOT="$HOME/Library/Application Support/OpenRidr/sdks/8.0.408" \
  "$DOTNET_ROOT/dotnet" restore OrderDesk.sln
DOTNET_ROOT="$HOME/Library/Application Support/OpenRidr/sdks/8.0.408" \
  "$DOTNET_ROOT/dotnet" build OrderDesk.sln -c Debug --no-restore
```

Smoke desktop sem Vite (usa `dist/` embutido): `cargo build` em `src-tauri` e executar `src-tauri/target/debug/openridr`. Com Tauri CLI: `bun run tauri dev`.

## Testids estáveis (E2E)

`workspace-path-input`, `workspace-open`, `workspace-candidates`, `trust-checkbox`, `trust-grant`, `trust-revoke`, `trust-note`, `trust-grant-reason`, `mode-restricted`, `mode-trusted`, `mode-badge`, `open-text-only`, `tree`, `tree-project-*`, `tree-file-*`, `editor`, `tab-*`, `dirty-marker`, `save-file`, `conflict-banner`, `conflict-overwrite`, `conflict-reload`, `conflict-compare`, `conflict-compare-buffer`, `conflict-compare-disk`, `conflict-buffer-hash`, `conflict-disk-hash`, `conflict-compare-diff`, `editor-revision`, `close-buffer`, `confirm-save`, `confirm-keep`, `confirm-discard`, `confirm-error`, `sdk-host`, `sdk-project`, `sdk-project-message`, `sdk-tfms`, `sdk-debug-adapter`, `sdk-apple-silicon`, `sdk-redetect`, `sdk-open-global-json`, `toolchain`, `eval-state`, `restore-panel`, `restore-action`, `restore-cancel`, `restore-retry`, `restore-reload`, `restore-state`, `restore-scope`, `restore-command`, `restore-log`, `restore-incomplete-*`, `install-panel`, `install-state`, `install-plan-open`, `install-plan`, `install-tool`, `install-source`, `install-platform`, `install-destination`, `install-staging`, `install-command`, `install-cwd`, `install-permissions`, `install-integrity`, `install-existing`, `install-confirm`, `install-cancel`, `install-retry`, `install-sha256`, `install-verified`, `install-log`, `lsp-panel`, `lsp-state`, `lsp-session`, `lsp-start`, `lsp-stop`, `lsp-restart`, `lsp-verbose`, `lsp-log`, `lsp-capabilities`, `lsp-diagnostics`, `lsp-coverage-note`, `lsp-error`, `lsp-hostroot`, `lsp-process`, `problems-list`, `problem-row-*`, `problem-stale`, `problems-empty`, `problems-details`, `problems-coverage`, `problems-counts`, `problems-filter`, `problem-navigate`, `problem-open-document`, `problem-no-position`, `editor-shell` (`data-cursor` = `line:character` 0-based), `editor-lsp-status` (`data-state` = idle/loading/ok/unavailable/unsupported/stale/timeout/failed), `rename-open`, `rename-input`, `rename-submit`, `rename-cancel`, `rename-preview`, `rename-source`, `rename-error`, `rename-file-index`, `rename-index-*`, `rename-file-*`, `rename-count-*`, `rename-diff-*`, `rename-stale`, `rename-apply`, `rename-cancel-preview`, `processes-panel`, `processes-row-*`, `processes-state-*`, `processes-terminate-*`, `processes-keep-*`, `processes-awaiting-*`, `processes-termination-*`, `bottom-tab-*`, `status-*`, `code-actions-open`, `code-actions`, `code-actions-summary`, `code-actions-list`, `code-action-*`, `code-action-disabled-*`, `code-action-reason-*`, `code-actions-empty`, `code-actions-close`, `code-actions-stale`, `action-preview-detail`, `toolbar-build`, `build-panel`, `build-startup`, `build-configuration`, `build-tfm`, `build-action`, `build-cancel`, `build-retry`, `build-state`, `build-id`, `build-project`, `build-target-path`, `build-command`, `build-cwd`, `build-sdk`, `build-saved-note`, `build-log`, `build-log-truncated`, `build-problems-count`, `build-problems-capped`, `build-stale`, `build-sealed`, `build-termination`, `build-awaiting`, `build-obsolete`, `build-disabled-reason`, `build-restore-required`, `build-restore-action`, `build-dirty-dialog`, `build-save-and-start`, `build-saved-only`, `build-decision-cancel`, `build-problems`, `build-problems-state`, `build-problem-row-*`, `build-problem-stale`, `build-problem-obsolete`, `build-problems-empty`, `problems-build-count`, `toolbar-run`, `run-action`, `run-stop`, `run-state`, `run-panel`, `run-identity`, `run-build-id`, `run-command`, `run-cwd`, `run-exit`, `run-termination`, `run-log`, `run-log-truncated`, `run-stale`, `run-sealed`, `run-awaiting`, `run-disabled-reason`, `run-dirty-warning`, `run-dirty-dialog`, `run-saved-build`, `run-save-rebuild`, `run-decision-cancel`, `run-warning-save-rebuild`.

## Evidência por ticket

`docs/evidence/NN-slug.md` com: ambiente (OS/arch/versões), comandos exatos e trecho real da saída, checklist AC/gates com resultado, limitações por ambiente (Windows não executado = não verificado), links para logs em `docs/evidence/logs/`. Nunca copiar saída ilustrativa. Marcar `[x]` no ticket só com evidência correspondente; itens parciais ficam `[ ]` com nota.

## Commits

Um commit por ticket, mensagem convencional (`feat(workspace): ...`, `feat(trust): ...`), incluindo código, testes, fixture quando criada, checklist atualizado e evidência. Sem push/remote/PR.

## Ferramentas e LSP (ticket 05)

- `tools.plan` (leitura, sem trust), `tools.install {planId, confirm:true}`, `tools.retry`, `tools.cancel` (segurança, sem trust), `tools.state`.
- `lsp.start`, `lsp.restart {verbose?}`, `lsp.stop` (segurança), `lsp.state`, `lsp.didOpen {path, text?}`, `lsp.didChange {path, text}`, `lsp.completion {path, line, character, revision?}`, `lsp.definition {path, line, character, revision?}`, `lsp.rename {path, line, character, newName, revision?}`, `lsp.codeActions {path, line, character, revision?}`.
- Destino: `<app home>/tools/csharp-ls/0.28`, staging `0.28.staging`, HOME isolado `<app home>/toolhome`.
- Hostroot: `<app home>/hostroot`; runtime 10 + SDK do projeto 8; marker de inputs. O SDK 10 do sistema não entra em `sdk/` (ver decisions).
- Eventos de UI: o editor envia didOpen uma vez por sessão e didChange (texto completo) com debounce de 250 ms; restart troca a sessão e reenvia o conjunto aberto.

## Diagnósticos (ticket 06)

- `diagnostics::Store` (origem/sessão/workspace/contexto/URI) é alimentado pelas publicações da sessão LSP atual e reconstruído no snapshot. Publicação de sessão/contexto anterior é ignorada; restart mantém a sessão anterior como `stale`, revogação/falha/stop marcam `analysisUnavailable` e mantêm os itens.
- `clientRevision` é atribuída na recepção (sem `version` do servidor). Mesma sessão e revisão atual maior → `awaiting` com itens retidos; outra sessão → `stale`. Categoria só vem de `source` com `compiler`/`analyzer`; caso contrário `Unknown`. `analyzerCoverage` é sempre `not_verified`.
- Problems usa os testids acima; navegação por posição é feita no `CodeEditor` (`target`), e itens awaiting/stale/sem posição não movem o cursor. Edição local marca awaiting de imediato até a próxima publicação.
- `diagnostics.probe {path?}` só existe com `OPENRIDR_ENABLE_PROBE=1` e trust; injeta um item sem `range` para o teste de UI. Não é usado pela UI normal.

## Buffers e conflitos (ticket 07)

- Aba = `{path, content, hash, revision, savedRevision, conflict}`. `hash` é o hash base (última versão de disco conhecida); `conflict` guarda `{content, hash}` da versão externa mostrada. “Modified” é `revision !== savedRevision || conflict`, então um buffer limpo com conflito pendente também aparece como pendência até a decisão explícita.
- `file.write` continua exigindo `expected_hash`. `Save` usa o hash base; `Overwrite disk` usa o hash da versão comparada, então uma mudança externa posterior conflita de novo e atualiza o banner sem gravar. Reload troca conteúdo/hash e incrementa a revisão uma vez com `savedRevision` igual; overwrite mantém o conteúdo e leva `savedRevision` à revisão da gravação. O `CodeEditor` aplica valores externos sem disparar `onChange`.
- Foco da janela e ativação de aba chamam `file.read`. Hash igual não faz nada; diferente atualiza/entra em conflito com o conteúdo do disco; falha mostra erro não destrutivo e mantém o buffer. Uma época por path e um conjunto de gravações em voo descartam leituras antigas.
- Fechar aba/workspace/app usa o mesmo diálogo: `confirm-save` (Save/Save all), `confirm-keep`, `confirm-discard`; erro fica em `confirm-error` e nada fecha. `beforeunload` cobre Chrome; Tauri cancela `onCloseRequested` e usa `destroy()` (permissão `core:window:allow-destroy`).
- Comparação é `<details>` no banner (`conflict-compare`) com painéis buffer/disco, hashes e contagem de linhas por LCS; acima de 1200 linhas por lado só os hashes aparecem (`shortcut:` no código).

## Completion e definição (ticket 08)

- `lsp.completion {path, line, character, revision?}` e `lsp.definition {path, line, character, revision?}` são síncronos, passam por `require_trust` e usam o pending id↔sender do cliente LSP com timeout de 5 s (`lsp::REQUEST_TIMEOUT`). `revision` é a revisão do buffer da UI e volta ecoada; `clientRevision` é a versão do documento no host (didOpen/didChange). Nenhum dos dois altera o snapshot.
- Gate: sessão `initialized`, capability anunciada (`completionProvider`/`definitionProvider`) e documento presente no `didOpen` da sessão. Sem isso a resposta é `state:"unavailable"|"unsupported"` com `documentKnown` e listas vazias — não é erro e não bloqueia edição. Antes de trust: `trust_required`.
- Obsolescência: a resposta só é aceita com o mesmo workspace/contexto, o mesmo `Arc<Lsp>`, a mesma sessão e a mesma versão de documento; senão `state:"stale"` e listas vazias. Stop/revogação/restart limpam pendências e trocam a sessão. Não há `$/cancelRequest`; o timeout limita a espera e o resultado tardio é descartado.
- Completion: `items[]` com `label`, `detail`, `kind`/`kindRaw`, `insertText`, `textEdit` normalizado (`range`/`newText`, aceita `insert`/`replace`), `documentation`, `sortText`; `totalItems`, `truncated` (cap 200), `isIncomplete` e `replaceRange` (range do primeiro textEdit ou `null`). `coverage`/`coverageNote` são ecoados.
- Definição: aceita `null`, um Location/LocationLink ou array; cada URI é canonicalizada e só vira location se for arquivo existente dentro do workspace. `rejectedTargets` conta o que foi recusado; se nada resta, `state:"unavailable"`. Cada location traz `path`, `uri`, `line`/`character` do `selectionRange` (ou `range`) e os ranges crus. A UI abre só a primeira.
- UI (`CodeEditor`): Ctrl+Space (keymap padrão) abre completion; Ctrl/Cmd+B pede definição. `activateOnTyping:false` evita requisições a cada tecla. O source envia a revisão atual, descarta resposta se documento/revisão mudaram e, no aceite, só aplica se prefixo/sufixo fora do range continuarem iguais (digitar para filtrar é permitido). Definição também descarta se a seleção mudou. `editor-lsp-status` (`data-state="idle|loading|ok|unavailable|unsupported|stale|timeout|failed"`) mostra o estado, truncamento e cobertura parcial. Atalhos sem trust/sessão mostram o motivo.

## Rename (ticket 09)

- `lsp.rename {path, line, character, newName, revision?}` é síncrono no pending do cliente LSP (timeout `REQUEST_TIMEOUT` de 5 s), passa por trust/capability/documento e, quando `renameProvider.prepareProvider` é anunciado, chama `textDocument/prepareRename` antes do rename. `null` no prepare vira `unavailable`; nenhuma edição é fabricada por texto.
- O `WorkspaceEdit` aceita `changes` e `documentChanges`/`TextDocumentEdit`; operação de arquivo (`create`/`rename`/`delete`) e entrada malformada recusam o conjunto. Cada posição é convertida de UTF-16 no host (`utf16_offset`, com CRLF e pares substitutos) e aplicada sobre o texto da sessão para documento aberto ou sobre o disco para não aberto. O host devolve, por documento, `path`, `uri`, `opened`, `documentVersion`, `editCount`, `edits`, `changes` (linha/antes/depois, com corte só de exibição), `displayTruncated`, `baseContent`, `newContent` e `baseHash`.
- Caps: 50 documentos, 2.000 edições e 1 MiB de texto (base+novo); acima disso a prévia inteira é recusada com motivo, nunca truncada. URI fora do workspace, arquivo ausente/ilegível, intervalo inválido/sobreposto e `TextDocumentEdit.version` divergente também recusam tudo. A resposta leva `renameId` (sessão+sequência+FNV de workspace/contexto/posição/nome/conteúdo), `prepared`, `documentCount`, `editCount`, `session`, `workspaceId`/`contextId` e `revision` ecoada; não existe `rename.apply` no host.
- UI: Shift+F6 (Rider) ou o botão focável “Rename symbol…” abre um prompt modal; antes de chamar o host a UI drena o debounce de `didChange` para o servidor ver o buffer atual. A prévia é um painel não modal (o editor continua editável) com arquivos, “not opened here”, contagem e linhas antes/depois; Apply/Cancel são explícitos. Na aplicação a UI revalida sessão/contexto/workspace/aba ativa, revisão+conteúdo+conflito dos buffers abertos e relê o hash de disco de todos os documentos (não abertos contra `baseHash`, abertos contra o hash base da aba); qualquer divergência mantém a prévia e recusa o conjunto. Sucesso troca todas as abas numa atribuição: abertas `revision+1` não salvas; não abertas viram abas `1/0` com `hash` base; `didChange` só para documentos já abertos na sessão. Nenhum arquivo é escrito e os diagnósticos não são marcados corrigidos.
- Resposta tardia é descartada por sequência/revisão/sessão; cancelar o prompt ou a prévia não altera buffer nem disco. Trocar de aba fecha a prévia com motivo; editar o buffer é recusado no Apply para o usuário ver o motivo. Revogação, stop ou troca de sessão/workspace bloqueia a aplicação com motivo textual (o host também responde `stale`/`trust_required` para novas requisições). A prévia foca o título, os botões são nativos e o `app-log`/log LSP registram a falha.

## Code actions (ticket 10)

- `lsp.codeActions {path, line, character, revision?}` usa o mesmo gate de `prepare_analysis_request` (trust, sessão `initialized`, `codeActionProvider`, documento no `didOpen`/`didChange`) e o timeout de 5 s. Envia `textDocument/codeAction` com `range` início==fim na posição e `context.diagnostics` somente dos itens **atuais** do store LSP para aquele documento cujo range cobre a posição (fonte/código/severidade/range/mensagem/codeDescription; nada de publicação `awaiting`/`stale`).
- A resposta real é uma lista (`null`/array); entrada sem `title` é contada como rejeitada e nunca vira ação. Cada ação guarda `title`, `kind`, `isPreferred`, `disabled` (motivo), `command` (nome, nunca executado), `data` e `edit`. `edit` reusa `parse_workspace_edit` e o mesmo `workspace_edit_preview` extraído do rename: por ação, todas as edições são validadas como conjunto (URI dentro do workspace, arquivo existente/legível, ranges válidos/não sobrepostos, versão do documento, caps 50/2.000/1 MiB) e o resultado traz `documents` com `baseContent`/`newContent`/`baseHash`/`documentVersion`. Falha de uma ação vira `supported:false` com motivo; não invalida as outras nem inventa edições. Ação só com comando ou `data` sem `edit` fica listada como não suportada; não há execução de comando.
- A resposta leva `actionId` (sessão+sequência+FNV de workspace/contexto/posição/versão), `session`, `workspaceId`/`contextId`, `documentVersion`, `actions`, `actionCount`, `totalActions`, `truncated`, `rejectedActions`, `unsupportedActions` e `diagnosticsUsed`. Não existe `codeActionApply` no host: aplicar só altera buffers.
- UI: botão focável “Code actions…” e Option+Enter (macOS)/Alt+Enter (Windows) no editor. Se análise/capability/documento não estão prontos, o botão fica desabilitado com o motivo e o atalho responde em texto; lista vazia responde “No code actions…” sem criar quick fix por CS. A lista mostra título, `kind` (ou “not provided by csharp-ls”), preferida, motivo de desabilitada e motivo de não suportada; escolher uma ação suportada abre a mesma prévia por documento do rename. Apply revalida sessão/contexto/workspace/aba, revisão+conteúdo+conflito dos buffers e hash de disco de todos os documentos; qualquer divergência aplica nada. Cancelar não altera buffer/disco. Editar o buffer, mover o cursor, trocar de aba, reiniciar/revogar ou resposta tardia descarta a lista/prévia; aplicar atualiza buffers como não salvos, envia `didChange` e não remove/marca diagnósticos como corrigidos.

## Build (ticket 11)

- `build.start {project?, configuration?, targetFramework?, savedNote?}` e `build.cancel`, com estado `build` no snapshot. O gate é o mesmo: `require_trust` → SDK do projeto → `restore::require_restored` (devolve o `restore_required`/`action` do ticket 04) → avaliação atual (`ok|partial`, `stale:false`, mesmo `workspaceId`/`contextId`) → `TargetPath` do projeto escolhido. `configuration`/`targetFramework` informados precisam coincidir com a avaliação corrente; caso contrário a resposta pede `evaluation.run`.
- Startup: apenas `kind:"console"` da avaliação; `library`/`test`/`unknown` são recusados com motivo. Sem `project`, zero candidatos é `invalid_state` com motivo; mais de um é `invalid_state` com `details.reason:"ambiguous"` e `details.candidates`. O build é um por vez e não começa com restore em andamento.
- Comando exato: `[<sdk-root>/dotnet, "build", <csproj absoluto>, "-c", <Configuration>, "-nologo", "-v:minimal", "--no-restore"]`, mais `-p:TargetFramework=<tfm>` se houver TFM; cwd na raiz, `DOTNET_ROOT` do SDK do projeto, `DOTNET_CLI_UI_LANGUAGE=en` e grupo de processos. O child fica no `Registry` (`kind:"build:<Project>"`), duas threads drenam stdout/stderr para o `LogBuffer` (stderr `[stderr]`; truncamento declarado) e o log sobrevive a falha/cancelamento.
- `exit` é sempre observado: ausente = `null`; `ok` só com `Some(0)`, cancelamento com término observado vira `cancelled`. `build.cancel` não exige trust, pede TERM→KILL ao grupo e reporta `requested`/`observed` separados; pedido não é prova de saída.
- Fronteiras: revogar/fechar/trocar termina o grupo e sela o registro; linhas/exit tardios são descartados. Revogar mantém o registro selado visível; fechar/trocar devolvem `build.idle` (o processo continua no painel de processos). `evaluation.run`, restore concluído, `sdk.redetect` e avaliação stale marcam `build.stale` com motivo. Salvar `.cs` depois do build não altera o host; a UI compara a edição com `finishedAt` e marca o snapshot como **obsoleto** (não corrigido).
- `problems`: parser textual conservador de `path(line,column): error|warning CODE: message [project]` (2 ou 4 números; 0-based; `range` só com 4). Linha não reconhecida fica só no log; path fora do workspace é rejeitado; duplicata do resumo de falha é deduplicada; cap 200 com `problemsTruncated`. `source:"dotnet"`, `category:"unknown"`, `severityRaw` 1/2; o id é `<buildId>:<index>` e cada item carrega `buildId`/`revision`. Nada é ingerido no store LSP.
- UI: aba Build com startup (só consoles; motivo quando nenhum/ambíguo), Configuration/TFM ligados ao contexto avaliado (mudar chama `evaluation.run`), Build/Cmd+F9/Ctrl+F9, diálogo explícito para buffers sujos (`Save all and build` / `Build saved content (N excluded)` / `Cancel`), aviso com `Restore now` quando dependências faltam, estado/exit/comando/cwd/SDK/nota salva/log bruto, cancelar e retry. Problems mostra a seção `Build · dotnet` separada da `LSP · csharp-ls`; clique com posição abre no `line:character`, sem posição abre o documento sem mover o cursor; edição posterior mostra `Obsolete`.

## Run (ticket 12)

- `run.start {project?, configuration?, targetFramework?}` e `run.cancel`, com estado `run` no snapshot. Gate: `require_trust` → avaliação atual (`ok|partial`, `stale:false`, mesmo workspace/contexto) → build atual do mesmo workspace/contexto, `state:"ok"`, `exit:0`, não `stale`/`sealed`, mesmo projeto/configuração/TFM e mesmo `evaluation.revision` → `TargetPath` igual ao avaliado e presente no disco. Falhas devolvem `details.reason` (`no_build`, `build_failed`, `build_stale`, `evaluation_changed`, `project_changed`, `build_context_mismatch`, `target_changed`, `source_changed`, `target_missing`, `target_not_executable`, `evaluation_required`, `configuration_not_evaluated`, `tfm_not_evaluated`, `ambiguous`) com `action` de build/avaliação e `buildLog` quando há log. Nada é restaurado ou compilado.
- Identidade: `runId`, `buildId`, `project{name,path}`, `targetPath`, configuração/TFM, `command`, `cwd`, `workspaceId`/`contextId`, `startedAt`/`finishedAt`. O comando é `[<sdk-root>/dotnet, <TargetPath>]` para `.dll` gerenciado; executável nativo é recusado se não tiver bit de execução. cwd é o diretório do `.csproj`; env carrega `DOTNET_ROOT` do SDK do projeto e desliga multi-level lookup/telemetria.
- Fonte salva: `build.start` captura hash FNV-1a dos fontes do startup + referências transitivas (excluindo `bin`/`obj`, que a avaliação já filtra). `run.start` relê e devolve `build_stale` com `changedSources`. Buffer sujo não entra na verificação; a UI mostra o aviso e as opções “Run saved build”/“Save all and rebuild”, nunca salva sozinha.
- Processo: `spawn_piped` no `Registry` compartilhado (`kind:"run:<Project>"`), grupo próprio, duas threads drenam stdout/stderr para o `LogBuffer` (stderr `[stderr]`; 400 linhas/60k; truncamento declarado). `run.cancel` (sem trust) envia TERM→KILL ao grupo e reporta `requested`/`observed`/`exit`; o log continua visível.
- Exit/estado: `running` → `exited` quando o registry observa saída (qualquer código, inclusive `null` = unknown) ou `cancelled` quando houve pedido/término observado; `failed` só para falha ao iniciar. Um segundo `run.start` é `invalid_state`. Revogar/fechar/trocar termina o grupo e sela (`sealed`, `stale`, `staleReason`); linhas/exit tardios são descartados. Revogar mantém o registro selado; fechar/trocar o descartam da UI, mas o processo permanece no painel.
- UI: a barra de startup (Startup | Configuration | TFM | Build | Run) tem `Run (Ctrl+R)`/`Stop` e o texto do atalho vira Shift+F10 em Windows. `run-state`, `run-identity`, `run-build-id`, `run-command`, `run-cwd`, `run-exit`, `run-termination`, `run-log`, `run-disabled-reason`, `run-dirty-warning`, diálogo `run-dirty-dialog` (`run-saved-build`, `run-save-rebuild`, `run-decision-cancel`). Run fica desabilitado com motivo quando trust/SDK/avaliação/build falho/stale/fonte salva depois do build; biblioteca nunca aparece como startup.

## DAP (ticket 14)

- `crates/host/src/dap.rs` é o cliente DAP stdio (`Content-Length` em bytes, reuso do framing LSP). Um processo netcoredbg por sessão em `Registry` (`debug:<Project>`), ids/`seq` monotônicos, `pending` por id, timeout de 10 s (launch 20 s), eventos lidos em thread própria e logs limitados. Uma sessão por vez.
- Métodos: `debug.start {project?, configuration?, targetFramework?}` (trust), `debug.stop` (segurança, sem trust), `debug.continue`, `debug.stepOver`, `debug.stepInto`, `debug.setBreakpoints {path, lines}` e `debug.variables {frameId}` (trust). `debug.state` continua sendo o snapshot.
- Gates de launch: trust → restore atual (`restore_required`) → adaptador `ready` (versão 3.2.0, plataforma/flag) → avaliação atual → configuração **Debug** avaliada → startup console → build atual com mesma identidade/revisão/configuração/TFM/TargetPath/fontes → assembly avaliado (`TargetPath` ou `TargetDir`+`TargetFileName`) existente. Falhas devolvem `details.reason` e não criam processo. `evaluation.projects[]` expõe `assemblyPath`.
- O argv do adaptador é exatamente `[netcoredbg, "--interpreter=vscode"]`; o launch usa `program = assemblyPath`, `cwd` do projeto, env com `DOTNET_ROOT` e `env` nunca é logado. `runInTerminal` spawna o argv exato com cwd/env no `Registry`, recusa interpretação por shell e responde com os pids observados.
- Breakpoints: `setBreakpoints` por fonte; `verified/pending/message` vêm do adaptador; sem símbolos fica pendente. Gutter `.cm-debug-gutter` separado dos números (`●` breakpoint, `▶` linha atual); Cmd+F8/Ctrl+F8 e clique alternam; F8/F7 step; Ctrl+D/Shift+F9 lança.
- Pausa: `stopped` incrementa a geração, busca threads → frames → scopes/variables; `debug.variables` recarrega um frame da geração atual. Continue/step limpam frames/locals antes do request; um `stopped` mais novo vence a resposta antiga; respostas tardias comparam `(session, stopSeq)`. Estados `preparing/running/paused/terminated/cancelled/failed`.
- Encerramento: `stop`/`disconnect {terminateDebuggee:true}` → exit do adaptador observado (ou TERM→KILL), termina processos de `runInTerminal`; `termination` requested/observed/exit separados; `exit` do debuggee ausente é `exitKnown:false` (“exit unknown”), nunca 0. `terminated` espontâneo limpa o adaptador em thread separada e registra `adapterExit`. Revogar/fechar/trocar sela (`sealed`, `staleReason`) e troca o `Arc<Dap>`; logs/output ficam; revogar mantém o registro, fechar/trocar voltam a idle e os processos ficam no painel.
- UI aba Debug (`src/DebugPanel.tsx`): estado, launch/stop/continue/step, threads/frames/locals, breakpoints pendente/verificado, identidade (sessão/build/assembly/adaptador/cwd/pids), exit/`exit unknown`, termination, log DAP/adaptador/host e output. Bloqueio textual via `debugDisabledReason`; o atalho usa refs para não capturar estado antigo. O painel de instalação (ticket 13) continua com `debug-state`/`debug-disabled-reason`; o DAP usa `dap-state` etc.
- Testids: `dap-panel`, `dap-state`, `dap-phase`, `dap-session`, `dap-build-id`, `dap-assembly`, `dap-adapter`, `dap-process`, `dap-exit`, `dap-termination`, `dap-thread-list`, `dap-log`, `dap-output`, `dap-disabled-reason`, `dap-sealed`, `dap-terminal-note`, `debug-launch`, `debug-stop`, `debug-continue`, `debug-step-over`, `debug-step-into`, `debug-frames`, `debug-frame-*`, `debug-locals`, `debug-scope-*`, `debug-local-*`, `debug-breakpoints`, `debug-breakpoint-*`.
- Evidência real: `docs/evidence/14-debug-dap.md` (fixture, async/multi-thread, stop, revogação, exit/processos, screenshots 14-*). Windows x64 não executado; flag ARM64 mantida OFF por padrão.

## Debugger (ticket 13)

- `crates/host/src/debugger.rs` guarda a versão aprovada `3.2.0-1092`, a release oficial `https://github.com/Samsung/netcoredbg/releases/tag/3.2.0-1092` e **somente** os assets publicados nela (osx-arm64, win64, linux-amd64, linux-arm64) com tamanho e SHA-256 lidos do metadata do GitHub. Não existe `osx-x64`; o host não inventa asset nem substitui versão.
- Gates: macOS arm64 exige a flag experimental para instalar (`experimental_required` quando OFF), mas o plano já mostra o asset `osx-arm64` com aviso *community supported*; com a flag ON, trust + `confirm:true` instalam o zip oficial. macOS Intel e Linux são `unsupported_platform`; Windows x64 tem plano/asset e caminho de instalação não executado aqui.
- Métodos: `debug.plan` (leitura, sem trust), `debug.install {planId, confirm:true}` (trust + consentimento + plataforma/flag + revalidação do plano), `debug.retry` (trust + plano confirmado), `debug.cancel` (ação de segurança, sem trust), `debug.state` e `debug.flag {enabled, confirm:true}` (opt-in de sessão, independente de trust/instalação; ligar replaneja o plano existente).
- Estados em `snapshot.debug.adapterState`: `not_checked|unavailable|not_installed|planned|downloading|installing|verifying|ready|cancelled|failed|experimental_disabled`. `ready` prova só executável + `--version` (contém `3.2.0`); `dapProven` é sempre `false` nesta fatia.
- Snapshot: `{adapterState, adapterMessage, appleSiliconExperimental, version, source{name,url}, os, arch, platform, assets[], asset, availability, availabilityReason, installable, gateReason, dapProven:false, destination, staging, plan, log, logTruncated, progress{downloaded,total}, sha256, verifiedVersion, verifiedOutput, error, note, reused, revision, workspaceId, contextId, startedAt, finishedAt, sealed, staleReason, stagingPresent, destinationPresent, busy}`. `adapterMessage` descreve sempre a disponibilidade da plataforma; `note`/`error`/`log` carregam a operação. O plano inclui `blockedReason`/`blockedCode` (ex.: `experimental_required`) e `planId` FNV-1a dos campos materiais. `appleSiliconExperimental` zera ao abrir/fechar workspace, como trust.
- Destino `<app home>/tools/netcoredbg/3.2.0-1092`, staging `3.2.0-1092.staging`, download `3.2.0-1092.staging.download`. Download com `ureq` (rustls), redirects manuais e só HTTPS nos hosts oficiais; hash SHA-256 comparado ao plano; extração zip com `enclosed_name`, recusa symlink/modo especial, caps de 200 entradas/200 MiB, ignora `__MACOSX`/`.DS_Store`; `--version` no staging antes da promoção. O executável é localizado por nome em qualquer profundidade (o zip oficial usa `netcoredbg/netcoredbg`), e o reuso procura no mesmo lugar. Instalação válida é reutilizada; parcial/cancelado/reprovado nunca vira `ready`; progresso é só bytes medidos. URLs no log descartam query (o redirect do GitHub traz token assinado).
- Testes de fronteira (`crates/host/tests/debugger.rs`): servidor HTTP local determinístico + zips controlados para origem/redirect inválidos, traversal/absoluto/symlink, zip corrompido, hash e versão errados, plano trocado, cancelamento/retry, reuso e o gate `experimental_required` do macOS arm64. O artefato controlado é um script rotulado (`controlled test artifact`) e só roda no harness. Seam: `OPENRIDR_ENABLE_PROBE=1` + `OPENRIDR_DEBUG_PLATFORM`/`OPENRIDR_DEBUG_ASSET_URL`/`OPENRIDR_DEBUG_ASSET_SHA256`.
- UI: `DebugAdapterPanel` em `src/ToolchainPanels.tsx`, renderizado no Toolchain do setup e na faixa de avisos do workspace (`compact`). Mostra estado, mensagem real da plataforma, asset/availability, versão/fonte, flag Apple Silicon com checkbox+botão explícitos, plano completo (incluindo bloqueio/código), log, progresso e SHA/versão verificada. `toolbar-debug` fica desabilitado com o motivo no `title`; a aba Debug repete a limitação. Nada disso bloqueia análise/Run/testes.
- Testids novos: `debug-panel`, `debug-state`, `debug-message`, `debug-source`, `debug-asset-state`, `debug-disabled-reason`, `debug-plan-open`, `debug-plan*`, `debug-install`, `debug-cancel`, `debug-retry`, `debug-flag-*`, `debug-log`, `debug-progress`, `debug-sha256`, `debug-verified`, `debug-dap`, `debug-error`, `debug-note`, `debug-tab`, `toolbar-debug`. `sdk-debug-adapter`/`sdk-apple-silicon` continuam existindo dentro do painel.
- Códigos de erro novos: `experimental_required` (macOS arm64 sem flag) e `unsupported_platform` (Intel/Linux/plataformas sem asset). A instalação real no app home padrão está registrada em `docs/evidence/13-instalacao-debugger.md`; E2E usa home isolado e não baixa o asset.
