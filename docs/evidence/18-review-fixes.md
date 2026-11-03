# Evidência — Correções da revisão (gates, identidade TRX, término Windows)

Implementação local em 28/10/2026, base `4168a23` (ticket 18). Fonte: achados da revisão de código; contratos em [SPEC](../SPEC.md), [ticket 12](../tickets/12-run.md), [ticket 17](../tickets/17-execucao-testes.md) e [implementation-notes](../implementation-notes.md).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2, arm64 (Apple Silicon) |
| Rust / Cargo | rustc 1.99.0 / cargo 1.99.0 |
| Bun | 1.3.9 |
| SDK global / do projeto | 10.0.100 / 8.0.408 (resolvido pelo `global.json` da fixture) |
| UI | Chrome real via Playwright contra Vite + host-http; janela Tauri não dirigida |
| Portas | 4317/1420 mortas antes de cada execução; nenhuma deixada ao final |

## Achados corrigidos

1. **`run.start` não contornava mais o restore.** O gate agora é `require_trust` → `restore::require_restored` (mesmo `restore_required`/`action` do ticket 04) → avaliação → build identificado. Teste novo de fronteira: restore + build reais, `src/OrderDesk.Cli/obj/project.assets.json` removido, `run.start` devolve `restore_required` com `consumer:"run.start"`, `action.method:"restore.start"`, nenhum processo `run:*` e `run.state:"idle"`. O teste de gate que excluía `run.start` passou a exigi-lo, e o teste de restore do ticket 11 inclui `run.start`.
2. **TRX não fabrica mais identidade.** `TestDefinitions` é a única fonte; sem `testId` ou sem definição o resultado fica `test_id:null`, `matched:false`, `fullyQualifiedName:""` e o `testName` só aparece como label bruto. A UI mostra `unattributed (no TestDefinitions identity)` e `TRX testName only: …`. Dois testes unitários cobrem ausência de definição e `testId` desconhecido com display name igual a um FQN descoberto.
3. **Término no Windows é tree-safe e rotulado pelo que foi pedido.** `cfg(windows)` usa `taskkill /PID <pid> /T` e depois `/T /F` com argv separado (sem shell), cai para o killer do child se o taskkill falhar, e `requested` reflete o pedido real (`taskkill /T`, `taskkill /T /F` ou `child kill (taskkill failed)`). Unix continua TERM→SIGKILL no grupo, com os mesmos rótulos e testes.
4. Removido `Dap::session()` sem chamador.
5. `executable_stub` renomeado para `gate_unhandled_executable` (gate de trust/restore para métodos executáveis não implementados).
6. Códigos de erro novos documentados na lista canônica: `build_stale`, `source_changed`, `shell_missing`, `experimental_required`, `unsupported_archive`, `ambiguous_selection`, com nota de que são erros distintos dos estados do snapshot.
7. Novos testids estáveis documentados: `rename-preview-summary`, `dap-identity`, `debug-platform`, `debug-message-error`, `debug-frames-empty`, `debug-locals-empty`, `debug-breakpoints-empty`, `toolbar-debug-stop`.
8. Bloco duplicado de spawn/env `dotnet` extraído para `dotnet_command(command, cwd, sdk_root)` e reutilizado em restore, build, descoberta, execução de testes e run; mesmos argv/cwd/env, sem mudança de comportamento.

## Comandos e resultados reais

```text
$ bun run build
tsc --noEmit && vite build
✓ 62 modules transformed.
✓ built in 1.70s
```

```text
$ cargo test -p openridr-host
running 118 tests   ... test result: ok. 118 passed; 0 failed
running 9 tests     ... test result: ok. 9 passed; 0 failed
running 54 tests    ... test result: ok. 54 passed; 0 failed
running 9 tests     ... test result: ok. 9 passed; 0 failed
running 5 tests     ... test result: ok. 5 passed; 0 failed
running 0 tests     ... test result: ok. 0 passed; 0 failed
```

Total: 195 testes, 0 falhas. Inclui o teste real de fronteira de restore/run e os dois testes unitários de TRX.

```text
$ bun run test:e2e
49 passed (6.0m)
```

Nota de execução: a primeira rodada completa teve 2 falhas (restore real com exit 1 e uma avaliação sem `eval-state`); os dois specs passaram isolados (6/6). A segunda rodada completa teve 1 falha de inicialização do csharp-ls (`lsp-state` "Starting…"), que passou no rerun isolado (9/9). A terceira rodada completa, acima, passou 49/49. Os artefatos gerados pela suíte (`docs/evidence/logs/18-*`, screenshots) foram restaurados ao estado do HEAD para não poluir este commit; os logs desta evidência são os arquivos `18-review-fixes-*.log`.

Logs: `18-review-fixes-build.log` *(evidência removida do repositório público)*, `18-review-fixes-cargo-test.log` *(evidência removida do repositório público)*, `18-review-fixes-e2e.log` *(evidência removida do repositório público)*.

## Limitações (explícitas)

- **Windows não executado.** O caminho `taskkill /T` → `/T /F` é `cfg(windows)`, std-only e compila no desenho, mas não foi compilado cruzado (target não instalado) nem executado; permanece **não verificado**. Unix é o único caminho exercitado.
- **macOS Intel não verificado.** A máquina é arm64; instalação/DAP em Intel seguem `unsupported_platform`.
- A suíte E2E continua sensível a falhas intermitentes de rede/restore e de inicialização do csharp-ls; rerun isolado confirmou que não são regressões destas mudanças.
