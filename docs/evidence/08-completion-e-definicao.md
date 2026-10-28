# Evidência — Ticket 08: completion e go-to-definition reais

Implementação local em 28/10/2026, base `499bb9d`. Fonte normativa: [ticket 08](../tickets/08-completion-e-definicao.md), [SPEC](../SPEC.md) AC-12 (parcial), AC-01/07, AC-32/33, P0.3 e Integrações .NET, e gates G1–G5 do [índice de tickets](../tickets/README.md).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408` (`global.json`, `rollForward: disable`) |
| csharp-ls | 0.28.0 (`serverInfo.version` `0.28.0.0`), sessão LSP stdio real; SHA-256 do launcher `0bcd0dbc…f3e882c` |
| Fixture | `fixtures/OrderDesk`, copiada para diretório temporário com espaço no nome (`Order Desk completion`) |
| App home da evidência | `$TMPDIR/openridr-e2e-home` (tool já verificada; `tools.plan` + reuso) |

Logs: [`logs/08-drive.log`](logs/08-drive.log), [`logs/08-host-http.log`](logs/08-host-http.log), [`logs/08-lsp.log`](logs/08-lsp.log), [`logs/08-completion.json`](logs/08-completion.json), [`logs/08-definition.json`](logs/08-definition.json), [`logs/08-cargo-test.log`](logs/08-cargo-test.log), [`logs/08-build.log`](logs/08-build.log), [`logs/08-playwright.log`](logs/08-playwright.log), [`logs/08-tauri-build.log`](logs/08-tauri-build.log). Nenhum SDK foi instalado; a fixture do repositório não foi alterada (as cópias são temporárias).

## O que foi construído

- **Requisição síncrona no cliente LSP** (`crates/host/src/lsp.rs`): `Lsp::request` usa o `post_request` existente (id ↔ `Receiver`), espera no máximo **5 s** e remove a entrada pendente no timeout. Resposta de erro do servidor vira falha com mensagem; `stop` limpa os senders pendentes e a espera termina em `unavailable`. Não há `$/cancelRequest`: a espera é limitada e o resultado é descartado (limitação registrada abaixo).
- **Gate de capacidade e de documento** (`crates/host/src/lib.rs`): `lsp.completion` e `lsp.definition` passam por `require_trust` e só enviam quando a sessão está `initialized` e o servidor anunciou `completionProvider`/`definitionProvider`. Documento ausente no `didOpen`/`didChange` daquela sessão → `unavailable` com `documentKnown:false`; nada é inventado. Nenhuma semântica C# é implementada no host.
- **Resposta como dados, não erro**: as duas respostas carregam `state` (`ok|unsupported|unavailable|stale|timeout|failed`), `message`, `session`, `workspaceId`, `contextId`, `path`, `line`, `character`, `revision` (revisão do buffer enviada pela UI), `clientRevision` (versão do documento LSP), `documentKnown`, `coverage`/`coverageNote` e listas vazias quando indisponível. Edição nunca é bloqueada por uma resposta indisponível.
- **Correlação e obsolescência**: a resposta só é aceita se o workspace/contexto continuam válidos, o `Arc<Lsp>` é o mesmo da sessão que enviou, a sessão é a mesma e a versão do documento no host não mudou. Caso contrário recebe `state:"stale"` e lista vazia (`response_is_current`). Os testes unitários exercitam ids fora de ordem, timeout e troca de revisão/sessão.
- **Completion real**: resposta em array ou `CompletionList`; cada item leva `label`, `detail`, `kind`/`kindRaw`, `insertText`, `textEdit` normalizado (`range`+`newText`, inclusive `insert`/`replace`), documentação quando o servidor manda, `sortText`. A lista é limitada a **200** itens e informa `totalItems`/`truncated`; `isIncomplete` vem do servidor. `replaceRange` é o range do primeiro `textEdit` (ou `null`, quando o servidor não manda range).
- **Definição real**: aceita `null`, um Location/LocationLink ou array. Cada URI é convertida em path e canonicalizada; só entra arquivo que existe **e** está contido na raiz do workspace. Destinos fora do workspace/inexistentes/inválidos são contados em `rejectedTargets` e nunca viram navegação; se nada sobra, o estado é `unavailable` com a contagem. A UI recebe `path`, `line`/`character` do `selectionRange` (ou do `range`) e os ranges crus.
- **UI** (`src/CodeEditor.tsx`): `@codemirror/autocomplete` (já presente via `codemirror`, nenhuma dependência nova) com `override` do source e `activateOnTyping:false`; **Ctrl+Space** abre a completion, **Ctrl+B/Cmd+B** pede definição (keymap Rider). O source envia `path/line/character/revision`, marca `loading` e descarta a resposta quando o documento/revisão mudou. Ao aceitar, a edição só é aplicada se o texto antes e depois do range de completion continuar igual — digitar para filtrar é permitido, edição concorrente fora do range descarta. Definição também descarta se seleção/documento/revisão mudaram. `editor-lsp-status` (`data-state`) mostra idle/loading/ok/unavailable/unsupported/stale/timeout/failed, truncamento e cobertura parcial.
- **Trust/revogação**: os dois métodos estão em `EXECUTABLE_METHODS`; antes de trust devolvem `trust_required` (teste de boundary percorre a lista). Revogar/parar/reiniciar encerra a sessão; respostas pendentes viram `unavailable`/`stale` e nada é aplicado. Os atalhos na UI explicam o motivo.

## Respostas reais do csharp-ls

`logs/08-completion.json` (didOpen de `order.` na linha 3, cursor no character 6):

```json
{
  "state": "ok",
  "session": 1,
  "clientRevision": 1,
  "revision": 1,
  "path": "src/OrderDesk.Cli/Program.cs",
  "line": 3,
  "character": 6,
  "documentKnown": true,
  "itemCount": 6,
  "totalItems": 6,
  "truncated": false,
  "items": [
    { "label": "Equals", "kind": "method", "insertText": "Equals" },
    { "label": "GetHashCode", "kind": "method", "insertText": "GetHashCode" },
    { "label": "GetType", "kind": "method", "insertText": "GetType" },
    { "label": "Id", "kind": "property", "insertText": "Id" },
    { "label": "ToString", "kind": "method", "insertText": "ToString" },
    { "label": "Total", "kind": "property", "insertText": "Total" }
  ]
}
```

`detail` e `documentation` vieram `null` nesta fixture; `textEdit` não foi enviado pelo csharp-ls aqui (`replaceRange:null`), então o item foi inserido no cursor com `insertText`. O caminho de `textEdit`/range é coberto por teste unitário controlado, não por esta publicação real.

`logs/08-definition.json` (mesma sessão, `new Order(...)` na linha 2, character 20):

```json
{
  "state": "ok",
  "session": 1,
  "clientRevision": 1,
  "locations": [
    {
      "path": "src/OrderDesk.Core/Order.cs",
      "line": 4,
      "character": 11,
      "range": { "start": { "line": 4, "character": 11 }, "end": { "line": 4, "character": 16 } },
      "selectionRange": null
    }
  ],
  "rejectedTargets": 0
}
```

A linha 4 (0-based) é o construtor `public Order(string id, decimal total)` de `OrderDesk.Core` — o csharp-ls resolveu o construtor, não o texto. `logs/08-lsp.log` mostra `initialize accepted · completion · definition · rename · code actions …`, `server csharp-ls 0.28.0.0` e `Finished loading solution` antes das requisições.

Fluxo real medido (`logs/08-drive.log`):

```text
workspace.open → trusted → evaluation ok → restore ok → evaluation ok
tools.plan/tools.install → ready (reuso verificado)
lsp.start → starting → ready (session 1, coverage full)
lsp.didOpen → version 1
lsp.completion → ok, clientRevision 1, 6 items (Id/Total properties)
lsp.definition → ok, src/OrderDesk.Core/Order.cs 4:11
lsp.didChange → version 2; lsp.definition → ok com clientRevision 2
lsp.stop → stopped; trust.revoke → restricted; lsp.completion → trust_required
```

## UI real (Playwright/Chrome, Vite + host-http)

Novo teste “completion and definition use the real session and discard stale answers” na mesma suíte/real install do ticket 05:

1. Abrir solução → trust → evaluate → restore/reload → instalar/reusar csharp-ls → abrir `Program.cs`.
2. **Sem** `lsp.start`, Ctrl+Space: `editor-lsp-status` em `unavailable` com “Analysis is not running”; captura `08-completion-unavailable.png`.
3. `Start analysis`, esperar `Ready`; o host confirma `documentKnown` só depois do `didOpen`.
4. Substituir o buffer por `order.` (revisão 2 no host) e descobrir a lista real: 6 itens, `Id`/`Total` como `property`.
5. Ctrl+Space abre o popup real; `08-completion.png` mostra os itens. Digitar `Id` (filtro local) e Enter insere `order.Id` no buffer; o disco continua intocado.
6. Consultar `lsp.definition` no host para `new Order` (linha 2, character 20) e então Cmd+B na UI: abre `src/OrderDesk.Core/Order.cs` e o cursor vai a `4:11`, o destino devolvido pelo servidor; captura `08-definition.png`.
7. Com o servidor real pausado (`SIGSTOP`), Cmd+B deixa o status em `loading`; antes do `SIGCONT`, a seleção é movida. A resposta tardia é descartada (`data-state="stale"`, “selection changed”) e nenhuma aba é aberta.
8. Servidor pausado de novo, Ctrl+Space: após 5 s o estado é `timeout` (“did not answer within 5 seconds”); digitar continua funcionando. `SIGCONT` depois não aplica a resposta antiga.
9. Revogar trust: Ctrl+Space e Cmd+B mostram `unavailable` com “requires workspace trust”; a aba atual não muda.

Capturas: [`screenshots/08-completion.png`](screenshots/08-completion.png), [`screenshots/08-completion-unavailable.png`](screenshots/08-completion-unavailable.png), [`screenshots/08-definition.png`](screenshots/08-definition.png).

## Comandos e resultados reais

```text
$ cargo test -p openridr-host
running 46 tests
test result: ok. 46 passed; 0 failed; 0 ignored
running 41 tests
test result: ok. 41 passed; 0 failed; 0 ignored
```

```text
$ bun run build
tsc --noEmit && vite build
✓ 58 modules transformed.
dist/assets/index-C4ZkWAJy.js   638.36 kB │ gzip: 204.32 kB
✓ built in 775ms
```

```text
$ bun run test:e2e
  ✓ completion and definition use the real session and discard stale answers (17.2s)
  … 26 testes anteriores
  27 passed (1.5m)
