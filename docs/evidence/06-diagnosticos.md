# Evidência — Ticket 06: navegar diagnósticos por documento e revisão

Implementação local em 28/10/2026. Fonte normativa: [ticket 06](../tickets/06-diagnosticos.md), [SPEC](../SPEC.md) P0.3 (AC-13/14/15), “Estado, erros e processos”, “Avaliação e ferramentas” e Testing Decisions, e gates G1–G5 do [índice de tickets](../tickets/README.md). Base: `8929fa3` (ticket 05).

## Ambiente real

| Item | Valor observado |
|---|---|
| OS / arch | macOS 27.2 (build 26B118), arm64 (Apple M2) |
| Rust / Bun / Node | cargo 1.99.0, rustc 1.99.0 / Bun 1.3.9 / Node v26.10.0 |
| SDK do host | 10.0.100 em `/usr/local/share/dotnet` |
| SDK do projeto | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408`, `global.json` com `rollForward: disable` |
| csharp-ls | 0.28.0 real via NuGet.org, `--tool-path`; sessão LSP stdio real |
| Fixture | `fixtures/OrderDesk`, sempre copiada para diretório temporário; `Program.cs` alterado só na cópia |
| App home da evidência | temporário (`openridr-06-evidence-*`); o E2E usa `$TMPDIR/openridr-e2e-home` |

Logs: [`logs/06-drive.log`](logs/06-drive.log), [`logs/06-host-http.log`](logs/06-host-http.log), [`logs/06-lsp.log`](logs/06-lsp.log), JSONs `logs/06-diagnostics-*.json`, [`logs/06-cargo-test.log`](logs/06-cargo-test.log), [`logs/06-playwright.log`](logs/06-playwright.log), [`logs/06-build.log`](logs/06-build.log). Nenhum SDK foi instalado; nenhum arquivo de `fixtures/` foi restaurado/alterado.

## O que foi construído

- **Modelo/snapshot store** (`crates/host/src/diagnostics.rs`): snapshots por origem (`LSP · csharp-ls`), sessão, workspace/contexto, URI/documento, revisão do cliente (`clientRevision`/`revisionKnown`), recebimento, severidade bruta e rótulo, código, mensagem, `range`, posição inicial quando utilizável, `codeDescription`, categoria e `stale`.
  - **Categoria** só vem de marcador explícito na `source` (`compiler`/`analyzer`); `source:"lsp"` com código `CS0246` permanece **Unknown**. Nenhum prefixo `CS`/`CA` é interpretado.
  - **Severidade** vem da fonte: 1 error, 2 warning, 3 information, 4 hint; ausente/desconhecida → unknown.
  - **Revisão**: o csharp-ls não envia `version`; a publicação é atribuída à revisão do cliente no momento da recepção (ordem do protocolo). `serverVersion` continua `null`.
  - **Origem**: `Store::ingest` recebe uma origem; snapshots de origens diferentes nunca se substituem. Há teste com `Build · dotnet` para provar a estrutura do ticket 11, sem implementar Build.
  - **Sessão/contexto tardios**: `ingest` exige sessão/workspace/contexto atuais; caso contrário, ignora sem limpar nem sobrescrever.
  - **Obsolescência**: mesma sessão com revisão coberta menor que a atual → `awaiting`; outra sessão, análise parada/falha ou trust revogado → `stale`; itens antigos permanecem e ficam `stale: true`. Publicação vazia só vale para aquele URI/revisão.
  - **Cobertura**: `coverage` (full/partial), `coverageNote`, `analyzerCoverage: not_verified` e `analysisUnavailable` são explícitos; não existe campo de “solução limpa”.
- **Host** (`crates/host/src/lib.rs`, `lsp.rs`): `diagnostics` no snapshot com `session`, `origin`, `snapshots`, `awaiting`, `counts`, `summaries` por origem/sessão, `currentItems`/`staleItems`, cobertura e analyzer coverage. Início de sessão (`lsp.start`/`restart`) cria o escopo; revogar/parar/falhar marca análise indisponível e mantém as publicações; abrir/fechar workspace limpa o store do workspace anterior. `diagnostics.probe` (somente com `OPENRIDR_ENABLE_PROBE=1`, atrás de trust) publica um diagnóstico sem `range` para exercitar o contrato “sem posição, sem navegação”.
- **Problems UI** (`src/ProblemsPanel.tsx`): aba inferior funcional, ativável por teclado, com colunas severidade/código/mensagem/localização/documento/origem/categoria/revisão/estado, contagem por severidade, linha de cobertura por origem/sessão, filtro por documento, banner de stale/awaiting e detalhes com mensagem completa, origem, categoria, fonte, sessão/revisão, estado e motivo textual. Sem posição: “Go to position” desabilitado com motivo; “Open document without positioning” separado. Sem alegação de solução limpa.
- **Navegação** (`src/CodeEditor.tsx`, `src/WorkspaceView.tsx`): item atual com posição abre o documento e move seleção/scroll do CodeMirror para `line:character` publicados; posição exposta em `data-cursor` para teste. Item awaiting/stale apenas seleciona, mostra o motivo e não move o cursor; abrir sem posicionar continua permitido. Edição local marca imediatamente o snapshot correspondente como awaiting (comparação com `receivedAt`), depois o host confirma pela revisão do cliente.
- **Estabilidade do ticket 05**: a promoção `installing` agora fica observável por até 300 ms, como `verifying` já fazia; o E2E voltou a observar `downloading → installing → verifying → ready` sem mudar o comportamento de instalação.

## Publicação real e fluxo

Trecho real de `logs/06-diagnostics-broken.json` (CS0246 após `lsp.didOpen`; buffer com `new OrderReport()`):

```json
{
  "id": "lsp:1:file:///…/Order%20Desk%20problems/src/OrderDesk.Cli/Program.cs:0",
  "severity": 1,
  "severityLabel": "error",
  "code": "CS0246",
  "source": "lsp",
  "category": "unknown",
  "message": "The type or namespace name 'OrderReport' could not be found (are you missing a using directive or an assembly reference?)",
  "position": { "line": 5, "character": 17 },
  "range": {
    "start": { "line": 5, "character": 17 },
    "end": { "line": 5, "character": 28 }
  },
  "codeDescription": "https://msdn.microsoft.com/query/roslyn.query?appId=roslyn&k=k(CS0246)",
  "stale": false
}
```

Snapshot correspondente: `origin.label` `LSP · csharp-ls`, `session: 1`, `clientRevision: 1`, `currentRevision: 1`, `state: "current"`, `serverVersion: null`, `analyzerCoverage: "not_verified"`.

Fluxo real medido (`06-drive.log`, cópia temporária com espaço no caminho):

```text
trusted
tool ready: ok in 4.49s
restore: ok in 0.96s
evaluated ok
lsp ready: ok in 2.20s
lsp ready session 1
CS0246 publication: ok in 1.12s
immediately after edit: awaiting rev 1 items 1
restarted session: ok in 2.07s
after restart: session 1 stale items 1
session 2 CS0246: ok in 1.12s
empty current publication: ok in 0.31s
fixed total 14.03s
probe position None range None
revoked: state stopped unavailable "Trust was revoked; analysis is unavailable and these diagnostics are not current. Editing stays available."
```

Arquivos reais: `06-diagnostics-awaiting.json` (revisão 1 coberta, atual 2, um item retido stale), `06-diagnostics-restart.json` (sessão 1 stale com CS0246; sessão 2 sem publicações ainda), `06-diagnostics-fixed.json` (sessão 2 `current` com `items: []`; sessão 1 stale), `06-diagnostics-probe.json` (`position: null`, `range: null`), `06-diagnostics-revoked.json` (todos stale, `analysisUnavailable` acima).

## UI real (Playwright/Chrome 155, Vite + host-http)

Novo teste “problems navigate real diagnostics by revision and keep stale items”, com temp copy e csharp-ls real:

1. Abrir solução → trust → evaluate → restore → instalar/reusar csharp-ls → Start analysis → abrir `Program.cs`.
2. Aba Problems via teclado (foco + Enter); estado inicial `No problems reported for src/OrderDesk.Cli/Program.cs`, com “solution is not claimed clean”.
3. Editar o buffer para `new OrderReport()`; esperar publicação CS0246 real e `Problems (1)`.
4. Conferir colunas/origem/categoria/revisão; clicar na linha e verificar `data-cursor` do editor igual a `line:character` publicados (5:17).
5. Pausar o processo real com `SIGSTOP`; editar de novo. A linha antiga permanece visível como **Awaiting diagnostics**, `problem-stale` aparece e “Go to position” fica desabilitado; o cursor não se move. `SIGCONT` → republicação real com duas ocorrências de CS0246.
6. Restart analysis: itens da sessão 1 continuam na lista como **Stale**; clicar neles não navega. Sessão 2 republica o estado atual.
7. Corrigir o buffer; `No problems reported for src/OrderDesk.Cli/Program.cs · revision 2` com “Only this covered document/publication is claimed; the solution is not claimed clean”; analyzer coverage “not verified”.
8. `diagnostics.probe` publica diagnóstico sem range; a linha aparece, mas “Go to position” está desabilitado, os detalhes dizem “No position was published” e o cursor não muda.
9. Revogar trust: `problem-stale` contém “Trust was revoked”, linhas ficam stale, e editar/salvar continua funcionando.

Capturas: [`screenshots/06-problems.png`](screenshots/06-problems.png) (CS0246, origem/categoria/revisão, detalhes) e [`screenshots/06-problems-awaiting.png`](screenshots/06-problems-awaiting.png) (item retido como Awaiting, banner, navegação bloqueada).

## Comandos e resultados reais

```text
$ cargo test -p openridr-host
running 39 tests  (unit: framing/hostroot/tools/SDK/restore + 11 novos de diagnostics)
test result: ok. 39 passed; 0 failed; 0 ignored
running 40 tests  (boundary: tickets 01–05 + LSP real com awaiting/stale/restart/revogação e probe)
test result: ok. 40 passed; 0 failed; 0 ignored
```

```text
$ bun run test:e2e
  ✓ install plan is exact, cancel is inert, and the real install reaches verifying
  ✓ problems navigate real diagnostics by revision and keep stale items
  ✓ analysis loads, republishes diagnostics, survives a crash and stops on revoke
  … 17 testes dos tickets 01–04
  20 passed (1.1m)
