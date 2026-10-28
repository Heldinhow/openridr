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
- Erro: `{ code, message, details? }`. Códigos usados: `trust_required`, `ambiguous_workspace`, `not_found`, `outside_workspace`, `path_not_allowed`, `conflict`, `sdk_missing`, `tool_missing`, `restore_required`, `confirmation_required`, `invalid_state`, `unsupported_platform`, `cancelled`, `operation_failed`, `stale_context`.
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
  "debug": { "adapterState": "not_checked", "adapterMessage": "...", "appleSiliconExperimental": false },
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

`workspace-path-input`, `workspace-open`, `workspace-candidates`, `trust-checkbox`, `trust-grant`, `trust-revoke`, `trust-note`, `trust-grant-reason`, `mode-restricted`, `mode-trusted`, `mode-badge`, `open-text-only`, `tree`, `tree-project-*`, `tree-file-*`, `editor`, `tab-*`, `dirty-marker`, `save-file`, `conflict-banner`, `conflict-overwrite`, `conflict-reload`, `close-buffer`, `sdk-host`, `sdk-project`, `sdk-project-message`, `sdk-tfms`, `sdk-debug-adapter`, `sdk-apple-silicon`, `sdk-redetect`, `sdk-open-global-json`, `toolchain`, `eval-state`, `restore-panel`, `restore-action`, `restore-cancel`, `restore-retry`, `restore-reload`, `restore-state`, `restore-scope`, `restore-command`, `restore-log`, `restore-incomplete-*`, `install-panel`, `install-state`, `install-plan-open`, `install-plan`, `install-tool`, `install-source`, `install-platform`, `install-destination`, `install-staging`, `install-command`, `install-cwd`, `install-permissions`, `install-integrity`, `install-existing`, `install-confirm`, `install-cancel`, `install-retry`, `install-sha256`, `install-verified`, `install-log`, `lsp-panel`, `lsp-state`, `lsp-session`, `lsp-start`, `lsp-stop`, `lsp-restart`, `lsp-verbose`, `lsp-log`, `lsp-capabilities`, `lsp-diagnostics`, `lsp-coverage-note`, `lsp-error`, `lsp-hostroot`, `lsp-process`, `problems-list`, `problem-row-*`, `problem-stale`, `problems-empty`, `problems-details`, `problems-coverage`, `problems-counts`, `problems-filter`, `problem-navigate`, `problem-open-document`, `problem-no-position`, `editor-shell` (`data-cursor` = `line:character` 0-based), `processes-panel`, `processes-row-*`, `processes-state-*`, `processes-terminate-*`, `processes-keep-*`, `processes-awaiting-*`, `processes-termination-*`, `bottom-tab-*`, `status-*`.

## Evidência por ticket

`docs/evidence/NN-slug.md` com: ambiente (OS/arch/versões), comandos exatos e trecho real da saída, checklist AC/gates com resultado, limitações por ambiente (Windows não executado = não verificado), links para logs em `docs/evidence/logs/`. Nunca copiar saída ilustrativa. Marcar `[x]` no ticket só com evidência correspondente; itens parciais ficam `[ ]` com nota.

## Commits

Um commit por ticket, mensagem convencional (`feat(workspace): ...`, `feat(trust): ...`), incluindo código, testes, fixture quando criada, checklist atualizado e evidência. Sem push/remote/PR.

## Ferramentas e LSP (ticket 05)

- `tools.plan` (leitura, sem trust), `tools.install {planId, confirm:true}`, `tools.retry`, `tools.cancel` (segurança, sem trust), `tools.state`.
- `lsp.start`, `lsp.restart {verbose?}`, `lsp.stop` (segurança), `lsp.state`, `lsp.didOpen {path, text?}`, `lsp.didChange {path, text}`.
- Destino: `<app home>/tools/csharp-ls/0.28`, staging `0.28.staging`, HOME isolado `<app home>/toolhome`.
- Hostroot: `<app home>/hostroot`; runtime 10 + SDK do projeto 8; marker de inputs. O SDK 10 do sistema não entra em `sdk/` (ver decisions).
- Eventos de UI: o editor envia didOpen uma vez por sessão e didChange (texto completo) com debounce de 250 ms; restart troca a sessão e reenvia o conjunto aberto.

## Diagnósticos (ticket 06)

- `diagnostics::Store` (origem/sessão/workspace/contexto/URI) é alimentado pelas publicações da sessão LSP atual e reconstruído no snapshot. Publicação de sessão/contexto anterior é ignorada; restart mantém a sessão anterior como `stale`, revogação/falha/stop marcam `analysisUnavailable` e mantêm os itens.
- `clientRevision` é atribuída na recepção (sem `version` do servidor). Mesma sessão e revisão atual maior → `awaiting` com itens retidos; outra sessão → `stale`. Categoria só vem de `source` com `compiler`/`analyzer`; caso contrário `Unknown`. `analyzerCoverage` é sempre `not_verified`.
- Problems usa os testids acima; navegação por posição é feita no `CodeEditor` (`target`), e itens awaiting/stale/sem posição não movem o cursor. Edição local marca awaiting de imediato até a próxima publicação.
- `diagnostics.probe {path?}` só existe com `OPENRIDR_ENABLE_PROBE=1` e trust; injeta um item sem `range` para o teste de UI. Não é usado pela UI normal.
