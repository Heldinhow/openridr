# Evidência — Ticket 01: abrir solução em modo restrito

Implementação local em 27/10/2026 (madrugada de 28/10 no UTC). Fonte normativa: [ticket 01](../tickets/01-abertura-restrita.md), [SPEC](../SPEC.md) P0.1–P0.3 e gates G1–G5 do [índice de tickets](../tickets/README.md).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Node | v26.10.0 |
| Bun | 1.3.9 |
| Rust | cargo 1.99.0, rustc 1.99.0 |
| Chrome | Google Chrome 155.0.8059.85 (canal `chrome`, sem download de browser) |
| Playwright | 1.64.1 |
| SDK do host (PATH) | .NET SDK 10.0.100 e 8.0.416 em `/usr/local/share/dotnet` |
| SDK fixado pela fixture | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` |
| Base de código anterior | `e304c57` (docs/tickets); nenhum código OpenIdea reutilizado |

Log completo: [`logs/environment.log`](logs/environment.log).

## O que foi construído nesta fatia

- `crates/host` (`openridr-host`): `Host::dispatch(method, params)`, snapshot de estado, abertura de `.sln`/`.csproj`/diretório, ambiguidade explícita, contorno textual lido só de arquivos, `file.read`/`file.write` com hash esperado e contenção de paths, stubs executáveis com gate de trust. Sem dependência de Tauri.
- `crates/host-http`: binário **só de desenvolvimento** em `127.0.0.1:4317`, `POST /call`, `GET /health`, CORS para `http://localhost:1420` e `http://127.0.0.1:1420`. Não é distribuído.
- `src-tauri`: app Tauri 2 com um comando `host_call`, janela `OpenRidr` 1440×900, mínimo 1100×700, tema escuro.
- `src/`: React 18 + Vite + TypeScript + CodeMirror 6; tela de setup/trust, janela principal (árvore, abas, breadcrumb, editor, painel inferior, status bar), salvamento por `Cmd+S`/`Ctrl+S`, conflito de disco e confirmação de fechamento.
- `fixtures/OrderDesk`: solução SDK-style net8.0 com console → biblioteca, projeto xUnit + adapter VSTest, `global.json` 8.0.408/`rollForward: disable`, `.gitignore` para `bin/obj`.
- `tests/e2e`: 7 testes Playwright contra Chrome real, Vite real, `host-http` real e arquivos reais.

## Comandos e saídas reais

### Frontend

```text
$ bun run build
tsc --noEmit && vite build
✓ 50 modules transformed.
dist/assets/index-CF6dPSWq.js   582.43 kB │ gzip: 189.83 kB
✓ built in 748ms
```

Log: [`logs/build.log`](logs/build.log). O aviso de chunk >500 kB é do CodeMirror e não é erro.

### Host (unit + boundary)

```text
$ cargo test -p openridr-host
running 6 tests
test text_outline_reads_solution_projects_without_evaluation ... ok
test save_reports_conflict_instead_of_overwriting_disk ... ok
test directory_resolution_requires_an_explicit_choice ... ok
test one_workspace_at_a_time_and_awkward_names_work ... ok
test file_access_is_confined_to_the_workspace ... ok
test executable_methods_fail_before_trust_and_spawn_nothing ... ok
test result: ok. 6 passed; 0 failed
```

Log: [`logs/cargo-test.log`](logs/cargo-test.log).

### Ponte HTTP de desenvolvimento

```text
$ curl -sS http://127.0.0.1:4317/health
{"ok":true,"pid":20882,"service":"openridr-host-http","version":"0.1.0"}

$ curl -sS -X POST http://127.0.0.1:4317/call -H 'content-type: application/json' \
    -d '{"method":"build.start","params":{}}'
{"error":{"code":"trust_required","message":"build.start requires workspace trust; no process was started."},"ok":false}

$ curl -i -X OPTIONS http://127.0.0.1:4317/call -H 'Origin: http://localhost:1420' ...
HTTP/1.1 204 No Content
Access-Control-Allow-Origin: http://localhost:1420
Access-Control-Allow-Methods: GET, POST, OPTIONS
Access-Control-Allow-Headers: content-type
```

Log: [`logs/host-http-smoke.log`](logs/host-http-smoke.log).