```

```text
$ bun run build
✓ built in …
```

Logs completos: `06-cargo-test.log`, `06-playwright.log`, `06-build.log`. O E2E é Chrome real contra a UI servida pelo Vite e o host HTTP real; não é a janela Tauri nativa (limitação de captura TCC já registrada nos tickets anteriores). O host E2E usa `OPENRIDR_HOME` temporário e `OPENRIDR_ENABLE_PROBE=1`; o probe é development-only e não aparece sem essa variável.

## Cobertura do ticket, ACs e gates

| Item do ticket / AC / gate | Resultado | Evidência |
|---|---|---|
| AC-14 — severidade, código, mensagem, posição, documento, origem, categoria, revisão | Modelo e UI com todos os campos; `LSP · csharp-ls`; categoria `unknown` para `source:"lsp"`; sem inferência por `CSxxxx` | `06-diagnostics-broken.json`, unit tests de categoria/severidade, E2E 2 |
| AC-14 — LSP/build não mesclados apagando origem | Store por origem; teste unitário com `Build · dotnet` mantém dois snapshots/contagens; nenhum build implementado | unit test `origins_are_not_merged_and_keep_their_own_counts` |
| AC-15 — edição deixa anterior obsoleto/awaiting, sem correção presumida | `didChange` incrementa revisão; snapshot vira `awaiting`, item retido `stale:true`; republicação real substitui só quando chega | `06-diagnostics-awaiting.json`, `06-diagnostics-restart.json`, unit tests, E2E 2 (SIGSTOP) |
| AC-15 — publicação vazia só no documento coberto; restore parcial/analyzers explícitos | “No problems reported for <doc> · revision N” com “solution is not claimed clean”; coverage partial/`not_verified` explícitos | `06-diagnostics-fixed.json`, E2E 2, capturas |
| AC-13 (parcial) — não anunciar correção antes de nova publicação | O item antigo não é apagado/resolvido por edição ou clique; só a publicação real o substitui; code actions ficam no ticket 10 | E2E 2, `06-diagnostics-awaiting.json` |
| AC-01/07 — trust/revogação | `diagnostics.probe` passa por `require_trust`; revogar encerra o LSP, marca análise indisponível, preserva itens stale e edição | E2E 2, unit test `revoke_keeps_items_but_marks_them_stale`, `06-diagnostics-revoked.json` |
| AC-16 (parcial) — falha/espera/reinício e log | Sessão nova invalida publicações antigas; falha/stop mantêm log e itens; restart testado real | `06-lsp.log`, `06-diagnostics-restart.json`, E2E 2 |
| Acesso por teclado, foco visível, estados textuais | Aba/linhas são botões nativos, ativados por foco+Enter no E2E; severidade/origem/categoria/estado têm texto, não só cor; detalhes têm nome acessível | E2E 2, capturas, CSS `:focus-visible` |
| Publicação sem posição não inventa navegação | Store não cria `position` sem `start.line/character`; UI desabilita “Go to position” e oferece abrir sem posicionar | unit test `a_diagnostic_without_a_range_has_no_position`, E2E 2 (`diagnostics.probe`), `06-diagnostics-probe.json` |
| G1 trust/fronteira | Nenhum consumidor novo executa sem trust; probe atrás de trust + env dev; revogação preserva edição e não alega limpeza | boundary tests, E2E 2 |
| G2 contexto/integridade | Origem/sessão/workspace/contexto/URI/revisão no snapshot; publicação tardia de sessão/contexto anterior ignorada; `serverVersion` não inventado | unit tests `late_publication…`, `an_old_context…`, `06-diagnostics-*.json` |
| G4 UI/privacidade | Strings em inglês, controles nativos com nome/motivo, foco visível, sem telemetria/upload; logs locais | E2E 2, capturas |
| G5 validação/proveniência | OS/arch/versões, SDKs, fixture, revisões, comandos, logs, resultados e limitações abaixo | esta página + logs |

## Limitações e pendências honestas

- **Windows x64 e macOS Intel não executados.** CRLF, colunas com caracteres não-ASCII e posições no Windows não foram verificados; o E2E usa macOS arm64. O ticket 18 deve repetir navegação/obsolescência nesses ambientes.
- **Sem versão do servidor.** O csharp-ls não envia `version`; a atribuição é a revisão do cliente na recepção. Uma publicação que já estava em voo quando o `didChange` foi escrito pode ser atribuída à revisão nova (limite de ordem do protocolo já registrado no ticket 05). A UI/host nunca apresenta revisão antiga como atual por sessão, mas essa corrida estreita permanece.
- **Diagnóstico sem range**: o csharp-ls real sempre publicou `range` neste cenário. O contrato “sem posição, sem navegação” foi exercitado por `diagnostics.probe` (dev-only, `OPENRIDR_ENABLE_PROBE=1`, documento sintético `diagnostics-probe/NoPosition.cs`) e por teste unitário; não é uma publicação real do servidor.
- **Analyzers** não foram exercitados; `source:"lsp"` não permite distinguir Compiler/Analyzer. Por isso a categoria fica `Unknown` e `analyzerCoverage` permanece `not_verified` em toda a fatia.
- **Build** ainda não existe; a estrutura aceita origem `Build · dotnet` (teste unitário), mas nenhuma publicação/merge real de build ocorre aqui. O ticket 11 deve reutilizar o store sem apagar a origem LSP.
- **Pausa do servidor** no E2E usa `SIGSTOP`/`SIGCONT` (Unix) para manter a janela de awaiting determinística; não há equivalente Windows executado. Parada/revogação/restart reais também cobrem os estados de indisponibilidade.
- **Acessibilidade**: foco/teclado e texto foram exercitados, mas não houve teste com leitor de tela nem auditoria WCAG completa. O painel de detalhes rola localmente; listas longas permanecem dentro do painel inferior.
- **`diagnostics.probe`** permanece no binário como os demais probes de desenvolvimento, atrás de `OPENRIDR_ENABLE_PROBE=1` e trust; nunca é chamado pela UI normal.
- **Ajuste no instalador**: o estado `installing` agora tem dwell de 300 ms para permanecer observável (mesmo motivo do dwell de `verifying`); não altera origem, confirmação, verificação ou reuso.
