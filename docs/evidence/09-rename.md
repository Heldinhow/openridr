# Evidência — Ticket 09: rename real com prévia válida

Implementação local em 28/10/2026, base `d43f248`. Fonte normativa: [ticket 09](../tickets/09-rename.md), [SPEC](../SPEC.md) AC-12 (parcial: rename, prévia, cancelamento e concorrência; completion/definição em 08), AC-11, AC-15, AC-01/07, AC-32/33, SPEC P0.3 e gates G1–G5 do [índice de tickets](../tickets/README.md).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` (`global.json`, `rollForward: disable`) |
| csharp-ls | 0.28.0 (`csharp-ls, 0.28.0 (Klebiškis)+72671fc7…`), sessão LSP stdio real |
| `renameProvider` real | anunciado com `prepareProvider` (`rename (prepare)` no resumo da sessão) |
| Fixture | `fixtures/OrderDesk`, copiada para diretório temporário com espaço no nome (`Order Desk rename`) |
| App home da evidência | `$TMPDIR/openridr-e2e-home` (ferramenta já verificada; `tools.install` reutilizou) |

Logs: `logs/09-drive.log` *(evidência removida do repositório público)*, `logs/09-rename.json` *(evidência removida do repositório público)*, `logs/09-lsp.log` *(evidência removida do repositório público)*, `logs/09-host-http.log` *(evidência removida do repositório público)*, `logs/09-environment.log` *(evidência removida do repositório público)*, `logs/09-cargo-test.log` *(evidência removida do repositório público)*, `logs/09-build.log` *(evidência removida do repositório público)*, `logs/09-playwright.log` *(evidência removida do repositório público)*, `logs/09-tauri-build.log` *(evidência removida do repositório público)*. As cópias da fixture são temporárias e foram removidas; a fixture do repositório não foi alterada.

## O que foi construído

- **Requisição real** (`crates/host/src/lib.rs`): `lsp.rename {path,line,character,newName,revision}` passa por `require_trust`, exige sessão `initialized`, `renameProvider` anunciado e o documento no `didOpen`/`didChange`; usa o pending id↔sender existente com timeout de **5 s**. Quando o servidor anuncia `prepareProvider`, envia `textDocument/prepareRename` e só prossegue se a posição for renomeável. `revision` é a revisão do buffer da UI, ecoada; a versão LSP do documento é capturada antes e revalidada na resposta (`analysis_invalidated`), junto de workspace/contexto/`Arc<Lsp>`/sessão. Nenhum rename textual é gerado quando o servidor não responde.
- **Prévia por documento** (`crates/host/src/lsp.rs` + `lib.rs`): `WorkspaceEdit` em `changes` e/ou `documentChanges` (`TextDocumentEdit`); operação de arquivo (`create`/`rename`/`delete`) e entrada malformada recusam o conjunto. Cada posição LSP é convertida de **UTF-16** para byte offset no host, considerando CRLF e pares substitutos; as edições são aplicadas sobre o texto da sessão para documentos abertos e sobre o disco para os não abertos. O resultado traz `path`, `uri`, `opened`, `documentVersion`, `editCount`, `edits`, `changes` (linha + antes/depois), `baseContent`, `newContent` e `baseHash`. URI fora do workspace, arquivo ausente/ilegível, intervalo inválido/sobreposto e `TextDocumentEdit.version` divergente rejeitam a prévia inteira.
- **Caps** (`lsp::RENAME_*`): 50 documentos, 2.000 edições e 1 MiB de texto (base+novo). Acima disso a prévia é **recusada com motivo**, não truncada; `displayTruncated` só encurta a exibição de linhas longas. A resposta leva `renameId` (sessão+sequência+FNV de workspace/contexto/posição/nome/conteúdo), `prepared`, `documentCount`, `editCount`, `session`, `workspaceId`/`contextId` e `revision`.
- **Aplicação só em buffers** (`src/WorkspaceView.tsx`): Shift+F6 (Rider) ou o botão focável “Rename symbol…” abrem um prompt modal. Antes da requisição a UI drena o debounce de `didChange` para o servidor ver o buffer atual. A prévia é um painel **não modal** (o editor continua editável) com arquivos, “not opened here”, contagem e linhas antes/depois. Apply revalida sessão/contexto/workspace/aba ativa, revisão+conteúdo+conflito dos buffers abertos e relê o hash de disco de todos os documentos (não abertos contra `baseHash`; abertos contra o hash base da aba). Qualquer divergência mantém a prévia com motivo e aplica **nada**; trocar de aba fecha a prévia. Sucesso atualiza todas as abas numa única atribuição: abertas ganham `revision+1` (não salvas, `savedRevision` inalterado); não abertas viram abas `1/0` com `hash` base; `didChange` só é enviado para documentos já abertos na sessão. Nenhum arquivo é escrito; salvar continua o caminho explícito do ticket 07.

## Resposta real do csharp-ls

`logs/09-rename.json` (sessão 1; `Order.cs` aberto na versão 1; cursor em `2:20`, classe `Order`):

```json
{
  "state": "ok",
  "session": 1,
  "workspaceId": 1,
  "contextId": 2,
  "revision": 0,
  "clientRevision": 1,
  "renameId": "rename-1-1-501bdb38d2410fe0",
  "prepared": true,
  "documentCount": 3,
  "editCount": 5,
  "documents": [
    { "path": "src/OrderDesk.Cli/Program.cs", "opened": false, "editCount": 1,
      "baseHash": "fnv1a64:ecb22c4dcaf0365c" },
    { "path": "src/OrderDesk.Core/Order.cs", "opened": true, "documentVersion": 1,
      "editCount": 2, "baseHash": "fnv1a64:9ded8fc98a762576" },
    { "path": "tests/OrderDesk.Tests/OrderTests.cs", "opened": false, "editCount": 2,
      "baseHash": "fnv1a64:c16b8aa2be384431" }
  ]
}
```

Edições reais (`09-rename.json`, 1-based como exibido na prévia):

```text
src/OrderDesk.Cli/Program.cs:3        − var order = new Order("A-104", 42m);
                                      + var order = new Purchase("A-104", 42m);