```

```text
$ bun run tauri build --debug --no-bundle
   Compiling openridr v0.1.0 (/Users/helder/Projetos/openridr/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.52s
       Built application at: /Users/helder/Projetos/openridr/src-tauri/target/debug/openridr
```

Logs completos: `08-cargo-test.log`, `08-build.log`, `08-playwright.log`, `08-tauri-build.log`. O E2E é Chrome real contra Vite + host HTTP real, csharp-ls real e fixture real; não é a janela Tauri nativa (TCC).

## Cobertura do ticket, ACs e gates

| Item do ticket / AC / gate | Resultado | Evidência |
|---|---|---|
| Completion real na fixture e edição de `OrderDesk.Core` | 6 itens reais em `order.`; `Id`/`Total` são `property`; aceite por Ctrl+Space/Enter inseriu `order.Id` no buffer | `08-completion.json`, E2E 4, `08-completion.png` |
| Definição console → biblioteca em documento/posição do servidor | `new Order` → `src/OrderDesk.Core/Order.cs` 4:11 (construtor), aberto pela UI com cursor em `4:11` | `08-definition.json`, E2E 4, `08-definition.png` |
| Capacidades e estados indisponível/parcial/carregando/falha | `unsupported` quando a capability falta (unit); `unavailable` sem sessão, em `starting` e com documento não aberto; `timeout` real com servidor pausado; `coverage` ecoada e “partial coverage” no status | unit `capabilities_absent…`, boundary `lsp_completion_and_definition…`, E2E 4 |
| Host ausente/projeto não carregado/resposta sem destino não bloqueiam nem fabricam | Sem LSP: `unavailable`, listas vazias; destino fora do workspace/inexistente: `rejectedTargets` e `unavailable`/nada aberto; edição continua | E2E 4, unit `definition_locations_must_exist_inside_the_workspace` |
| Cancelamento/troca de documento/posição/revisão/sessão invalida resposta pendente | Host rejeita sessão/contexto/versão trocados (`stale`); UI descarta seleção/documento/revisão e edição fora do range; restart esquece documentos; E2E cobre seleção e timeout | unit `a_response_is_current…`, boundary restart, E2E 4 |
| Edição de completion só em revisão válida, sem sobrescrever buffer concorrente | Resposta só aplicada se prefixo/sufixo do range continuarem iguais; digitar para filtrar é permitido; mudança concorrente descarta | E2E 4 (digitação/aceite), código do source |
| Trust/revogação no host para início/reinício/solicitação; atalhos respeitam | `lsp.completion`/`lsp.definition` em `EXECUTABLE_METHODS`, `trust_required` antes de trust; revogação invalida; UI explica | boundary list, E2E 4 (revoke) |
| Proteção mínima de buffer de 01 permanece | Nenhuma escrita nova: completion altera apenas o buffer; save continua com `expected_hash` | suíte 01/07 verde (27 testes), E2E 4 não grava |
| G1 trust/fronteira | Nenhuma requisição antes de trust; sem semântica própria; destinos validados no host | boundary, unit |
| G2 contexto/integridade | sessão/contexto/versão do documento na resposta; resposta tardia descartada; sem inventar `serverVersion`/posição | unit + boundary + JSONs |
| G3/G5 proveniência | comandos, SDK, fixture, sessão, revisões, logs e capturas reais; limites explícitos | esta página + logs |
| G4 UI/privacidade | Atalhos documentados no status, foco no editor, estados textuais (não só cor), nada de rede externa/telemetria | E2E 4, capturas |

ACs da SPEC tocados: **AC-12 parcial** (completion/definição; rename em 09), **AC-01/07** (gate de trust), **AC-32/33 parcial** (macOS arm64).

## Limitações e pendências honestas

- **Windows x64, macOS Intel e CRLF não executados.** Posições UTF-16/CRLF no Windows, paths e keymap não têm prova própria; o ticket 18 deve repetir completion/definição nesses ambientes. A conversão de coluna usa o índice UTF-16 do documento LSP (mesma convenção do CodeMirror), validada só em macOS/ASCII e nos testes de URI com não-ASCII.
- **Sem `$/cancelRequest`.** Não há cancelamento no protocolo: a espera é limitada a 5 s, a entrada pendente é removida no timeout e o resultado tardio é descartado; o servidor pode concluir trabalho já pedido. A UI invalida o resultado por sequência/revisão/seleção.
- **Não há renovação/retry automático de uma completion que voltou `unavailable`** durante o `starting`/documento ainda não aberto; o usuário repete Ctrl+Space. O estado é explícito no status.
- **Cobertura parcial não foi exercitada por uma sessão real partial** neste ticket: o status sabe mostrar “partial coverage” e o host ecoa `coverage`, mas o E2E rodou com `coverage:"full"`. O caminho `partial` herdado do ticket 05/06 continua sem prova de UI nesta fatia.
- **`detail`/`documentation`/`textEdit` não vieram do csharp-ls nesta fixture** (`null`); o parser aceita string/MarkupContent e `TextEdit`/`InsertReplaceEdit`, cobertos por unit tests controlados. Snippets estão desligados (`snippetSupport:false`), então `insertText` é inserido como texto.
- **Cap de 200 itens** é provado por unit test com lista pequena (limite 2); não houve publicação real com mais de 200 itens. `truncated` e `totalItems` são explícitos.
- **Janela nativa Tauri não foi dirigida**: o build compila e a ponte `host_call` é a mesma, mas cliques/atalhos foram exercitados no Chrome via Vite + host HTTP (TCC bloqueia automação da janela no macOS). Smoke humano da janela nativa continua recomendado.
- **Nenhum screenshot da janela nativa**; as imagens são do mesmo build/UI servidos ao Chrome real, viewport 1440×900.
- **Definição só abre a primeira location** quando o servidor devolve várias; locations adicionais ficam na resposta (`locations`) mas a UI navega para a primeira. O contrato aceita array e o limite é explícito.
