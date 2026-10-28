# Evidência — Ticket 10: code actions reais no contexto atual

Implementação local em 28/10/2026, base `5a75082`. Fonte normativa: [ticket 10](../tickets/10-code-actions.md), [SPEC](../SPEC.md) AC-13, AC-11, AC-15, AC-01/07, AC-32/33, SPEC P0.3 e gates G1–G5 do [índice de tickets](../tickets/README.md).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` (`global.json`, `rollForward: disable`) |
| csharp-ls | 0.28.0 (`csharp-ls, 0.28.0 (Klebiškis)+72671fc7…`), sessão LSP stdio real |
| `codeActionProvider` real | anunciado com `resolveProvider: true` |
| Fixture | `fixtures/OrderDesk`, copiada para diretório temporário (`Order Desk actions`, `Order Desk actions diagnostic`) |
| App home da evidência | `$TMPDIR/openridr-e2e-home` (o mesmo das suítes 05–09) |

Logs: [`logs/10-lsp.log`](logs/10-lsp.log), [`logs/10-lsp-diagnostic.log`](logs/10-lsp-diagnostic.log), [`logs/10-csharp-ls-actions.json`](logs/10-csharp-ls-actions.json), [`logs/10-code-actions-diagnostic-actions.json`](logs/10-code-actions-diagnostic-actions.json), [`logs/10-cargo-test.log`](logs/10-cargo-test.log), [`logs/10-build.log`](logs/10-build.log), [`logs/10-playwright.log`](logs/10-playwright.log), [`logs/10-tauri-build.log`](logs/10-tauri-build.log). As cópias da fixture são temporárias e foram removidas; a fixture do repositório não foi alterada.

## O que foi construído

- **Requisição real** (`crates/host/src/lib.rs`): `lsp.codeActions {path,line,character,revision}` reusa `prepare_analysis_request` (trust, sessão `initialized`, `codeActionProvider`, documento no `didOpen`/`didChange`) e o timeout de 5 s. Envia `textDocument/codeAction` com `range` início==fim na posição e `context.diagnostics` somente dos snapshots **current** do store LSP para aquele URI cujo range cobre a posição; itens `awaiting`/`stale`, de outra origem ou fora da posição nunca entram no contexto. A versão do documento é capturada antes e revalidada pelo mesmo `analysis_invalidated` (workspace/contexto/`Arc<Lsp>`/sessão/versão) do rename.
- **Catálogo real, sem fabricação** (`crates/host/src/lsp.rs`): a resposta é `null` ou array; entrada sem `title` é contada como rejeitada. Cada ação preserva `title`, `kind`, `isPreferred`, `disabled` (motivo), `command` (título/nome, nunca executado) e `data`; o `edit` fica cru para ser validado pelo mesmo parser do ticket 09.
- **Prévia por ação** (`crates/host/src/lib.rs`): `parse_workspace_edit` + `workspace_edit_preview` (extraído do rename) validam o conjunto inteiro da ação — URI dentro do workspace, arquivo existente/legível, ranges válidos/não sobrepostos, versão do `TextDocumentEdit` e caps 50 documentos/2.000 edições/1 MiB. O resultado traz, por documento, `path`, `uri`, `opened`, `documentVersion`, `edits`, `changes`, `baseContent`, `newContent` e `baseHash`. Falha de uma ação vira `supported:false` com motivo; as outras continuam listadas. Comando, `data` sem `edit`, operação de arquivo ou edição inválida nunca viram preview nem execução.
- **Resposta**: `actionId` (sessão+sequência+FNV de workspace/contexto/posição/versão), `session`, `workspaceId`/`contextId`, `documentVersion`, `actions`, `actionCount`, `totalActions`, `truncated`, `rejectedActions`, `unsupportedActions`, `diagnosticsUsed`, `state`/`message`/`revision`. Não existe `codeActionApply` no host.
- **UI** (`src/WorkspaceView.tsx`, `src/CodeEditor.tsx`): botão focável “Code actions…” e Option+Enter (macOS)/Alt+Enter (Windows) com o editor focado. Estados indisponíveis/unsupported/empty são textuais e não bloqueiam edição; a lista mostra título, kind (ou “not provided by csharp-ls”), preferida, motivo de desabilitada e motivo de não suportada. Ação suportada abre a **mesma** prévia por documento do rename (Apply/Cancel); Apply revalida sessão/contexto/workspace/aba, revisão+conteúdo+conflito dos buffers e relê o hash de disco de todos os documentos. Qualquer divergência aplica nada. Editar o buffer, mover o cursor, trocar de aba, reiniciar/revogar, resposta tardia ou contexto novo descarta lista/prévia de ação; cancelar é inerte. Aplicar atualiza buffers como não salvos, envia `didChange` e não altera o store de diagnósticos.

## O que o csharp-ls 0.28.0 realmente retornou

Sessão real 8, `src/OrderDesk.Core/Order.cs` aberto na revisão 1, cursor 0-based `2:20` (UI `3:21`). `10-csharp-ls-actions.json` e `10-lsp.log`:

```text
[client] code actions at 2:20 · session 8 · 6 returned, 5 supported, 1 unsupported, 0 diagnostic(s) · actions-8-1-a298b2f4ea590485
```

| Título retornado | Resultado |
|---|---|
| `Generate overrides...` | suportada, 1 documento |
| `Extract interface...` | **não suportada**: o edit cria `src/OrderDesk.Core/IOrder.cs`, que não existe no disco; o conjunto é recusado por arquivo ausente/ilegível |
| `Generate constructor 'Order()'` | suportada, 1 documento |
| `Convert to positional record` | suportada, 1 documento, 1 edição |
| `Extract base class...` | suportada, 1 documento |
| `Add 'DebuggerDisplay' attribute` | suportada, 1 documento |

Na mesma fixture, `Order.cs` 0-based `5:0` (linha `{` indentada) retornou `[]`; a UI mostrou “No code actions from csharp-ls at 6:1” e continuou editável (`10-code-actions-empty.png`). O servidor **não enviou** `kind`, `isPreferred`, `disabled` nem `command` nesta fixture; a UI mostrou “not provided by csharp-ls”/“not preferred”. O parser e os caminhos de comando/desabilitada/`data` têm teste unitário dedicado.

Com um erro real no buffer (`MissingType` em `Program.cs`), o diagnóstico `CS0246` atual foi enviado como contexto e o servidor retornou o quick fix real:

```text
[client] code actions at 4:28 · session 10 · 4 returned, 3 supported, 1 unsupported, 1 diagnostic(s) · actions-10-9-1364f480c33cbd39
```

`10-code-actions-diagnostic-actions.json`: `Generate type 'MissingType' - Generate class 'MissingType' in new file` (não suportada; arquivo novo), `Generate type 'MissingType' - Generate class 'MissingType'` e `... nested class ...` (suportadas), além de `Convert to 'Program.Main' style program`. A UI mostrou “1 current diagnostic” e o botão do quick fix (`10-code-actions-diagnostic.png`). Nenhum arquivo foi escrito.

Observação de protocolo: quando o cliente anuncia `codeAction/resolve` (`resolveSupport`), o csharp-ls devolve as refatorações limpas apenas com `data`, sem `edit`. O app **não** anuncia resolve (mesmas capabilities do ticket 09), então as ações reais vêm com `edit`; uma ação `data`-only seria listada como não suportada com motivo, não resolvida silenciosamente.

## UI real (Playwright/Chrome, Vite + host-http)

Três testes novos na suíte real do ticket 05, com o mesmo csharp-ls e a mesma fixture:

1. **“code actions are listed from csharp-ls and preview/apply without writing disk”**: antes da análise o botão está desabilitado com “Start analysis”; depois de Ready/`codeActionProvider`, Alt+Enter em `Order.cs 2:20` abre a lista real (6 ações; a lista e o motivo da `Extract interface...` ficam visíveis, `10-code-actions-list.png`). Escolher `Convert to positional record` abre a prévia com `data-origin="action"`, 1 arquivo/1 edição e a linha `record Order(string Id, decimal Total);` (`10-code-actions-preview.png`). Cancelar mantém `public sealed class Order`, revisão 0 e os três arquivos byte-idênticos. Cursor em `5:0` mostra honestamente “No code actions” (`10-code-actions-empty.png`). Reabrir, escolher e Apply deixa o buffer com `record Order(...)`, aba modificada/revisão 1, disco intocado e a mensagem de que diagnósticos não são marcados corrigidos (`10-code-actions-applied.png`).
2. **“code action lists and previews are invalidated before any write”**: editar o buffer fecha a lista; editar com a prévia aberta fecha a prévia com “Code action preview discarded” e nada é aplicado; alterar `Order.cs` no disco após a prévia faz Apply recusar o conjunto com “changed on disk”, mantendo a prévia e o buffer (`10-code-actions-stale.png`); com o servidor pausado (`SIGSTOP`), pedir ações, mover o cursor e retomar (`SIGCONT`) não abre lista; revogar trust fecha a prévia e mantém buffer/disco. Nenhum arquivo muda.
3. **“code actions forward the current diagnostic and list the real quick fix”**: inserir `new MissingType()` em `Program.cs`, aguardar a publicação `CS0246` atual, pedir ações no identificador: a lista mostra “1 current diagnostic” e os `Generate type 'MissingType'` reais (`10-code-actions-diagnostic.png`); fechar não escreve nada.

Capturas: [`screenshots/10-code-actions-list.png`](screenshots/10-code-actions-list.png), [`10-code-actions-preview.png`](screenshots/10-code-actions-preview.png), [`10-code-actions-empty.png`](screenshots/10-code-actions-empty.png), [`10-code-actions-applied.png`](screenshots/10-code-actions-applied.png), [`10-code-actions-stale.png`](screenshots/10-code-actions-stale.png), [`10-code-actions-diagnostic.png`](screenshots/10-code-actions-diagnostic.png).

## Comandos e resultados reais

```text
$ cargo test -p openridr-host
test result: ok. 62 passed; 0 failed; 0 ignored
test result: ok. 41 passed; 0 failed; 0 ignored
```

```text
$ bun run build
tsc --noEmit && vite build
✓ 58 modules transformed.
dist/assets/index-xFVq1EVf.js   659.44 kB │ gzip: 209.36 kB
✓ built in 802ms
```

```text
$ bun run test:e2e
  ✓ code actions are listed from csharp-ls and preview/apply without writing disk
  ✓ code action lists and previews are invalidated before any write
  ✓ code actions forward the current diagnostic and list the real quick fix
  … 29 testes anteriores
  32 passed (2.4m)
```

```text
$ bun run tauri build --debug --no-bundle
   Compiling openridr-host v0.1.0 (…/crates/host)
   Compiling openridr v0.1.0 (…/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.13s
       Built application at: /Users/helder/Projetos/openridr/src-tauri/target/debug/openridr
```

## Cobertura do ticket, ACs e gates

| Item do ticket / AC / gate | Resultado | Evidência |
|---|---|---|
| Solicitar na posição/revisão atuais, respeitar capabilities e oferecer só o retornado | `codeActionProvider` exigido; 6 ações reais em `Order.cs 2:20`, 4 com diagnóstico em `Program.cs`; lista vazia real em `5:0`; botão desabilitado antes da análise; nenhum quick fix por CS | `10-csharp-ls-actions.json`, `10-code-actions-diagnostic-actions.json`, `10-lsp*.log`, E2E 1/3 |
| Contexto de diagnóstico atual e não-stale | `context.diagnostics` só de snapshots `current` do URI cobrindo a posição; E2E real com `CS0246`; unidade cobre fora da posição e `awaiting` (store do ticket 06) | E2E 3, unit `diagnostic_context_requires...`, `10-lsp-diagnostic.log` |
| Revalidar documento/contexto/revisão/todas as edições antes de aplicar; concorrência e resposta tardia não sobrescrevem | Apply revalida sessão/contexto/workspace/aba, buffers e hash de disco; buffer edit, cursor, disco, SIGSTOP/SIGCONT e revogação recusam/descartam; cancelar não muda nada | E2E 1/2, unit `rename_preview_*`, `workspace_edit_preview_*`, `10-code-actions-stale` |
| Só operações suportadas/validadas; comando não vira shell; trust/limites no host; não suportada explica motivo/log | `lsp.codeActions` em `EXECUTABLE_METHODS`; `trust_required` antes de trust (inclusive chamada direta); comando só é registrado e nunca executado; `Extract interface...` e arquivo novo aparecem não suportadas com motivo | `10-cargo-test.log`, unit `unsupported_actions...`, `10-csharp-ls-actions.json`, E2E 1 |
| Após aplicar, buffer modificado/decisão de salvar preservados; diagnóstico não vira corrigido antes de publicação | Aba `revision 1`/`saved 0`, disco intacto, `didChange` enviado, store de diagnósticos não é tocado; mensagem “diagnostics are not marked fixed” | E2E 1, `10-code-actions-applied.png`, código do host (nenhuma escrita no store) |
| Gates G1–G5, UI acessível, foco/teclado e estados ingleses honestos | Botão nativo focável, heading focado, Alt/Option+Enter, lista com motivos textuais, empty/unavailable honestos, sem modal bloqueante | E2E 1–3, capturas |
| G1 trust/fronteira | Nenhuma requisição antes de trust; host gate em todas as chamadas | boundary `executable_methods_fail_before_trust...` |
| G2 contexto/integridade | workspace/contexto/sessão/versão na requisição e na resposta; `actionId`; versão por documento | unit + E2E 1/2, `10-lsp*.log` |
| G3/G5 proveniência | Comandos, SDK, fixture, sessão, revisões, hashes, logs e capturas reais; limites explícitos | esta página + logs |
| G4 UI/privacidade | UI em inglês, docs PT-BR, foco/teclado, estados textuais; sem rede externa/telemetria | E2E 1–3, capturas |

ACs da SPEC tocados: **AC-13** (ações retornadas, aplicação em buffers e não antecipar correção), **AC-11** (buffers preservados), **AC-15 parcial** (diagnóstico não removido/marcado corrigido), **AC-01/07** (trust/revogação), **AC-32/33 parcial** (macOS arm64).

## Limitações e pendências honestas

- **Windows x64, macOS Intel e CRLF não executados.** A conversão UTF-16/CRLF é a mesma do ticket 09 e tem teste unitário, mas não há prova de UI/servidor nesses ambientes; o ticket 18 deve repetir.
- **Ação só com `Command` não apareceu em nenhuma resposta real do csharp-ls 0.28.0 nesta fixture.** O caminho é recusado por código e tem teste unitário (`unsupported_actions_are_listed_with_reasons_and_never_applied`); não houve execução de shell para provar em runtime real.
- **Ações `data`-only (lazy) não são resolvidas.** O app não anuncia `codeAction/resolve`; quando o cliente anuncia, o csharp-ls devolve refatorações limpas sem `edit`. Elas seriam listadas como não suportadas com motivo. Não há `lsp.codeActionResolve` nesta fatia.
- **Caps de prévia não atingidos por uma ação real** (máximo observado: 1 documento/1 edição, ou 3 edições no `Add 'DebuggerDisplay'`). Os limites 50/2.000/1 MiB são recusados por código e têm teste unitário; não houve sessão real acima dos caps.
- **Diagnóstico real exercitado só com `CS0246` e um item atual.** A filtragem de itens `awaiting`/`stale` e de ranges fora da posição tem teste unitário; não foi dirigida uma resposta real com múltiplos diagnósticos sobrepostos.
- **Janela nativa Tauri não foi dirigida**: o build compila e a ponte `host_call` é a mesma; cliques/atalhos foram exercitados no Chrome via Vite + host HTTP (TCC bloqueia automação da janela no macOS). Smoke humano da janela nativa continua recomendado.
- **Sem screenshot da janela nativa**; as imagens são do mesmo build/UI servidos ao Chrome real, viewport 1440×900.