src/OrderDesk.Core/Order.cs:3         − public sealed class Order
                                      + public sealed class Purchase
src/OrderDesk.Core/Order.cs:5         − public Order(string id, decimal total)
                                      + public Purchase(string id, decimal total)
tests/OrderDesk.Tests/OrderTests.cs:11 − var order = new Order("A-104", 42m);
                                      + var order = new Purchase("A-104", 42m)
tests/OrderDesk.Tests/OrderTests.cs:19 − var order = new Order("A-104", 42m);
                                      + var order = new Purchase("A-104", 42m)
```

`09-lsp.log` mostra `initialize accepted · … · rename (prepare) …`, `server csharp-ls 0.28.0.0`, `Finished loading solution`, `textDocument/prepareRename id 3 answered`, `textDocument/rename id 4 answered` e `rename Purchase at 2:20 · session 1 · 3 document(s), 5 edit(s) · rename-1-1-501bdb38d2410fe0`.

Fluxo real medido (`09-drive.log`):

```text
workspace.open → OrderDesk.sln · solution
trust.grant → granted=true
evaluation.run → ok
restore.start → running · dotnet restore … -nologo -v:minimal
restore observed exit 0
evaluation reload → ok
tools.install → ready (reused=true)
lsp.start → ready
lsp.didOpen src/OrderDesk.Core/Order.cs → version 1 · session 1
document known: true · client revision 1
lsp.rename → state ok · prepared true · 3 documents · 5 edits · rename-1-1-501bdb38d2410fe0
lsp.stop → stopped; trust.revoke → restricted
lsp.rename after revoke → trust_required: lsp.rename requires workspace trust; no process was started.
disk unchanged for the three fixture files (sha256 before = after)
```

## UI real (Playwright/Chrome, Vite + host-http)

Dois testes novos na suíte real do ticket 05 (mesmo csharp-ls e fixture reais):

1. **“rename previews real cross-project edits, cancels inert and applies to buffers only”**: abrir solução → trust → restore → abrir `Core/Order.cs` → análise Ready → cursor em `2:20` → Shift+F6. O prompt mostra `Order.cs · 3:21 · revision 0 · session N` e o input recebe foco (`09-rename-prompt.png`). “Preview rename” devolve **3 arquivos / 5 edições**; o índice da prévia mostra `Cli/Program.cs` e `Tests/OrderTests.cs` como “not opened here”, e o detalhe traz as linhas antes/depois (`09-rename-preview.png` e `09-rename-preview-files.png`). Cancelar mantém a revisão do editor, não abre aba do arquivo de testes e deixa os três arquivos **byte-idênticos**. Reabrir pelo botão e Apply deixa as três abas modificadas (`Order.cs`, `Program.cs`, `OrderTests.cs`), o buffer com `public sealed class Purchase`/`public Purchase(` e o disco **intocado** (`09-rename-applied.png`). O Save explícito grava só `Core/Order.cs`; `Program.cs` e `OrderTests.cs` continuam com `new Order` no disco e abas modificadas.
2. **“a stale rename preview is rejected as a whole and late answers are discarded”**: com a prévia aberta, alterar `tests/OrderDesk.Tests/OrderTests.cs` no disco → Apply mostra `changed on disk`, mantém a prévia, não cria abas e não altera os outros arquivos. Depois, com nova prévia, editar o buffer de origem → Apply mostra `changed after the preview` e nada é aplicado. Com o servidor pausado (`SIGSTOP`), abrir prompt, enviar e cancelar antes da resposta; após `SIGCONT` nenhuma prévia aparece. Por fim, nova prévia e `trust.revoke` → Apply mostra “Analysis is not running”, mantém a prévia e não muda buffers nem disco (`09-rename-stale.png`).

Capturas: [`screenshots/09-rename-prompt.png`](screenshots/09-rename-prompt.png), [`screenshots/09-rename-preview.png`](screenshots/09-rename-preview.png), [`screenshots/09-rename-preview-files.png`](screenshots/09-rename-preview-files.png), [`screenshots/09-rename-applied.png`](screenshots/09-rename-applied.png), [`screenshots/09-rename-stale.png`](screenshots/09-rename-stale.png).

## Comandos e resultados reais

```text
$ cargo test -p openridr-host
running 55 tests
test result: ok. 55 passed; 0 failed; 0 ignored
running 41 tests
test result: ok. 41 passed; 0 failed; 0 ignored
```

```text
$ bun run build
tsc --noEmit && vite build
✓ 58 modules transformed.
dist/assets/index-BDW7JHYd.js   650.08 kB │ gzip: 207.29 kB
✓ built in 929ms
```

```text
$ bun run test:e2e
  ✓ rename previews real cross-project edits, cancels inert and applies to buffers only
  ✓ a stale rename preview is rejected as a whole and late answers are discarded
  … 27 testes anteriores
  29 passed (2.1m)
```

```text
$ bun run tauri build --debug --no-bundle
   Compiling openridr-host v0.1.0 (…/crates/host)
   Compiling openridr v0.1.0 (…/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.07s
       Built application at: ~/Projetos/openridr/src-tauri/target/debug/openridr
```

## Cobertura do ticket, ACs e gates

| Item do ticket / AC / gate | Resultado | Evidência |
|---|---|---|
| Rename real com capacidade/contexto atuais e prévia console↔biblioteca | `prepareRename` aceito e `textDocument/rename` devolveu 3 documentos/5 edições, incluindo `Cli/Program.cs` e `Tests/OrderTests.cs` | `09-rename.json`, `09-lsp.log`, E2E 1 |
| Sem fallback textual; servidor ausente/capability ausente não fabricam edição | `unavailable`/`unsupported` antes da requisição; parse recusa `WorkspaceEdit` não suportado; nenhum parser C# | unit + boundary, código do host |
| Cancelar prévia sem diff de documento/buffer/disco | E2E cancela com revisão inalterada e três arquivos byte-idênticos | E2E 1, `09-rename-preview.png` |
| Apply explícito revalida todos os documentos, inclusive não abertos e conflitos | Releitura de hash no host + revisão/conteúdo/conflito dos buffers; falha mantém a prévia | E2E 2, unit `rename_preview_*` |
| Mudança durante prévia/confirmação invalida o conjunto inteiro | Arquivo não aberto alterado, buffer de origem alterado e revogação recusam tudo; nada parcial | E2E 2, `09-rename-stale.png` |
| Resposta tardia não aplica nada | Cancelar com servidor pausado; após `SIGCONT` nenhuma prévia; host descarta sessão/versão trocadas | E2E 2, unit `a_response_is_current…` |
| Revisões/modificado refletem o que foi aplicado; save segue 07 | Abertas `revision 1 → saved 0`; não abertas `1/0`; disco intocado; Save explícito gravou só `Order.cs` | E2E 1, `09-rename-applied.png`, teste 07 verde |
| Diagnósticos não viram corrigidos | Nenhuma escrita no store; mensagem diz que diagnósticos não são marcados corrigidos | código do host/UI, E2E 1 |
| Revogação/stop/sessão bloqueiam pendência | `lsp.rename` em `EXECUTABLE_METHODS`; após revoke `trust_required`; Apply com análise parada recusa e mantém a prévia | boundary, `09-drive.log`, E2E 2 |
| UI acessível e teclado | Shift+F6, botão focável, input com foco, prévia com título focável, botões nativos, motivos textuais | E2E 1, `09-rename-prompt.png` |
| G1 trust/fronteira | Nenhuma requisição antes de trust; host gate em todas as chamadas diretas | boundary `executable_methods…`, `09-drive.log` |
| G2 contexto/integridade | workspace/contexto/sessão/versão do documento na resposta e na revalidação; hashes base; nenhum estado inventado | unit + E2E 2, `09-rename.json` |
| G3/G5 proveniência | Comandos, SDK, fixture, sessão, revisões, hashes, logs e capturas reais; limites explícitos | esta página + logs |
| G4 UI/privacidade | UI em inglês, docs PT-BR, foco/teclado, estados textuais; nada de rede externa/telemetria | E2E 1–2, capturas |

ACs da SPEC tocados: **AC-12 parcial** (rename; completion/definição em 08), **AC-11** (buffers preservados), **AC-15** (diagnóstico não antecipado), **AC-01/07** (trust/revogação), **AC-32/33 parcial** (macOS arm64).

## Limitações e pendências honestas

- **Windows x64, macOS Intel e CRLF não executados.** A conversão UTF-16 com pares substitutos e CRLF tem teste unitário (`utf16_offsets_handle_astral_characters_and_crlf`), mas não há prova de UI/servidor nesses ambientes; o ticket 18 deve repetir.
- **Sem `$/cancelRequest`.** A espera é limitada a 5 s, a entrada pendente é removida no timeout e a resposta tardia é descartada; o servidor pode concluir trabalho já pedido. A UI invalida por sequência/revisão/sessão.
- **Caps de prévia não atingidos por um rename real** (3 documentos/5 edições). O limite de 50/2.000/1 MiB é recusado por código e os caminhos de parse/intervalo/fora do workspace têm teste unitário; não houve sessão real acima dos caps.
- **Sem `rename.apply` no host** por decisão de escopo: aplicar só altera buffers; a revalidação é na UI com leitura de hash no host. Não há transação no sistema de arquivos porque nenhuma escrita acontece.
- **Troca de workspace com prévia aberta não foi dirigida por E2E**: a view é desmontada e a aplicação deixa de existir; o host rejeita contexto/workspace antigos (`stale_context`/`accept_result`/`analysis_invalidated`) e a UI compara `workspaceId`/`contextId`. Revogação e troca de sessão foram exercitadas.
- **Janela nativa Tauri não foi dirigida**: o build compila e a ponte `host_call` é a mesma; cliques/atalhos foram exercitados no Chrome via Vite + host HTTP (TCC bloqueia automação da janela no macOS). Smoke humano da janela nativa continua recomendado.
- **Nenhum screenshot da janela nativa**; as imagens são do mesmo build/UI servidos ao Chrome real, viewport 1440×900.
- **`displayTruncated` só tem teste unitário de conversão**, não uma linha real acima de 2.000 caracteres na fixture; o corte é só visual e não afeta o texto aplicado.
