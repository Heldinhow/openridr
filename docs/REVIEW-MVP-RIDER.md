# Review — MVP Rider (F1–F9)

Data: 29/10/2026. Fixed point: `0130761` (main antes da F1). Revisado até `2bb59dd` (após as correções). Branch `mvp-rider`, 21 commits, somente local.

## Padrões

Fonte: `tsconfig.json` strict, estilo existente em `src/`, Rust 2021, regra de zero dependência nova de UI, testids como contrato.

| Achado | Situação |
|---|---|
| Comentário em português em `src/shell/toolWindows.ts` | **corrigido** |
| 15 `actionById.get(...)!` em `WorkspaceView` (crash possível em render) | **corrigido** — `requireAction` tipado |
| `messageOf` duplicado em 6 arquivos | **corrigido** — uma exportação em `host/client.ts` |
| 5 `eslint-disable` mortos em `src/git.tsx` (repo sem ESLint) | **corrigido** |
| Escrita atômica tmp+rename duplicada (`settings.rs`/`run_configs.rs`) | **corrigido** — `fsutil::write_json_atomic` |
| Trust E2E cobriria só 3 dos 14 métodos executáveis | **dispensado** — o boundary Rust `executable_methods_fail_before_trust_and_spawn_nothing` itera `EXECUTABLE_METHODS` (fonte única), já cobre `git.*`/`nuget.*` |
| `WorkspaceView.tsx` com 4.770 linhas e 96 hooks (Divergent Change) | **deferido** — débito registrado em `implementation-notes.md`; exige fatia própria |
| `git.rs` com três painéis no mesmo arquivo | **deferido** — idem |
| Bloco run→stdout/stderr repetido em `git.rs` (branch/create/checkout/pull/push) | **deferido** — refactor mecânico de host, sem ganho funcional nesta fase |
| Helpers `openTrusted`/`openTextOnly` copiados em ≥5 specs | **deferido** — mover para `environment.ts` na próxima fatia de testes |
| Cópias restantes de `messageOf` em TrustControls/ToolchainPanels/TerminalPanel/DebugPanel/ProcessPanel/SetupScreen | **deferido** — fora do escopo da correção; mesma dívida |

Build: `bun run build` limpo. Host: `cargo test -j1 -p openridr-host --lib -- --skip tools::tests` = 164 passed.

## Spec

A revisão de spec por subagent falhou três vezes por capacidade/autenticação do provedor (`ttft_admission_refused`). Substituída por verificação dirigida no código, nos contratos de maior risco:

- **Dependências**: `package.json` sem nenhuma dependência de UI nova. ✅
- **Trust**: `EXECUTABLE_METHODS` inclui `lsp.references`, `lsp.documentSymbol`, os 9 `git.*` e os 4 `nuget.*`; boundary test itera a lista. ✅
- **Git**: `commit --only` + `--amend`, sem force; push com confirmação; árvore suja recusada; `git.info` read-only. ✅
- **NuGet**: `--no-restore`, restore explícito, CPM recusado com `nuget_cpm_unsupported`, cap de 500 com `truncated`, falha nunca vira lista vazia. ✅
- **Busca**: caps 5.000/200/2.000/1 MiB, `workspace.files`/`search.text`/`workspaceSymbol` com estados honestos. ✅
- **Run configs**: arquivo por solução com `fnv1a64`, `activeId`, config ativa como fonte da verdade. ✅
- **F7**: gutter só com `Main`/teste inequívoco, `Refactor This` inexistente, references somente leitura, rename inline com testids preservados. ✅
- **Testes de teste**: debug de teste permanece desabilitado com motivo (decisão registrada), como aprovado na F5. ✅

**Limite desta revisão:** a verificação foi por amostragem dirigida, não pelo diff completo. A suíte Playwright completa não roda nesta VPS (sem .NET SDK, `channel: chrome` ausente) e o csharp-ls/DAP reais não foram exercitados aqui; tudo isso está em `implementation-notes.md` como não verificado.

## Resumo

- Padrões: 12 achados — 5 corrigidos, 1 dispensado com evidência, 6 deferidos como débito.
- Spec: nenhum requisito ausente ou errado encontrado na amostra verificada; cobertura real limitada pelo ambiente.