### Playwright (Chrome real + host real + arquivos reais)

```text
$ bun run test:e2e
Running 7 tests using 1 worker
  ✓ opens the fixture in restricted mode with trust disabled (1.1s)
  ✓ opens a file, edits, saves with Cmd+S and reopens the saved text (783ms)
  ✓ read and write failures keep the editor usable and preserve edits (734ms)
  ✓ detects a disk conflict and never overwrites without an explicit choice (857ms)
  ✓ closing a modified buffer or workspace requires an explicit choice (1.1s)
  ✓ ambiguous and empty directories ask the user instead of guessing (836ms)
  ✓ direct host calls fail before trust and reject paths outside the workspace (46ms)
  7 passed (7.8s)
```

Log: [`logs/playwright.log`](logs/playwright.log).

Os testes abrem `fixtures/OrderDesk` de verdade, editam/salvam `Program.cs`, escrevem no arquivo pelo processo de teste para induzir conflito, fecham aba/workspace com buffer sujo e chamam o host HTTP diretamente. `beforeEach`/`afterEach` restauram o conteúdo original da fixture; `workers: 1` porque host e arquivo são estado compartilhado.

### Fixture (somente autoria, não fluxo do app)

```text
$ dotnet restore OrderDesk.sln
$ dotnet build OrderDesk.sln -c Debug --no-restore
Compilação com êxito. 0 Aviso(s) 0 Erro(s)
$ dotnet test OrderDesk.sln -c Debug --no-build
Aprovado! – Com falha: 0, Aprovado: 2, Ignorado: 0, Total: 2
```

Log: [`logs/fixture-build.log`](logs/fixture-build.log). Executado com `DOTNET_ROOT`/PATH apontando para o SDK 8.0.408 fixado. O app não executou dotnet em nenhum fluxo desta fatia.

### Smoke desktop

```text
$ cargo build            # src-tauri, usa dist/ embutido
$ ./src-tauri/target/debug/openridr

CGWindowList: owner=openridr, onscreen=1, bounds 1440x900, layer=0
$ screencapture -x -l2844 docs/evidence/screenshots/desktop-smoke.png
could not create image from window
$ screencapture -x /tmp/desktop-display.png
could not create image from display
```

Log: [`logs/desktop-smoke.log`](logs/desktop-smoke.log). A janela real foi criada e permaneceu estável; a captura de tela foi recusada pelo TCC de Screen Recording desta máquina (o mesmo ocorre para captura de display inteiro). `osascript`/System Events também está sem permissão assistiva. Portanto **não houve screenshot do desktop** e a janela não foi dirigida por automação; o conteúdo visual foi verificado no mesmo `dist/` servido ao Chrome real:

- [`screenshots/ui-trust-screen.png`](screenshots/ui-trust-screen.png) — setup/trust, caminho completo, Restricted mode, trust desabilitado e motivo textual.
- [`screenshots/ui-workspace.png`](screenshots/ui-workspace.png) — árvore textual, abas, breadcrumb, editor, painel inferior e status bar com keymap.

Essas duas imagens são da UI real (Vite + host-http + fixture), não do desktop; não substituem a prova de interação desktop.

## Cobertura do ticket e dos gates

| Item do ticket / gate | Resultado | Evidência |
|---|---|---|
| Auditar base reutilizada; janela desktop real | Nenhum código/base OpenIdea reutilizado; app próprio. Janela Tauri criada 1440×900. Sem screenshot por TCC. | `docs/decisions.md`, `logs/desktop-smoke.log` |
| Abrir solução/projeto; um workspace; ambiguidade explícita | `.sln`, `.csproj` e diretório; 0 candidatos → erro claro; >1 → `ambiguous_workspace` com lista e escolha do usuário. | E2E 1/5, `cargo-test` |
| Caminho completo, Restricted mode, trust desmarcado, `Open text only`, sem ações executáveis | UI mostra caminho completo; checkbox e botão de trust desabilitados com motivo “ticket 02”; zero stubs executáveis. | E2E 1, screenshot trust |
| Contorno textual, nunca avaliado; sem MSBuild/restore/LSP/tarefas/build/run/debug/testes/shell | Rótulo “Text outline · not evaluated”; parser lê `.sln`, `.csproj` e `global.json` como texto; teste de boundary comprova ausência de `bin/obj` após chamar os 10 métodos. | `cargo-test`, E2E 1 |
| Abrir/editar/salvar texto sem solução/SDK/LSP; árvore/editor/toolbar/abas/breadcrumb/modificado; falha preserva buffer | Projeto isolado `Beta.csproj` editado/salvo no E2E 6; conflito mantém buffer; falha de leitura e falha de escrita (diretório sem permissão) preservam UI/buffer e mostram linha de erro. | E2E 2/3/4/6 |
| Fechar/substituir buffer e trocar/fechar workspace com edição exige escolha; conflito sem sobrescrita silenciosa | Modal Keep editing / Discard; conflito oferece somente `Reload from disk (discard my edits)` e `Overwrite disk`; disco não muda antes da escolha. | E2E 4/5 |
| Fronteira host para paths | Canonicalização + contenção; rejeita `..`, absoluto externo e symlink para fora; nomes com espaço/`;`/`&` funcionam. | `cargo-test` (2 testes) |
| G1 trust/execução | Stubs `evaluation.run`, `restore.start`, `lsp.start`, `tools.install`, `terminal.start`, `build.start`, `run.start`, `debug.start`, `tests.discover`, `tests.run` retornam `trust_required` antes de qualquer processo; chamada HTTP direta também. Revogação pertence a 02. | `cargo-test`, E2E 7 |
| G2 contexto/integridade | Caminho, tipo, modo, projeto/TFM textual, aba e hash identificam o estado; conflito não sobrescreve; sem processos. Contexto MSBuild/TargetPath ficam em 03+. | E2E, `cargo-test` |
| G3 consentimento/instaladores | Nenhum instalador, download ou plano nesta fatia; não aplicável ainda. | — |
| G4 UI/privacidade | UI em inglês, docs PT-BR, tema escuro/cobre, sem telemetria; foco visível, labels, teclado (Cmd+S), motivos textuais; sem estado só por cor. | screenshots, E2E |
| G5 validação/proveniência | Ambiente, comandos, logs, limitações e ACs acima; Windows não executado. | esta página |

ACs da SPEC tocados: **AC-01** (abertura/restrição/edição), **AC-08 parcial** (abertura/ambiguidade/contorno textual; árvore avaliada em 03), **AC-11 parcial** (edição e segurança mínima; fluxo completo em 07), **AC-32** (shell/editor desta fatia), **AC-33 parcial** (macOS arm64), **AC-34 parcial** (base mínima).

## Limitações e pendências honestas

- **Windows não foi executado** nesta máquina. Nenhum comportamento de CRLF, paths, shell ou permissões Windows está verificado; fica como pendência de G5/18.
- **Desktop não foi dirigido**: só foi provado que o binário Tauri compila, inicia e cria janela 1440×900. A captura por `screencapture` e a leitura por System Events foram bloqueadas por TCC (Screen Recording e Assistive Access). A UI foi exercitada no Chrome via ponte HTTP, que usa o mesmo `dispatch` do host. Não há prova de interação real com mouse/teclado na janela nativa.
- **Trust pós-02 não testado**: o branch `trusted` dos stubs retorna `invalid_state`, mas nenhum caminho de UI o alcança nesta fatia.
- **Conflito**: apenas o fluxo mínimo (reload/overwrite). Comparação lado a lado, múltiplos buffers conflitantes e revisão de rename ficam em 07.
- **Acessibilidade**: labels, foco visível, teclado e motivos textuais foram implementados e alguns verificados por Playwright; não houve teste com leitor de tela nem auditoria WCAG automatizada.
- **Auditoria de licenças**: nenhum código OpenIdea incorporado. Dependências diretas declaradas (React/CodeMirror/Vite MIT; Tauri/tiny_http/serde/tempfile MIT ou Apache-2.0) não passaram por auditoria jurídica de distribuição; isso permanece no ticket 18.
- **Ponte HTTP**: sem autenticação, escuta apenas em loopback e é binário de desenvolvimento; nunca deve ser empacotada.
- O aviso de build de chunk >500 kB não é falha; é o bundle do CodeMirror.
