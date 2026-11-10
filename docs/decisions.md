# Decisões de implementação (registro incremental)

Formato: data, decisão, motivo, consequência. Só entram decisões pequenas tomadas durante a implementação; decisões grandes continuam na [SPEC](SPEC.md).

<!-- Append-only: num conflito de merge ficam as duas entradas, a desta branch primeiro e as antigas na
     ordem em que entraram — nunca reordenar nem reescrever entrada velha. -->

## 2026-10-27 — Base própria, sem código do OpenIdea

O shell é escrito do zero (Tauri 2 + Rust + React/TS). Nenhum arquivo do OpenIdea foi clonado ou incorporado; a "auditoria da base" resulta em: nada reutilizado, nenhuma licença de terceiro herdada além das dependências declaradas. Avisos e identidade OpenRidr são preservados. Motivo: não há checkout local e a SPEC exige auditar antes de reutilizar; copiar sem auditoria seria pior.

## 2026-10-27 — Host separado da UI, com ponte HTTP só de desenvolvimento

A lógica de host vive em `crates/host` (`openridr-host`), sem dependência de Tauri. A janela Tauri (`src-tauri`) e a ponte HTTP (`crates/host-http`) chamam a mesma função `dispatch(method, params)`. A ponte existe apenas para Playwright exercitar a UI real contra o host real e ferramentas reais; não é empacotada no app, escuta em 127.0.0.1 e não tem autenticação (portanto nunca é distribuída). Motivo: Tauri não é automatizável por Playwright no macOS; sem a ponte, E2E de UI viraria só contrato ou clique manual.

## 2026-10-27 — Abertura de workspace por caminho digitado/colado

O diálogo nativo de arquivos fica para depois. A tela inicial aceita caminho absoluto de `.sln`, `.csproj` ou diretório; o host lista candidatos e exige escolha quando há mais de uma solução/projeto. Custo menor e testável por teclado; trocar por seletor nativo não muda o contrato do host.

> **Estendida em 2026-11-02**: o host (`workspace.open`) passou a aceitar caminho relativo com `baseDir` e `.slnx`. A abertura externa (Finder, Dock, `openridr`) usa o mesmo fluxo: sem workspace montado, cai na tela inicial, que mostra candidatos, erro e o prompt de Trust como na digitação.

## 2026-10-27 — Trust só em memória

> **Substituída em 2026-11-01** por "Trust decision persistida por projeto" (abaixo). O código já divergia desta entrada: `trusted.json` guarda os caminhos confiados desde antes.

Confiança vale para a sessão do app e é zerada ao abrir/trocar/fechar workspace. Não há persistência de trust, para não transformar uma decisão antiga em autorização silenciosa.

## 2026-10-27 — Raízes de SDK detectadas, nenhum SDK instalado pelo app

O host inventaria SDKs em contexto neutro (cwd temporário) a partir de: `dotnet` do PATH, `~/Library/Application Support/OpenRidr/sdks/*` e `~/.dotnet`, se existirem. O SDK 8.0.408 usado pela fixture foi instalado pelo operador nessa pasta do usuário, fora do repositório; o app apenas detecta e nunca instala/altera SDK nem `global.json`.

## 2026-10-27 — Ferramentas em `~/Library/Application Support/OpenRidr/tools`

Destino absoluto fora do repo, por versão (`tools/csharp-ls/0.28`). Instalação local de ferramenta (`dotnet tool install --tool-path`), nunca global, sempre após plano confirmado.

## 2026-10-27 — Estado do host é consultado por polling

A UI chama `workspace.state` a cada ~400 ms enquanto há operação em andamento. Evita eventos Tauri/SSE duplicados na ponte HTTP e mantém uma única forma de leitura. Limite: operações muito longas mostram progresso apenas pelo que a fonte informa (sem percentual inventado).

## 2026-10-28 — Ticket 01: hash de conteúdo e conflito explícito

`file.read` devolve `{path, content, hash}`; `file.write` exige `expected_hash`. O hash é FNV-1a 64 do texto (suficiente para detectar mudança entre leitura e gravação; não é assinatura). Se o disco mudou, o host devolve `conflict` com `currentContent`/`currentHash` e não grava; a UI só oferece `Reload from disk (discard my edits)` e `Overwrite disk`. Gravação usa arquivo temporário + rename no mesmo diretório. Comparação lado a lado fica no ticket 07.

## 2026-10-28 — Ticket 01: trust desabilitado na UI, gate real nos stubs

Até o ticket 02 não existe forma de conceder trust: checkbox e `Trust and inspect project` ficam desabilitados com motivo textual. Os dez métodos executáveis já existem no dispatcher e retornam `trust_required` antes de qualquer processo; teste de boundary abre um workspace temporário, chama todos e verifica que nenhum `bin/`/`obj/` foi criado. O branch pós-trust retorna `invalid_state` (não implementado), pronto para 02+.

## 2026-10-28 — Ticket 01: fixture validada só na autoria

A fixture foi restaurada/compilada/testada uma vez com o SDK 8.0.408 fixado, via `DOTNET_ROOT` apontando para `~/Library/Application Support/OpenRidr/sdks/8.0.408`; 2 testes xUnit passaram. Isso é evidência de autoria, não fluxo do app: nenhuma ação do OpenRidr executa dotnet nesta fatia. `bin/obj` ficam no `.gitignore` da fixture.

## 2026-10-28 — Ticket 01: E2E usa a fixture real e restaura o arquivo

Os testes Playwright abrem `fixtures/OrderDesk` de verdade, editam/salvam `Program.cs` e induzem conflito escrevendo no arquivo pelo processo de teste; `beforeEach`/`afterEach` restauram o conteúdo original. Workers = 1 porque o host e o arquivo são estado compartilhado. A captura de tela do desktop foi bloqueada pelo TCC de gravação de tela desta máquina; a evidência registra a janela via CoreGraphics e usa screenshots do mesmo build servido ao Chrome.

## 2026-10-28 — Ticket 02: um gate executável e tokens de contexto

`require_trust` é o único ponto que libera stubs executáveis: confere trust, workspace aberto e devolve `ContextToken {workspaceId, contextId}`. O snapshot também carrega `contextId`; a UI ignora snapshot com contexto menor, para que `app.state` inicial ou polling não façam a tela voltar a um estado antigo. `Host::accept_result(token)` rejeita resultado de contexto antigo com `stale_context`; hoje todos os fluxos são síncronos, então o token é o seam que tickets assíncronos (03+) usarão antes de gravar estado. Revogar/fechar/trocar e redetectar incrementam o contexto; trust grant também, porque muda o que pode executar.

## 2026-10-28 — Ticket 02: inventário por raiz, host 10 e projeto por global.json

Cada raiz detectada roda seu próprio `dotnet --list-sdks` em cwd temporário, sem `DOTNET_ROOT` (não funde raízes nesta plataforma). Host de análise é o maior SDK 10.x estável entre as raízes. O SDK do projeto só é resolvido após trust, lendo `global.json` do root, aplicando `version`/`rollForward`/`allowPrerelease` com seleção em estágios (.NET: patch mantém a versão exata; feature/minor/major preferem a banda pedida e só então sobem; latest* escolhem o topo do escopo). A maior versão satisfatória vence e a raiz escolhida fica no estado. Ausência vira `sdk_missing`; nunca há fallback para o SDK 10 nem escrita no arquivo. `redetect` só refaz o inventário e, se já houver trust, re-resolve o `global.json` em modo somente leitura; não avalia MSBuild nem grava arquivo.

## 2026-10-28 — Ticket 02: processos observados, não presumidos

O registro guarda `state running|exited`, `exit: null` como desconhecido, `awaitingDecision`, `kept` e o resultado de término. `try_wait` é consultado a cada snapshot; `processes.terminate` envia TERM ao grupo, espera ~2 s e envia KILL, reportando o que observou; `keep` mantém o processo visível sem alegar limpeza. Revogar/fechar/trocar marca pendentes, mas não mata sozinho. O child controlado vem de `processes.probe`, método de desenvolvimento atrás de `OPENRIDR_ENABLE_PROBE=1`; PTY/build/run reais ficam nos tickets 12/15.

## 2026-10-28 — Ticket 02: trust compacto no workspace e painel de processos

A tela Toolchain mantém o card completo de trust e as linhas de SDK; dentro do workspace, o mesmo estado aparece numa faixa compacta para não esmagar árvore/editor. O painel de processos aparece no setup após fechar workspace (pendência visível) e no workspace apenas para processos daquele workspace, evitando que histórico de outro workspace ocupe a área do editor.

## 2026-10-28 — Ticket 03: avaliação por projeto, sem restore

`.sln` recusa `-getProperty`/`-getItem` (MSB1063), então a solução só fornece lista de projetos, configurações e pastas (parse textual); cada `.csproj` roda um único `dotnet msbuild ... -getProperty... -getItem...` com o muxer da raiz do SDK do projeto (8.0.408), cwd no diretório do projeto e `-p:Configuration`/`-p:TargetFramework` explícitos. Sem MSBuild para soluções, sem restore/build/tasks. O TargetPath é sempre o valor avaliado; multi-TFM sem seleção fica sem TargetPath e pede TFM, nunca escolhe um. `restored` é tri-estado (`restored`/`not_restored`/`unknown`) derivado de `obj/project.assets.json` vs. mtime do `.csproj`; `unknown` não vira banner de solução limpa.

## 2026-10-28 — Ticket 03: invalidação sem resposta tardia

O host publica `running` antes de executar (fora do lock), marca `stale` quando `file.write` toca `.csproj/.props/.targets/.sln/global.json` (incrementando o contexto), e no polling compara mtime/tamanho de projeto, `Directory.Build.*`, `Directory.Packages.props` e `global.json`. Revogar/trocar/fechar zera a avaliação; o token capturado antes rejeita o resultado tardio (`stale_context`). A UI avalia uma vez por contexto confiável e só refaz por seleção explícita de configuração/TFM, Re-evaluate ou retry — nunca aplica resultado antigo. Detalhes do projeto ficam numa coluna própria (visível sem esconder a árvore).

## 2026-10-28 — Ticket 04: restore é uma ação única, com processo registrado

`restore.start`/`restore.retry` só rodam com trust e SDK do projeto resolvido; `restore.cancel` pode parar o processo sem trust (é ação de segurança, como `processes.terminate`). O comando é sempre o argv separado `[<root>/dotnet, "restore", <sln|csproj>, "-nologo", "-v:minimal"]`, cwd na raiz do workspace, `DOTNET_ROOT` da raiz do SDK do projeto e grupo de processos próprio. O child entra no mesmo `Registry` dos outros processos, então cancelar/revogar/trocar workspace usa TERM→KILL e reporta o que observou; a UI mostra "requested" e "observed" separados e `exit` nulo vira "exit unknown".

Um restore por vez: com outro em execução, `restore.start` devolve `invalid_state` em vez de enfileirar. O log guarda as últimas 400 linhas/60k caracteres e diz quando truncou; stdout vem cru e stderr com prefixo `[stderr]`. Credenciais em URL de feed são mascaradas (`https://***@host/...`) e o ambiente nunca é logado. `restore.start {project?}` permite restaurar um projeto da solução; a UI sempre usa o escopo do workspace (solution ou project).

`restored` deixa de ser só mtime: `obj/project.assets.json` mais novo que o `.csproj` **e** `obj/project.nuget.cache` com `success: true`. O NuGet reescreve o assets mesmo em falha (NU1100), então mtime sozinho declararia sucesso falso; cache ausente/ilegível resulta `unknown`.

Sucesso marca a avaliação como stale (mesmo contexto confiável, sem incrementar `contextId`) e a UI só recarrega pelo botão explícito, que chama `evaluation.run` com configuração/TFM atuais; o TargetPath antigo aparece rotulado como avaliação anterior. Falha/cancelamento também marcam stale e preservam log e marcadores por projeto. Revogar/fechar/trocar durante restore termina o grupo, sela o registro (`sealed`, `staleReason`) para que um exit tardio não vire resultado do contexto novo e mantém o processo visível no painel; ao trocar de workspace o registro selado é descartado da UI, mas o processo continua no registry com o término observado.

Build/run/debug/testes passam por `restore::require_restored` antes de qualquer ferramenta: com assets ausentes/velhos ou restore falho devolvem `restore_required` com `details.action = {method:"restore.start"}` e nunca restauram. Implementar os consumidores reais fica nos tickets 11/12/16/17, que devem chamar a mesma política.

## 2026-10-28 — Ticket 05: csharp-ls via `--tool-path` com plano revalidado

A instalação roda exatamente `dotnet tool install csharp-ls --version 0.28.0 --tool-path <appHome>/tools/csharp-ls/0.28.staging`, com HOME isolado em `<appHome>/toolhome` (NuGet.Config default só com nuget.org), cwd neutro e nenhum acesso global. O plano inclui todos os campos materiais e um `planId` FNV-1a; confirmar revalida, mudança material devolve `confirmation_required` e nada é baixado. O candidato fica em staging, é promovido só após exit 0 e verificado (`--version` contém 0.28.0 + SHA-256 do launcher). Ferramenta válida existente é reutilizada sem sobrescrita; staging parcial nunca vira ready. O `dotnet tool install` real imprime “Skipping NuGet package signature verification.”; o host não inventa assinatura/hash de pacote. Motivo: G3/AC-04/05/06 e recuperação honesta de falha/cancelamento.

## 2026-10-28 — Ticket 05: hostroot com runtime 10 e somente o SDK do projeto

O hostroot copia o muxer e liga `host/fxr` 10, runtimes 10 + 8, SDK 8.0.408, `packs`, `metadata`, `sdk-manifests` e `templates` do root do projeto. O SDK 10 do sistema **não** é ligado em `sdk/`: na primeira composição com os dois SDKs, o MSBuildLocator do csharp-ls registrou 10.0.100 como default e o projeto net8 recebeu 14 erros `CS0518` (“Predefined type 'System.Object' is not defined”). Com só 8 em `sdk/`, o tool (net10.0) roda com o runtime 10 e o MSBuild usa 8.0.408; `--list-runtimes` prova o runtime 10. O root é reconstruído por marker quando os inputs mudam e nunca altera os SDKs/global.json. Windows: symlinks e ordem de descoberta não validados; o ticket 18 repete.

## 2026-10-28 — Ticket 05: cliente LSP stdio e diagnósticos por revisão do cliente

O host fala LSP por stdio com framing Content-Length em bytes/UTF-8, leitor em thread, ids/correlação, resposta a `workspace/configuration` (`[{}]`) e requests do servidor (`null`), `initialized` + `workspace/didChangeConfiguration`, stderr limitado e initialize com erro encerra o processo expondo a mensagem de SDK. `shutdown`→`exit` é seguido de término observado; se o processo não sair, o grupo é terminado. Cada restart troca a sessão (`Arc<Lsp>` novo), então mensagens tardias não alcançam a UI; as publicações da sessão antiga não são reaproveitadas. Diagnósticos são guardados por URI com `receivedAt`, itens normalizados e `clientRevision` (o csharp-ls não envia `version`); `serverVersion` fica `null`. `didOpen` envia o buffer; `didChange` envia texto completo com versão monotônica. Problems/obsolescência ficam no ticket 06.

## 2026-10-28 — Ticket 05: painéis ocupam a faixa de avisos, mas rolam
Instalação e análise ficam em duas linhas compactas na faixa de avisos; o plano detalhado é um `<details>` que abre sob demanda e o log é colapsável. A faixa tem `max-height: 30vh` e rola, e o grid do editor mantém 220px mínimos: sem isso, os novos painéis escondiam o banner de conflito/botões do editor no teste de conflito do ticket 01. Controles continuam nativos, com motivo textual em `title` e estado ao lado.

## 2026-10-28 — Ticket 06: snapshots por origem/sessão/revisão, sem inferir categoria

`crates/host/src/diagnostics.rs` guarda o último publication por origem/sessão/URI. `clientRevision` é a revisão do cliente na recepção (o csharp-ls não envia `version`); não há versão de servidor inventada. Mesma sessão com revisão coberta menor que a atual vira `awaiting` e mantém os itens com `stale:true`; sessão diferente ou análise parada/falha/revogada vira `stale` com motivo e sessão. `ingest` rejeita sessão/workspace/contexto que não sejam os atuais, então uma resposta tardia não limpa nem sobrescreve o snapshot corrente. Abrir/fechar workspace limpa o store; reiniciar mantém a sessão anterior como stale. Categoria só aceita marcador explícito `compiler`/`analyzer` na `source`; `source:"lsp"` com CS0246 fica `Unknown`, e analyzer coverage é sempre `not_verified`.

Problems é uma lista de botões nativos com colunas textuais e detalhes focáveis; item atual com posição navega pelo CodeMirror (`line:character`), item awaiting/stale não move o cursor e oferece abrir sem posicionar. Um item sem range nunca ganha posição. A publicação vazia só produz “No problems reported for <documento> · revisão N”; não existe estado de solução limpa. A UI também marca awaiting localmente quando o buffer mudou depois do `receivedAt`, para a edição ter efeito imediato entre polls.

`diagnostics.probe` (dev-only, trust + `OPENRIDR_ENABLE_PROBE=1`) injeta um diagnóstico sintético sem `range` para exercitar o contrato de navegação; não substitui a publicação real do csharp-ls usada no restante do fluxo. No instalador, `installing` ganhou dwell de 300 ms para continuar observável, como `verifying`, sem mudar a instalação.

## 2026-10-28 — Ticket 07: revisão de buffer, conflito comparável e fechamento com decisão

Cada aba guarda `revision` (+1 por mudança de conteúdo) e `savedRevision`; “modified” é a diferença entre elas ou conflito pendente, não comparação de texto. Recarga aplicada pelo CodeMirror não conta como edição. O conflito guarda o conteúdo/hash do disco para comparação lado a lado (LCS por linha, omitido acima de 1200 linhas por lado); `Overwrite disk` grava com o hash comparado, então uma segunda mudança externa conflita de novo. Foco e ativação de aba releem o arquivo e só levantam conflito, nunca trocam o buffer. Diálogos de fechar aba/workspace/app oferecem Save/Save all, Keep editing e descarte explícito; falha mantém buffer e diálogo com erro. `beforeunload` cobre Chrome; Tauri cancela `onCloseRequested` e usa `destroy()` com `core:window:allow-destroy`.

## 2026-10-28 — Ticket 08: completion/definição síncronas na sessão LSP existente

`lsp.completion` e `lsp.definition` usam o `post_request`/pending já existente com timeout de 5 s. Não há motor semântico nem leitura de texto para adivinhar: a capability precisa ter sido anunciada, o documento precisa estar no `didOpen`/`didChange` daquela sessão, e o resultado só é aceito se workspace/contexto, `Arc<Lsp>`, sessão e versão do documento continuarem os mesmos; caso contrário a resposta é `stale` com listas vazias. Indisponível/unsupported/timeout/falha também são respostas de dados (`state`), nunca erros que travem a edição. Completion devolve itens reais (label/detail/kind/insertText/textEdit normalizado), limita a 200 e expõe `totalItems`/`truncated`/`isIncomplete`; definição aceita `null`, um Location/LocationLink ou array, canonicaliza cada URI e só entrega arquivo existente dentro do workspace, contando destinos rejeitados em `rejectedTargets`. Não há `$/cancelRequest`: o timeout limita a espera e o resultado tardio é descartado. Na UI, Ctrl+Space (keymap padrão do CodeMirror) e Ctrl/Cmd+B (Rider) chamam o host com `path/line/character/revision`; a resposta é descartada se documento/revisão/seleção mudaram e o item só é aplicado se o texto fora do range de completion continuar igual (digitar para filtrar é permitido, edição concorrente não). O status `editor-lsp-status` mostra loading/ok/unavailable/unsupported/stale/timeout/failed.

## 2026-10-28 — Ticket 09: rename real com prévia por documento e aplicação só em buffers

`lsp.rename {path,line,character,newName,revision}` reusa o gate de `prepare_analysis_request` (trust, sessão `initialized`, `renameProvider`, documento no `didOpen`/`didChange`) e o timeout de 5 s. Quando o servidor anuncia `renameProvider.prepareProvider`, envia `textDocument/prepareRename` e só prossegue se a posição for renomeável; `null` vira `unavailable`, nunca rename textual. A resposta tardia é descartada pelo mesmo `analysis_invalidated` (workspace/contexto/Arc/sessão/versão do documento). O `WorkspaceEdit` aceita `changes` e `documentChanges` (TextDocumentEdit), recusa operações de arquivo (`create/rename/delete`) e entradas malformadas. O host converte cada posição LSP UTF-16 (com CRLF e pares substitutos) e aplica as edições sobre o texto da sessão para documentos abertos ou sobre o disco para os não abertos; calcula `newContent`, `baseHash`, `changes` (linha antes/depois) e recusa o conjunto inteiro se o URI sair do workspace, o arquivo não existir/for ilegível, o intervalo for inválido/sobreposto ou a versão do `TextDocumentEdit` divergir. Caps: 50 documentos, 2.000 edições e 1 MiB de texto por prévia; acima disso a prévia é recusada (não truncada). A resposta inclui `renameId` (sessão+sequência+FNV de workspace/contexto/posição/nome/conteúdo), `prepared`, `documentCount`/`editCount`, `session`, `workspaceId`/`contextId` e `revision` ecoada; não há `rename.apply` no host porque aplicar muda somente buffers.

Na UI, Shift+F6 (Rider) ou o botão “Rename symbol…” abrem um prompt modal com o nome; o debounce de `didChange` é drenado antes da requisição para o servidor ver o buffer atual. A prévia é um painel não modal (o editor continua editável), com arquivos, contagem, “not opened here”, linhas antes/depois e botões Apply/Cancel. Aplicar revalida sessão/contexto/workspace/aba ativa, revisão+conteúdo+conflito de cada buffer aberto e relê pelo host o hash de disco de todos os documentos (não abertos contra `baseHash`; abertos contra o hash base da aba); qualquer divergência mantém a prévia e recusa o conjunto. Sucesso atualiza todas as abas numa única atribuição: abertas ganham `revision+1` (não salvas), não abertas viram abas `1/0` com `hash` base; `didChange` é enviado aos documentos abertos na sessão. Nada é gravado; salvar continua o caminho explícito do ticket 07 e os diagnósticos não são marcados corrigidos. Cancelar/prompt não alteram buffer nem disco; resposta tardia, troca de aba/workspace/sessão ou revogação impedem a aplicação com motivo textual.

## 2026-10-28 — Ticket 10: code actions reais, catálogo honesto e prévia validada
`lsp.codeActions {path,line,character,revision}` reusa o gate de análise (trust, sessão `initialized`, `codeActionProvider`, documento no `didOpen`/`didChange`) e envia `textDocument/codeAction` com range início==fim e `context.diagnostics` apenas dos itens atuais do store LSP que cobrem a posição; publicações `awaiting`/`stale` e outras origens nunca entram no contexto. A resposta real (array ou `null`) é interpretada item a item: `title` obrigatório, `kind`/`isPreferred`/`disabled`/`command`/`data` preservados, entrada sem título rejeitada. `edit` passa pelo mesmo parser e pelo mesmo `workspace_edit_preview` do ticket 09 (extraído para reuso): o conjunto inteiro é validado por ação (workspace, leitura, ranges, versão, caps 50/2.000/1 MiB) e devolve `documents` com `baseContent`/`newContent`/`baseHash`/`documentVersion`. Ação com comando, `data` sem `edit`, operação de arquivo ou edição inválida é listada com `supported:false` e motivo; nenhum comando é executado e não há `lsp.codeActionApply`.

A UI usa botão focável e Option+Enter/Alt+Enter; a lista mostra título, kind, preferida, desabilitada e motivo de não suportada, sem fabricar quick fix a partir de código CS quando a resposta é vazia. Escolher uma ação suportada abre a mesma prévia/Apply do rename, com revalidação de sessão/contexto/workspace/aba, revisões/conteúdo/conflito dos buffers e hash de disco de todos os documentos; divergência aplica nada. Editar o buffer, mover o cursor, trocar de aba, reiniciar/revogar ou resposta tardia invalida lista/prévia de ação; cancelar é inerte. Aplicar atualiza buffers não salvos, envia `didChange` e não altera o store de diagnósticos. No csharp-ls 0.28.0, sem anunciar `codeAction/resolve`, as ações reais vêm com `edit`; o servidor não envia `kind`/`isPreferred`/`disabled`/`command` nesta fixture, e uma ação que cria arquivo novo (`Extract interface...`) é recusada por documento ausente, como esperado.

## 2026-10-28 — Ticket 11: build explícito do contexto avaliado, com diagnóstico separado

`build.start {project?, configuration?, targetFramework?, savedNote?}` exige trust, SDK do projeto resolvido, política `restore::require_restored` (mesmo `restore_required`/`action` do ticket 04), avaliação atual (`ok|partial`, não stale, mesmo workspace/contexto) e um `TargetPath` avaliado para o projeto escolhido. `configuration`/`targetFramework` informados precisam ser iguais aos da avaliação corrente: a UI troca a seleção chamando `evaluation.run`, nunca reutiliza um modelo antigo. O startup vem dos projetos `kind:"console"` da avaliação; `library`/`test`/`unknown` são recusados com motivo e, com mais de um console e `project` omitido, a resposta é `invalid_state` com `details.reason:"ambiguous"` e `details.candidates`. Um build por vez (outro em execução ou um restore em andamento devolve `invalid_state`).

O argv é exatamente `[<sdk-root>/dotnet, "build", <csproj absoluto>, "-c", <Configuration>, "-nologo", "-v:minimal", "--no-restore"]` mais `-p:TargetFramework=<tfm>` quando selecionado; cwd é a raiz do workspace, `DOTNET_ROOT` é a raiz do SDK do projeto e o child entra no `Registry` com grupo de processos próprio. Duas threads drenam stdout/stderr para o `LogBuffer` (400 linhas/60k, stderr com `[stderr]`, truncamento declarado); o log é preservado após falha/cancelamento. `exit` é observado, nunca presumido: `null` nunca vira 0 nem `ok`. `build.cancel` é ação de segurança sem trust, pede término do grupo e reporta `requested`/`observed`; o exit que não veio continua `null`. Revogar/fechar/trocar durante um build termina o grupo e sela o registro (`sealed`, `staleReason`), então linhas/exit tardios não viram resultado do contexto novo; fechar/trocar descartam o registro da UI (o processo segue no painel de processos) e revogar mantém o registro selado visível.

`build.problems` é parseado conservadoramente do texto de MSBuild (`path(line,column): error|warning CODE: message [project]`, aceitando 4 números para `range`), com posição 0-based apenas quando impressa; linhas não reconhecidas ficam só no log bruto. Paths são resolvidos contra a raiz/absolutos e canonicalizados; fora do workspace é rejeitado (não vira item navegável). Diagnósticos idênticos impressos de novo no resumo de falha são deduplicados; o cap é 200 com `problemsTruncated` e aviso textual. `source:"dotnet"` e `category:"unknown"` não vêm de prefixo `CS`/`CA`; a lista Build é exibida na aba Problems com origem `Build · dotnet`, separada do store `LSP · csharp-ls` (nenhum item é copiado entre origens).

Política de salvamento: o build lê o disco; o host não salva buffer, e a UI decide explicitamente entre “Save all and build”, “Build saved content (N unsaved buffers excluded)” e “Cancel”. `savedNote` registra no estado qual conteúdo salvo foi usado (o host guarda o texto informado pela UI; não é uma verificação de hash de buffers). Alterar avaliação/contexto, restaurar dependências, redetectar SDK ou deixar a avaliação stale marca o build como `stale` com motivo; salvar uma fonte depois do build marca o snapshot como **obsoleto** na UI (comparação com `finishedAt`), nunca como falha corrigida. Build falho/stale é o contrato que Run/Debug (tickets 12/14) devem consumir para não autorizar execução.

## 2026-10-28 — Ticket 12: Run consome o build identificado e recusa fonte salva alterada

> **Substituída em 2026-11-01** por "Run/Debug constroem implicitamente quando não há build; Restore segue explícito": os gates de build ausente/stale/falho/fonte salva deixaram de recusar o lançamento; `source_changed` e afins iniciam um novo build. Restore continua explícito e o run continua um por vez.

`run.start {project?, configuration?, targetFramework?}` exige trust e um build atual, bem-sucedido e não stale do mesmo workspace/contexto, projeto, configuração e TFM. Ele não compila, não restaura e não substitui path: o comando é exatamente `[<sdk-root>/dotnet, <TargetPath>]` quando o TargetPath é um `.dll` gerenciado (ou o próprio arquivo quando é um executável real), com cwd no diretório do projeto, env explícito e grupo de processos. TargetPath ausente/não-executável, build falho/stale, startup `library`/`test`/`unknown`, configuração/TFM não avaliados e ausência de build devolvem motivo + `action`/`buildLog` em `details`; `build_stale` é o código específico para resultado antigo. Sempre há um run por vez.

O build captura no início o hash FNV-1a dos arquivos-fonte salvos do projeto de startup e dos projetos referenciados (fechamento transitivo); `run.start` relê esses arquivos e recusa com `build_stale`/`reason:"source_changed"` quando qualquer hash mudou. A verificação não vê arquivos novos/removidos que só apareceriam numa nova avaliação; esses exigem `evaluation.run`/rebuild. Edição em buffer não salva não bloqueia o run: a UI avisa que o build salvo está rodando e oferece “Save all and rebuild” ou “Run saved build”, sem salvar sozinha. Fonte salva depois do build bloqueia a UI por tempo de salvamento e o host pelo hash.

`run.cancel` é ação de segurança sem trust e usa TERM→KILL no grupo, reportando `requested`/`observed`/`exit` separados. Revogar/fechar/trocar termina o grupo e sela o registro (`sealed`, `stale`, `staleReason`), então linha/exit tardio não vira resultado do contexto novo; revogar mantém o registro selado visível, fechar/trocar deixam o processo no painel. O estado do run é `idle|running|exited|failed|cancelled`; `exited` é neutro e o exit observado aparece como está — `exit:null` é desconhecido, nunca sucesso.

## 2026-10-28 — Ticket 13: netcoredbg aprovado, asset real e gate experimental no Mac ARM64

A versão aprovada é `3.2.0-1092` e a fonte é a release oficial `Samsung/netcoredbg` no GitHub. O metadata consultado em 28/10/2026 (`releases/tags/3.2.0-1092`, digest reconsultado e inalterado) publica quatro assets: `netcoredbg-osx-arm64.zip` (3.389.504 bytes, sha256 `f4fa33…4e59ea`), `netcoredbg-win64.zip` (3.524.161, `3c410a…b9274a`), `netcoredbg-linux-amd64.tar.gz` e `netcoredbg-linux-arm64.tar.gz`. **Não existe asset `osx-x64`**; Mac Intel fica indisponível sem substituir versão. O SHA-256 do plano é o `digest` do GitHub, não um hash calculado pelo OpenRidr; o host não inventa assinatura, notarização nem layout.

No Mac ARM64 o asset existe e é **community supported** (README da mesma tag: “may not work as expected”). A política é: flag experimental OFF → plano disponível mas `debug.install` recusa com `experimental_required` e nada é baixado; flag ON + trust + `confirm:true` → instala **somente** o `netcoredbg-osx-arm64.zip` oficial, verifica o SHA-256 publicado, extrai com segurança, roda `--version` (precisa conter `3.2.0`) e promove o staging. A flag é opt-in de sessão, independente de trust; ligá-la replaneja o plano exibido, então o consentimento seguinte autoriza a instalação experimental. Ela sozinha não instala e `ready` não declara DAP. A instalação real nesta máquina (app home padrão) levou 1,6 s, produziu `NET Core debugger 3.2.0-1 (9744e1f, Release)`, executável sha256 `ffc51b6f…24ece`, e um plano novo reutilizou a instalação sem baixar de novo. Mac Intel e Linux continuam `unsupported_platform`; Windows x64 tem plano/asset e caminho de instalação, não executado aqui.

Download: cliente HTTP bloqueante (`ureq` + rustls) com redirects desligados no agente e validação manual de cada `Location` — só HTTPS em `github.com`, `objects.githubusercontent.com` e `release-assets.githubusercontent.com`, no máximo 5 saltos. O corpo é gravado em `<staging>.download`, contado e hasheado durante a cópia (limite 20 MiB), comparado ao SHA-256 do plano e só então extraído. Extração usa o crate `zip` com `enclosed_name()` (recusa traversal/absoluto), recusa symlink e modo especial, limita 200 entradas e 200 MiB, restaura o modo Unix e ignora `__MACOSX`/`.DS_Store`. O executável (`netcoredbg`/`netcoredbg.exe`) é procurado por nome em qualquer profundidade (o zip oficial usa `netcoredbg/netcoredbg`), roda `--version` **dentro do staging**, e só então é promovido. Instalação válida existente é encontrada e reutilizada sem download; staging parcial, cancelado ou reprovado nunca vira `ready`. Progresso é só bytes observados. URLs logadas descartam query/assinatura. Um teste de fronteira (servidor local determinístico + zip controlado) força plataforma/URL/hash via `OPENRIDR_ENABLE_PROBE=1` + `OPENRIDR_DEBUG_PLATFORM`/`OPENRIDR_DEBUG_ASSET_URL`/`OPENRIDR_DEBUG_ASSET_SHA256`; o artefato controlado é um script rotulado e só existe no harness.

Dependências novas: `ureq 2.12` (rustls) e `zip 2` (deflate). Motivo: o host não tinha cliente HTTPS nem leitor de zip, e o ticket exige download/extração com validação própria em vez de delegar a um instalador global.

## 2026-10-28 — Ticket 14: cliente DAP stdio, gates de launch e prova real (flag mantida)

> **Substituída em 2026-11-01** por "Run/Debug constroem implicitamente quando não há build; Restore segue explícito" apenas no gate de build: a ausência de build Debug passou a iniciar um build implícito, e fonte salva alterada também. Os demais gates (trust, restore, adaptador/flag, avaliação, configuração Debug, assembly) e a proibição de restore implícito continuam.

O host fala DAP com o netcoredbg instalado em `crates/host/src/dap.rs`, usando o mesmo framing `Content-Length` em bytes do LSP (reuso de `lsp::read_frame`/`write_message`). O adaptador é iniciado exatamente como `[netcoredbg, "--interpreter=vscode"]`, com stdio piped, cwd no diretório do projeto e o `Registry` compartilhado (`kind:"debug:<Project>"`; `Arc<Mutex<Registry>>` passou a ser o dono único). Cada mensagem leva `seq` monotônico; requests têm ids com `pending` por id, timeout de 10 s (launch 20 s) e a resposta tardia é descartada. Eventos usam `body` (não `params`): `initialized`, `stopped`, `continued`, `output`, `exited`, `terminated`, `breakpoint`, `thread`, `process`, `module`. Reverse requests: `runInTerminal` spawna o argv exato em `Registry`, com cwd/env enviados, sem shell e recusando `argsCanBeInterpretedByShell`; retorna os pids observados. Outros reverse requests respondem `null`. Uma sessão por vez.

Launch gates em `debug.start`: trust → `restore::require_restored` (Debug nunca restaura) → adaptador `ready` com `--version` 3.2.0 e gate de plataforma/flag (Mac ARM64 exige opt-in; Intel/Linux `unsupported_platform`) → avaliação atual (`ok|partial`, mesmo workspace/contexto) → configuração **Debug** avaliada → startup console → build atual (`ok`, `exit:0`, não stale/sealed, mesmo `workspaceId`/`contextId`, mesma revisão de avaliação, mesmo projeto/configuração/TFM/TargetPath, hashes das fontes salvos inalterados) → assembly avaliado no disco. A avaliação ganhou `TargetDir`/`TargetFileName` e `assemblyPath` (`TargetPath` ou a junção), sem path hardcoded. Falha devolve `details.reason` (`no_build`, `build_failed`, `build_stale`/`source_changed`, `build_context_mismatch`, `assembly_missing`, `configuration_not_evaluated`, `evaluation_stale`, `tool_missing`, `experimental_required`, `unsupported_platform`) e não spawna processo. `Debug` só aceita `configuration` já avaliada; trocar para Release exige `evaluation.run`, nunca reaproveita TargetPath.

Breakpoints: `debug.setBreakpoints {path, lines}` guarda por fonte e, com sessão viva, envia `setBreakpoints` imediatamente; o estado `verified/pending/message` é sempre o que o adaptador devolveu. Sem símbolos permanece pendente; o evento `breakpoint` atualiza a linha (comparação por path absoluto/relativo). A UI tem uma gutter própria (`.cm-debug-gutter`) separada dos números: `●` breakpoint, `▶` linha atual; clique no marcador e Cmd+F8 (Mac)/Ctrl+F8 (Windows) alternam; F8/F7 fazem step over/into; Ctrl+D (Mac)/Shift+F9 (Windows) lançam. Todos os controles são botões nativos com nome, foco e motivo textual.

Pausa: o evento `stopped` incrementa a geração de stop e busca threads → `stackTrace` → `scopes` → `variables` do frame de topo em thread própria. `debug.variables {frameId}` recarrega um frame da pausa atual; frame de geração antiga é recusado. `continue`/`next`/`stepIn` só rodam pausados, incrementam a geração antes do request e limpam frames/locals; se um novo `stopped` chegar antes da resposta, a resposta antiga não sobrescreve a pausa nova. `continued` invalida a geração. Respostas tardias de threads/scopes/variables comparam `(session, stopSeq)` e são descartadas.

Encerramento: `debug.stop` (segurança, sem trust) envia `disconnect {terminateDebuggee:true}`, espera o exit do adaptador por até 3 s, termina o grupo se necessário, termina processos de `runInTerminal` ainda vivos e reporta `termination {requested, observed, exit, killedAfterGrace}` — exit ausente continua `null` ("exit unknown"). `terminated` espontâneo também dispara uma limpeza do adaptador em thread separada, porque o debuggee sair não prova que o netcoredbg saiu; o exit observado vai para `adapterExit`. `DapView.exit` é o exit do debuggee (evento `exited`), nunca presumido. `exitKnown:false` é exibido como `exit unknown`.

Revogar/fechar/trocar: `stop_debug` sela a sessão (`sealed`, `staleReason`), envia disconnect/termina o grupo e limpa remanescentes do debuggee; mensagens DAP tardias são ignoradas pelo `sealed` e pela troca de `Arc<Dap>`. Revogar mantém o registro selado visível; fechar/trocar voltam a `Dap::idle` e os processos seguem no painel `Registry`. Abrir/fechar resetam a flag experimental e a captura de breakpoints do contexto.

UI: aba Debug com `preparing/running/paused/terminated/cancelled/failed`, launch/stop/continue/step, lista de threads/frames, locals do frame, breakpoint pendente/verificado, identidade (sessão/build/assembly/adaptador/comando/cwd/pid do adaptador e do debuggee), exit/`exit unknown`, termination requested/observed, logs DAP/adaptador/host e output do debuggee preservados; motivo textual quando bloqueado. Testids novos: `dap-panel`, `dap-state`, `dap-phase`, `dap-session`, `dap-build-id`, `dap-assembly`, `dap-adapter`, `dap-process`, `dap-exit`, `dap-termination`, `dap-thread-list`, `debug-launch`, `debug-stop`, `debug-continue`, `debug-step-over`, `debug-step-into`, `debug-frames`, `debug-frame-*`, `debug-locals`, `debug-scope-*`, `debug-local-*`, `debug-breakpoints`, `debug-breakpoint-*`, `dap-log`, `dap-output`, `dap-disabled-reason`, `dap-sealed`, `dap-terminal-note`. `debug-state`/`debug-disabled-reason` continuam sendo do painel de instalação do ticket 13.

Prova real: macOS 27.2 arm64, netcoredbg 3.2.0-1092, fixture `OrderDesk` (breakpoint linha 3, step over, locals, continue, exit 0) e projeto async multi-thread temporário (breakpoint em método async, step over/into, breakpoint no worker, três thread ids, stop/revogação com árvore observada). Windows x64 não foi executado. A flag ARM64 **não foi removida**: o ticket exige mantê-la; a prova fica registrada para a decisão futura. #256 continua risco não reproduzido. Detalhes em `docs/evidence/14-debug-dap.md`.

## 2026-10-28 — Ticket 15: PTY real com shell descoberto, buffer próprio e término observado

`crates/host/src/terminal.rs` abre **uma** sessão PTY com `portable-pty 0.9` (host only; nenhuma dependência nova no frontend). O shell é descoberto, nunca assumido: `$SHELL` só vale se for caminho absoluto e executável; senão `/bin/zsh`, `/bin/bash`, `/bin/sh` (Unix) ou `COMSPEC`, PowerShell, `cmd.exe` (Windows, implementado e não executado). O argv é `[<shell>]`, cwd é o **root do workspace** e `TERM` só é definido se ausente. `terminal.start {cols?,rows?}` passa por `require_trust` antes de qualquer coisa observável: chamada direta em Restricted mode devolve `trust_required` e nenhum shell nasce. `terminal.write {data}`/`terminal.resize {cols,rows}` também exigem trust; `terminal.stop {confirm:true}` é ação de segurança sem trust, como `run.cancel`.

Entrada é byte cru no master do PTY: a UI manda a linha digitada + `\r` e Ctrl+C manda `0x03`, que o line discipline entrega como SIGINT ao **processo em foreground** (o shell é session leader com o PTY como controlling terminal, garantido pelo `forkpty` do portable-pty). O teste real interrompe `sleep 30` em <1 s e o `echo` seguinte responde. O shell não recebe interpolação de argv nem comandos fabricados; `dotnet test` é digitado pelo usuário (ou pelo operador de evidência) e roda de verdade.

O `Registry` ganhou `register_pty(kind, workspaceId, pid, killer)`: o shell entra no mesmo painel/processos dos outros tickets e `try_wait` é substituído por um slot preenchido pela thread que faz `wait()` no child. `terminate` mantém TERM→KILL no **grupo** (o shell é líder de sessão); antes do grupo, `terminal.stop`/fronteiras enviam TERM ao **foreground pgroup** lido com `tcgetpgrp` para alcançar um job com job control próprio, e no Windows o `ChildKiller` (TerminateProcess) é a limitação já registrada. `exit` é o código que o waiter observou; shell morto por sinal **não tem código** e fica `null` (“exit unknown”), nunca 0. `requested`/`observed`/`killedAfterGrace` são o que o registry viu; falha de término continua “still running” e visível.

Output é drenado em thread própria para um buffer de **4.000 linhas/120k caracteres** com `outputSeq` monotônico (incrementa por chunk) e aviso de truncamento. Sequências ANSI/OSC (cores, cursor, bracketed paste) são removidas porque a view é texto, não emulador; o texto imprimível do shell é preservado. O log sobrevive a exit/stop/falha e o input fica desabilitado quando o shell não está rodando.

Fronteiras: revogar trust termina o grupo, sela (`sealed`, `staleReason`) e mantém o registro visível; abrir/fechar/trocar termina, sela e depois volta o registro a `idle` (o processo continua no painel com o término observado). Output/exit tardios são ignorados enquanto selado, então não se anexam ao workspace novo. `sdk.redetect` incrementa contexto mas **não** derruba o shell: terminal não depende do modelo avaliado. A UI (`TerminalPanel.tsx`) mostra shell/cwd/tamanho/estado/processo, output rolável, input de uma linha, botão/atalho Ctrl+C, Start/Close e o diálogo “Keep terminal” vs “Terminate shell and close”; depois do close mostra exit/termination e mantém o log. O tamanho do PTY é medido do painel real e enviado por `terminal.resize` com debounce.

Windows: o caminho existe (ConPTY, descoberta por `COMSPEC`/PowerShell/`cmd.exe`, killer por `TerminateProcess`), mas **não foi executado** nesta máquina; a evidência registra o gap. macOS 27.2 arm64, zsh interativo e `dotnet test` real estão em `docs/evidence/15-terminal-pty.md`.

## 2026-10-28 — Ticket 16: descoberta VSTest por projeto explícito, MTP antes de argv

`tests.discover {project, targetFramework?}` exige trust, `restore::require_restored` (mesmo `restore_required`/`action` de 04), SDK do projeto resolvido, avaliação atual (`ok|partial`, não stale, mesmo workspace/contexto) e um projeto `kind:"test"` escolhido explicitamente — sem `project` ou com console/biblioteca a resposta é `invalid_state` com motivo. O TFM informado precisa ser o TFM avaliado; a UI troca a seleção por `evaluation.run`, nunca reutiliza o modelo antigo. Projeto multi-TFM sem TFM selecionado devolve `invalid_state` com `details.reason:"tfm_required"` antes de qualquer comando; TFM que o projeto não declara também é recusado. Um projeto MTP é detectado **antes de existir argv VSTest**: texto do `.csproj` com `Microsoft.Testing.Platform`/`UseMicrosoftTestingPlatformRunner` (mesmo `false`, por segurança) ou packageRef MTP devolve `state:"mtp_unsupported"` com exatamente `Microsoft.Testing.Platform is not supported in this version. Use the terminal.`, comando vazio, exit `null` e nenhum processo. Sem `Microsoft.NET.Test.Sdk` + adapter conhecido (xUnit/NUnit/MSTest) o estado é `adapter_missing` com orientação e comando vazio; o host nunca instala/edita package e nunca usa TRX como descoberta.

O comando é exatamente `[<sdk-root>/dotnet, "test", <csproj absoluto>, "-c", <Configuration>, "--no-restore", "--list-tests", "--nologo", "-v:minimal"]` + `["-f", <tfm>]`, cwd na raiz, `DOTNET_ROOT` do projeto e grupo próprio no `Registry` (`kind:"tests:<Project>"`). `--list-tests` **compila** o projeto (observado); descoberta não é livre de build. O parser só aceita o marcador inglês `The following Tests are available:` e linhas indentadas até a primeira linha não indentada; essa é a única lista real que o VSTest imprime. A identidade é o nome completamente qualificado impresso (trim), com id estável `test:<fnv1a64(project|tfm|fqn)>`, independente da posição; `displayName` guarda o mesmo texto porque `--list-tests` não fornece um segundo nome — ele nunca é usado sozinho como identidade e linhas repetidas contam uma vez. Duplicatas de nome, nomes aninhados (`Outer+Inner`) e espaços são preservados.

Estados de `tests.discovery`: `idle|discovering|completed|empty|failed|cancelled|adapter_missing|mtp_unsupported`. Marcador com zero linhas e exit 0 é `empty` (não falha); exit não zero ou exit 0 sem marcador é `failed` com log; `exit:null` nunca vira sucesso. `tests.discover_cancel` é ação de segurança sem trust, termina o grupo e preserva os itens já impressos com `partial:true`; o exit é só o observado. Uma descoberta por vez. Cada operação é um `Arc<Tests>` novo; threads antigas escrevem no registro antigo e `sealed` descarta linhas tardias, então discoveryId/revisão/contexto antigos não substituem itens/seleção atuais. Revogar/fechar/trocar encerra+sela (revogar mantém o registro visível; fechar/trocar voltam a idle); avaliação nova, restore concluído, `sdk.redetect` e avaliação stale marcam `stale` (nunca `failed`). A UI bloqueia checkbox quando stale/selada/parcial; edição de buffer ou save depois do início também aparece como stale local, comparando `editedAt` com `startedAt` (o host não vê buffers).

## 2026-10-28 — Ticket 17: execução VSTest real com TRX embutido e infraestrutura separada

`tests.run {project, targetFramework?, selection:"all"|"selected", testIds?}` reusa o gate do 16 (trust → `restore::require_restored` → SDK do projeto → avaliação atual → projeto de teste explícito/TFM), recalcula preflight MTP antes de qualquer argv e exige uma descoberta **atual** (`completed`, não stale/sealed, mesmo workspace/contexto/projeto/TFM). `selected` valida cada id contra os itens correntes; lista vazia, id desconhecido ou duplicado devolve `details.reason` (`empty_selection`/`invalid_selection`/`duplicate_selection`) e nada roda — nunca cai para `all`. Identidades com gramática de filtro VSTest (`| & ( ) = ! ~` aspas/quebras) são recusadas (`filter_refused`), não escapadas por adivinhação. Nome listado mais de uma vez (dois `[Fact(DisplayName=…)]` iguais) continua uma entrada, mas ganha `occurrences>1` e seleção individual vira `ambiguous_selection`: a lista real do VSTest não tem FQN nesse caso e o filtro por esse nome não casa nenhum teste. Run all sem filtro executa os dois e os resultados ficam `matched:false` em vez de inventar identidade.

O argv é `[<sdk-root>/dotnet,"test",<csproj>,"-c",<cfg>,"--no-build","--no-restore","--nologo","-v:minimal"]` + `["-f",<tfm>]` + `["--filter","FullyQualifiedName=A|FullyQualifiedName=B"]` só em `selected` + `["--logger","trx;LogFileName=openridr.trx","--results-directory",<appHome>/test-results/<ws>-<seq>-<ms>]`. O TRX é o logger embutido do VSTest; nenhum package/logger é instalado e o diretório fica fora do repo. cwd é a raiz, `DOTNET_ROOT` do SDK do projeto, grupo próprio (`kind:"tests:run:<Project>"`) e um run por vez (também não começa com descoberta em andamento).

O TRX é lido conservadoramente sem dependência XML: `<UnitTestResult>` fornece outcome/duration/Output, `<TestDefinitions>`/`<TestMethod className name>` mapeia o FQN para a identidade descoberta. `Passed/Failed/NotExecuted` ficam como a fonte diz; `NotExecuted` só vira `skipped` quando o `ResultSummary` do adapter imprime `Test 'FQN' was skipped in the test run.` (xUnit observado), senão permanece `notExecuted`; outcome desconhecido fica `unknown`. TRX parcial entrega os resultados completos e `trxState:"partial"`; fora de cancelamento (que fica `cancelled`) um TRX parcial é falha de infraestrutura, assim como ausente/corrompido, com o caminho bruto preservado — nunca sucesso. `completed` inclui falha de asserção (exit 1 explicado por teste failed/notExecuted); exit ≠ 0 sem teste que explique, exit `null`, ou TRX sem resultado é `failed` + `infrastructureFailure:true`. `cancel` mantém resultados parciais/log/TRX, estado `cancelled`, exit observado ou `null`; nenhum teste recebe `cancelled`.

Posição só nasce de `in <file>:line <n>` no stack trace quando o arquivo resolve dentro do workspace; o path volta relativo e a linha 0-based. Sem isso não há botão de navegação, só o log/TRX. Resultados ficam ligados a `runId`/`discoveryId`/revisões/contexto; avaliação/restore/redetect e selo marcam stale/sealed preservando resultados, e a UI compara `editedAt` com o início do run para marcar obsolescência local. Cada run troca o `Arc<TestRun>`; threads/exit antigos não substituem o run atual.

## 2026-10-28 — Ticket 18: aceitação e correções de auditoria (sem mudança de contrato)

A auditoria de aceitação encontrou divergências pequenas entre a SPEC/rótulos e o comportamento observado. Correções: no macOS, Run passou a responder a `Ctrl+R` (o handler usava Cmd+R, contra a SPEC e o rótulo `Run (Ctrl+R)`; `run.spec.ts` atualizado); o painel inferior pode encolher até 120 px e a faixa de avisos mantém 90 px em janelas de 1100×700; trocar de aba reseta a rolagem do painel inferior; o painel do terminal pode rolar (antes `overflow:hidden` podia esconder shell/cwd/controles); o rótulo LSP não duplica “partial coverage” e um processo sem exit mostra “exit not observed”. Nenhum contrato de host/snapshot mudou. O spec de aceitação fica em `tests/e2e/zz-acceptance.spec.ts` e roda por último para não alterar sessões/sequências assumidas pelos testes por ticket. Release continua bloqueado por Windows x64, macOS Intel, licenças/identidade e packaging; ver `docs/release-decision.md`.

## 2026-10-28 — Correções da revisão: run no gate de restore, TRX sem identidade inventada, término Windows

`run.start` passou a validar `restore::require_restored` antes da avaliação/build: um build bem-sucedido não autoriza lançar com assets invalidados, e Run continua sem restaurar implicitamente. O parser TRX só atribui identidade via `TestDefinitions`; resultado sem `testId`/definição fica `unattributed` (`test_id:null`, `matched:false`, FQN vazio) e nunca casa por display name. No Windows, `Registry::terminate` passou a pedir `taskkill /T` e, após a carência, `/T /F`, com fallback rotulado ao killer do child; Unix mantém TERM→SIGKILL no grupo. O caminho Windows não foi executado e permanece não verificado. Evidência: `docs/evidence/18-review-fixes.md`.

## 2026-10-29 — Ticket 24: NuGet sem PackageReference e fonte independente

No JSON válido do `dotnet list package`, projeto com `projects[]` presente e sem a propriedade `frameworks` representa lista vazia; `frameworks` presente com tipo inválido ou array vazio continua erro. Assim, a consulta `--outdated` não transforma a ausência em falha nem em prova falsa de pacotes atualizados. A consulta de fontes permanece independente: seu erro é apresentado como fonte desconhecida/consulta parcial em texto neutro, sem converter sucesso dos dados de pacotes em erro. A tabela usa overflow horizontal próprio, largura mínima e menos espaço vertical antes dos dados. Evidência de parser/UI é determinística; NuGet real/rede não foi executado.

## 2026-10-29 — Ticket 25: metadados neutros do executável Git

Settings precisa mostrar o caminho read-only resolvido pelo PATH mesmo em Restricted mode, mas a consulta de versão continua trust-gated (`git.info`). O novo `git.path` apenas procura o executável no PATH sem abrir repositório nem iniciar processo; `git.info` permanece a consulta completa protegida por trust. Settings oculta o caminho interno de settings.json e o keymap usa tabela sem rótulos redundantes.

## 2026-10-28 — MVP Rider: escopo, ordem e casca (grill F1–F9)

O grill do MVP fixou a ordem **F1 casca → F2 ações/keymap → F3 busca → F4 run configs → F5 tests → F6 git → F8 settings → F9 nuget → F7 código**. F1/F2 são fundação, F3 consome o registro de ações, F4/F5/F6 dependem da barra e F7 (gutter, ⌥↩, menu de contexto, rename inline, Find Usages) é a fatia mais cara em UI, coerente com o handoff ("depois NuGet e gutter icons/Alt+Enter"). Consequência: F4/F5 entregam Run/Debug/Test pela tool window e pelo widget; os ícones de gutter de run/teste ficam todos na F7, junto do gutter de breakpoint existente. Registro completo em `~/.opencode/plan/openridr-ui/GRILL-MVP.md`; spec em [SPEC-MVP-RIDER](SPEC-MVP-RIDER.md).

A barra superior é **uma só, de 44 px**, e absorve projeto, branch, run config, Run/Debug, busca e Settings, mantendo Configuration/TFM/startup compactos. Não há segunda fileira: duas barras custariam 84 px de cromo em 1100×700. Na F1 o branch aparece desabilitado ("Git not connected") e o run config até a F4.

**Testids são contrato**: a F1 pode mudar o DOM dos painéis, nunca os 372 nomes existentes, e a suíte E2E inteira (17 specs, 1.326 `getByTestId`) roda a cada fatia. Cada fatia adiciona seus testids e mantém `role="tab"` no cabeçalho inferior.

Persistência: layout (tamanho de slot, tool window ativa) em `localStorage`; run configs e settings em JSON no app home (`settings.json`, `run-configs/<fnv1a64(caminho da solução)>.json`). Nada dentro do repo, nada de estado de domínio no webview.

Casca: slots **esquerdo e inferior** apenas, uma tool window ativa por slot, sem split/detach/reordenar; o slot direito fica fora até existir tool que o use. Os 5 painéis atuais (Build, Problems, Debug, Terminal, Tests) migram para o registro na F1.

## 2026-10-28 — MVP Rider: ações, teclado e busca

A F2 entrega o registro central (id, título, atalho, disponibilidade, executor) e um popup próprio de **Find Action (⇧⌘A)**, que a F3 evolui para **Search Everywhere** (⌘T e Shift Shift, duplo toque de 400 ms) com All/Classes/Files/Symbols/Actions/Text. Um componente só evita dois popups e deixa a F2 testável.

Teclado: quando o foco está no editor, o CodeMirror resolve primeiro; o handler global (bubble, ignora `defaultPrevented` e campos de texto, salvo ações marcadas `editor:true`) só age no que sobrou. Precedência: remap do usuário > perfil > padrão. Conflito no mesmo atalho gera warning no registro, não bloqueia o app.

Busca (F3): Files = walk do root ignorando `bin/obj/.git/node_modules` (cap 5.000 arquivos, truncamento declarado); Classes/Symbols = novo `lsp.workspaceSymbol` no host sobre `workspace/symbol` do csharp-ls (handler confirmado na fonte 0.28, não exercitado no app), cap 500, separando tipo de membro pelo `kind`; sem trust/análise a aba diz "Analysis not ready" e não inventa lista vazia; Actions = registro; Text = `search.text` no host com walker próprio (sem ripgrep), cap 200 arquivos/2.000 ocorrências, pula arquivos > 1 MiB. A prévia mostra trecho + caminho e Enter abre; ⇧↩ (split) fica para S6.

## 2026-10-28 — MVP Rider: run configs, testes, Git, Settings, NuGet

Run config (F4): a **config ativa é a fonte da verdade** de `{project, configuration, targetFramework, args, workingDirectory, env, envFiles}`. A toolbar escreve na config ativa e dispara `evaluation.run`; nunca reusa TargetPath de outra configuração (regra do ticket 14). Default "Run <StartupProject>" derivada da avaliação é somente leitura; editar cria cópia nomeada. Run/Debug chamam `build.start` + `run.start`/`debug.start` existentes, sem pipeline novo; o gutter de Run fica para a F7.

Tests (F5): a árvore agrupa por projeto > prefixo do FQN > classe > método, declarando que o agrupamento é **derivado do nome** (FQN completo em tooltip), sem parser de fonte. "Rerun failed" usa os ids `failed`/`notExecuted` do último run completo e desabilita quando stale. Quick list ⌥⇧U entra mínimo (Run all, Run selected, Debug, Rerun failed, Stop); "teste atual no cursor" fica para a F7.

Git (F6): CLI `git` do PATH, sem libgit2 e sem instalar nada. Commit = `git commit --only -- <arquivos marcados>` (com `--amend` quando amend), então o index do usuário não é tocado; desmarcar não faz `restore --staged`. Checkout/pull recusam árvore suja e merge/rebase em andamento, sem auto-stash. Push só com confirmação (remote, branch, commits à frente) e sem force; credenciais do credential helper do usuário, nenhum token no app. Log = `git log --max-count=200` da branch atual com parents, sem grafo/paginação; refresh ao abrir, ao focar e por botão (sem polling). Sem repo, as tool windows dizem "Not a git repository".

Settings (F8): global, sem override por projeto; tema só escuro (tokens atuais são um tema), páginas Appearance (zoom/densidade), Keymap (perfis Rider macOS/IntelliJ + remap), Editor (fonte, tamanho, indentação/tabs), Build (SDKs detectados + Redetect, sem instalar SDK) e Version Control (caminho do git, read-only). Aplica ao vivo e persiste em `settings.json`.

NuGet (F9): instalados + desatualizados + update unitário com plano e confirmação. `dotnet list <csproj> package --format json` e `--outdated --format json`; update = `dotnet add <csproj> package <id> --version <v> --no-restore` seguido de restore explícito. CPM (`Directory.Packages.props`) lista mas recusa update; sem busca/instalação de pacote novo.

Código (F7): gutter Run/Test, popup ⌥↩ com prévia (reusa o preview do ticket 10), menu de contexto do editor, rename inline no CodeMirror (range da palavra, Enter aplica com revalidação, Esc cancela) e `lsp.references` (⇧F12) somente leitura. **Refactor This fica fora do MVP** (S1); o menu aponta para Rename.

Plataforma e ritual: Windows não executado nesta fase (release continua bloqueado, `docs/release-decision.md`); 1 commit por fatia, spec Playwright da fatia + suíte inteira verde, `/code-review` no fim. Nenhuma dependência nova (sem Zustand, cmdk, simple-git, lucide).

## 2026-10-28 — F1: recentes só de sessão, faixas no rail e slot esquerdo

O popup do projeto lista as soluções abertas nesta sessão; nenhum caminho recente é gravado, porque a F1 persiste apenas layout e a decisão anterior proíbe estado de domínio no webview. Abrir outra solução pelo popup exige zero buffers sujos e a WorkspaceView remonta por `workspace.id`, então abas do workspace anterior não sobrevivem. O slot esquerdo é o Solution Explorer; o inferior, Build/Problems/Debug/Terminal/Tests. As duas faixas dividem um rail fixo de 34 px à esquerda para continuarem clicáveis com a tool window oculta; clicar no ícone ativo alterna a visibilidade e ⇧Esc oculta a última ativa (exceto em input/textarea/contenteditable).

## 2026-10-28 — F2: registro único de ações, keymap e settings

A F2 passou barra, faixas, menu ••• e Find Action (⇧⌘A) a ler o mesmo registro (`src/actions/registry.ts`); ações de fatias futuras já estão registradas, mas desabilitadas com motivo (F3/F4/F5/F6/F7/F8). Atalhos são tecla+modificadores (`primary` = ⌘ no macOS, Ctrl fora dele), com perfis Rider (padrão) e IntelliJ; o remap do usuário em `settings.json` vence o perfil e o padrão e pode ser resetado pelo modal Keymap, que a F8 embute em Settings. Colisão é aviso no registro (paleta e Keymap), nunca erro: as duas ações continuam listáveis/remapeáveis e a primeira registrada vence no teclado. Ações indisponíveis continuam desabilitadas na paleta/menus com o motivo textual; quando alcançadas por um atalho existente, reportam o mesmo motivo em vez de sumir. Para não perder a memória muscular já testada, ⌃R e Ctrl+D permanecem como atalhos secundários de Run/Debug no macOS, ao lado de ⌃F5/⌥F5.

O CodeMirror continua primeiro: o handler global roda em bubble e ignora `defaultPrevented`, `input`/`textarea`/`select` e contenteditable fora do editor; só ações com `editor:true` podem rodar com foco no editor, e Tab passa a indentar via `indentWithTab` para cumprir o recorte. `settings.read`/`settings.write` gravam `settings.json` no app home (respeita `OPENRIDR_HOME`) com tmp+rename, validam objeto e o bloco `keymap` antes de substituir o arquivo e não exigem trust: nenhum processo ou arquivo do repositório é tocado. Evidência: `tests/e2e/actions.spec.ts` e `crates/host/src/settings.rs`.

## 2026-10-28 — MVP Rider: debug de teste desabilitado com motivo

`dotnet test` não expõe um fluxo de debug reutilizável pelos métodos atuais: `debug.start` lança o assembly do projeto, não um teste, e debugging de teste é P1 em `docs/PROPOSTA-UI-2026-10-28.md`. A tool window Tests e a quick list mostram "Debug selected" desabilitado com o motivo, em vez de mapear para `debug.start` e prometer algo que não é. Run, rerun failed e stop continuam reais. Consequência: a F5 não entrega debug de teste e o AC correspondente foi corrigido no ticket.

## 2026-10-29 — Ticket 21: indisponibilidade visual e confirmação após o plano

Controles `:disabled` de ação mantêm semântica nativa, mas recebem cores neutras e cursor de bloqueio; inputs/selects desabilitados usam o mesmo tratamento. Nos planos de csharp-ls e netcoredbg, o disclosure abre ao gerar o plano e o botão de confirmação fica depois dos campos materiais dentro dele, então não existe confirmação isolada antes da revisão. A faixa Workspace tools sobe de 24 para 30 vh conforme a decisão do ticket 05 e mantém rolagem própria/scrollbar identificável. Nenhum gate de host foi alterado.

## 2026-10-29 — Aproximação visual ao Rider escuro

A paleta passa a seguir o Rider (fundo `#191a1c`, chrome `#26282c`, seleção `#30426e`, acento `#548af7`; medidos em `docs/evidence/rider-reference`). Header: Back/Forward inertes, projeto com badge, branch, Build/Run/Debug/Stop só com ícone (nomes acessíveis via `aria-label`), run config, ⋮, Search e Settings. O painel inferior continua aberto por padrão e a geometria fixa de `ui-layout.spec.ts` (árvore 280, header 44, status 24, bottom 220) não mudou. Não imitamos o que não tem funcionalidade (rail direito, Monitoring, AI, Database). Comparações lado a lado em `docs/evidence/rider-compare/`.

## 2026-10-29 — Tela inicial no formato do Welcome do Rider

A tela sem workspace vira sidebar (logo, Projects, Toolchain) + painel com campo de caminho, botão Open e lista de recentes; Toolchain continua na mesma página rolável para manter os `data-testid` e specs. Só entra o que existe: sem New Solution, Clone, Attach to Process nem Remote Development; sem seletor nativo (exigiria plugin de dialog do Tauri, dependência nova). Os recentes (até 6) passam a persistir em `localStorage` (`openridr.recent`), o que substitui o "só nesta sessão" anterior; no app Tauri e no navegador de dev o storage é separado. Evidência: `ui-layout.spec.ts` e `workspace.spec.ts` (20 passam), screenshot do welcome.

## 2026-10-29 — Open abre o seletor nativo (macOS)

O botão Open da tela inicial chama o método do host `workspace.pick`, que roda `osascript` (`choose file`, tipos sln/slnx/csproj) e devolve `{ path }` ou `{ path: null }` ao cancelar; o caminho segue o fluxo normal de `workspace.open` (Restricted). Escolhido no lugar de `tauri-plugin-dialog` para não adicionar dependência; só macOS (outros SO devolvem `unsupported`) e não escolhe pasta. O campo de caminho continua, com o botão "Open path" (e2e e Enter). Não verificado: cancelamento e escolha reais no seletor.

## 2026-10-29 — Histórico de projetos no host (substitui o localStorage)

O host grava cada `workspace.open` bem-sucedido em `recent.json` no app home (respeita `OPENRIDR_HOME`), mais recente primeiro, sem duplicatas, até 30 caminhos; `recent.list` devolve só os que ainda existem e não exige trust. A UI busca a lista ao montar e a cada troca de workspace. Isso substitui a chave `openridr.recent` do `localStorage` (histórico anterior nela não é migrado) e passa a ser compartilhado entre o app Tauri e a ponte de dev. Projetos abertos antes desta mudança não estão no histórico.

## 2026-10-29 — Welcome só com Open no app

No app Tauri a tela inicial tem apenas o botão Open (seletor nativo) e os recentes; o campo de caminho e "Open path" só aparecem fora do Tauri (navegador de dev/E2E), porque os specs Playwright não conseguem dirigir o seletor nativo e dependem de `workspace-path-input`/`workspace-open`.

## 2026-10-29 — Open e recentes entram direto no workspace

Escolher no seletor ou clicar num recente abre o workspace e entra no IDE em Restricted mode, sem a etapa "Review workspace trust" da tela inicial (que ficava abaixo da lista e parecia que nada abriu). O trust continua no menu da toolbar. O formulário de caminho (navegador/E2E) mantém a revisão.

## Debug em um clique (2026-10-29)

> **Substituída em 2026-11-01**: o host agora constrói implicitamente e o clique em Debug só faz flag experimental e instalação/plano do netcoredbg antes de `debug.start`; restore continua explícito. O confirm por sessão continua.

- **Substitui** "Debug never builds or restores implicitly" (ticket 14) apenas para o clique na toolbar: o atalho/ação `debug.start` segue estrito e bloqueado pelos gates. O confirm da flag é por sessão (não persiste) e deve listar flag, restore e build.

- Clicar em Debug bloqueado executa `prepareAndDebug` (WorkspaceView): confirm da flag experimental Apple Silicon → `debug.flag` → plan/install (reusa o executável verificado) → restore → evaluation → build Debug → `debug.start`. Cada gate do host continua valendo; nada é feito sem o confirm. O botão usa `aria-disabled` (clicável) em vez de `disabled`.
- Solution Explorer/abas: badge `C#` (`FileIcon`), chevrons, contagem de projetos. Também: barra de ícones no cabeçalho (locate/expand/collapse), nó Dependencies e resumo de inspeções (✕/⚠ do arquivo ativo). Controles de avaliação continuam visíveis e compactos (testes clicam `eval-run`/"Project details"; o menu ⋮ foi adiado por isso). O run no gutter já existia. Faltam inlay hints, usages e Scratches; sem comparação lado a lado com o Rider.

## 2026-10-29 — Aproximação visual: largura inferior e processos

A área inferior aberta passa a ocupar a largura sob Solution e editor; a árvore mantém sua largura e o painel conserva os controles de tamanho. Isso substitui a geometria da decisão de aproximação visual anterior, preservando os slots, `role="tab"` e o layout salvo. Um layout sem preferência salva continua com o painel inferior fechado, conforme a implementação já existente nesta continuação; abrir ou fechar é persistido.

As referências e pacotes de projetos avaliados ficam dentro de um disclosure “Dependencies”, inicialmente recolhido e operável pelo teclado. A lista de processos é recolhível para não deslocar o editor permanentemente; decisões pendentes a abrem, continuam identificadas no resumo e permanecem acessíveis. O host e as garantias de trust, restore e término não mudam.

## 2026-10-29 — Fontes NuGet no SDK 8

`dotnet nuget list source` aceita `short` e `detailed`, não JSON. O host usa `--format short`, interpreta apenas linhas `E <localização>` e `D <localização>`, omite fontes desabilitadas e falha de forma explícita em saída desconhecida. O formato curto não inclui nome amigável: a localização é exibida como identificação, sem inventar um nome. A interface descreve essas fontes como configuradas e habilitadas, e informa que a origem específica de cada pacote não é reportada; não apresenta a lista como prova da fonte de um pacote. SDK 8.0.408 observado com saída em inglês e português; `global.json` permanece inalterado.

## 2026-10-29 — Run em um clique e consentimento do debugger dentro do app

> **Substituída em 2026-11-01**: não há mais `prepareAndRun`/`prepareBuild`; o host constrói implicitamente quando o build não serve e a UI chama `run.start`/`debug.start` direto. O diálogo de consentimento (`debug-consent-dialog`) continua.

- **Substitui** "Run never builds or restores implicitly" apenas para o clique na toolbar (como já era para Debug): com Run bloqueado e o workspace em Trusted, `prepareAndRun` restaura (se preciso), avalia, builda e roda. O atalho/ação `run.start` segue estrito. `prepareBuild` é compartilhado com `prepareAndDebug`, que agora também usa a run config ativa (config, TFM, args, env).
- O consentimento da flag experimental do netcoredbg deixou de usar `window.confirm`: o WKWebView do Tauri não o exibe e devolve `false`, então o clique em Debug no app desktop não fazia nada. Agora é o diálogo `debug-consent-dialog` ("Enable and debug"). Provado no navegador; o fluxo no app desktop precisa ser reconferido após rebuild do `.app`.

## 2026-10-30 — Breakpoints guardam condição; hit count e log message ficam fora

- O breakpoint por fonte passa a guardar `condition` além de linha, verified e message (`BreakpointView.condition`). `Launch.breakpoints` carrega as views, então a condição sobrevive ao restart. Mute all é um flag do host (`breakpointsMuted`): ligado, cada fonte é enviada com lista vazia; desligado, a lista volta. Mute também sobrevive ao próximo launch.
- Condição é gravada sem sessão e reenviada no `setBreakpoints` quando a sessão está viva. Se o adaptador anunciou `supportsConditionalBreakpoints: false`, o host recusa com `unsupported` e a UI desabilita o editor com o motivo. Antes de qualquer sessão a capability é desconhecida e o editor fica habilitado.
- **Hit count e log message não entram.** O netcoredbg 3.2.0-1092 não anuncia `supportsHitConditionalBreakpoints` nem `supportsLogPoints` (capabilities medidas no `initialize`, ticket 01). A UI mostra essa limitação em texto, em vez de um campo que falharia. Reabrir quando um adaptador que anuncie os dois for verificado.
- Remover pela lista reenvia a fonte sem a linha. Editar condição e mute all não mudam o formato de `debug.setBreakpoints`, que continua recebendo linhas.

## 2026-10-30 — Exceções: só não tratadas por padrão; Run to Cursor é breakpoint temporário

- Exception filter é `user-unhandled` por padrão. A caixa "Break on thrown exceptions" troca para `all` (filtro que o netcoredbg anuncia em `exceptionBreakpointFilters`). A escolha vive no estado do host como os breakpoints e sobrevive ao próximo launch. Se o adaptador anunciar filtros sem `all`, a caixa fica desabilitada com o motivo.
- Run to Cursor não cria breakpoint persistente: `debug.runToCursor` guarda uma linha temporária no estado, enviada com a fonte e retirada na próxima retomada (`continue`/step). A linha nunca entra em `Data.breakpoints`, então não aparece na lista.
- Não usa `goto`, porque netcoredbg 3.2.0-1092 não anuncia `supportsGotoTargetsRequest`.

## 2026-10-30 — Restore e avaliação rodam sozinhos depois do Trust (Fase 1)

- **Substitui** "nada roda sem pedido explícito" para restore e avaliação. Um workspace Trusted com SDK do projeto resolvido restaura uma vez por contexto ao abrir; ao terminar com sucesso, avalia de novo. Mudança de arquivo de projeto em disco (detectada pelo poll de estado) reavalia; não restaura. Em Restricted nada executa.
- Cancelamento e nova tentativa ficam na status bar (`status-restore-cancel`, `status-restore-retry`). Revogar o Trust interrompe restore e avaliação (host, já existente).
- Run e Debug por atalho/ação continuam estritos; o clique na toolbar continua preparando o que faltar. **Substituído em 2026-11-01**: atalho e toolbar usam o mesmo host, que constrói implicitamente quando não há build; o clique em Debug só prepara flag/instalação do adaptador.
- Falha de restore não repete sozinha: vira estado `failed` com "Retry restore".

## 2026-10-30 — Terminal: host expõe bytes crus por offset (`terminal.read`)

- `terminal.read {since}` devolve `{revision, chunk: {from, end, truncated, bytes}}` com os bytes do PTY exatamente como escritos (sem `strip_ansi`, sem corte de 4.000 linhas). Buffer de 1 MiB; o que cai antes do offset vem com `truncated: true`. Offsets valem por `revision`. Leitura é somente-leitura, como `terminal.state`.
- `bytes` vai como array JSON de inteiros (sem dependência nova). Upgrade para base64 se a vazão incomodar.

## 2026-10-30 — Terminal interativo com xterm.js (dependência aprovada)

- **Dependência nova aprovada pelo mantenedor:** `@xterm/xterm` e `@xterm/addon-fit` (emulador de terminal no navegador/WKWebView, sem canvas addon). Candidata registrada em `research/03`; nenhuma outra biblioteca de terminal entra.
- O terminal abre sozinho com prompt quando o workspace está Trusted e o shell ainda não existe (uma vez por contexto). Restricted não inicia shell.
- A tela renderiza os bytes crus de `terminal.read` (fatia 06). Digitação vai direto ao PTY (`terminal.write`, em fila, ordem preservada). Resize do painel vai para `terminal.resize`.
- **Testids:** `terminal-output` passa a ser o elemento do xterm (mesmo id, mesmo papel de saída). `terminal-input`/`terminal-send` ficam como **espelho de acessibilidade**: uma linha digitada e enviada com Enter, como antes. Nenhum testid foi renomeado ou removido.
- **Não verificado:** latência de eco abaixo de 100 ms (o teste E2E não mede tempo); comportamento de IME e clipboard no WKWebView do app empacotado.

## 2026-10-30 — App local instalável (uso pessoal)

- O bundle `.app` é gerado por `bun run app:install` (`scripts/install-app.sh`) e copiado para `/Applications/OpenRidr.app`. `bundle.active` passa a `true`, com target `app`.
- Assinatura ad-hoc, sem notarização. Ícone provisório: o PNG já existente no repositório. Procedimento e aviso do Gatekeeper em `docs/install-app.md`.
- Atualizar = rodar o comando de novo após subir a versão. Nada é instalado em `~/Library` ou fora de `/Applications`.

## 2026-10-31 — M7: previews de rename/code actions/Find Usages docados abaixo do editor

- **Substitui** o overlay flutuante ancorado no cursor (`.editor-popup`, `position:absolute`). Rename preview, lista/preview de code actions e Find Usages agora são um painel docado no fluxo flex do `.editor-pane`, abaixo do editor (inicialmente `order:2`, `min-height: 100px`, `max-height: min(40%, 300px)`; **superseded**: `order` saiu no ticket 34 e o teto passou a 50% no ticket 32; rolagem interna; Apply/Cancel sticky). O texto do editor encolhe, não é coberto.
- Removidos `editorPopupStyle()` e os `data-anchor-x/y` do `CodeEditor` (só serviam o overlay). `data-testid`, `role="dialog"`, foco no heading, Esc, guards de Apply e D12/D13 não mudaram.
- O teste D-12 passou a afirmar a geometria nova: popup não sobrepõe `.cm-scroller` e o editor mantém ≥100 px a 1100×700. Histórico de tetos medidos a 1100×700: 45% deixou o editor com 71 px, 32% passou; o valor final é 50% desde o ticket 32 (ver abaixo).
- **Superseded (ticket 34):** a regra de janelas baixas que ocultava o resumo `p.muted` e a linha de dicas do LSP foi removida: texto de segurança/truncamento e o `role="status"` não se escondem. Ver "ticket 34".
- **Escopo M7 dividido.** **M7a (overlap/docking): resolvido** por esta decisão. **M7b (rename inline sobre o identificador, com range destacado e Enter aplicando; UI-AUDIT linha M7, SPEC F7 #61): continua aberto/pendente** — não implementado: exigiria reposicionar o `inline-rename` do `CodeEditor` sobre a palavra com decoração de range, mudança de fluxo (Enter hoje abre prévia, não aplica) que reabre decisões F7/D12. M7 **não** está fechado.
- **Supersede** "popup no cursor"/"popups próximos à posição do editor" de `SPEC-MVP-RIDER.md` (⌥↩) e do ticket 22 para code actions, rename preview e Find Usages; o rename inline continua regido por F7/M7b.

## 2026-10-31 — M7a residual (ticket 32): o painel inferior cede ao preview docado

- Achado no `.app` real a 1100×700: com a tool window inferior (Problems) aberta, a 1ª ação/3ª linha do preview ficava cortada e o editor com ~4 linhas. O e2e do ticket 31 não pegava: o corte era por rolagem dentro do painel e o teste comparava só caixas.
- Decisão: enquanto existe um `.editor-popup`, a linha do grid do painel inferior encolhe para `min(--output-height, 96px)` (abas continuam visíveis e clicáveis; volta ao tamanho do usuário ao fechar o preview). O preview ganha `max-height: min(50%, 300px)`. Sem overlay, sem mudança de testid/foco/Esc/guards.
- Teste: a 1100×700 com Problems aberto, as 2 primeiras ações + Close (code actions) e a 3ª linha + Apply/Cancel (rename) ficam dentro do painel, o painel termina acima da dock e o editor fica ≥100 px. Red antes do CSS (preview terminava 17 px abaixo da dock), green depois.
- M7b continua aberto.

- **1440×900 também muda (ticket 32):** a regra `:has(.editor-popup)` vale para qualquer altura, não só 1100×700 — com qualquer preview (rename, code actions, Find Usages) aberto, a linha do painel inferior cai para ≤96 px e volta ao tamanho do usuário ao fechar.

## 2026-10-31 — Ticket 34: revisão final do M7a

- (A) O resumo/aviso dos popups **não é mais ocultado**; só espaçamento e tamanho cedem (`.editor-popup p` 0,72rem, margens 1 px). (B) A linha `role="status"` do editor fica visível durante o preview, em uma linha com reticências e texto completo no `title`; comentário corrigido. (C) Os popups docados vêm **depois** de `.editor-body` no DOM e perdem `order`: a ordem de Tab é a visual; nada depende de combinador `~`.
- Testes (vermelhos sem a correção, log em `docs/evidence/logs/m7-ticket34-red.md`): resumo "Nothing has been applied" e status visíveis, popup depois do editor no DOM, a 1100×700 com Problems aberto.

- **Follow-up do ticket 34:** (i) o bloco `conflict-banner` continua entre o breadcrumb e `.editor-body`; só mudou a posição *relativa aos popups*: antes os popups ficavam acima dele, agora todos os popups vêm depois do editor e o banner fica logo acima do editor (nenhuma mudança de testid/foco; o spec `workspace.spec.ts` de conflito de disco passa). (ii) Texto de `.editor-popup p` subiu para 0,75rem (12 px) com line-height 1,25 — cabe sem quebrar os asserts de 1100×700, então não há motivo para menos. (iii) `rustfmt`: o crate tem dívida de formatação anterior; só os hunks deste ticket foram formatados (linha dupla e asserts do teste).

## 2026-10-31 — UI shape (T3 Code): decisões abertas do `.scratch/ui-shape/spec.md`

Redesenho visual das 14 telas sobre os tokens de `DESIGN.md` (preto verdadeiro, superfícies por alfa, linhas de 1px, raios 6/8/12, controles de 32px, barra de 52px). Sem dependência nova, sem mudança de contrato do host, `data-testid` e `role="tab"` preservados. As cinco decisões em aberto foram resolvidas assim (não houve resposta do mantenedor; reabrir se discordar):

1. **Largura do slot esquerdo:** fica em **280px** (código e wireframe); o `320px` do `DESIGN.md` não foi aplicado para não deslocar os specs de layout.
2. **Menu bar `File/Edit/View…`:** continua **fora** (barra única, decisão F1).
3. **Cluster projeto/branch/Git:** **controles inline** na barra; o card flutuante do T3 não foi implementado.
4. **Raio de diálogo e Search Everywhere:** **12px**.
5. **Tema claro:** **não** implementado; tokens claros seguem só documentados.

Consequências para contratos de teste: barra superior 44→52px, status 24→28px, painel inferior padrão 220→280px (o e2e fixa 220 via `storageState`), "Modified" saiu do texto do breadcrumb para um selo (`editor-revision`), nomes de projeto/branch/Run configuration truncam no meio (`MidText`), `1 parent` não repete em cada linha do log, o prefixo de namespace igual ao projeto não vira um nível extra na árvore de testes, o preview do Search Everywhere fica ao lado da lista. Specs atualizados em conjunto.

Não verificado visualmente nesta rodada (ver `docs/implementation-notes.md`): rename inline, code actions e Find Usages com resultados e "Order em Classes e em Symbols" — os specs com csharp-ls real passam, mas não foram capturados porque instalar a ferramenta no home do app fora dos specs não foi autorizado —, estado de restore incompleto na árvore, `Changes +n −n` (o host não expõe estatística de linhas), colunas Requested/Resolved do NuGet (o host só expõe Current/Latest).

## 2026-11-01 — UI shape polish: ajustes sem redesenho

Passe de polish sobre as telas das issues 01–14, só CSS (mais 1 variável inline e 1 frase de texto). Sem dependência nova; `data-testid`, `role="tab"` e comportamento preservados.

- **Árvore do Solution Explorer:** a regra das linhas de 32px (`.workspace-view .side .tree-row`, especificidade 0,3,0) anulava o recuo `.tree .l1/.l2/.l3` e deixava arquivo e projeto no mesmo nível; o recuo agora é 10/28/46px com seletor mais específico. Linha de projeto virou **duas camadas** (nome em cima; `kind · TFM · restore` embaixo, alinhados ao nome) porque os 280px fixos não comportam nome + tags na mesma linha. Cabeçalho e faixa de avaliação ficam fixos; só a árvore rola.
- **Fonte Inter:** continua só na pilha de fontes (sem arquivo embutido). Embutir exigiria dependência/asset novo, fora do escopo do polish; segue pendente.
- **Piso de 12px:** todo `font-size` em rem abaixo de 0,75rem passou a 0,75rem; 11px ficam só em selo/badge (regra do `DESIGN.md`).
- **`Settings`:** altura fixa `min(720px, (100vh − 48px) / --ui-zoom)`; o `vh` não é escalado pelo `zoom` do workspace, então sem dividir por `--ui-zoom` o botão Save sai da janela a 120% (pego por `settings.spec.ts:181`).
- **Cor:** os ícones de step do Debug deixaram de ser azuis (azul só para ação, foco e seleção); run/debug verdes e stop vermelho ficam (estado/ação primária).

## 2026-11-01 — Trust decision persistida por projeto; welcome redesenhada

Substitui "Trust só em memória". Decisões do grill (welcome-trust), todas aceitas pelo Helder:

- **Uma pergunta por projeto.** A *Trust decision* (`trusted` ou `restricted`) é gravada por caminho canônico em `trusted.json` no app home, que passa de lista de caminhos para mapa `caminho → "trusted"|"restricted"`. Leitura tolerante: o formato antigo (lista) é lido como tudo `trusted` e reescrito como mapa só na próxima gravação; arquivo ilegível nunca é sobrescrito por uma leitura. `OPENRIDR_TRUST_STORE=off` continua desligando leitura e escrita. Quem nunca decidiu (inclusive projetos já no histórico) recebe a pergunta uma vez; a decisão antiga "restrito" nunca existiu em disco, então nada é presumido.
- **Onde se pergunta.** Um *Trust prompt* modal antes de entrar no IDE, estilo Rider: **Trust project** (foco inicial), **Open in Restricted mode**, **Cancel**. Esc = Cancel, Enter aciona o botão focado, Tab circula entre os três; Cancel volta à welcome sem abrir. Vale para o seletor, para os recentes e para o formulário de caminho (navegador/E2E), que deixa de ter o card "Review workspace trust". Projeto com decisão gravada abre direto, no modo decidido.
- **Onde a informação fica depois.** Fora da barra de cima: o `status-mode` da barra de status vira botão com popover (estado + Trust/Restrict) e a lista de recentes da welcome mostra um selo por projeto e um menu "⋯" para mudar a decisão ou remover da lista sem abrir o projeto (métodos de host por caminho: `trust.get`, `trust.set`). Mudar a decisão do projeto aberto segue as garantias de `trust.grant`/`trust.revoke` (para processos, sela registros, incrementa contexto).
- **Exceção ao contrato de `data-testid`** (AGENTS.md): `mode-badge`, `trust-banner`, `trust-prompt`, `trust-prompt-open` e `trust-prompt-dismiss` saem da toolbar a pedido explícito (junto com `open-text-only` e `workspace-path`, que saem do `SetupScreen` porque o formulário de caminho deixa de ter o outline da spec) e os specs que os usavam migram para o modal e para `status-mode`. `trust-checkbox`, `trust-grant`, `trust-revoke` e demais ids de `TrustControls` permanecem.
- **Recentes com data.** `recent.json` passa a `[{path, openedAt}]` (ms); lê o formato antigo (entradas sem data aparecem sem "último acesso"). "Remover da lista" apaga só a entrada do histórico, nunca a decisão de trust nem arquivos.
- **Welcome.** Só **Open** tem suporte real; New (`dotnet new`) e Clone (`git clone`) ficam fora do escopo, sem botões falsos. Projects é a vista principal (busca, estados vazio/erro, atalhos ↑/↓/Enter, ⌘O, `/`); Toolchain vira segunda vista na navegação lateral. Só tema escuro (decisão de tokens única mantida).

## 2026-11-01 — Cancel não deixa rastro

Cancel e Esc no *Trust prompt* não deixam rastro em Recentes nem em `trusted.json`. `workspace.open` só chama `recent::record` quando já existe uma Trust decision gravada para o caminho; sem decisão, o registro acontece quando a decisão é tomada para o workspace aberto (`trust.set`/`trust.grant`/`trust.revoke`). Cancel dentro do IDE (Open recent para um projeto indeciso) reabre o workspace anterior em vez de fechar o host; Cancel/Esc são ignorados enquanto a decisão está em voo. `trust.set` decide e aplica sob um único lock, aceita só caminho absoluto existente (`not_found` senão) e devolve erro (`trusted_unreadable`/`trusted_failed`) se a decisão não pôde ser gravada.

## 2026-11-01 — Release alfa 0.3.0-alpha.1: DMG ad-hoc, só arm64

`bundle.targets` passa a `["app", "dmg"]`, com `signingIdentity: "-"` (ad-hoc, sem Developer ID, sem notarização) e `bundle.icon` explícito (sem ele o `.app` saía sem ícone). Versão sincronizada em `package.json`, `tauri.conf.json`, `src-tauri/Cargo.toml` e nos dois crates; locks atualizados pelo cargo. O DMG é gerado em Apple Silicon e é arm64 apenas. O ícone é original (monograma OR nos tokens do app); fonte em `src-tauri/icons/source.svg`, derivados via `bun run tauri icon`. Build do DMG: `CI=true bun run tauri build --bundles dmg` — o `CI=true` pula o AppleScript do Finder (layout da janela), que estoura timeout sem permissão de automação.

## 2026-11-01 — Run/Debug constroem implicitamente quando não há build; Restore segue explícito

Run e Debug deixam de recusar `no_build`/`build_failed`/`build_stale`: quando não existe build atual e utilizável para o workspace/contexto/projeto/configuração/TFM, o host inicia o mesmo pipeline de `build.start`, mostra `run.state`/`dap.state = building` e só lança quando o build observado termina com exit 0. Build falho ou cancelado não lança: a aba Run/Debug fica `failed`/`cancelled` com o motivo e os diagnósticos continuam na aba Build (`build.problems` + log). Build atual executa direto, sem rebuild; `source_changed`, `target_missing` e `assembly_missing` contam como build não utilizável e são reconstruídos. Restore **não** muda: `restore_required` e a ação explícita continuam, e o build implícito nunca restaura. Um segundo `run.start`/`debug.start` enquanto o anterior espera o build devolve `invalid_state`; `run.cancel`, `debug.stop` e `build.cancel` cancelam o build pendente e o lançamento (sem inventar exit ou termination). Fronteiras (revogar/fechar/trocar) cancelam pendentes e selam os registros como antes.

Motivo: o Rider constrói ao executar; exigir um Build manual antes de Run/Debug era a diferença funcional restante apontada em `docs/RIDR-VS-RIDER.md` e transformava `Ctrl+R`/`Ctrl+D` numa mensagem de erro.

Consequência: a UI não chama mais `build.start` no caminho de Run (`prepareAndRun`/`prepareBuild` removidos) e o clique em Debug só faz a flag experimental e o plano/instalação do netcoredbg antes de `debug.start`. Os gates de UI continuam bloqueando trust/SDK/avaliação/configuração/startup, build manual em andamento e run/debug já ativo (incluindo `building`); os gates de build ausente/stale/falho/fonte salva saíram, porque o host resolve com o build implícito. As entradas de 2026-10-28 (tickets 12/14) e as de 2026-10-29/10-30 que dizem "Run/Debug nunca constroem" ficam **substituídas** por esta; restore implícito continua proibido em todas elas. Evidência: `docs/evidence/auto-build-run-debug.md`.

## 2026-11-01 — Alfa 3: Debug sem pergunta (ticket 35)

**Substitui** a decisão de 2026-10-31 que criou o diálogo `debug-consent-dialog` ("Enable and debug") e o gate de UI que exigia adaptador `ready` e flag experimental ligada antes de clicar Debug.

- Clicar Debug (atalho/ação `debug.start`, botão da toolbar e `debug-launch` do painel) executa o fluxo automático: ativa `debug.flag {enabled:true, confirm:true}` quando macOS arm64 e a flag está desligada, faz `debug.plan`/`debug.install` somente quando o adaptador não está `ready` (reusa o 3.2.0-1092 já verificado), espera o adaptador e chama `debug.start`. Sem modal e sem checkbox; o `confirm` do host continua exigido para ligar a flag e para instalar.
- O host continua a autoridade: build implícito quando não há build utilizável, restore explícito, gates de trust/SDK/avaliação/configuração/startup e build/sessão em andamento continuam valendo. A UI não builda nem restaura por conta própria.
- `executionGates.debugDisabledReason` deixa de bloquear por adaptador ausente e flag desligada, porque o fluxo resolve os dois. O painel Debug mostra `dap-experimental-note` (macOS arm64 experimental, suporte da comunidade, release aprovada 3.2.0-1092) enquanto a flag está ativa; é aviso não bloqueante e não promete suporte oficial.
- Saem do código `debug-consent-dialog`, `debug-consent-accept`, `debug-consent-cancel` e o estado `debugConsent`. `debugger.spec.ts` e `ui-fold.spec.ts` não clicam consentimento e afirmam o diálogo ausente.
- Evidência: `docs/evidence/alpha3-debug-trust.md`.

## 2026-11-01 — Alfa 3: confiar no próprio aviso de bloqueio (ticket 36)

- Quando Build, Run ou Debug está bloqueado por Restricted mode, o próprio parágrafo do motivo mostra o botão **Trust this project** (`build-trust`, `run-trust`, `dap-trust`). Ele chama `trust.grant {remember: true}` pelo `grantTrust()` único do `WorkspaceView`, então a escolha é persistida. Sem checkbox no aviso; o popover de `status-mode` e o menu "⋯" dos recentes mantêm o fluxo com checkbox.
- Falha do grant aparece como `trust-grant-error` (nota da workspace; distinto do `trust-error` do popover de `TrustControls`) e não finge sucesso; sucesso limpa a mensagem e atualiza o snapshot, então o motivo de Restricted some e o restore automático de workspace Trusted segue como já era.
- Nenhum indicador de confiança novo na barra superior: `status-mode` e o menu "⋯" dos recentes continuam sendo os únicos lugares.
- Evidência: `docs/evidence/alpha3-debug-trust.md` e `tests/e2e/gates.spec.ts`.

## 2026-11-01 — Alfa 3: dados de teste isolados e higiene do repositório (ticket 37)

> **Substituída em parte em 2026-11-07**: o caminho fixo `os.tmpdir()/openridr-e2e-home` virou `openridr-e2e-home-<porta da UI>` (ver a entrada de 2026-11-07, issue #210). O resto da entrada continua valendo.

- O host E2E sempre roda com `OPENRIDR_HOME` temporário (`e2eHome` = `OPENRIDR_E2E_HOME` ou `os.tmpdir()/openridr-e2e-home`) e `OPENRIDR_TRUST_STORE=off`. `environment.spec.ts` prova que `app.appHome` é exatamente `e2eHome`; nenhum teste escreve `trusted.json`/`recent.json` no app home real (mtime, tamanho e SHA-256 iguais antes/depois das execuções).
- `bun run host:http` passa a rodar com `OPENRIDR_HOME=${OPENRIDR_HOME:-$PWD/.scratch/host-home}`: o smoke manual não escreve no app home real. Para usar o home real de propósito, exporte `OPENRIDR_HOME` explicitamente.
- `.gitignore` cobre `.openchamber/`, `.playwright-mcp/`, `.scratch/design-ref/`, `.scratch/ui-shape/`, `.scratch/host-home/` (novo diretório do `host:http`) e `docs/evidence/logs/*`. Logs já versionados continuam versionados; a regra vale para arquivos novos.
- Evidência: `docs/evidence/alpha3-debug-trust.md`.

## 2026-11-02 — Solution Explorer fiel ao Rider (tickets 38–41)

- Fonte e aceite em `docs/SPEC-SOLUTION-EXPLORER.md`. A árvore passa a vir de `SolutionTree`/`TreeRow` (linha única); `kind`/TFM viram sufixo cinza `· console · net8.0` para manter os `toContainText` existentes.
- **Só entra o que tem backend.** Fora: Add, Unload, EF Core, Run Multiple, More Run/Debug, Publish, Advanced Build, Local History, Refactor, Inspect, Reformat, Diagrams, Tools, Cut/Copy/Paste/Delete/Rename, Open In, Replace in Files, Scratches and Consoles. `Run Unit Test` fica visível e desabilitado (como no Rider). Rename/Delete exigiriam métodos de escrita novos no host; ficam para uma decisão própria.
- **Reload Project = re-avaliação do modelo inteiro** (o host não avalia um projeto isolado); o rótulo segue o Rider, o efeito é o de `evaluation.run`.
- **Build/Run/Debug por projeto** usam o gate do próprio projeto e uma `RunConfig` explícita, nunca mudam o startup project; só projeto console avaliado executa. Restricted bloqueia com o motivo existente e o menu oferece "Trust this project" (mesmo `grantTrust`).
- **`workspace.tree {dir}`**: listagem de um nível, só leitura, sem trust, confinada ao workspace (como `workspace.files`). `no index` é heurístico: pasta ignorada do host, dotfile ou arquivo que não é fonte avaliada; `.gitignore` não é lido.
- `EditorContextMenu` ganhou um nível de submenu e o modo `compact` (motivo no tooltip, como o Rider).

## 2026-11-02 — QA top 4 (docs/SPEC-QA-TOP4.md)

- **Modified compara texto (achado 3).** Substitui a regra de 2025 "`revision` ≠ `savedRevision`": `Tab.savedContent` guarda o texto da última leitura/gravação e dirty é `content !== savedContent` (ou conflito). Undo até o texto salvo limpa a aba, o contador e os diálogos de Build/Run/fechar. `revision` continua sendo só o contador de versão usado para descartar respostas atrasadas do LSP. Aba aberta por edição aplicada (rename/code action) em arquivo não aberto usa `savedContent = null` (texto em disco desconhecido ⇒ dirty). O teste D-12 de `csharp-ls.spec.ts` deixou de afirmar "Modified" depois do undo.
- **Avaliação falha continua navegável (achado 4).** Projeto `failed` sem `sources` mostra na árvore os `.cs` do outline textual + o próprio `.csproj`; o painel Problems ganha a seção "Evaluation · MSBuild" (erro do projeto, código MSB/NU/CS extraído do texto, clique abre o `.csproj`) e a aba conta esses erros. Não muda o gate do Build (continua exigindo modelo avaliado).
- **Diagnósticos no editor (achado 2).** `collectDiagnostics` junta publicações LSP em estado `current` e problemas do Build que não estejam stale/sealed/obsoletos (uma entrada por caminho+linha+código). O CodeMirror pinta `Decoration.mark` (`.cm-diagnostic-<severidade>`, tooltip nativo com a mensagem) e uma `gutter` (`.cm-diagnostic-marker`) sem dependência nova (`@codemirror/lint` não está instalado). Edição local deixa o item LSP `awaiting` e some a marca até a próxima publicação, coerente com "não marcar como corrigido por antecipação". A barra de status mostra `status-diagnostics` ("N errors · M warnings") e abre Problems.
- **Análise inicia sozinha, instala só com plano confirmado (achado 1).** Com Trusted + csharp-ls `ready` + SDK do projeto resolvido + restore atual (`lspStartBlocker`, a mesma regra do painel), a UI chama `lsp.start`. Isso acontece uma vez por fronteira (`workspace.id:contextId:restore.revision`): o host para a análise sozinho quando um restore começa ou um input do projeto muda, e então a UI inicia de novo; parada pelo usuário ("Stopped by the user.") e falha nunca são desfeitas sozinhas. A barra de status mostra `status-analysis` e o botão `status-analysis-action`: "Install analysis" (só abre o painel Workspace tools; o plano e o "Confirm and install" continuam sendo a única via de instalação), ou "Start analysis" (desabilitado com o motivo no tooltip enquanto restore/SDK não estiverem prontos). O host não mudou.
- **Limite conhecido do achado 1.** "Instalada" = `ready` no host em execução. Um launcher que já está no disco de uma sessão anterior continua `absent` até o plano ser confirmado (o host só executa o launcher depois da confirmação, `tools::begin_install`), então no primeiro uso após abrir o app a barra mostra "Install analysis" e o plano reutiliza o launcher sem baixar. Verificar o launcher presente sem confirmação mudaria o modelo de confiança do host; fica como decisão do mantenedor. Uma reavaliação do modelo (TFM, configuração) depois do início marca a cobertura como parcial (comportamento do host); "Restart analysis" no painel renova a sessão.

## 2026-11-02 — QA devflow: atalhos globais, startup persistido e Build Solution

- **Atalhos globais em captura.** O handler de `keydown` do app roda antes do CodeMirror; quando o registry reconhece ação ou chord, o evento é consumido e o `defaultKeymap` não o vê. Consequência deliberada: um override do usuário que colida com o editor vence o CodeMirror (ex.: `build.start` remapeado para `⌘/`). `⌘G` fica no CodeMirror porque a ação Go to Line ainda é stub (achado 16); `search.line` tem `editor: false` até a implementação real.
- **Startup project persistido por solução.** O store de run configurations ganha `startupProject` (opcional) e `runConfigs.setStartup`; `runConfigs.write` preserva o campo quando não o recebe. Com 2+ consoles a UI não bloqueia a toolbar: usa a escolha salva ou o primeiro candidato console. A escolha é uma preferência de execução, não uma alteração em arquivo de projeto; por isso vive no app home, como as run configurations.
- **Build Solution separado do build do startup.** `build.solution` roda `dotnet build <sln> -c <config> --no-restore` e publica problemas de todos os projetos; `BuildView.solution` distingue esse build. Run/Debug aceitam um build de solução quando configuração, TFM, avaliação e fontes salvas continuam atuais, sem rebuild implícito; um build por projeto (`build.start`, ⌘F9) mantém a identidade do projeto e a checagem de TargetPath. Build Solution não substitui a ação de build do startup na toolbar.
- **Limite conhecido:** `build.solution` envia `-p:TargetFramework` da avaliação; em solução com projetos multi-TFM que não compartilhem o TFM selecionado o build pode falhar em vez de escolher um default. A mensagem de erro do MSBuild é preservada; uma decisão futura pode omitir o TFM para o escopo de solução.

## 2026-11-02 — Abas, Recent Files e perfil Rider (ticket 03)

- **Format/Organize Usings ficam de fora.** O host não expõe `textDocument/formatting` nem `source.organizeImports`; a UI não anuncia nem simula essas ações.
- **Recent Files é lista da sessão.** Até 20 caminhos, mais recente primeiro, sem persistência entre sessões e sem escrever no app home. A lista é atualizada ao abrir um arquivo e ao ativar uma aba.
- **Find in Files continua sem regex/máscara.** `search.text` é a única fonte e aceita consulta e `caseSensitive`; regex e máscara de arquivo exigiriam mudança no host.
- **Perfil Rider edita com ⌘D/⌘⌫.** `Prec.high` sobrepõe `copyLineDown`/`deleteLine` só em `rider-macos`; o perfil IntelliJ mantém o keymap padrão do CodeMirror.

## 2026-11-02 — Debug de teste continua fora: falta attach ao testhost

- **Decisão.** Não habilitar "Debug selected" nesta rodada; a ação continua visível e desabilitada. Estende a entrada de 2026-10-28 ("debug de teste desabilitado com motivo") e não deve ser reaberta sem uma decisão nova sobre attach/testhost.
- **Motivo.** Testes rodam num único `dotnet test` (`test_run.rs`), e o VSTest cria o `testhost` como processo filho próprio; o host não expõe nem endereça esse pid. A camada DAP só envia `launch` do assembly console avaliado (`dap.rs`), e o projeto de teste é uma biblioteca, sem entrada para lançar. Não existe, hoje, nem `attach`, nem `processId`, nem filtro de teste no contrato do debug.
- **Alternativas descartadas.** (1) Lançar `vstest.console` sob o netcoredbg: o adaptador depuraria o runner, não o teste. (2) `attach` no `testhost`: o host não tem o pid do testhost e o VSTest não o publica.
- **Consequência.** O texto único (`testDebugUnavailable` em `src/TestsPanel.tsx`) passa a dizer que falta um attach de test-host: "Debugging tests needs a test-host attach the debugger does not support yet. Use Run selected.", no painel, na quick list, no menu do editor e na ação. Usar Run selected continua sendo o caminho suportado. Uma decisão futura precisa definir a integração de attach/host (descobrir o testhost ou lançar o teste sob o depurador), com mudanças no host e nos gates de debug.

## 2026-11-02 — Sessão do editor por solução (d9) e isolamento do E2E

- O store por solução (`run-configs/<hash>.json`) ganha `session`: abas (caminho relativo + cursor), `activePath` e breakpoints por arquivo (linhas 1-based). Conteúdo de buffer nunca é gravado. `runConfigs.write`/`setStartup` preservam a sessão; `session.read`/`session.write` gravam só ela sob o mesmo lock.
- Ao abrir um workspace, a UI reabre as abas salvas uma vez, restaura cursor/scroll da aba ativa e reaplica breakpoints somente com Trusted. Arquivo removido do disco não impede as outras abas.
- `OPENRIDR_SESSION_STORE=off` faz `session.read` devolver vazio e `session.write` não gravar. O host compartilhado do Playwright usa off para um spec não restaurar as abas do anterior; o app e `tests/e2e/medium-editor-session.spec.ts` (host isolado) mantêm ligado. Mesmo precedente de `OPENRIDR_TRUST_STORE=off`.

## 2026-11-02 — Abrir do Finder e do terminal ("Abrir Com")

- **Associações no bundle, não em runtime.** `bundle.fileAssociations` declara `.sln`, `.slnx` e `.csproj` com `exportedType` (UTIs `com.openridr.*`), `role: Editor` e `rank: Default`; o mesmo bloco declara `public.folder` com `rank: Alternate` para o ícone do Dock aceitar pasta sem virar o padrão de abertura de pastas. Quem lê isso é o LaunchServices a partir do Info.plist; o app não registra nada ao rodar.
- **Um único caminho de abertura.** O shell Tauri só entrega o caminho (argv quando o binário é aberto direto; `RunEvent::Opened` quando o LaunchServices reusa a instância aberta) à UI, que chama o mesmo `workspace.open` da tela inicial. Trust, prompt, `recent.json` e recusa de caminho continuam no host; não nasce um segundo fluxo de confiança.
- **Caminho relativo resolvido no host.** `workspace.open` aceita caminho relativo, juntando um `baseDir` opcional (absoluto) ou o cwd do host antes de canonicalizar. Extensões aceitas: `.sln`, `.slnx`, `.csproj` sem diferenciar maiúsculas; diretório só abre com exatamente uma solution ou um projeto. Abrir continua sem executar nada: lê texto para o outline e para as run configurations.
- **`.csproj` é projeto.** Abrir um `.csproj` do Finder produz o mesmo workspace `kind: project` que escolher o arquivo na tela inicial; a solution que o contém não é procurada.
- **`.slnx` abre e navega.** O outline lê `<Project Path="..."/>` do XML, além do `.sln` textual. O SDK 8.0.408 da fixture não restaura nem compila `.slnx`; restore/build falham com o erro do dotnet, sem sucesso inventado. O recorte é abrir/navegar, não executar `.slnx`.
- **Sem segunda janela e sem perder buffer.** O macOS reusa a instância do bundle; o caminho fica pendente no processo Tauri (o último vence) até a UI consumir. Com um workspace aberto, a abertura externa passa pelo mesmo guard de buffers sujos do "Open Recent": se há alteração não salva, ela é recusada com a mensagem existente em vez de descartar edições.
- **`openridr <caminho>` é script fino sobre `open -a`.** Valida que o caminho existe e delega ao LaunchServices, que reusa a instância aberta. Nada é copiado para o PATH automaticamente; o link em `~/.local/bin` ou `/usr/local/bin` é ação manual documentada. `OPENRIDR_APP` aponta para um bundle de desenvolvimento.
- **Testes.** Unidade no host para resolução/allowlist e no shell para argv/URL; E2E contra a ponte HTTP para caminho relativo, `.slnx`, `.csproj`, diretório ambíguo e extensão recusada. O evento nativo do Finder e o arrasto para o Dock só são provados no bundle real; ficam como verificação manual registrada, com `OPENRIDR_HOME` isolado.
- **Follow-ups da revisão.** Com workspace montado, a escolha ambígua (diretório com mais de uma solution ou projeto) aparece em botões na área de mensagens, e clicar abre o escolhido pelo `openSolution` existente. Um prompt de Trust pendente não é substituído por outra abertura externa: a nova recebe `external-open-error` ("Finish the Trust decision for the pending project before opening another one.") e o workspace permanece.

## 2026-11-03 — Licença MIT e higiene do repositório público

- O repositório passa a ser público sob **MIT** (`LICENSE`, "Copyright (c) 2026 Helder Santos"); `license = "MIT"` em `package.json` e nos `Cargo.toml` de `openridr-host`, `openridr-host-http` e `openridr` (antes declaravam Apache-2.0). Supera o G-LEGAL de `release-decision.md` apenas quanto ao arquivo `LICENSE`; avisos de terceiros para o DMG continuam pendentes (`THIRD-PARTY-NOTICES.md` ainda não existe).
- Removidos do versionamento (continuam no histórico git, ver relatório do PR): capturas do JetBrains Rider (software proprietário), `.scratch/` inteiro (agora no `.gitignore`), `docs/evidence/logs/` e PNGs de evidência com caminhos de home (`/Users/<nome>`, `/private/var/folders/...`) ou nomes de worktree. Links para esses arquivos viraram "evidência removida do repositório público".
- Spec e tickets de Welcome + Trust saíram de `.scratch/welcome-trust/` para `docs/SPEC-WELCOME-TRUST.md` e `docs/tickets-welcome-trust/`.
- Caminhos `/Users/<nome>` e `/private/var/folders/...` em textos rastreados foram trocados por `~` e `<tmp>`. Reescrever o histórico git não foi feito (decisão do mantenedor).

## 2026-11-03 — Release 0.3.0-alpha.5: Developer ID, notarização e hardened runtime

- **A identidade vem do ambiente.** `bundle.macOS.signingIdentity` continua `"-"` (ad-hoc) para o build local; o build de release exporta `APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)"`, que o CLI do Tauri aplica por cima do config. O nome da identidade não é segredo, mas também não precisa morar no repositório. `hardenedRuntime` passa a estar explícito (`true`, já era o padrão).
- **Sem entitlements.** O app não tem JIT próprio, não carrega dylib de terceiro e só dá `spawn` em `dotnet`/`csharp-ls`/`netcoredbg`; o WebContent do WKWebView é assinado pela Apple. Um entitlements plist só entraria se a notarização ou o runtime exigissem, com o mínimo necessário.
- **Notarização explícita, não a do Tauri.** Nada de `APPLE_ID`/`APPLE_PASSWORD` no ambiente (o Tauri notarizaria por conta própria e o build passaria a depender de segredo). O fluxo é `xcrun notarytool submit --keychain-profile notary --wait` no `.app` zipado, `stapler staple` no `.app`, DMG refeito com `hdiutil` (o bundler de DMG refaz o `.app` e descartaria o ticket), DMG assinado, notarizado e grampeado.
- **Consequência para o usuário.** A primeira abertura do DMG não exige mais `xattr` nem botão direito. O `bun run app:install` local continua ad-hoc; a diferença está documentada em [release-signing.md](release-signing.md) e [install-app.md](install-app.md).

## 2026-11-04 — Quit pergunta antes de fechar

- **Menu Quit próprio + `ExitRequested`.** O quit predefinido do Tauri chama `terminate:` no NSApplication e nunca chega à webview que guarda os buffers. O menu padrão é mantido e só o último item do submenu do app (o Quit predefinido) vira um `MenuItem` com id `quit` e Cmd+Q que emite `openridr-quit`; `RunEvent::ExitRequested`, enquanto a janela `main` existe, chama `prevent_exit` e repete o pedido. O diálogo de saída existente ganha o caso `quit` com os mesmos três botões (Save all / Keep editing / Discard and close app); só depois da resposta a UI chama `quit_app`, que marca `QUITTING` e encerra. Sem janela, o `ExitRequested` da última janela destruída segue direto, como hoje, e o botão Close continua no `onCloseRequested` + `destroy()` sem diálogo duplicado.
- **AppleScript e Dock ficam de fora.** `osascript -e 'quit app "OpenRidr"'`, o Quit do Dock, logout e desligamento também chegam por `terminate:` nativo, fora do menu e do `ExitRequested`. Interceptá-los exigiria implementar `applicationShouldTerminate:` no delegate do NSApp via objc/unsafe; fica registrado como limitação conhecida, não como sucesso. A prova no app real cobre Cmd+Q e o item Quit do menu.
- **Sem dependência nova.** `core:default` já cobre os comandos e o evento; nenhum plugin ou crate foi adicionado.

## 2026-11-04 — Gate de fonte ausente

- **Correção.** `Build::changed_sources` compara o hash capturado no início do build com o hash atual e não trata mais `None` como "alterado" por si só. Um `SourceHash.hash` ausente significa "arquivo não legível no início"; se o arquivo continua ausente, não há mudança. Um arquivo que existia e foi apagado depois continua contando como mudança (`Some` vs `None`) e dispara o rebuild implícito; esse rebuild captura `None` e a sessão passa a iniciar. Arquivo criado depois do build também conta (`None` vs `Some`).
- **Por quê.** A avaliação não envelhece ao apagar `.cs` (`check_external_changes` só olha projeto/imports), então o caminho apagado continuava na lista de fontes. Com a regra antiga, todo build capturava `None` para ele e o gate nunca liberava — só um Re-evaluate curava. A regra nova mantém a proteção real (edição/apagamento depois do build) sem exigir reavaliação para um arquivo que já não existe.
- **Alcance.** A mesma função serve Run e Debug; os dois passam a aceitar o build feito depois da remoção. Os comentários de `capture_sources` e `changed_sources` foram corrigidos.

## 2026-11-04 — Encoding e fim de linha preservados

- **Um só par decode/encode.** `crates/host/src/fsutil.rs` ganha `decode_text` e `TextFormat::encode`: detectam BOM (UTF-8 `EF BB BF`, UTF-16LE `FF FE`, UTF-16BE `FE FF`), decodificam com `String::from_utf8`/`String::from_utf16` estritos e guardam encoding + estilo de fim de linha. `file.read` e `file.write` usam o mesmo par para conteúdo e hash; não existe segunda cópia da regra.
- **O editor trabalha em LF.** A leitura normaliza `\r\n` e `\r` para `\n`; a escrita reaplica o estilo detectado no arquivo atual. Arquivo misto vira CRLF (a primeira ocorrência de `\r\n` define o estilo), como no `SPEC-FIX-ALPHA5`.
- **Hash de conflito sobre texto normalizado.** `expected_hash` cobre o texto LF sem BOM; por isso é o mesmo entre leitura, reload e gravação, e um arquivo que só mudou de fim de linha não gera conflito falso. O `currentContent` do conflito também é o texto LF, então diff e buffer concordam.
- **Encoding desconhecido continua erro.** Bytes que não são UTF-8/UTF-16 válidos devolvem `operation_failed` citando o encoding; a escrita revalida o arquivo atual antes de gravar e nunca substitui o original. UTF-16 sem BOM segue não suportado (seria adivinhação).
- **Fora de escopo.** `settings`, `global.json`, `.sln` e `.csproj` lidos pela avaliação continuam `read_to_string` UTF-8: são arquivos do app/build, não buffers do editor.

## 2026-11-04 — Árvore fresca após mudanças externas

- **Diretórios de fontes no watch.** `watch_paths` passa a incluir os diretórios dos `Compile` items avaliados e os diretórios de projeto, mais subdiretórios (até 256, sem `bin`/`obj`/dotfiles). O `check_external_changes` do poll de 400 ms compara o fingerprint deles; criar, renomear ou apagar um `.cs` marca a avaliação stale. Editar o conteúdo de um arquivo existente não muda a lista de entradas e continua não invalidando o modelo — o contrato de `is_evaluation_input` não mudou.
- **Fingerprint de diretório é a lista de entradas, não o mtime.** O save do host é atômico (arquivo temporário + rename), então o mtime do diretório mudaria a cada save de fonte. O fingerprint de diretório é a lista ordenada de `.cs` e subdiretórios; o de arquivo continua mtime + tamanho.
- **Fingerprint antes do MSBuild.** A lista observada é capturada no início da avaliação e mesclada (`merge_watched`) com os caminhos que só o novo modelo descobriu. Um arquivo criado enquanto o MSBuild roda continua vendo a diferença no próximo poll, em vez de ser mascarado por um `stat` tardio.
- **Reavaliação automática com debounce.** A UI reusa o efeito que já reavaliava arquivos de projeto: 500 ms, uma execução por rajada, só com a janela visível, sem buffer sujo e sem build/restore/evaluation em andamento. Falha continua no caminho existente (mensagem + banner de stale com `eval-stale-run`), sem inventar árvore.
- **Expansão preservada.** O efeito que semeava `expandableKeys` a cada revisão passa a semear só a primeira; reavaliações mantêm o que o usuário abriu ou fechou. `selectedProjectPath` já era mantido quando o caminho ainda existe, e o arquivo ativo continua nos tabs.
- **Limite.** O cap de 256 diretórios por workspace pode deixar de fora uma árvore maior (`shortcut:` no código); nesse caso o banner de stale continua sendo o caminho manual.

## 2026-11-04 — Buffer limpo recarrega do disco

- **Poll da aba ativa, sem watcher.** O `checkDisk` já trocava buffer limpo pelo texto do disco e abria conflito para buffer sujo, mas só rodava em foco, troca de aba e reabertura. Um intervalo de 1,5 s em `WorkspaceView` chama o mesmo `checkDisk` para `activePathRef`, sem tocar em `workspace.state`. A janela oculta não relê (`document.hidden`); foco e troca de aba continuam iguais.
- **Uma leitura por caminho.** Um `Set` de leituras em andamento impede que o poll empilhe `file.read`; o `diskEpoch` existente descarta resposta atrasada de uma escrita/save mais nova.
- **Cursor preservado.** O `CodeEditor` já reaplica linha/coluna e não cria passo de undo quando o `value` muda por recarga limpa; nenhum estado novo de cursor foi criado para o poll.
- **Custo.** Relê o arquivo ativo inteiro a cada 1,5 s; se aparecer em perfil, o próximo passo é um `file.hash` no host (`shortcut:` no código). Sem dependência nova.

## 2026-11-04 — Debug baixa sozinho com transparência

- **One-click continua.** Clicar Debug segue ativando a flag experimental da sessão e instalando/reusando o netcoredbg 3.2.0-1092 sem diálogo de confirmação (a decisão de 2026-11-01 permanece). O que muda é a evidência mostrada depois do clique.
- **Progresso observado, nunca porcentagem.** A UI reusa `debug.progress` (bytes medidos; total do `Content-Length` ou do metadata) no componente pequeno `ToolDownloadStatus`, renderizado na aba Debug enquanto baixa/extrai/verifica. É esse componente que o ticket 14 pode reutilizar para o csharp-ls; nada foi abstraído além disso.
- **Falha acionável.** `prepareAndDebug` passa a lançar `debug.error`/`debug.note` em vez da mensagem do gate, então o toast diz o que falhou. A mesma caixa mostra o erro e um `Try again` (`debug-install-retry`) que chama `debug.retry` direto, porque a falha é do download; depois do retry o botão Debug inicia a sessão. O host continua a autoridade: `debug.retry` exige trust e o plano confirmado inalterado; um plano mudado pede nova confirmação.
- **SHA-256 antes de executar.** O pipeline existente (hash do arquivo baixado contra o SHA-256 da release, extração segura, `--version`, só então promoção) não mudou. Os testes de divergência provam que a divergência aborta sem promoção, e o teste novo de falha de rede prova o retry sem baixar de novo.
- **Instalado é instalado, mesmo em host novo.** `Debugger::view` inspeciona o destino uma vez por processo (mede o SHA-256, não executa) e expõe `installed`/`installedBinary`/`installedSha256`. Settings/Workspace tools mostra origem, versão aprovada (ou verificada) e o SHA-256 do executável instalado. O plano deixa de dizer "stays blocked on this platform": com o binário presente e a flag desligada, o motivo nomeia a flag de sessão e diz que não há download a fazer.
- **Limite.** Um host recém-iniciado não roda `--version` só para pintar a tela; até a sessão iniciar, a versão é a aprovada 3.2.0-1092. O checksum exibido é o do executável em disco; o SHA-256 da release é o do arquivo baixado, checado durante a instalação.

## 2026-11-04 — Mensagens de abertura e avaliação

- **BUG-007 não reproduzia.** As capturas do relatório abriram a alpha.4 instalada porque `scripts/openridr` sem `OPENRIDR_APP` chama `open -a OpenRidr`; no HEAD o argv já chega à UI e mostra a mensagem. O conserto é no launcher e na prova, não em código de app novo.
- **Launcher valida antes de abrir.** `scripts/openridr` aceita pasta ou extensão `sln`/`slnx`/`csproj` (case-insensitive) e recusa o resto com a mesma orientação da tela de pasta vazia, sem chamar `open`; a linha `openridr: opening <bundle>` nomeia o bundle efetivo (`OPENRIDR_APP` ou `OpenRidr`).
- **Zero projetos culpa o arquivo.** Quando `workspace.kind == "solution"` e o outline tem zero projetos, a nota da árvore diz que o arquivo não lista projetos e não é uma solução válida; não fala em SDK.
- **Fallback usa a mensagem do host.** Para os demais estados de `projectSdk` (sem `global.json`, sem `version`, JSON inválido), a nota renderiza `projectSdk.message` e acrescenta a ação de corrigir/adicionar `global.json`, em vez da frase fixa sobre o SDK 10.
- **"Avaliação anterior" exige revision > 1.** `hasEarlierEvaluation` é a única condição; com revision 1 o estado diz "not current" e o banner diz "Nothing is current yet", inclusive em `eval-target-path`. O `reloadReason` dos menus nunca afirmou avaliação anterior e continua igual.

## 2026-11-04 — Nomes acessíveis desambiguados

- **Rótulo visível não é nome acessível.** As três "Run" e as duas "Save" continuam iguais na tela; `aria-label`/`aria-label` em `th` carregam o contexto. Nada de `data-testid` mudou.
- **Toolband e toolbar.** O botão do toolband vira "Run tool window" (sufixo em todos os tool windows, não só Run); a toolbar vira "Run startup project". O keymap usa "`título — grupo`" e cai no id quando o grupo é igual ao título (`run.start`), porque o grupo visível não muda.
- **Settings e arquivo.** `settings-save` = "Save settings"; `save-file` = "Save file". O texto "Saving…" continua visível durante a gravação.
- **Menu da árvore.** `EditorContextMenu` ganhou `label` opcional; só o alvo `project` passa "Project context menu". Root continua "Solution context menu" e editor continua "Editor context menu" — file/dir não foram renomeados nesta rodada.

## 2026-11-04 — Operações de arquivo na árvore

- **Três métodos e um resolvedor.** `file.create` (`path`, `content` opcional), `file.rename` (`from`/`to`) e `file.delete` (`path`) usam `resolve_path_in_workspace`; só o pai é canonicalizado, então o último componente pode ser um symlink e a operação renomeia/apaga o link, nunca o alvo. `must_exist` separa a origem existente do destino novo. `file.read`/`file.write` continuam no resolvedor antigo, que exige arquivo existente e segue symlinks dentro do workspace.
- **Confinamento explícito.** Qualquer componente `..` é recusado antes de canonicalizar; pai ausente ou que não é pasta devolve `not_found`; pai canônico fora do workspace (inclusive via symlink) devolve `outside_workspace`; a raiz do workspace não pode ser renomeada/apagada. Criar ou renomear sobre um caminho existente devolve `already_exists` e nunca sobrescreve; mover uma pasta para dentro dela mesma devolve `path_not_allowed`.
- **Apagar é permanente, com confirmação.** Não há Trash nem dependência nova: `file.delete` usa `remove_file`/`remove_dir_all` (pasta com conteúdo). A UI mostra o caminho, avisa que não pode ser desfeito e só chama o host depois do clique em Delete; Cancel fecha sem efeito.
- **Stale sem avaliar nem buildar.** `mark_inputs_stale` reusa `is_evaluation_input` (a mesma lista do save) e, para pasta renomeada/apagada, marca como entrada de avaliação mesmo sem extensão conhecida. O motivo contém "changed on disk" para a reavaliação automática do ticket 05 rodar; a operação em si nunca executa MSBuild, restore ou build. Criar/renomear/apagar `.cs` não marca stale no host: o watch de diretórios do ticket 05 faz isso no poll seguinte.
- **UI sem estado novo de buffer.** Itens `New File…`/`Rename…`/`Delete` no menu de projeto e de arquivo/pasta; um diálogo de nome (criar/renomear) e um de confirmação de delete, ambos com erro do host. Nome digitado não pode conter `/` nem `\`; o projeto usa a pasta do `.csproj` para criar e o próprio `.csproj` para renomear/apagar. Aba aberta do caminho antigo é preservada; salvar depois usa o erro existente de arquivo ausente/conflito.

## 2026-11-04 — Instância única

- **Dependência aprovada.** `tauri-plugin-single-instance` 2.5.5 entra em `src-tauri` e é registrado como o primeiro plugin do builder; é o único caminho novo, sem socket próprio.
- **Segunda execução encaminha, não abre janela.** O plugin encerra o segundo processo e entrega `argv`/`cwd` ao primeiro; o callback foca a janela `main` e chama o mesmo `offer_open` do argv de inicialização, então o `.sln` passa pelo fluxo normal de `workspace.open` e pelo guard de buffer sujo da UI.
- **Caminho relativo é resolvido na origem.** Um argv relativo é unido ao `cwd` da segunda execução antes de ser encaminhado, porque o primeiro processo tem outro `cwd`; o caminho de inicialização usa o mesmo helper.
- **Escopo do lock é o identificador.** No macOS o plugin usa `/tmp/<identifier>_si.sock`, então a instância única vale para o identificador do bundle, não por `OPENRIDR_HOME`; duas cópias com homes diferentes não rodam ao mesmo tempo. `open -a`/Finder continuam no caminho LaunchServices de hoje; quando já existe instância, o evento chega por `RunEvent::Opened`.

## 2026-11-04 — Runtime das ferramentas x SDK do projeto

- **A análise exige runtime, não SDK.** `sdk::AnalysisRuntime` (antes `HostSdk`) é escolhido de `dotnet --list-runtimes`: `Microsoft.NETCore.App` 10.x estável, maior versão entre as raízes. O `sdk.host.version` do snapshot passa a ser essa versão de runtime; `sdk.host.sdkPath` fica nulo porque raiz de runtime não tem `sdk/`. A cópia/links do card são do ticket 13.
- **Descoberta por raízes reais.** `discover_roots` cobre `DOTNET_ROOT`, `/usr/local/share/dotnet`, `/opt/homebrew/opt/dotnet` e seu `libexec`, `~/.dotnet`, cada SDK gerenciado no app home e **todas** as entradas do PATH. Cada candidato é resolvido pelo muxer (`canonicalize` do arquivo) e deduplicado pela raiz canônica; a primeira fonte vence (PATH repetido ou symlink não viram raízes extras).
- **Root composto separa as origens.** Muxer, `host/fxr/10.x` e `shared/Microsoft.NETCore.App/10.x` vêm da raiz do runtime 10; `sdk/<versão do projeto>`, `packs`, `metadata`, `sdk-manifests` e `templates` continuam vindo da raiz do SDK do projeto. `check_versions` passa a recusar qualquer SDK listado além do fixado, em vez de comparar com um "SDK host".
- **Instalar ≠ executar.** O plano do csharp-ls roda `dotnet tool install` com o maior SDK detectado (qualquer versão, inclusive o 8 do projeto); `dotnet_root` do plano e a verificação `--version` apontam para a raiz do runtime 10. Sem SDK nenhum o plano falha separado de "falta runtime"; sem runtime 10 o plano nem é produzido.
- **SDK do projeto intacto.** `global.json`/TargetFramework, `rollForward`, versão resolvida e `sdkRoot` absoluto não mudaram; os gates de evaluation/restore/build/run/test continuam exigindo `project_sdk.state == "resolved"`.
- **Limite.** Windows e macOS Intel seguem não executados; a prova de uma raiz só-runtime usa raízes falsas nos testes (`describe`/descoberta), enquanto a análise real desta máquina usa a raiz de runtime detectada.

## 2026-11-04 — Pedido de pré-requisito por projeto

- **Runtime ≠ SDK na tela.** Card, banner de boas-vindas, Settings e status bar chamam `sdk.host` de "Analysis runtime"/".NET 10 runtime"; o link e a mensagem vêm do host (`/download/dotnet/10.0`). Nenhum texto manda instalar SDK 10 para analisar.
- **SDK do projeto cita a origem.** `sdk.project.globalJsonPath` + `requestedVersion` viram `global.json requests <versão>`; sem `global.json`/versão, a UI diz que o `TargetFramework` declarado decide e não resolve um SDK por conta própria. O link é o `instructionsUrl` do host (página de SDKs).
- **A UI não recalcula regra.** Estados, versões, mensagens e URLs vêm do snapshot (`sdk.host`, `sdk.project`); a única composição local é juntar a origem à versão para exibição. Gates de restore/build/run/debug/test/analysis perdem o "host SDK is never substituted".
- **Ajuda sem ambiente.** O texto do runtime manda instalar pela página e usar Redetect SDKs, sem `DOTNET_ROOT`, cópia manual de arquivo ou edição de projeto.
- **Prova.** `tests/e2e/sdk-prerequisites.spec.ts`: OrderDesk 8.0.408 + runtime 10 sem banner; MissingSdk cita 8.0.999 e o link do SDK; runtime ausente (snapshot sobreposto, porque a máquina tem runtime real) mostra ".NET 10 runtime" e `/download/dotnet/10.0`.

## 2026-11-04 — Download do csharp-ls com progresso e retry

- **Plano revisável intacto.** A única via de instalação continua `dotnet tool install --tool-path <staging>` depois de `tools.plan` + `tools.install { confirm: true }`; nada de instalação global, e a limitação de assinatura do NuGet continua no plano e no log.
- **Progresso é byte observado, nunca porcentagem.** `dotnet tool install` não informa total nem percentual. O host mede os bytes que o instalador escreveu no staging, no HOME isolado (cache NuGet) e no `TMPDIR` próprio do app (scratch do NuGet) e expõe `tools["csharp-ls"].progress = { downloaded, total: null }`; a UI reusa `ToolDownloadStatus` do ticket 06, sem fork. O pico por tentativa é monotônico e cada tentativa desconta o que já existia antes dela.
- **Falha não vira pronto.** Exit diferente de 0 mantém `state: "failed"`, não promove o staging e não preenche `sha256`/`verifiedVersion`; o erro nomeia a fonte, manda verificar a rede, explica que o retry recomeça de staging limpo e aponta o log. A tentativa seguinte remove o staging antes de iniciar; a promoção/verificação continua sendo a única porta para `ready`.
- **Reuso não mudou.** Uma instalação verificada continua sendo executada só após a confirmação e reutilizada sem download nem processo; `reused` e o SHA-256 continuam iguais.
- **Seam de teste.** `OPENRIDR_TOOLS_MUXER`, habilitado apenas com `OPENRIDR_ENABLE_PROBE=1`, substitui o muxer do plano por um instalador controlado; o `planId` cobre o argv, então o teste passa pelo plano/confirmação reais e não toca a rede. Em produção a variável não existe.
- **Limite.** "Downloaded" é o que o instalador já gravou em disco, não o contador da rede; o total é desconhecido e não é inventado. O download real de ~99 MB e o reuso são cobertos pelo teste de instalação real existente; a prova no app Tauri real fica para `verify-openridr`.

## 2026-11-04 — Proposta: embutir o netcoredbg

- **Só proposta, sem código.** `docs/PROPOSAL-NETCOREDBG-EMBED.md` mede o asset osx-arm64 3.2.0-1092 (3.389.504 bytes; árvore extraída 9.587.448 bytes) e compara download no primeiro Debug (atual), embutir e bundle de ferramentas sob demanda.
- **Recomendação e custo.** Embutir o osx-arm64 no bundle, reassinado com Developer ID e coberto pela notarização; o DMG cresce ~3,4 MB e o app instalado ~9,14 MiB, e toda atualização do adaptador passa a exigir release do app.
- **Fora.** SDK .NET nunca é embutido; csharp-ls continua baixado pelo usuário e o fluxo revisável do ticket 14 não muda.
- **Decisão pendente de Helder.** Nenhum comportamento mudou nesta rodada; o download no primeiro Debug (decisão de 2026-11-01) continua sendo o que o código faz.

## 2026-11-04 — netcoredbg embutido

- **Pin versionado e verificado no build.** `vendor/netcoredbg/netcoredbg-osx-arm64-3.2.0-1092.zip` (3.389.504 bytes, SHA-256 `f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea`) entra no repositório. `scripts/vendor-netcoredbg.sh` compara o hash e falha o build quando diverge; a extração para `src-tauri/resources/netcoredbg/` é gerada e não versionada (só o `.entitlements` é rastreado).
- **Assinatura e notarização.** O script reassina `libdbgshim.dylib` e `netcoredbg` com `APPLE_SIGNING_IDENTITY` quando presente (Developer ID + hardened runtime + timestamp) e ad-hoc `-` no build local. O binário aninhado entra na assinatura/notarização do app; o `bundle.resources` copia a árvore para `Contents/Resources/netcoredbg/` e o `THIRD-PARTY-NOTICES.md` para `Contents/Resources`.
- **Entitlements mínimos, com prova.** Sem entitlements o adaptador hardened assinado falha ao iniciar o debuggee (timeout/`configurationDone` 0x80004005). Só `disable-library-validation` falha no `configurationDone`; só `allow-jit` morre (SIGSEGV). Com **`com.apple.security.cs.allow-jit` + `com.apple.security.cs.disable-library-validation`** uma sessão DAP real imprimiu a saída do debuggee. Nada além disso foi adicionado.
- **Só osx-arm64.** A resolução embutida existe apenas em macOS arm64; Windows, Intel e Linux seguem sem bundle e sem asset substituto. `OPENRIDR_BUNDLED_NETCOREDBG` é override de diretório para testes, nunca usado pelo app.
- **Bundle primeiro; tools como fallback.** `<exe>/../Resources/netcoredbg/netcoredbg` vence uma instalação antiga em `<app home>/tools/netcoredbg/3.2.0-1092`; `tauri dev` sem resources usa o fallback. O host continua verificando `--version` (3.2.0) antes de qualquer lançamento.
- **Sem download no Debug.** Com o adaptador embutido, `view()` reporta `bundled: true`, estado `ready` (após a flag experimental), versão e SHA-256 do executável; a UI mostra "Bundled with OpenRidr" e esconde plano/progresso/retry. `debug.install` e `debug.retry` recusam com `invalid_state` ("netcoredbg is bundled with OpenRidr; there is nothing to download or install."); o pipeline de download antigo fica como caminho morto guardado.
- **Licença.** `THIRD-PARTY-NOTICES.md` traz MIT do netcoredbg 3.2.0-1092 e dos componentes Roslyn/dbgshim/.NET, com bytes e SHA-256 do zip; `vendor/netcoredbg/LICENSE` preserva o texto upstream.
- **csharp-ls intacto.** Plano revisável, download, progresso, retry e o runtime .NET 10 da análise não mudaram.
- **Limite.** A prova no app Tauri real foi feita com home vazia (sem `tools/`) e com o DMG montado somente leitura; a medição real ficou em 5.671.389 → 10.454.440 bytes (DMG) e 14,68 → 23,85 MiB (app). Quarentena/translocation real e macOS Intel continuam não verificados. Ver `docs/evidence/embed-netcoredbg.md`.

## 2026-11-04 — Polimento visual: ícones e fontes

Spec: `docs/SPEC-UI-POLISH.md`. D4, D6, D7 e D9 respondidas pelo Helder; as demais seguem como assumidas no spec.

- **Dependências aprovadas (D7), só estas três, via bun:** `lucide-react` (ISC), `@fontsource-variable/inter` e `@fontsource-variable/jetbrains-mono` (OFL-1.1). Nada além disso.
- **Lucide é o único conjunto de UI (D1)**; o glifo C# é o `language-csharp` do MDI (Apache-2.0) vendorizado em `src/icons/CSharpIcon.tsx`. Sem hexágono do C#, sem logo do Visual Studio (D2, D11).
- **Cor (D3, D4):** ícones de arquivo neutros (`--muted`, `--fg` na linha selecionada). Play e Debug continuam verdes, Stop vermelho.
- **Traço:** 1,5 px na tela em qualquer tamanho, via `vector-effect: non-scaling-stroke` (`.icon *`) com `strokeWidth=1.5`, porque os tamanhos vêm do CSS e não do prop `size`.
- **Fontes (D5, D6, D10):** `@font-face` próprio só com o subset latin (cobre pt-BR; `latin-ext` fica de fora). O pacote Fontsource traz todos os subsets e o Vite emitiria todos os woff2. "JetBrains Mono" (valor da Settings) e "JetBrains Mono Variable" apontam para o mesmo arquivo. Default novo do editor: JetBrains Mono; valor salvo não é migrado. Ligaduras desligadas (`font-variant-ligatures: none`). `main.tsx` espera as duas fontes carregarem antes de montar o React para o xterm medir a célula com a fonte real.
- **Alvo (D9):** a toolbar da árvore passa de 22/24 px para 32×32 (ícone de 16 px).
- **Escala:** `--fs-ui` 14, `--fs-label` 13, `--fs-meta` 12, `--fs-badge` 11. Títulos de popup/boas-vindas ficam em px literal (15/16/24); `0.85em` do inlay do debugger fica relativo.
- **Debug `step*` (D8):** Lucide `redo-2` / `arrow-down-to-line` / `arrow-up-from-line`; Codicons só entraria se a captura reprovasse.

## 2026-11-04 — Paridade com o Rider: decisões da grill (G01–G23)

Fonte: `docs/RIDER-BEHAVIORS.md`, `docs/GAPS-RIDER-RIDR.md`, `docs/RETRO-RIDER-GAPS.md`.
Spec: `docs/SPEC-RIDER-PARITY.md`; tickets: `docs/tickets-rider-parity/` (01–16).

- **Escopo e prioridade.** A tabela G01–G23 é a lista fechada; observações sem gap são contexto. Ordem: P0 G01; P1 G02, G03, G04, G08, G09, G10, G13, G14, G15, G20; P2 G05, G06, G07, G11, G12, G16, G17, G18, G19, G21, G22, G23. G10, Add Existing, Move e Exclude viram tickets explícitos marcados `adiado`, com motivo e o que falta; nada sai do backlog.
- **Idioma.** Spec, tickets, retro, decisões e evidência em pt-BR; UI e comentários de código continuam em inglês, como o produto e o `AGENTS.md`; assunto de commit no padrão convencional em inglês do histórico, corpo em pt-BR quando precisar.
- **G02/G05 (keymap).** O perfil `rider-macos` já usa ⇧F12 para Find Usages e o `intellij` usa ⌥F7; o menu do Rider observado ao vivo mostra ⇧F12, então G02 fica como está e a decisão registra a fonte. Símbolo continua ⌥⌘⇧T no `rider-macos`; o mapeamento de Ctrl+Shift+Alt+N do keymap base para macOS não foi confirmado e fica pendente em vez de trocar por um atalho de quatro modificadores não verificado.
- **G03/G04.** Search Everywhere ganha duplo Shift **e** ⌘T nos dois perfis, aparece na Settings e na dica da tela inicial. As janelas de ferramenta ganham ⌘1..⌘8 na ordem da faixa (Solution, Commit, Build, Run, Problems, Debug, Terminal, Tests); Git e NuGet ficam sem padrão, mas remapeáveis. O atalho alterna (abre, foca, fecha) e aparece na Settings.
- **G01 (análise).** Find Usages, Rename e code actions compartilham um aviso acionável quando a análise não está pronta: "Install analysis" abre o painel de ferramentas e já pede o plano revisável (a confirmação continua sendo a única via de instalação) ou "Start analysis". Quando a análise fica ativa, a última ação pendente roda uma vez sozinha. Nada instala sem confirmação.
- **G08 (descoberta).** A descoberta roda sozinha uma vez por fronteira de workspace/contexto/revisão de restore, para o projeto de teste selecionado, assim que o modelo avaliado está pronto; falha aparece no painel com a causa. Não há laço a cada edição; trocar o projeto selecionado dispara a descoberta dele.
- **G06/G09 (navegação).** Abrir arquivo por Search Everywhere, Find in Files, recentes, Find Usages, definição ou link de teste devolve o foco ao editor; clique simples na árvore mantém o foco na árvore. A posição `arquivo:linha` da stack trace de teste vira um link com esse texto e abre o arquivo na linha; a stack crua continua visível.
- **G14 (Finder).** Nova operação de host `workspace.reveal` com validação de contenção no workspace, sem gate de trust; no macOS usa `open -R` e nas demais plataformas devolve erro claro de não suportado. Cobre arquivo, pasta, projeto e solução. "Open with" fica adiado: enumerar aplicativos exigiria dependência nova.
- **G15 (menu da solução).** Build Solution, Restore Solution, Copy Path, Find in Files e Add Project desabilitado com motivo. Rebuild/Clean ficam adiados por exigirem operações novas no host; não inventamos item sem backend.
- **G17 (Run Unit Test).** Em projeto que não é de teste o item desaparece; em projeto de teste fica habilitado para rodar todos os testes daquele projeto (`selection: all`), com os mesmos gates e motivos do painel Tests.
- **G20 (Build output).** O host devolve spans de localização (linha/coluna no log) produzidos pelo mesmo parser conservador de MSBuild; a UI só renderiza botões e nunca reparseia o log. Linha não reconhecida continua texto.
- **G07 (Find do editor).** O painel ganha contador "n de m". O escopo "na seleção" fica adiado dentro do ticket 08: escopo com replace é caro e arriscado para P2; o motivo e o que falta ficam registrados. Para retomar, faltam um escopo explícito no estado da busca, comandos de next/prev/all/replace limitados ao range (sobre `SearchCursor`) e testes de seleção/replace; nenhum checkbox falso é exibido até lá.
- **G12 (próxima falha).** Botão "Next failure" no cabeçalho dos resultados cicla as falhas, expande o detalhe, rola até ele e navega para a origem quando existe; sem atalho novo.
- **G18 (configuração derivada).** Continua somente leitura, com botão "Duplicate to edit" no aviso chamando o duplicate existente. Edição direta exigiria um store de overrides por solução/contexto (chave solução + contexto + id derivado), validação contra cada reavaliação, persistência própria e reset para "voltar à derivada"; o duplicate já cobre o caso real e mantém a derivada intacta, então o modelo fica.
- **G11 (chords).** Os chords ⌘U+R/D/L/F continuam; a Settings mostra os chords e uma nota de que não são remapeáveis (o override por atalho único já funciona). Remapear chord fica adiado: exigiria um modelo persistido de dois estágios por ação (prefixo + tecla, não a lista de shortcuts de uma tecla), captura de duas teclas no KeymapPanel, detecção de conflito com o prefixo ⌘U e migração do `chordTable` fixo; nenhum desses existe hoje.
- **G19 (breakpoint).** Breakpoint em linha sem código é aceito, mas marcado como "sem código nesta linha" (visual atenuado + tooltip) até o debugger reportar um bound breakpoint; não recusamos nem movemos às cegas. O comportamento do Rider nesse caso continua **não verificado** e fica registrado como pendência.
- **G19 — implementação (ticket 11).** A marca "sem código" é derivada do texto vivo do editor (`CodeEditor.tsx`): linha vazia ou só com comentário (`//`, `/* */`, inclusive bloco aberto em outra linha) ganha o ponto vazado atenuado, sem tinta na linha e com o tooltip "No code on this line; the debugger will not stop here.". Quando o DAP devolve `verified: true` para aquela linha, a marca volta ao ponto cheio e à tinta normais, mesmo que a heurística de texto a considere vazia. A linha não é recusada nem remapeada; o host já expunha `verified`/`message` por breakpoint e não mudou. Pendente: confirmar no Rider o que ele faz com breakpoint em linha em branco — a implementação atual é a decidida nesta grill, não uma cópia observada.
- **G13/G21/G22/G23 (chrome).** G13 é reprodução primeiro: o código já monta menu de pasta; se abrir, vira prova + "Show in Finder", se não, corrige o handler. A barra de status passa a mostrar o nome exato do perfil ativo. Os três botões de ferramenta ausentes mantêm o tooltip que explica que não fazem parte do OpenRidr; os ícones da Solution já têm `aria-label`/`title` e ganham prova.
- **Verificação.** Aceite de cada ticket no app desktop real via `agent-desktop`; Playwright novo só para contrato de host ou regressão estável. Nenhum ticket é dado como pronto sem evidência no app real; o que não rodar entra como "não verificado" com motivo.
- **Dependência de Debug.** `fix/debug-dotnet-path` não é importada, mesclada nem cherry-picked; `crates/host/src/child_env.rs` e os arquivos de ambiente do Debug não são tocados. Se um spec de Debug falhar pelo sintoma que aquela branch corrige, fica "bloqueado por `fix/debug-dotnet-path`", sem contorno.
- **Limites.** Sem dependência nova; sem screenshots, código ou strings copiadas do Rider; MIT. Commits só locais, um por ticket, com `scripts/commit-only.sh`; sem push, merge, PR ou release. `DESIGN.md` e `PRODUCT.md` não são tocados.

## 2026-11-04 — Solution Explorer visual: chrome fora da árvore, tipo por cor/ícone

Spec: `docs/SPEC-SOLUTION-EXPLORER-VISUAL.md`; gaps: `docs/GAPS-SOLUTION-EXPLORER-VISUAL.md`.

- **Chrome.** A árvore deixa de mostrar `MSBuild evaluated` / `Text outline`, o botão Evaluate, o `revision N` e o botão `Project details`; os rodapés `MSBuild has not run…` e `Partial workspace…` também saem. É a revisão do bullet de `docs/decisions.md` (2026-11-04, paridade) que mantinha os controles visíveis por causa dos specs. O estado já duplicado na barra de status vira a única leitura; o botão `eval-run` vira o ícone de refresh ao lado dela e `eval-state` passa a ser o filho do segmento `status-evaluation`, preservando o texto completo (inclusive `revision N`, lido por regex) e o `title` verificado pelo teste de layout. `Project details` continua só no menu ⋮ (`tree-more-details`), com o painel `eval-details` inalterado. Os avisos acionáveis (`eval-disabled-reason`, `eval-stale`, erro por projeto, restore) ficam: são erro, não chrome.
- **Projeto por tipo, incluindo web.** O ícone distingue console/app, library, test, web e unknown; os chips `· console`/`· net8.0` saem da linha e viram `data-project-kind`/`data-project-tfms` (os specs passam a ler atributos). `web` não existe no modelo avaliado (`OutputType=Exe`), então vem do texto do `.csproj` já lido pelo outline (`Microsoft.NET.Sdk.Web` ou `UsingMicrosoftNETSdkWeb=true`); `test` continua vencendo, e a avaliação não muda.
- **Cor e ícone.** Tokens `--icon-*` no `:root`, com fallback escuro e `light-dark()` dentro de `@supports` (o build só distribui o tema escuro; o claro é forward-compatible, não verificado). Valores partem das cores do Seti (MIT) e foram medidos ≥ 6:1 sobre `--panel` no escuro e sobre `#fff` no claro. Glifos continuam Lucide (ISC) + `CSharpIcon` (MDI, Apache-2.0); classe/interface/enum/record/struct usam um badge de letra desenhado aqui, sem fonte/SVG do Rider, Seti ou codicons. Zero dependência nova.
- **Símbolo C#.** `workspace.symbolKinds { paths }` é read-only, sem trust, confinado ao workspace, teto de 256 arquivos/256 KB, e devolve só `class|interface|enum|record|struct` para `.cs`. O parser em `outline.rs` ignora comentários/atributos e prefere a declaração com o nome do arquivo; sem evidência a chave falta e a UI mantém o glifo C#. Não usa LSP nem compila.
- **Dependencies.** Referência usa o ícone do projeto referenciado, pacote tem ícone/cor próprios e cada TFM ganha uma linha de framework derivada (`Microsoft.NETCore.App (net8.0)`, `Microsoft.AspNetCore.App` para web), sem inventar versão. A lista continua plana (decisão anterior); agrupar Projects/Packages/Frameworks fica fora e exigiria nova decisão + specs.
- **Gerado/excluído.** `no index` mantém o fundo âmbar e passa a esmaecer nome e ícone.
- **Limites.** Verificação visual no app real e Playwright ficam pendentes da liberação da porta 1420; o que não rodou entra como "não verificado" nas notas de implementação e na retro.

## 2026-11-04 — Dependencies agrupadas (Frameworks / Packages / Projects)

- **Substitui** o item "Dependencies lista projetos e pacotes em linhas planas (sem grupos)" do aceite de
  `docs/SPEC-SOLUTION-EXPLORER.md` (e a lista plana implementada no ticket 38). A decisão 10 daquele
  documento — zero dependência nova e `data-testid` existentes preservados — continua valendo. Pedido
  explícito do Helder via orquestrador.
- **Forma.** `Dependencies` tem três grupos expansíveis, como no Rider: `Frameworks`, `Packages` e
  `Projects`. Dependencies e grupos começam fechados; `Expand All` abre os três; grupo sem filhos não é
  renderizado (nada de nó vazio). Chaves de expansão `solution:deps:<projeto>:<grupo>`; testids
  `tree-deps-group-<projeto>-<grupo>`, mantendo `tree-deps-*`, `tree-framework-*`, `tree-package-*` e
  `tree-ref-*`.
- **Conteúdo.** `Frameworks` deriva da TFM já avaliada/lida do texto (`Microsoft.NETCore.App (net8.0)`,
  `Microsoft.AspNetCore.App` para web, `.NET Standard`, `.NET Framework`), sem inventar versão de patch.
  `Packages` mostra nome e versão discreta. `Projects` usa o ícone do projeto referenciado. Os sufixos
  `· project`/`· package` saem porque o grupo já nomeia o tipo; os specs passam a mirar linhas/testids.
- **Fora.** `Assemblies` não é inventado: o host não expõe inventário de assemblies. Categorias do Rider sem
  dado real continuam fora.
- **Prova.** `tests/e2e/solution-explorer.spec.ts` cobre a estrutura em texto (Frameworks/Projects no Cli,
  Packages ausente) e `tests/e2e/evaluation.spec.ts` cobre pacote e referência no modelo avaliado. E2E e app
  real seguem pendentes da liberação da porta; sem captura, o que não rodou fica "não verificado".

## 2026-11-04 — Revisão externa do Solution Explorer: parser, status e cache

- **`workspace.symbolKinds` muda de `{ kinds, scanned }` para `{ entries, scanned, skipped, truncated }`** com `known` (mtime em ms) no request. A UI pede só arquivos visíveis, em lotes de 128, mescla respostas e poda paths removidos; o host faz `stat` e só lê o que mudou. O root é clonado e o lock do host é solto antes do I/O. Motivo: 256 arquivos em silêncio e re-scan a cada arquivo novo piscavam/ficavam sem badge; ver a revisão do provedor externo (9 itens) em `docs/implementation-notes.md`.
- **Parser** passa a exigir palavra-chave minúscula em posição/forma de declaração e ignora strings (verbatim/raw/char), comentários e BOM; stem com ponto (`Foo.Designer.cs`) casa `Foo`.
- **Status bar** usa `evaluationStatusView`: `Evaluating…`/`Evaluation failed` aparecem antes de haver projeto, o `not current` vem antes do revision, a classe `error` volta e o `Partial workspace` (`tree-partial`) reaparece discreto ali, não na árvore.
- **Pacotes** têm chave `name@version` (+ TFM quando existir) e são deduplicados antes de renderizar.
- **Testes:** parser/symbol no host (9/9 e 18/18), unit no Bun (21/21) e cargo lib (235/235). E2E/app seguem pendentes de liberação.

## 2026-11-04 — Confinamento de arquivos: negação por forma antes do stat, canônica depois

- **Duas checagens, nesta ordem, em todo resolvedor de caminho do host** (`resolve_in_root`, `resolve_path_in_workspace`, `resolve_reveal_in_workspace`, `workspace.tree`, `nuget::resolve_project`): primeiro `fsutil::stays_inside` normaliza `.`/`..` **sem tocar no disco** e recusa por forma com `outside_workspace`; depois, sobre o caminho já canonicalizado, `canonical.starts_with(root)` barra symlink (componente final ou intermediário) que resolva para fora. A checagem léxica não substitui a canônica.
- **Motivo:** canonicalizar antes de decidir fazia um escape **inexistente** responder `not_found` e um **existente** `outside_workspace` — um oráculo de existência do filesystem para `file.read`, que não pede trust, e negação não fail-closed (o host `stat`ava o caminho de fuga antes de decidir). Não houve escape de leitura/escrita em nenhum vetor testado; o defeito era de contrato e observabilidade.
- **`not_found` volta a significar só "não existe dentro do workspace".** Um caminho que escapa, exista ou não, é sempre `outside_workspace`.
- **Revisão (mesmo dia):** a checagem do caminho inteiro deixava brecha de observabilidade — symlink dentro do workspace apontando para fora, com alvo **inexistente**, respondia `not_found` em vez de `outside_workspace`. A decisão canônica passa a resolver o **ancestral existente mais profundo** quando o alvo não existe (`fsutil::canonical_confined`): fora → `outside_workspace`, tudo dentro → `not_found`. E a checagem léxica vale só para entrada **relativa**: um caminho absoluto pode ter prefixo não canônico (`/tmp` × `/private/tmp`, APFS ignorando caixa) e ser o mesmo arquivo, então para absoluto a contenção é decidida pela canonicalização — o helper único (`confine`/`confine_raw`) atende as cinco fronteiras.
- **Revisão 2 (mesmo dia):** symlink que existe e não resolve — pendente **ou laço** — é sempre `outside_workspace`: o host não prova para onde aponta, então não prova que está dentro, e a resposta é a mesma para qualquer alvo do outro lado (inclusive um pendente cujo nome do alvo seria interno). `not_found` fica restrito a "nada aqui e toda a ascendência resolvível está dentro". As três fronteiras que ainda faziam `canonicalize` + `starts_with` por conta própria (preview de rename/code action, `definition_location`, `outline_project` lendo o caminho vindo do `.sln`) passaram pelo mesmo `confine`; nenhuma delas responde à UI com código de erro, então a mudança observável é só deixar de ler/oferecer um alvo atrás de link pendente.
- Prova e matriz de vetores: `docs/evidence/file-access-confinement.md`.

## 2026-11-05 — Menu de contexto: um dono por linha e guard contra o menu do WebKit

- **Contexto.** Aceitação da alpha.7, ticket 05: `AXShowMenu` em linha de pasta/arquivo abria o menu do
  WebKit. Reprodução no app real mostrou duas causas: (a) linhas sem `onContextMenu` — solution folder,
  `Dependencies`, grupos e folhas de dependência; (b) `Shift+F10`/tecla de menu nunca abriram nada,
  porque o WebKit não os converte em `contextmenu`. Fora da árvore, toolbar/abas/status bar mostravam o
  menu de navegador do WKWebView.
- **Forma.** `TreeRow` é o único dono da decisão: sempre `preventDefault` no `contextmenu`, trata
  `Shift+F10`/`ContextMenu` no teclado e entrega uma **posição** (`MenuPosition`) para quem desenha o
  menu. `TreeTarget` ganhou `solution-folder`; `Dependencies`, grupos, framework, pacote e referência
  abrem o menu do **projeto dono** (mesma escolha do Rider: são comandos do projeto). A pasta do `.sln`
  reaproveita os comandos da solução sem `Copy Path`/`Show in Finder`, porque a pasta pode ser virtual.
- **Guard.** `src/nativeMenu.ts` instala um listener de `contextmenu` no documento, **só em produção**
  (`import.meta.env.PROD`): cancela tudo que não é `input`/`textarea`/`[contenteditable]`. Em
  desenvolvimento o menu do WebKit fica, porque é o caminho para "Inspect Element" no webview. Sem
  dependência nova e sem swizzle nativo.
- **Nomes acessíveis.** Os menus da árvore agora dizem de que linha são (`File`/`Folder`/`Solution folder`
  context menu); projeto e raiz mantêm `Project`/`Solution context menu`. Nenhum `data-testid` mudou.
- **Prova.** E2E novo (`native-context-menu.spec.ts`, vermelho antes/verde depois), unit do guard, e app
  real antes/depois em `docs/evidence/native-context-menu.md`. Windows/Intel e o DMG desta correção não
  rodaram.

## 2026-11-05 — Revisão do menu de contexto: terminal, precedência de teclado e dono do foco

- **Terminal é exceção do guard.** O xterm trata `contextmenu` só para focar o `textarea` escondido e
  deixa Copiar/Colar no menu do sistema; cancelar ali removia a única forma de copiar de um shell.
  A lista de exceções passa a ser "onde o menu do sistema é a funcionalidade": `input`, `textarea`,
  `[contenteditable]:not([contenteditable="false"])` e `.xterm`. `contenteditable="false"` deixou de
  passar por acidente do seletor.
- **`Shift+F10` e tecla de menu pertencem à linha focada.** O handler global do `WorkspaceView` roda em
  capture e casa atalhos do keymap primeiro (IntelliJ macOS: `Shift+F10` = Run All Tests; Rider Windows:
  Run). Nova regra: se o alvo está em `.tree-row`, esses dois gestos abrem o menu da linha e o evento
  segue para ela; fora da árvore o keymap vale. O `Shift+F10` que não é menu também não arma o
  duplo-Shift.
- **Quem abre o menu é dono do foco.** `onMenu` passa a receber também a linha (`row`), e o menu devolve
  o foco a ela ao fechar — o clique direito não move o foco no WKWebView, então `document.activeElement`
  devolvia uma linha antiga.
- **Solution folder sem `Add Project…`.** A ação adiciona na raiz da solução, não na pasta; o menu da
  pasta fica com Build/Restore/Reload/Find in Files.
- **Operação.** `git stash` é global ao repositório e este repo tem várias worktrees: para provar
  "falha antes", copie o arquivo para `/tmp` e restaure, nunca use `stash` (um pop puxou o stash de
  outra sessão e trouxe conflitos Rust para este worktree).

## 2026-11-05 — Foco ao fechar o menu da árvore (re-check da revisão)

- **Quem devolve o foco é o menu, e só enquanto ele o segura.** `closeTreeMenu` só chama
  `row.focus()` quando o `document.activeElement` ainda está no próprio menu (ou é o `body`). Uma ação
  que abre diálogo/painel e foca o campo (rename, new file, delete, Find in Files) ou o editor mantém o
  foco: o React aplica `autoFocus` no commit, depois do handler do clique, então o campo sempre vence —
  e um `focus()` síncrono dentro de uma ação futura também não seria roubado.
- **Abrir arquivo pela árvore mantém o foco na árvore**, inclusive pelo menu (`Open`). É a convenção
  já testada em `focus-after-navigation.spec.ts` ("a plain tree click keeps the focus in the tree");
  mover o foco para o editor nesse caminho seria mudar produto, não este ticket.
- **Clicar fora mira onde o usuário clicou.** Se o alvo é focável (editor), ele fica com o foco; a linha
  recupera o foco quando nada mais o tomou (Esc, clique em área não focável).

## 2026-11-05 — Ferramentas obrigatórias no setup

- A abertura segue setup verificado de csharp-ls e netcoredbg → trust → workspace; falha no setup bloqueia o workspace e oferece retry. Ferramentas verificadas são reutilizadas entre projetos e aberturas.
- Instalar e verificar ferramentas de origem, versão e checksum fixos não executa código do projeto e não depende de trust. Análise, restore, build, test e debug continuam exigindo trust; em modo restrito não são iniciados. Com trust, a análise inicia automaticamente.
- O drawer passa a diagnóstico e reinstalação. Isso substitui as decisões anteriores de plano revisado para csharp-ls e flag experimental/consentimento para netcoredbg; não substitui os demais gates de segurança. SDK do projeto não é instalado nem imposto; ausência de runtime .NET 10 é explicada com link adequado.
- O pacote `csharp-ls` 0.28.0 deve ser obtido da flat container oficial do nuget.org e conferido antes da instalação usando SHA-512 fixo `a138b9c8f9de9dd6343e9b0d0c8f4d51637bba7e38606b7c2b08b1d91e8eefa39f12123057f6625f425812b4361e52841bd2856704ec127ff50144ce05170a99`. Em 2026-11-05, o hash calculado do `.nupkg` coincidiu com `packageHash`/`SHA512` no catálogo nuget.org da versão. A instalação deve usar somente fonte local temporária; manter também hash persistido do launcher para verificar reutilização.

## 2026-11-05 — Gate de setup: snapshot, retry e reutilização sem trust

- `setup.start` é idempotente e não exige trust: verifica runtime .NET 10, habilita o netcoredbg embutido (quando existe bundle) e inicia/reutiliza o csharp-ls. O snapshot publica `setup.state` (`idle|running|ready|failed|blocked`), erro, tentativa e runtime; a UI usa esse estado para bloquear o workspace, mostrar progresso e oferecer retry. `setup.state` nunca é `ready` só porque o processo de instalação saiu 0.
- A UI chama `setup.start` ao abrir o projeto e só mostra trust/workspace depois de `ready`. Uma instalação já verificada é reutilizada de forma síncrona, então a segunda abertura não exibe o gate. `workspace.close` interrompe o setup em andamento; a próxima abertura recomeça.
- Uma instalação antiga sem `.openridr-integrity` é verificada uma vez (`--version`) e adotada, em vez de baixar o mesmo pacote de novo; um launcher inválido continua sendo substituído pelo pacote verificado. `setup.reinstall` apaga somente a instalação do app home e é recusado enquanto a análise usa o launcher.
- O netcoredbg embutido não usa mais checkbox/flag experimental na UI: o `debug.flag` continua existindo como gate de plataforma para instalações não embutidas, mas o setup liga a flag apenas quando há bundle. O drawer é diagnóstico (versão, checksum, caminho, log, Reinstall), não um plano a confirmar.

## 2026-11-05 — Tests e NuGet: o que foi implementado e o que continua pendente

- **Tests**: abas Explorer/Continuous Testing/Coverage (as duas últimas desabilitadas com motivo), contadores por resultado, estado agregado por grupo, painel de detalhe da seleção com duração, expandir/recolher tudo e exportação JSON para `appHome/exports/tests-*.json` via `tests.export`. A árvore continua derivada do FQN; não existe sessão "All tests from Solution" porque a descoberta é por projeto/TFM. Debug de teste, Continuous Testing e cobertura continuam sem capacidade no host e aparecem desabilitados, nunca simulados.
- **NuGet**: escopo Projeto/Solução (`nuget.listSolution` agrega uma listagem real por projeto), fonte selecionada e prerelease passam para `dotnet list package --outdated` (`--source`, `--include-prerelease`), implícitos/transitivos vêm de `obj/project.assets.json` (ausência/erro é explícita), seleção de updates em lote (`nuget.planBatch`/`nuget.applyBatch`) com um `dotnet add` por pacote, falha por pacote e um restore explícito por projeto ao fim, e Discover (`nuget.search`) pela `SearchQueryService` v3 da fonte escolhida.
- **Pendências explícitas**: instalação de um pacote descoberto e update em lote não foram confirmados contra um projeto real nesta máquina (os gates, planos e rejeições têm teste; o `dotnet add` real de lote não foi executado para não alterar a fixture). Escopo Solução não foi exercitado no app real. Debug de teste/Continuous Testing/cobertura seguem fora do host. Windows e macOS Intel não foram executados.

## 2026-11-05 — Correções da revisão: lote, setup, assets, SSRF e saída

- **Lote não mascara `dotnet add`:** `nuget::apply` devolve `UpdateResult{state:"failed"}` para exit ≠ 0 sem erro de host; `applyBatch` agora checa `state == "ok"`, conta a falha, propaga a mensagem e restaura somente projetos com pelo menos um update aplicado. CPM é recusado por item (o plano pode ser criado; `apply` falha só aquele item); o trust/contexto é rechecado antes de cada item; `download-N` é removido ao terminar/cancelar e temporários antigos são limpos.
- **Instalação só de pacote verificado:** um `.openridr-integrity` válido cujo launcher não responde à versão fixa é descartado e a instalação volta ao `prepare_local_package` (fonte local + SHA-512). Launcher antigo sem registro não é adotado. `Reinstall` baixa para staging, verifica a cópia antes de promover e só troca o destino atomicamente; falha mantém a instalação anterior, inclusive o estado `ready`. A hash registrada é a do apphost; não há manifesto por DLL (limitação documentada), o pacote é que fixa o conteúdo.
- **Setup é gate de host:** `trust.grant`, `trust.set` e todo `require_trust` recusam com `setup_required` enquanto o setup não estiver verificado. A UI não é a única barreira; o teste de host usa um seam público `#[doc(hidden)]` (`allow_setup_for_tests`) para os testes antigos de trust sem baixar ferramentas.
- **Assets reais:** implícitos vêm de `project.frameworks.<tfm>.dependencies` com `autoReferenced: true` e de `downloadDependencies`; `targets` só fornece transitivos. TFM longo (`.NETStandard,Version=v2.0`) é normalizado, RID (`net8.0/osx-arm64`) é dobrado no TFM base e `BaseIntermediateOutputPath` é lido quando é literal. Limite: caminho com propriedades MSBuild não é avaliado.
- **Discover/SSRF:** a fonte precisa estar em `dotnet nuget list source`; só HTTPS, hosts loopback/privados/link-local e userinfo são recusados no índice e no `@id`; redirects do `ureq` estão desligados. `dotnet list --source` passa pela mesma validação.
- **Saída do app:** `WindowEvent::Destroyed` da janela principal, `ExitRequested` sem janela e `Exit` fecham o workspace, parando LSP/netcoredbg. `kill(-pid)` só é enviado enquanto o líder ainda existe; depois de reaped, o pgid pode ter sido reutilizado e não é sinalizado.
- **Paridade visual:** Tests mantém a árvore após o run (estado agregado por nó, detalhe/duração no painel direito, barra vertical e aba de sessão); NuGet usa seções recolhíveis de updates/instalados/implícitos com checkbox geral, badge e barra vertical. `All tests from Solution` é rotulado com o projeto atual porque o host descobre por projeto/TFM; a aba não finge uma sessão de solução.

## 2026-11-05 — Rodada 3: feed verificado, trust.set, swap atômico, DNS e grupos

- **Worker de instalação:** `begin_install_worker` recusa `dotnet tool install` quando `local_package` é `None` e não está no seam de probe. A checagem fica no worker, não no chamador; todo caminho de release passa por `--add-source` + config local do pacote SHA-512.
- **`trust.set`:** com um workspace aberto, só o caminho desse workspace pode ser decidido; outro caminho retorna `invalid_state` até fechar. Para o caminho aberto, `trusted` continua exigindo `setup_ready`; `restricted`/revoke é ação de segurança. Sem workspace (welcome), a decisão persiste porque não inicia processo nem libera execução.
- **Reinstall:** em macOS a promoção usa `renamex_np(RENAME_SWAP)`, uma única troca atômica; o staging substituído (diretório antigo) é removido depois. Falha da troca deixa o destino intacto; em plataformas sem `RENAME_SWAP` resta o fallback de dois renames (documentado).
- **Discover/DNS:** um resolver do `ureq` resolve o host uma vez, recusa qualquer resposta loopback/privada/link-local (v4/v6) e o cliente conecta nos endereços validados. `dotnet list --source` aceita apenas fonte presente em `dotnet nuget list source` e HTTPS. Redirects do `ureq` são desligados; redirects internos do `dotnet` não são controláveis e ficam como limite documentado.
- **Seams:** `probe_muxer` e `allow_setup_for_tests` existem só com `debug_assertions`; o build release retorna `None`/não expõe o método. Verificado por `nm`/`strings` no `.rlib` e no binário do app: zero símbolos/strings.
- **Grupos de processo:** o registro observa a saída com `waitid(WNOWAIT)`, mantém o líder não-reaped (pid/pgid reservados), envia TERM ao grupo, espera, escala para KILL e só então reapa. Assim netos (BuildHost) não ficam órfãos e um pgid reciclado não é sinalizado. Teste com shell que cria `sleep 60` e sai; `pgrep BuildHost` vazio após `cargo test --workspace` e após fechar o app.
- **Lote:** antes de cada restore, trust/contexto são rechecados; revogação registra restore falho e não executa comando. `BaseIntermediateOutputPath` expandido/herdado marca `assets.state="unsupported"` e o painel mostra “implícitos indisponíveis (obj fora do padrão)” em vez de lista vazia.

## 2026-11-05 — Rodada 4: IPv6 mapeado, staging verificado e seams de release

- **SSRF IPv6:** o classificador normaliza IPv4 mapeado/compatível (`::ffff:a.b.c.d`, `::a.b.c.d`) e o prefixo NAT64 `64:ff9b::/96`, e rejeita 0.0.0.0/8, CGNAT 100.64/10, 198.18/15, multicast, fc00::/7, fe80::/10 e ::1. A validação é usada na fonte, no índice e no `@id`; o resolver conecta nos IPs já validados.
- **Worker:** além de exigir `local_package`, ele relê o `.nupkg` e confere o SHA-512 imediatamente antes de `dotnet tool install`. Nenhum caminho de release inicia o comando sem `--add-source` para o feed local verificado.
- **Staging/rollback:** a launcher é verificada e a integridade é gravada **no staging** antes do swap. O diretório anterior fica em staging até a verificação pós-swap; falha antes do swap mantém a instalação anterior, falha depois troca de volta e restaura o registro `ready`. Em macOS o swap é um `renamex_np(RENAME_SWAP)`; o fallback não-macOS de dois renames não consegue rollback completo e fica documentado.
- **Probes:** `processes.probe`, `diagnostics.probe`, `nuget.probe`, `OPENRIDR_ENABLE_PROBE`, `OPENRIDR_TOOLS_MUXER`, `OPENRIDR_TOOLS_PROBE_DOWNLOAD_DELAY_MS`, `OPENRIDR_BUNDLED_NETCOREDBG`, `OPENRIDR_DEBUG_*`, `OPENRIDR_FORCE_HOSTROOT` e `allow_setup_for_tests` ficam em `cfg(debug_assertions)`. O release não contém esses nomes (verificado por `strings`/`nm`).
- **Restore do lote:** o guard do host é mantido entre a checagem de trust e o início síncrono do restore; uma revogação concorrente espera o restore terminar em vez de existir janela entre check e execução. A alternativa de registrar o restore no mecanismo normal fica como evolução.
- **trust.set na welcome:** decisão para caminho sem workspace é intencional (não inicia processo); ao abrir, o host aplica a decisão armazenada mas `require_trust` ainda exige setup ready. Teste de host cobre isso.
- **PGID:** o líder só é reaped depois de TERM/KILL e da limpeza do grupo; o comentário `ponytail:` em `finish_group` registra o risco teórico de reuso de PGID se o desenho mudar para reaper antes.
- **BaseIntermediateOutputPath:** `Directory.Build.props`/`targets` encontrados na árvore (busca textual) ou propriedade expandida marcam `assets.state="unsupported"`; o painel mostra “implícitos indisponíveis”, nunca uma lista vazia.

## 2026-11-05 — Rodada 5: ferramentas são globais ao workspace

- Trocar/fechar workspace não sela mais a instalação de csharp-ls. O estado e o staging pertencem ao app home, não ao projeto; a instalação termina sozinha (ou faz rollback) e a próxima abertura reutiliza. `stop_services` continua parando somente análise/debug do workspace.
- O único caminho que sela ferramentas é o shutdown do `Host` (fim do app), para não deixar processo de instalação órfão. O teste `a_reinstall_survives_a_workspace_switch_and_never_loses_both_copies` troca de workspace durante `verifying` (depois do swap), exige `ready` com uma launcher existente e prova que `setup.start` seguinte reutiliza em vez de apagar.
- Isso substitui a decisão anterior de que "toda fronteira de workspace para uma instalação real"; ferramentas não pertencem ao contexto do workspace.

## 2026-11-05 — Ícones da Solution Explorer vêm do T3 Code (MIT) e do @pierre/trees (Apache-2.0)

- **Origem de cada glifo.** `.cs` = hexágono `C#` do Simple Icons (CC0), copiado do sprite do T3 Code;
  documento genérico (`.sln`, `.slnx`, `.csproj`, `*.DotSettings[.user]`, desconhecido), `json`, `markdown`,
  `docker`, `yml`, `text` e `git` = símbolos vendorizados de `@pierre/trees` 1.0.0-beta.6 (Apache-2.0) em
  `src/icons/fileGlyphs.tsx`; `.xml`/`.props`/`.razor`/`.resx`/config/script continuam no Lucide (ISC) que já
  era dependência. Pares de cor claro/escuro e a lógica nome→extensão seguem o T3 Code (MIT). O pacote
  `@pierre/trees` **não** entra como dependência (é beta). Textos de licença e atribuição:
  `THIRD-PARTY-NOTICES.md`; mapa completo: `docs/SPEC-SOLUTION-FILE-ICONS.md`.
- **`.cs` usa o hexágono como padrão.** O selo de tipo do PR #23 (`C`/`I`/`E`/`R`/`S`) deixa de ser o ícone
  principal e passa a ser um selo de 9 px sobreposto no canto inferior direito, com `title` do tipo e o
  `data-symbol-kind` da linha. A informação não se perde; o arquivo passa a ser reconhecido como C# de
  relance. Substitui o D8 de `SPEC-SOLUTION-EXPLORER-VISUAL` (badge no lugar do glifo C#), não o D7 (o
  contrato `workspace.symbolKinds` continua igual).
- **Pasta é chevron e nome.** Pastas de fonte, pastas do `.sln` e Show all files não desenham ícone de
  pasta; o slot de 16 px fica vazio para o nome alinhar com o de arquivo. Projetos e a raiz mantêm o glifo
  semântico (tipo de projeto/solução), porque na referência essas linhas são pastas e aqui são projetos.
  Nenhum `data-testid` muda.
- **Guia e seleção.** Cada nível ≥ 1 desenha uma linha de 1 px (`--line`) na coluna do chevron do ancestral;
  a linha atual/selecionada ganha contorno `--accent` com `box-shadow: inset` (não muda altura nem reflow).
- **Paleta = T3 Code.** Escuro usa os hex do T3 Code (tema da captura de referência): `csharp #9d6afb`,
  `json #ffa359`, `markdown #5ecc71`, `docker #69b1ff`, `doc/text #adadb1`, `git #d5512f` — contraste medido
  ≥ 4,5:1 sobre `--panel`. O tema claro usa os pares do T3 Code e fica ≥ 3:1 (mínimo de WCAG 1.4.11 para
  ícone, não texto); ele não é distribuído e continua **não verificado**.

## 2026-11-05 — Revisão dos ícones: licença literal, tipo acessível e piso de 3:1

- **Licença:** o aviso MIT do T3 Code mantém o copyright literal do repositório ("Copyright (c) 2026 T3 Tools
  Inc."); o `@pierre/trees` lista pacote/versão/arquivo de origem, o texto Apache-2.0, o `NOTICE.md` do
  pacote e o que foi modificado nos SVGs (sprite removido, classes e `opacity` próprias, cores fixas →
  `currentColor`). Onde há obrigação de atribuição, o texto é literal, não parafraseado.
- **Acessibilidade do tipo de símbolo:** o selo de 9 px é decorativo (`aria-hidden`); o tipo vai numa
  descrição (`aria-describedby` + `.sr-only`) renderizada **fora** do `<button>` da linha. Dentro do botão o
  texto do nó entraria também no nome acessível (o E2E mostrou "Order.cs C# class"); fora, o nome continua
  sendo só o arquivo e a descrição lê "C# class".
- **Contraste de glifo de dois tons:** ícone é gráfico não textual (WCAG 1.4.11), então a opacidade do corpo
  da página entra no orçamento de contraste. `.5` deixava o documento cinza em 2,95:1 no `--panel`; `.75`
  mede ≥ 3,86:1 (pior caso, linha atual focada em `--select`) e 5,31:1 no painel. No tema claro a
  transparência reprovaria (2,34:1), então lá o corpo usa `opacity: 1` (3,72:1 sobre branco) — tema não
  distribuído, não verificado. O piso de 3:1 virou teste E2E que lê os tokens do CSS real.

## 2026-11-05 — Estado de testes tem um só ponto de composição; download medido é dono do progresso

Duas leituras do mesmo estado divergiam sem erro visível: o snapshot injetava o run real e o export serializava
a vista da descoberta (run sempre idle); o progresso do setup era publicado pelo leitor do pacote (com
`Content-Length`) e sobrescrito por uma soma de bytes em disco que contava o mesmo `.nupkg` três vezes. Regra
nova: **quem lê o estado de testes chama `Inner::tests_view()`** e **um número medido nunca é substituído por
uma estimativa**. Consequência: a observação de disco do `dotnet tool install` só existe onde não há medição
(instalador de probe), e `Progress.total` deixou de ser sempre `None`. Alternativa recusada: ensinar o export a
lembrar do run — o mesmo erro voltaria no próximo leitor.

## 2026-11-05 — Análise embutida: csharp-ls 0.28.0 + runtime .NET 10 privado

- **Versões fixadas, não escolhidas por conveniência.** `csharp-ls` 0.28.0 (o mesmo pin que o instalador já exigia; nupkg 21.643.814 bytes, SHA-512 `a138b9c8…170a99`) roda no runtime oficial `dotnet-runtime-10.0.14-osx-arm64.tar.gz` (33.251.887 bytes, SHA-512 `19c5781d…52b0382`, conferido contra o `releases-index.json` do canal 10.0). O `csharp-ls.runtimeconfig.json` pede `Microsoft.NETCore.App 10.0.0`, então 10.0.14 (LTS, patch mais alto do canal na data) é compatível; subir/descer de versão exige nova prova.
- **Layout no bundle, sem shim do NuGet.** `Contents/Resources/csharp-ls/` é o payload `tools/net10.0/any` extraído (sem `.nupkg` duplicado, sem launcher gerado) e `Contents/Resources/dotnet/` é a árvore oficial do runtime (muxer, `host/fxr/10.0.14`, `shared/Microsoft.NETCore.App/10.0.14`, `LICENSE.txt`, `ThirdPartyNotices.txt`). O host executa `<hostroot>/dotnet <Resources>/csharp-ls/CSharpLanguageServer.dll`; não há downloader de análise no app.
- **O `hostroot` não mudou de ideia, só de fonte.** Continua compondo SDK do projeto (externo, `global.json`) + runtime 10 em links no app home, com o muxer copiado. A única diferença é que a raiz do runtime 10 vem do bundle (`AnalysisRuntime::for_analysis`), com o inventário externo como fallback de `tauri dev`. Embutir o SDK continua fora de escopo: projeto que pede 8 usa o SDK 8 instalado, nunca o 10.
- **Bundle vence cache e download.** Com o par completo, `tools.view().state = ready` depois de um `--version` real (uma vez por sessão, sem rede), `plan = null` e `tools.plan`/`install`/`retry`/`setup.reinstall` recusam com "bundled; nothing to install". O launcher instalado em `<app home>/tools` não é executado quando o bundle existe. Sem bundle, o fluxo da alpha.8 fica igual.
- **Assinatura do runtime privado.** Muxer e 13 dylibs são reassinados no build (Developer ID + hardened + timestamp quando `APPLE_SIGNING_IDENTITY` existe; ad-hoc local) e o muxer leva `allow-jit` + `disable-library-validation` — sem `disable-library-validation` o `libhostfxr` do runtime não carrega sob hardened runtime ad-hoc (Team IDs ad-hoc distintos), e sem `allow-jit` o CLR não JITa. O app em si não ganha entitlements; o processo do analisador é filho.
- **Licenças.** `THIRD-PARTY-NOTICES.md` cobre csharp-ls (MIT) e o runtime (MIT), e o tarball leva `LICENSE.txt`/`ThirdPartyNotices.txt` para dentro de `Contents/Resources/dotnet/`.
- **Custo real.** DMG 10.699.101 → **61.774.547 bytes** (+51,0 MiB, ~5,8×); `.app` release 24,25 → 163,29 MiB. O runtime extraído é 83 MiB e o payload do csharp-ls 57 MiB; o UDZO comprime as duas árvores pior que o zip/nupkg original.

## 2026-11-05 — Integridade do par embutido: a referência é a assinatura do app, não um hash em runtime

- **Decisão do Helder (rodada 2, re-check do PR #32).** O conteúdo de `Contents/Resources/{csharp-ls,dotnet}` é atestado pela **assinatura de código e notarização do `.app`** — o selo de recursos que o Gatekeeper valida na instalação. Não existe hash fixado comparado em runtime: o host não tem uma referência confiável embutida para isso, e inventar uma (hash no binário) só moveria a confiança para o mesmo artefato.
- **O fingerprint (caminho/tamanho/mtime) é detector de mudança, não atestação.** Ele derruba o estado `ready` quando a árvore muda durante a sessão — inclusive entrada, dependências gerenciadas, muxer e componente do runtime — mas não prova autenticidade; quem reescreve a árvore e restaura mtimes não é detectado por ele.
- **SHA-256 do DLL e do par é informativo.** Aparece no SetupGate, no Workspace tools e no log com esse rótulo. A UI não pode dizer que o par foi "verificado contra hash".
- **Build ad-hoc não conta como assinatura confiável.** Enquanto a release não for assinada com Developer ID + notarizada, a garantia é só a de um build local; o `vendor-analysis.sh` já usa `APPLE_SIGNING_IDENTITY` quando presente.

## 2026-11-06 — Suíte de testes hermética: o teste decide o ambiente; ticket é issue do GitHub

> **Substituída em parte em 2026-11-07** por "O fluxo do repositório é o main flow": a spec da frente passa a ser o corpo da issue-mãe e o fluxo não tem retro. O resto da entrada (ambiente hermético, paralelismo, skip) continua valendo.

- **Ambiente é decisão do teste, não do shell.** Um `OPENRIDR_BUNDLED_NETCOREDBG` exportado derrubava 10 dos 14 testes do adaptador (medido: `4 passed; 10 failed`) e 2 dos 98 do host. O harness (`crates/host/tests/support`) remove uma vez por binário as variáveis `OPENRIDR_*` que o host lê, e cada helper que precisa de uma a define e a restaura no `Drop`. Nada muda no produto: o host continua lendo o ambiente; quem abre mão da configuração implícita é o teste.
- **Nenhum teste escreve o ambiente do processo compartilhado.** O ambiente comum de um binário de teste é definido **uma única vez**, antes de qualquer teste: `support::prepare()` (integração) e `test_harness::prepare()` (unidade da lib) removem as `OPENRIDR_*` herdadas e apontam `OPENRIDR_HOME` para uma home temporária do processo; a integração liga também o probe. Depois disso ninguém escreve env. Teste que precisa de valores diferentes (instalação de ferramenta, bundle, hostroot, store de trust, `SHELL`, probe desligado) roda o **corpo** num processo-filho dedicado: o corpo vira uma folha `#[ignore]` (`*_isolated`) e um wrapper chama `run_isolated_leaf`, que exige `1 passed`. O filho recebe ambiente controlado (`env_clear` + só `PATH`/`HOME`/`TMPDIR`/`LANG`/`LC_ALL`), não o do shell. Um teste-guard varre `crates/host/src/**` e `crates/host/tests/**` recursivamente e falha se `set_var`/`remove_var`/`EnvVar::` aparecer fora de folha isolada (ou do módulo de harness), se uma folha `*_isolated` não for `#[ignore]`, ou se um teste não isolado não chamar `prepare`/`host` antes de tocar no host. O problema vira impossível por construção, sem lock, leitor sem guard, thread interna do host lendo env ou deadlock.
- **Paralelismo default, sem lock de ambiente.** `cargo test` roda com o paralelismo default e `--test-threads=1` não é pré-requisito de verde. Como o ambiente do processo é imutável depois do setup, leitura concorrente é segura por definição; os corpos isolados rodam em paralelo, cada um no seu processo.
- **Nada de caminho global nem nome fixo em `/tmp`.** Teste usa `tempfile`/home por processo já existente (`app_home()` devolve home temporária quando o executável é de teste) e alvos de escape dentro de um `tempfile::tempdir()` novo, nunca um nome previsível como `/tmp/openridr-created.txt` nem `<pid>` adivinhável. Duas sessões no mesmo Mac (com `CARGO_TARGET_DIR` compartilhado e o app instalado em `/Applications`) não podem mais interferir por ambiente, home ou nome fixo.
- **Skip só quando a ferramenta não existe; instalação quebrada falha.** O teste do launch profile resolve o SDK pelo `global.json` e lança pelo muxer da raiz resolvida com `DOTNET_ROOT`. Sem raiz que satisfaça o pin, imprime skip com a versão pedida e as raízes vistas (e falha se `OPENRIDR_REQUIRE_DOTNET=1`); muxer resolvido que não existe ou não inicia é falha, não skip. Skip é "não verificado", nunca passe.
- **Teste que depende de ferramenta resolve como a produção.** O teste do launch profile passa a resolver o SDK pelo `global.json` e a lançar o fixture pelo muxer da raiz resolvida com `DOTNET_ROOT` — o mesmo caminho de Run/Debug — em vez de chamar o `dotnet` do PATH. Sem raiz que satisfaça o pin, skip com a versão pedida e as raízes vistas; skip é "não verificado", não passe.
- **Quarentena do E2E é decisão pendente, não conserto.** Os specs E2E que a issue #43 relata ficam nomeados com motivo (o `debugger.spec.ts` foi escrito para o download do netcoredbg, que virou bundle) e bloqueados pela decisão do Helder sobre manter/migrar/quarentenar o Playwright. Sem rodar Playwright não há falha-antes/passa-depois, então não se declara conserto.
- **Tickets passam a ser issues do GitHub.** Decisão do Helder de 2026-11-06: `/to-tickets` publica em `Heldinhow/openridr` (`Part of #43`, bloqueio em texto `Blocked by #N`), não em `docs/tickets-<tema>/`. Spec, RETRO e evidência continuam em `docs/`.

## 2026-11-06 — Playwright só no smoke noturno; E2E não é verificação de UI de sessão

> **Substituída em parte em 2026-11-07** por "A superfície de verificação de UI segue o que mudou": Playwright em modo web volta a ser verificação de UI de sessão para interface pura. O smoke noturno como único E2E automatizado, o "nunca em PR" e a quarentena com motivo continuam.

- **Decisão do Helder (issue #67).** A suíte E2E deixa de ser gate de PR e deixa de ser ferramenta de verificação de sessão: sessão de agente prova UI com `agent-desktop`. O que sobra é um **smoke noturno automatizado**, cuja lista é `scripts/e2e-smoke.txt` (workspace, build, run, qa2-diagnostics: 25 testes, ~2,8 min) e cujo workflow pertence à issue #56.
- **Quarentena com motivo, não conserto sem prova.** `debugger.spec.ts` (12 de 13 falhando, medido) testa o fluxo `debug.install` anterior ao bundle; fica `test.skip` no arquivo com o motivo. `csharp-ls.spec.ts:150`, `testsuite.spec.ts` (`tests-filter-count`), `tests.spec.ts:317` e `ui-layout` em 1440 px ficam na tabela como relatados e **não re-verificados** nesta rodada. A tabela vive em `docs/evidence/e2e-quarantine.md`.
- **Spec desatualizada não é quarentena.** `workspace.spec.ts` esperava o texto do download (`osx-arm64 asset exists`); com o netcoredbg embutido a expectativa passou a "Bundled with OpenRidr" — corrigida, não pulada.
## 2026-11-06 — Release CI: o script é o processo, o workflow só o chama

- **Um script, dois invocadores.** `scripts/release-macos.sh` (preflight, build, auditoria de Mach-O, notarização, grampo, DMG, verificação, sha256) é chamado pelo workflow e pela máquina do Helder. "Artefato idêntico ao processo manual" só é verificável se não existirem dois processos: um YAML que reimplementa os passos do `release-signing.md` divergiria na primeira mudança. `tauri-apps/tauri-action` foi recusado pelo mesmo motivo (ele faz bundle/notarização próprios).
- **A tag é a identidade da release.** O preflight exige `v<semver>` e igualdade com as cinco fontes de versão (`package.json`, `tauri.conf.json`, `src-tauri/Cargo.toml`, `crates/{host,host-http}/Cargo.toml`), mais o primeiro heading de `RELEASE-ALPHA.md`. Bump continua sendo um commit `chore(release)` antes da tag. A tag também precisa ser ancestral de `origin/main`: release de galho solto não passa.
- **Credenciais: API key do App Store Connect, não Apple ID.** `notarytool --key/--key-id/--issuer` é revogável e não expõe o Apple ID pessoal em log de runner; `--issuer` só é passado quando `APPLE_API_ISSUER` existe (Team API Key; Individual não tem issuer). Nomes: secrets `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_API_KEY` (base64 do `.p8`), `APPLE_API_KEY_ID`, `APPLE_API_ISSUER`; variable `APPLE_SIGNING_IDENTITY` (pública, já nos docs). `NOTARY_KEYCHAIN_PROFILE` continua aceito no Mac, para não quebrar o ritual local com o keychain `notary`. O certificado entra num keychain temporário com senha gerada no runner e é removido em `if: always()`.
- **Fail-fast antes de gastar runner.** Sem credenciais, o run falha em ~30 s listando os nomes ausentes; nenhum caminho publica artefato não assinado. `workflow_dispatch` com `publish=false` roda build+assinatura+notarização+verificação e para antes de publicar: é o ensaio com credenciais reais sem tag.
- **Auditoria de todos os Mach-O, com uma exceção nominal.** Medido na alpha.11: 18 Mach-O no `.app` (1 binário, 15 do runtime .NET, 2 do netcoredbg); 17 com a identidade do Helder e `dotnet/shared/Microsoft.NETCore.App/10.0.14/createdump` com `Developer ID Application: Microsoft Corporation (UBF8T346G9)`. O script exige, por arquivo, `codesign --verify --strict`, `flags=0x10000(runtime)`, `Timestamp=` e `Authority` igual à nossa — exceto `createdump`, aceito só por esse nome e essa autoridade. Comparação por `Authority` porque dylib não tem `TeamIdentifier`. Foi o risco real da alpha.9.
- **Publicação direta como pré-release.** O push da tag é o ato de aprovação; depois de verificado, `gh release create --prerelease` (sufixo `-` na versão) publica DMG + corpo gerado. Rerun da mesma tag edita a release e sobe o DMG com `--clobber` em vez de falhar.
- **O corpo é gerado, não digitado.** `release-notes.sh` concatena `RELEASE-ALPHA.md` com um bloco `## Verificação` (sha256, bytes, resultados de `codesign`/`spctl`/`stapler` do run) e **recusa** notas que já contenham um SHA-256: um hash fixo no doc só pode estar errado. O hash manual saiu do `RELEASE-ALPHA.md`.
- **O DMG não é reproduzível byte a byte.** Duas montagens do mesmo `.app` ad-hoc deram `69516096` e `69516278` bytes. O contrato "conteúdo idêntico" é sobre o caminho e a lista de verificações, não sobre o hash do arquivo; por isso o hash sempre vem do run.
- **Sem E2E no release.** `bun run check` + `bun test tests/unit` rodam no workflow; a suíte E2E tem falhas conhecidas no HEAD e não bloqueia release. Limpeza e cache: só `~/.bun/install/cache`/registry cargo, nunca `target/` nem as árvores assinadas (cache assinado é atestação emprestada).

## 2026-11-06 — Revisão do release CI: o que se publica é o commit da tag, assinado arquivo por arquivo

Revisão externa (Pi, GPT-6.1 Terra) do diff de `feat/issue-44-release-ci` contra `origin/main`;
todos os achados alto/médio foram corrigidos com teste que falha antes e passa depois.

- **`HEAD` tem de ser o commit da tag.** O workflow faz checkout da própria tag (`ref: inputs.tag ||
  github.ref`) e o preflight recusa `HEAD` diferente de `refs/tags/<tag>^{commit}`. Antes, um
  `workflow_dispatch` na ponta de um branch passaria pelo preflight (a tag está em `main`) e buildaria
  outro commit.
- **O `.p8` é escrito no shell principal.** `notary_key_path` era chamado por substituição de comando
  (`$(...)`), então `NOTARY_KEY_FILE` nascia dentro do subshell e o `trap EXIT` não removia o arquivo
  no caminho manual. Agora `prepare_notary_key` grava a variável no processo do script, cria o arquivo
  com `mktemp` sob `umask 077` (0600 desde o primeiro byte) e a remoção é garantida.
- **Auditoria de Mach-O sem falso negativo.** A detecção por magic number cobria só três variantes e
  podia confundir arquivo de dados com magic; passou a usar `file -b` (32/64 bits, duas endiannesses,
  fat32/fat64). Cada arquivo também passou a ser verificado com `codesign --verify --strict`, não só
  lido com `-dv`: assinatura inválida em binário aninhado agora aborta antes da notarização.
- **`--dry-run` é sempre ad-hoc.** O ensaio limpava o ambiente? Não: herdava `APPLE_SIGNING_IDENTITY`
  e os scripts de vendor/Tauri assinavam com Developer ID de verdade. O modo limpa a variável antes do
  build e o ensaio não pode mais assinar nada real.
- **Caminho manual sem `.p12`.** `APPLE_CERTIFICATE`/`APPLE_CERTIFICATE_PASSWORD` só são exigidos no
  caminho do CI; com `NOTARY_KEYCHAIN_PROFILE` (certificado no keychain do Mac) o preflight não pede
  os `.p12` — antes pedia e o doc mandava exportar à toa.
- **Uma release por vez, e nada de hash órfão.** `concurrency: group: release` (sem `github.ref`) para
  um dispatch e um push da mesma tag não disputarem a release; ao republicar, o DMG sobe com
  `--clobber` **antes** de editar o corpo, porque o texto anuncia um hash e nunca pode conviver com o
  anexo anterior. O caminho do keychain é registrado antes de ser criado, para o `if: always()`
  apagá-lo mesmo se o import falhar no meio.
- **Heading e hash exatos.** A versão do primeiro heading é comparada palavra por palavra (antes,
  prefixo: `0.3.0` aceitava as notas de `0.3.0-alpha.11`) e a procura de SHA-256 escrito à mão passou a
  ser case-insensitive.
- **Segunda rodada do re-check: magic bytes e chave no dono do `trap`.** `file` responde `data` para
  cabeçalho fat64, então a auditoria passou a ler os quatro primeiros bytes (thin 32/64 bits nas duas
  endiannesses, fat/fat64 e variantes trocadas) com `cafebabe` desambiguado do `.class` do Java pelo
  `nfat_arch` (< 30 é fat; 45+ é versão de classe). E `notarize` — que captura a saída do `notarytool`
  em `$(...)` — passou a chamar `prepare_notary_key` **antes** da captura: criar a chave dentro do
  subshell deixava o arquivo órfão, porque o `trap EXIT` roda no shell principal.
- **Terceira rodada (baixos do re-check): caminho da chave antes do arquivo, e recusar o que a
  varredura não sabe ler.** `prepare_notary_key` define `NOTARY_KEY_FILE` num diretório privado
  (`$RUNNER_TEMP/notary-key.<pid>`, 0700) **antes** de criar o arquivo, então o `trap` conhece o
  caminho desde o primeiro instante — o `mktemp` dentro de `$(...)` deixava uma janela em que o arquivo
  existia sem caminho registrado. Na auditoria: arquivo com menos de 8 bytes não é candidato a Mach-O,
  caminho com quebra de linha faz falhar (a varredura é por linha) e symlink apontando para fora do
  `.app` faz falhar; links internos (normais num bundle) continuam aceitos porque o alvo regular é
  auditado.
- **O bloco publicado não afirma o que não foi verificado.** O resultado de `codesign` no corpo passou
  a dizer "app e app no DMG montado" (o DMG em si é verificado por `spctl`/`stapler`, não por
  `codesign --deep`).
## 2026-11-06 — Primeira abertura com quarentena: o atraso é o diálogo do macOS, não o bundle

- **Diagnóstico anterior estava errado e foi corrigido com medição.** A leitura "15–18 s em `_dyld_start` porque o Gatekeeper verifica 18 Mach-O assinados" vinha de `sample` num processo suspenso pela quarentena: enquanto o diálogo "… é um app baixado da internet" está na tela o app **não é iniciado** (`launchInQuarantine`), então ele fica em `_dyld_start` por todo o tempo que ninguém clicar. Medido no alpha.11 notarizado/grampeado: diálogo em 0,40–0,53 s (0,17 s até a avaliação começar, 0,18 s até o XProtect terminar), código em 0,14–0,24 s depois do clique, janela em 0,97–1,42 s depois do clique. `spctl --assess` no bundle inteiro: 0,27 s.
- **O teto do que o bundle pode economizar é sub-segundo.** Todo o trabalho do sistema antes do diálogo cabe em ≤ 0,53 s (app no disco); remover o ticket grampeado (`Contents/CodeResources`, consulta online) dá 0,32–0,38 s, mesma ordem. As variantes de bundle (runtime em `tar.gz` para 1 Mach-O, runtime em `Frameworks`, sem runtime) testariam esse custo: ficam **não medidas** porque nenhum marco nessa condição passou de 0,53 s (limite combinado ~2 s), não porque tenham sido esquecidas. Nas condições que abrem de dentro da imagem o intervalo clique→código passa de 2 s: o prompt ali aparece em 1,38–1,51 s e `ditto` da mesma cópia custa 1,95 s, então leitura da imagem é a explicação compatível — o split entre cópia e arranque não foi isolado. Encolher o payload é outra frente, fora de escopo.
- **O caminho caro tem instrução, não conserto de código.** Abrir direto da janela do DMG custa 5,1–6,6 s de clique→janela, contra 0,97–1,42 s com o app copiado; copiar da imagem custa 1,95 s medidos (`ditto`) e a leitura dos 164 MB da imagem UDZO é a explicação compatível, não um split medido entre cópia e arranque. Arrastar para Aplicações antes de abrir elimina esse custo de leitura — o diálogo da primeira abertura continua existindo e continua sendo de um clique só. É isso que as notas de release e `install-app.md` passam a dizer.
- **Aprovação é por caminho.** Reabrir a mesma cópia no mesmo diretório não mostra mais o diálogo (2ª/3ª abertura do app dentro do mesmo DMG: 0,19/0,22 s até o código). Medir de novo exige cópia nova + xattr de quarentena novo com UUID novo; limpar o cache do `syspolicyd` exige root, então "primeira avaliação de um cdhash nunca visto" ficou como limitação declarada. O harness reprodutível é `scripts/measure-first-open.sh`; números e limitações em [evidence/first-open.md](evidence/first-open.md).
- **O diálogo é apresentado na frente e recebe teclado.** Capturado sobre Finder e sobre T3 Code maximizado, centralizado; com ele na tela o Return equivale a *Abrir* (app arrancou 0,25 s depois). Tela cheia em outro Space não foi medida, então "o diálogo aparece" vale para as condições testadas (terminal, Finder e janela maximizada na frente), não para qualquer arranjo de janelas.

## 2026-11-06 — Hit count medido no host; watches sobrevivem ao restart

> **Substitui parcialmente** "Breakpoints guardam condição; hit count e log message ficam fora" (2026-10-30) no que diz respeito ao hit count. Log message continua fora: o netcoredbg 3.2.0-1092 também não anuncia `supportsLogPoints`.

- **O motivo antigo foi medido, não presumido.** O netcoredbg 3.2.0-1092 não anuncia `supportsHitConditionalBreakpoints` e **ignora `hitCondition`**: num probe com `hitCondition: "3"` o debuggee parou nas 5 iterações; com `condition: "i == 3"` parou só na terceira. O evento `stopped` também não traz `hitBreakpointIds`. Ou seja, não existe caminho DAP para hit count neste adaptador; a UI deixava de ter o campo por isso.
- **A contagem passa a ser do host, com auto-continue.** Cada breakpoint guarda um alvo inteiro (só `N`, a N-ésima parada; operadores como `>=`/`%` ficam fora). Quando chega `stopped` com `reason: "breakpoint"`, o host casa o frame do topo (caminho relativo + linha) com o breakpoint armado, incrementa o contador daquela sessão e retoma sozinho até atingir o alvo; no alvo, a pausa é mostrada normalmente. Pausa manual (`reason: "pause"`), passo e exceção não contam. O contador zera a cada launch — hit count é propriedade da sessão, como no Rider.
- **Condição filtra antes da contagem.** O adaptador aplica a `condition` primeiro; o host só vê as paradas que passaram. Assim, condição + hit count no mesmo breakpoint conta apenas os hits que satisfazem a condição. Explicitar isso na UI evita a leitura "conta todo hit".
- **O auto-continue é visível e interrompível.** Enquanto conta, a fase mostra que o host está retomando por hit count (`hit count 2/3`), e um Pause manual do usuário vale: a próxima parada por `reason: "pause"` não é engolida. Sem essa fase, o app pareceria travado entre hits.
- **Watch é estado do host, não da sessão.** Expressões adicionadas continuam existindo em Restart/novo launch, como breakpoints/mute/filtros, e somem só ao fechar/trocar o workspace. Não persistem em disco (não há decisão de formato/limpeza). A avaliação passa a usar a thread/frame **selecionados** (hoje era sempre o frame do topo), reavaliada na pausa e ao trocar de thread/frame.
- **Evaluate continua só de leitura.** O netcoredbg anuncia `supportsSetExpression`/`supportsSetVariable`, mas atribuir valor não é "avaliar expressão pausado"; `setExpression` fica fora desta frente.

## 2026-11-06 — O fim sem parada é explicado pelo host, não inferido pela UI

- **Decisão (grill da #46, rodada 1, aprovada pelo Helder).** Quando uma sessão termina sem ter parado nunca e nada estava configurado para fazê-la parar, a explicação vem pronta do host: `DapView` ganha `noStopExplanation` (`string | null`), derivado no ponto único de fim de sessão a partir dos fatos da própria sessão. O React só renderiza o texto. Motivo: a regra ficaria duplicada no painel e em qualquer outro leitor do snapshot, e a decisão de 2026-11-05 ("A UI não recalcula regra") mais o precedente da frase de falha de infraestrutura do test runner já colocam explicação de resultado no host.
- **"Parou alguma vez" é fato da sessão, não geração de pausa.** Um booleano marcado no primeiro evento de parada do adaptador (breakpoint, exceção, pause, run-to-cursor) e zerado em cada lançamento; continuar depois não desfaz o fato. O contador de geração de pausa avança também em cada resume, então não responde essa pergunta.
- **Só `terminated` explica.** Sessão cancelada pelo usuário já mostra o motivo da parada e sessão falha mostra a falha do adaptador; dizer "rodou até o fim" nos dois seria falso.
- **Stop source silencia a frase; mute tem frase própria.** Breakpoints armazenados, break on thrown exceptions e run-to-cursor pendente contam como motivo de parada configurado. Com breakpoints mutados a frase é "Breakpoints are muted; …", porque afirmar que nenhum breakpoint foi setado seria mentira para quem mutou.
- **Alternativas recusadas.** (a) Só reescrever a fase da sessão: o texto continuaria atenuado e concatenado à nota genérica, que é exatamente a leitura de "falha do depurador" que a issue quer corrigir. (b) Trocar a aba para Console e rolar o output: rouba a aba escolhida pelo usuário e esconde estado no React; a frase aponta para o Console, que já é o padrão. (c) Cobrir "breakpoints setados mas nunca atingidos": é um diagnóstico diferente (breakpoint não verificado, condição falsa) e ficou fora desta rodada.
- **Consequência.** `dap-no-stop` é o testid novo do parágrafo de informação que substitui a nota atenuada de fim nesse caso; nenhum testid existente muda.
- **Revisão de código (2026-11-06, rodada do #46).** Dois ajustes ao redor da mesma decisão: a frase exige que o adaptador tenha **reportado o exit do debuggee** — `terminated` também é o estado quando o adaptador morre sozinho, e a morte dele não é conclusão do programa; e "nenhum breakpoint" olha as **linhas** armazenadas, não o mapa de fontes, porque remover o último breakpoint deixa uma lista vazia para aquele arquivo (o mesmo que nenhum breakpoint). O campo "parou alguma vez" continua separado do exit do debuggee: parar e continuar não apaga o fato, e um adaptador que morre depois de uma parada não merece explicação de conclusão.

## 2026-11-06 — Diff do Git mostra o arquivo inteiro, unificado opcional, e o hash é o do commit

- **Decisão (grill da #48, rodada 1, aprovada pelo Helder/orquestrador).** O painel de diff deixa de renderizar só os hunks: o host passa a devolver o texto completo dos dois lados e o painel mostra o arquivo inteiro, com conectores entre as regiões alteradas e um modo unificado opcional. O hash exibido passa a ser o do commit `HEAD`, não o blob extraído de `index abc..def`.
- **`git.diff` fica intacto; entra `git.fileDiff`.** O gutter do editor chama `git.diff` a cada save e não pode baixar o arquivo inteiro; o painel chama o método novo, que devolve `base`/`current` completos, `hunks` (só cabeçalhos de `git diff -U0`), o patch `-U3` como fallback e o `commit` `HEAD`.
- **Base do diff não muda.** Unstaged continua `Index → Worktree` e staged `HEAD → Index`; por isso o hash do commit é uma linha de contexto (`HEAD <short> · <assunto>`) e o rótulo esquerdo do unstaged continua `Index`. Rotular o lado esquerdo de unstaged como `HEAD` seria falso.
- **Alinhamento é do Git, não do React.** Não entra algoritmo de diff em TypeScript nem dependência nova (o repo não tem `@codemirror/merge`): o host manda os hunk headers do `-U0` e o React monta as linhas do arquivo inteiro a partir dos dois textos. Para arquivo adicionado/deletado os hunks são desnecessários: todas as linhas são `added`/`removed`.
- **Limites e fallback no host.** Arquivo completo por lado só até 1 MiB ou 30.000 linhas; binário (byte `0x00` nos primeiros 8 KB) e codificação que `fsutil::decode_text` não decodifica também derrubam `full`. Nesse caso o painel mantém a visão de hunks de hoje e a nota diz o motivo — nunca uma visão de arquivo inteiro inventada.
- **Conector é gutter, não SVG.** O painel esquerdo ganha um gutter `after` com um marcador por linha alterada (metade vermelha para removida, metade verde para adicionada, bloco azul para modificada). O alinhamento é o da própria linha do editor; nada de medir pixel nem sincronizar uma terceira coluna.
- **Modo unificado e persistência.** Um toggle `Split | Unified` na toolbar (default split) com `aria-pressed`; a escolha persiste em `localStorage` na chave `openridr.commitDiffMode.v1`, no mesmo padrão defensivo de `openridr.layout.v1`. A visão unificada usa o mesmo conjunto de linhas (projeção `-`/`+`/espaço), a mesma contagem e a mesma navegação; o gutter mostra a numeração nova.
- **Alternativas recusadas.** (a) Inchar `git.diff`: faria o gutter pagar o arquivo inteiro a cada save. (b) Baixar `@codemirror/merge` para diffs no React: dependência nova sem decisão, e o Git já entrega os blocos. (c) Rotular o lado esquerdo unstaged de `HEAD · <hash>`: mentira sobre a base. (d) Conectores em SVG por pixel: custo alto e frágil no scroll.
- **Consequência.** Testids novos (`commit-diff-mode`, `commit-diff-commit`, `commit-diff-note`, `commit-diff-unified`) e nenhum renomeado; abrir diff pelo Git log fica para issue própria "Related to #48".

## 2026-11-06 — Diff do Git: correções da revisão independente (rodada 1)

- **Confinamento de symlink antes de ler ou diffar.** `validate_path` é lexical; um diretório symlink levaria a leitura do host e o lado worktree do `git diff` (inclusive o `--no-index` de untracked) para fora do workspace. A leitura resolve o diretório real e o exige dentro da raiz do repositório antes de qualquer byte; arquivo symlink é tratado como objeto link (o alvo em texto), nunca seguido. A janela TOCTOU residual entre a verificação e o uso fica declarada como limite conhecido; não foi demonstrada nem fechada nesta rodada, e não se afirma que só um sandbox a resolveria.
- **Captura limitada na origem.** `spawn_bounded` mata o filho ao bater o teto e devolve `truncated`; stderr é drenado em thread própria com cópia limitada (sem deadlock nem crescimento). Conteúdo de blob usa `cat-file -s` e só lê se couber em 1 MiB; worktree lê 1 MiB + 1 byte e recusa; patch para em 4× o limite de caracteres (960 KB) e mantém o contrato de 240.000 caracteres do `git.diff` legado. Nada lê o arquivo inteiro para depois recusá-lo.
- **Ausência comprovada, erro real separado.** `rev-parse --verify --quiet` distingue ausência (exit 1, adicionado/deletado/repo sem commits) de erro de leitura; o conteúdo só é lido depois de `cat-file -s`. Erro real vira `unreadable`/`failed`, nunca adição/deleção inventada; patch vazio com lado ilegível vira `failed` ("could not be read"), não "no diff".
- **Contexto divergente derruba a montagem.** `buildCommitRows` compara os pares de contexto (antes, entre e depois dos hunks) e devolve `null` quando divergem; o painel cai no patch com a razão `unaligned` em vez de mostrar uma linha que esconde a diferença.
- **UTF-16 tem ranges do próprio Git sobre o texto decodificado.** Mudança em UTF-16 é binária para o Git e vem sem hunks; o host grava os lados decodificados em temporários privados (0600, nome único, limpeza por `Drop`) e pede `-U0` ao `git diff --no-index`, devolvendo só os inteiros dos ranges. Sem ranges, `full` cai com `unaligned` — a spec não foi enfraquecida.
- **Alternativas recusadas.** (a) Recusar arquivo UTF-16: contraria a spec e o editor, que mostra o texto. (b) Algoritmo de diff em TypeScript: dependência/risco novo sem necessidade, e o Git já resolve. (c) Contar com o Git para não seguir symlink: não se sustenta — `--no-index` lê conteúdo externo atrás de diretório symlink (provado em teste). (d) Recusar todo symlink: um link versionado é objeto legítimo do Git, com o alvo como conteúdo.

## 2026-11-06 — Solution Explorer: status Git por arquivo e marcador de symlink (#49)

Spec: `docs/SPEC-SOLUTION-GIT-STATUS.md`; tickets #107 (symlink), #108 (status), #109 (ignorado), #110 (verificação). Grill aprovado pelo Helder/orquestrador na rodada 1.

- **Só linhas de arquivo.** Fontes avaliadas e arquivos listados no modo "Show all files" recebem status; pasta e projeto não agregam o status dos filhos nesta rodada, e o nó `Imports` citado no problema da issue fica fora (o aceite só pede status e symlink). `no index` continua sendo a heurística existente ("não é fonte avaliada"/dotfile) e não muda de significado.
- **Fonte do status é o snapshot que já existe.** A árvore consome o `git.status` do `WorkspaceView` por prop; não nasce uma chamada por arquivo nem um segundo fluxo de confiança. Estados derivados do par `index`/`worktree` com precedência `conflicted` > `untracked` > `added` > `modified`; o rótulo granular do painel Commit (`Staged + modified`, `Renamed`) fica no `title`/descrição, sem cor própria. Arquivo deletado não é estado da árvore (não existe no disco).
- **Acessibilidade não depende de cor.** A linha ganha a tag com a palavra do estado (`· modified`, `· added`, `· untracked`, `· ignored`, `· conflicted`), o atributo `data-git-status` e uma descrição AX (`Git status: Modified`) separada do nome acessível; a cor é token novo (`--git-*`) aplicado só ao nome/tag. Os valores medidos ficam em `docs/evidence/solution-git-status.md`: ≥ 4.5:1 sobre painel, hover, linha selecionada, linha focada e a linha `no index`. O fundo âmbar de `no index`, a seleção e a densidade da linha não mudam.
- **Raiz do repo ≠ raiz do workspace.** O caminho do `git.status` (relativo à raiz do repositório) é mapeado para o caminho do workspace por função pura; mudança fora do workspace é descartada. O gutter do editor e `git.diff` continuam com o comportamento anterior — limitação registrada, fora do escopo.
- **Refresh no foco, com guarda.** Os gatilhos de commit/checkout/branch/pull/push já chamavam o refresh; entra o evento de foco da janela com debounce, uma chamada em voo por vez e descarte de resposta velha quando workspace ou trust mudou. Sem watcher e sem polling.
- **Ignorado é `.gitignore` de verdade, distinto de `no index`.** `git.ignored { paths }` é trust-gated, recebe no máximo 256 caminhos visíveis e devolve `{ state, message, ignored }`. A fonte é `git status --porcelain=v1 -z --ignored=matching -- <paths>` com `--literal-pathspecs` (argv fixo, sem shell/stdin, teto de entrada e de saída): ele conhece o index, então um arquivo rastreado nunca aparece como ignorado, e um diretório ignorado responde pelos arquivos abaixo dele. `check-ignore` foi descartado porque a saída segura para nomes com newline (`-z`) exige `--stdin`, e sem `-z` o argv não separa nomes de forma inequívoca; o default do `check-ignore` já exclui rastreados (é o `--no-index` que os inclui), então a escolha é sobre forma de I/O, não sobre correção do filtro.
- **Symlink é marcado sem seguir o alvo.** `workspace.tree` devolve `symlink` lido do próprio caminho e `dir` descreve o objeto (`DirEntry::file_type`), não o alvo: um link é `dir: false` mesmo apontando para pasta, então nem o glifo custa um stat externo em Restricted; `workspace.symlinks { paths }` (read-only, sem trust, lote de 256) cobre as fontes avaliadas, que não passam pela listagem. O confinamento é do diretório real que contém o link, nunca do alvo; link pendente, ciclo e link para fora são respondidos como link e nada atrás é lido. A árvore nunca expande pasta symlink (sem chevron), e `workspace.tree` recusa listar um caminho que passa por link (`invalid_state` para link que resolve dentro/ciclo; `outside_workspace` para link que escapa, mantendo o contrato de confinamento de 2026-11-04).
- **Trust.** Status e ignorado só em Trusted; o marcador de symlink é metadado read-only e aparece também em Restricted, com o mesmo confinamento.
- **Limites declarados.** Pasta symlink não é navegável pela árvore (os fontes abaixo dela não aparecem); mudança externa só aparece no foco ou no Refresh do painel Git; `ignored` só é consultado para caminhos visíveis e é omitido durante um filtro da árvore; Windows e Intel não executados.

## 2026-11-06 — Aceitação de Git usa remoto bare local; fetch e pull são coisas diferentes

- **Decisão (grill da #50, rodada 1, aprovada pelo Helder/orquestrador).** A aceitação de Git roda contra um **remote de teste local**: um repositório bare no diretório temporário com dois clones descartáveis — o clone que o app abre (`user`, raiz do repositório == raiz do workspace) e a contraparte (`peer`, dirigida por shell). Sem rede, sem login, sem `git config --global`. Motivo: exercita Git real e o efeito no remoto sem publicar código nem depender de credencial; autenticação e transporte de rede ficam declaradamente não verificados.
- **O fixture é script, não diretório montado à mão — e é não destrutivo.** `scripts/git-remote-fixture.sh` imprime caminhos e baseline; sem argumento cria um state novo com `mktemp` sob a raiz temporária real, e um argumento só é aceito se for caminho absoluto novo e contido nessa raiz. Caminho existente, `/`, home, o checkout, relativo/`..` ou symlink para fora são recusados sem apagar nada; recomeçar é rodar de novo, nunca reaproveitar/limpar o state anterior. A raiz temporária (ou o pai real de um state custom) que resolva para dentro do checkout é recusada **antes** de criar qualquer coisa, e a validação do override do launcher precede qualquer rotação de state.
- **O lançador de verificação aceita workspace preparado.** `desktop.sh up` ganha `VERIFY_OPENRIDR_DESKTOP_SOLUTION`: caminho absoluto de `.sln`/`.csproj` que substitui a cópia padrão de `fixtures/OrderDesk`, mantendo `OPENRIDR_HOME` isolado e as checagens de instância única e de PID. O default continua a fixture.
- **O contador `↓` é o de refs de tracking locais.** Ele reflete a contraparte depois de um fetch ou de um push próprio, não sozinho; o pull faz fetch e por isso incorpora mesmo com o contador velho. Por isso a evidência separa fetch manual (precondição) de pull (ação da UI), e a falta de uma ação de fetch vira ticket próprio em vez de "refresh obscuro".
- **Estado criado fora do app precisa ser declarado.** Quando a UI não consegue produzir um estado (merge divergente, conflito), o estado é criado por CLI no clone descartável e a evidência diz isso; a UI é observada só no estado, nunca como autora.
- **Consequência.** `docs/evidence/git-remote-acceptance.md` passa a ser o formato de referência da aceitação de Git (quadro por cenário com ação, prova fora da UI e status, incluindo "falhou" e "parcial"); o `⌥⌘W` que encerra o app e o pull divergente viraram issues, não correções desta frente.

## 2026-11-06 — CI em PR: dois PRs, check obrigatório só depois do primeiro run verde, MSRV = toolchain

Decisões do grill da issue #56 (respondidas pelo Helder via orquestrador: 1b, 2a, 3a, 4b, 5a).

- **Dois PRs, não um.** O `cargo fmt` é big bang (596 hunks em 33 arquivos) e o clippy tem ~40 avisos reais: os dois vão num PR mecânico (`ci/issue-56-fmt`), sem nenhuma edição manual, e o PR de CI fica em cima dele. Formatação misturada com workflow esconde a revisão do workflow e é irrevisável de qualquer jeito.
- **Checks obrigatórios só depois do primeiro run verde.** O workflow nasce com `workflow_dispatch` para poder ser exercitado a partir do branch; o ruleset fica pronto em `.github/rulesets/main.json` + `docs/ci-main-ruleset.md` e **não é aplicado** — settings do GitHub são decisão do Helder. Um teste do host que só falhe em runner limpo é quarentenado com motivo + issue filha, em vez de deixar a `main` travada.
- **O check obrigatório tem nome estável.** `ci` é um agregador (`needs` dos quatro jobs) e `security` é próprio; renomear job interno não mexe no ruleset. `push` em `main` não roda `governance` (não há título/corpo de PR) e o agregador trata `skipped` como aceitável, não como falha.
- **Toolchain pinada, MSRV honesto.** `rust-toolchain.toml` em `1.99.0` (a versão em que o repo compila) e `rust-version = "1.99"` nos três crates. MSRV = a toolchain pinada; baixar exige medição, não é promessa. O CI instala a toolchain via `rustup` (o runner já tem rustup), sem ação de toolchain a pinar nem uma segunda cópia do número da versão.
- **Segurança em duas camadas, uma agora.** `security` é determinístico: gitleaks no histórico do PR. O "security review por PR que bloqueia achado alto" precisa de revisão por LLM com credencial → issue filha bloqueada por decisão de credencial/custo do Helder; até lá o bloco `## Security review` do template é onde o autor responde pelo diff.
- **Smoke E2E noturno por lista.** A lista de specs vive em `scripts/e2e-smoke.txt`, não no YAML: a quarentena da #67 é quem decide, e mudar a lista não mexe em workflow. Lista vazia ou nome sem arquivo falham com mensagem, em vez de rodar a suíte inteira por acidente.
- **Gate de corpo é script, com self-check.** O template só preenche; o que reprova PR sem issue ou sem Evidência é `scripts/ci-pr-body.sh`, exercitado por `scripts/ci-pr-body-selfcheck.sh` (13 casos, incluindo isenção de Dependabot/Renovate e o template real como guarda anti-drift) no próprio job `governance`.
- **`zizmor` destravou dois achados reais no release.** O `zizmor` reprovava o workflow de release antes de existir CI de PR: credencial persistida no `.git/config` (`persist-credentials: false` agora) e cache de bun num workflow que assina e publica (`no-cache: true`). Os 8 avisos `template-injection` informativos (expansão de `steps.*.outputs.*` do nosso próprio `release-macos.sh` dentro de `run:`) ficam fora com `min-severity: low`; reescrever o caminho de release sem poder rodá-lo não se pagava nesta frente.

## 2026-11-06 — Risco de PR por caminho tocado: gate dentro do `ci`, aprovação do mantenedor é review, auto-merge só no baixo

Decisões do grill da issue #74 (respondidas pelo Helder via orquestrador: 1a, 2a, 3a+3b, 4a, 5 sim, 6a, 7 sim).

- **O tier é calculado pelos caminhos, e é entrada única do gate.** Um script do repo lê os caminhos alterados do PR (`git diff --name-only` base...head) e imprime `low`, `medium` ou `high`: baixo = `docs/**`, `*.md` (exceto `AGENTS.md`), `tests/**`, `crates/*/tests/**` e `*.css`; médio = `src/**`, `crates/**`, `src-tauri/**`, `fixtures/**`, `rust-toolchain.toml` e **todo caminho não casado por nenhuma regra** (default conservador); alto = `.github/workflows/**`, `.github/rulesets/**`, `AGENTS.md`, `scripts/release*`, `scripts/vendor-*`, `scripts/ci-*`, `src-tauri/resources/*.entitlements`, `src-tauri/tauri.conf.json` (assinatura e updater), `crates/host/src/trusted.rs` e mudança de dependência. Misto vence o mais alto. O label `risk:*` é só saída: editar o label não baixa o tier.
- **Mudança de dependência é diff de tabela, não arquivo tocado.** `Cargo.lock` e `bun.lock` tocados = alto (eles só se movem quando dependência ou versão muda). Em `package.json` e nos `Cargo.toml` (raiz e `src-tauri`) o script compara **as tabelas de dependência do arquivo na base e no head** (ordenadas, via `jq`/`awk`), então bumpar `version`, mexer em `scripts` ou em `[package]` continua médio, e acrescentar/trocar/remover dependência é alto. Consequência aceita de propósito: **PR do Dependabot cai em alto** e passa pelo mantenedor.
- **O gate é um job do próprio `ci.yml`, no `needs` do agregador `ci`.** Assim o ruleset da `main` não muda — é o que `docs/ci-main-ruleset.md` já previa ("o job dela entra no needs de `ci`"). Fail-closed: erro ao calcular o tier vira `high` e o gate falha; job pulado nunca é verde num PR (a lista de jobs PR-only do agregador passa a ser dado, não um `if` por nome).
- **"Aprovação do mantenedor" é review `APPROVED` de OWNER/MEMBER/COLLABORATOR depois do último commit.** Dado que já vem no payload de reviews, sem chamada extra, e sem depender de alguém lembrar de rotular. Aprovação de um head antigo não vale — espelha o `dismiss_stale_reviews_on_push` do ruleset. Depois do APPROVED, o run precisa ser re-disparado (`gh run rerun --failed`): a alternativa (workflow separado reagindo a `pull_request_review`) exigiria um terceiro check obrigatório no ruleset, ou seja, mexer em settings — recusada.
- **Auto-merge é armado pelo job `risk` no tier baixo, com o setting como pendência do Helder.** `allow_auto_merge` está desligado no repo hoje; o comando (`gh api --method PATCH repos/Heldinhow/openridr -F allow_auto_merge=true`) fica documentado em `docs/ci-main-ruleset.md` e não é aplicado pela sessão. Com o setting desligado o job avisa (`::warning::` + job summary) em vez de reprovar: um PR de risco baixo não fica vermelho por causa de um setting que ainda não foi ligado. A regra da sessão (armar auto-merge quando o tier é baixo) entra no `AGENTS.md`.
- **O job cria os labels `risk:*` sozinho, idempotente.** Cores fixas (`low` verde, `medium` âmbar, `high` vermelho) e `--force`, então não há passo manual de setup. Falha ao rotular é aviso, não reprovação: o tier calculado é que manda.
- **Teto de três rodadas de revisão e achado resolvido por escrito.** Depois de três rodadas de review/correção sem verde, a sessão para e repensa a abordagem em vez de emendar a quarta; todo achado é corrigido ou respondido com o motivo escrito. Vira regra do `AGENTS.md`, junto com o fluxo de aprovação do risco alto.
- **Revert rate é script + doc, não dashboard.** `scripts/revert-rate.sh` consulta o GitHub (PRs mergeados e commits `revert:` no mês) e imprime a linha; `docs/evidence/revert-rate.md` guarda uma linha por mês. Roda quando o mantenedor quiser (precisa de token), com a lógica de classificação testada por JSON de fixture, sem rede.
- **Alternativas recusadas.** (a) CODEOWNERS + "require review from code owners": já descartado no #56 (um mantenedor, zero forks) e continuaria sem exigir aprovação em caminho de risco médio. (b) Check obrigatório separado para o risco: re-roda mais barato, mas exige editar e reaplicar o ruleset (setting). (c) Label aplicado à mão como sinal de aprovação: some o registro de quem aprovou e quando, e a aprovação de head antigo passaria. (d) Ruleset com aprovação obrigatória global: travaria PR de risco baixo, que é exatamente o que o issue quer destravar. (e) Bloquear por `paths` no próprio ruleset: regra de settings e sem tier misto.
## 2026-11-06 — Proxy corporativo vem do ambiente; a guarda SSRF fica no host de destino (#51)

Decisões do grill da issue #51 (respondidas pelo orquestrador: Q1–Q8 como recomendado). Substitui o ponto que o #38 deixou aberto de propósito (`try_proxy_from_env(false)` em `crates/host/src/http.rs`).

- **Configuração só por ambiente, agora.** `HTTPS_PROXY`/`HTTP_PROXY`/`ALL_PROXY` e as versões minúsculas (minúscula primeiro, como o `curl`), com `NO_PROXY`/`no_proxy`. Campo na tela e proxy do *System Settings* do macOS ficam fora desta rodada: o segundo virou a issue #132 (o app aberto pelo Finder/Dock nunca herda o ambiente do shell, então é lá que mora o caso real).
- **Proxy do operador é isento da regra de endereço privado; o destino não é.** O `ureq` entrega ao resolver o netloc do **proxy**, então a guarda antiga (`SafeResolver`) ou recusava um proxy em `localhost`/rede privada — que é onde proxy corporativo vive — ou, aceitando-o, deixava o destino sem validação nenhuma. A guarda passou a rodar onde ela ainda tem informação: o destino é validado (classificação literal + DNS) **antes** de qualquer conexão quando há proxy, e o resolver continua recusando endereço privado quando o que ele resolve é o destino (conexão direta, garantia forte de antes).
- **Diferença de garantia aceita e declarada.** Com proxy o DNS do destino é resolvido duas vezes (host, para validar; proxy, para conectar). A promessa passa a ser "nome que resolve para privado é recusado", não "a conexão vai ao IP validado". Sem proxy nada muda.
- **Falha fechada em proxy inutilizável.** Esquema que o cliente não suporta (`socks4/5`, `https://`), valor malformado ou proxy IPv6 fazem a requisição falhar com mensagem que nomeia a variável e o esquema — nunca conexão direta: cair direto fura a política da rede que o proxy existe para aplicar.
- **Sem dependência nova e sem UI nova.** `std` + `ureq` 2.12 cobrem proxy HTTP; o teste usa um proxy local em `TcpListener`. Nenhum `data-testid` novo.
- **Seam de instalador controlado preservado.** `OPENRIDR_ENABLE_PROBE=1` em build de debug continua liberando destino loopback na guarda (é o que a suíte do netcoredbg usa); release compila a exceção para fora. A política de produto do NuGet (`validate_source_arg`: só HTTPS, host literal privado recusado) não depende da flag.
- **Filhos herdam o ambiente.** `dotnet restore/build/test` e as ferramentas seguem sem filtro de proxy (é o que o NuGet já usa); um teste falha se alguém passar a setar ou remover essas chaves no comando.
## 2026-11-06 — Update reconcilia branch divergente com merge declarado; conflito aborta

- **Decisão (grill da #112, rodada 1: Q1a, Q2a, Q3, Q4).** O **Update** faz `git fetch --quiet` e depois `git merge --ff --no-edit @{upstream}`: quando a branch e o upstream andaram, o resultado é um merge commit; fast-forward continua fast-forward. As flags são explícitas de propósito — `pull.rebase`/`pull.ff`/`merge.ff` do usuário não mudam a política, e o diálogo declara o override. Rebase fica fora desta frente: reescreve commits locais e deixa um estado de rebase que a UI não conclui nem aborta (#114); a escolha de política (merge/rebase) vira ticket próprio depois da #114.
- **Fetch separado do merge para provar a posse do abort (achado alto da revisão).** Um `git pull` opaco não diz se o merge que ficou em andamento foi ele que começou; abortar por "existe `MERGE_HEAD`" poderia matar um merge de outro processo. Por isso o host busca primeiro (só refs), resolve o commit do upstream e guarda o hash, e só aborta quando `MERGE_HEAD` é exatamente esse commit. Merge em andamento de outro commit é reportado e deixado intacto.
- **Conflito aborta, e isso é provisório.** Se o merge conflitar, o host roda `git merge --abort` e devolve erro em palavras do app com os caminhos dos dois lados; branch, commits e árvore voltam ao estado anterior (o fetch já moveu as refs de tracking). Deixar o merge aberto para resolução no editor é o que o Rider faz e o destino quando a #114 entregar concluir/abortar; até lá, um merge aberto seria o mesmo beco sem saída do bug #112, um passo depois.
- **Três classes de falha param de vazar conselho de configuração.** Conflito, non-fast-forward e identidade ausente viram mensagem do app; o stderr cru continua em `Operation.output` para o que o app não prever. Motivo: o "Expected" da issue proíbe despejar o hint do Git sobre editar `pull.rebase`.
- **Detecção pelo estado, não pelo contador.** A decisão de abortar olha o status depois da falha (marcador de merge em andamento e arquivos conflitados), nunca `ahead`/`behind` da tela — os contadores só valem até o último fetch (#113). O diálogo acrescenta `Ahead` e uma frase de política, sem afirmar "divergiu" com contador possivelmente velho.
- **Consequência.** `docs/SPEC-GIT-UPDATE-RECONCILE.md` e evidência em `docs/evidence/git-update-reconcile.md`; tickets #127 (host) e #128 (UI + evidência); a escolha de política (rebase) e concluir/abortar merge (#114) continuam abertas de propósito.

## 2026-11-06 — Menu nativo não pode possuir atalho do keymap (issue #111)

- **Decisão (grill da #111, rodada 1, aprovada pelo Helder/orquestrador).** O menu nativo do macOS deixa de possuir `⌘W` e `⌥⌘W`; o keymap continua dono dos dois (`editor.closeTab` e `git.update` nos três perfis) e o webview volta a receber as teclas. O item `Close Window` do menu File passa a ser item próprio, **sem acelerador**, com id tratado no shell, e a duplicata do menu Window sai. `⌘Q` continua nativo e continua pedindo decisão à UI antes de sair.
- **Medição que abriu a frente.** O macOS resolve key equivalent de menu **antes** de entregar a tecla ao webview. No app instalado, `File > Close Window` é ⌘W e `File > Close All` é ⌥⌘W; pressionar `⌥⌘W` (atalho anunciado de Update) e pressionar `⌘W` (atalho anunciado de Close Tab) **derrubaram o PID**, sem diálogo. As duas colisões são as únicas da tabela de defaults, medidas nos perfis `rider-macos`, `intellij` e `visual-studio`. A do `⌘W` existia desde que o atalho foi documentado (a linha do `evidence/e2e-alpha5/REPORT.md` dizia "listado, não executado").
- **`Close All` não é nosso: o AppKit o injeta.** Num spike AppKit descartável (fora do repo), um menu File com item `performClose:` + ⌘W ganha `Close All` com ⌥⌘W; com o mesmo item **sem** acelerador, o injetado nasce sem tecla; sem nenhum item `performClose:`, ele não aparece. O acelerador do injetado é derivado do item original, então tirar o acelerador do nosso item resolve as duas pontas sem tentar apagar o item injetado (que reage a mudanças do menu).
- **Fechar janela não perde o prompt.** O item próprio chama `close()`, que emite `CloseRequested` — o mesmo caminho do botão vermelho, que a UI já intercepta para perguntar sobre buffer sujo. `destroy()` seria errado: não emite evento e fecharia sem perguntar. O atalho de fechar janela (⌘W) deixa de existir, como num IDE: quem fecha é o menu File, o botão vermelho ou `⌘Q`.
- **Guarda do lado TS.** `src/actions/reservedShortcuts.ts` versiona os aceleradores que o menu nativo possui (comentados contra `src-tauri/src/main.rs`) e `tests/unit/nativeMenuShortcuts.test.ts` reprova qualquer default de keymap que colida com a lista, nomeando ação, perfil e atalho. É armadilha de regressão, não prova do menu: o shell não exporta a tabela e o AppKit não existe em teste de unidade. E2E nunca pegaria isso — roda em Chrome, sem menu nativo.
- **Alternativas recusadas.** (a) Remapear o keymap (⌘W/⌥⌘W para outras teclas): quebra a paridade Rider e o que a Settings anuncia. (b) Manter ⌘W nativo e mudar só o Update: deixa `Close Tab` morto e não fecha a classe. (c) Apagar o `Close All` injetado depois do launch: funciona no spike, mas depende de adivinhar quando o AppKit reinjeta. (d) Fazer o item nativo reencaminhar a tecla ao webview: exigiria manter dois donos do mesmo atalho e a Settings deixaria de ser a fonte única.
- **Consequência.** `bun run check`/hook de pre-commit **não** compilam `src-tauri` (crate fora do workspace Cargo); a checagem do shell é `cd src-tauri && cargo check`, e a prova de comportamento é o app real. Windows e Linux continuam com `Menu::default` e podem ter a mesma sombra: não verificado, não suportado.
- **Correções da revisão independente (rodada 1, #111).** Dois achados médios, ambos reais. (a) Ligar a tecla física com Option fez o default `tests.quickList` (⌥⇧U) **roubar digitação**: ⌥⇧U é tecla morta/composta no macOS, e o handler do editor daria `preventDefault` nela. Agora, com foco no CodeMirror, um atalho **só de Option** (sem ⌘/⌃) que compõe outro caractere cede à digitação (`typesACharacter`) — dentro do editor o caractere entra, fora dele a ação roda; continua valendo o conselho de dar ⌘/⌃ a qualquer atalho com Option. (b) Override salvo **antes** desta correção guardava o caractere composto (`∑`), e o matcher novo só olhava a tecla física — o binding do usuário pararia de funcionar. `matches` passou a aceitar as duas formas (caractere composto ou tecla física), então nenhum `settings.json` precisa migrar. Testes: +2 em `tests/unit/keymapOptionKeys.test.ts`; prova no app em `docs/evidence/native-menu-keymap.md` (S7, S8).
- **Complemento (mesma frente, ticket #129) — Option compõe caractere.** Com o menu corrigido, `⌘W` passou a fechar aba e `⌥⌘W` seguiu sem efeito: no macOS, `⌥W` entrega `event.key === "∑"` e o matcher comparava esse caractere com a tecla guardada (`"w"`), então **todo** atalho ⌥+letra/dígito estava morto e a captura da Settings gravava `∑`. Agora, com `alt`, a tecla vem do `code` do evento (`KeyW` → `w`); sem `alt`, ou para teclas que Option não compõe (F5, Enter, setas), continua vindo do `key`. Limite declarado: em layout não-US o atalho com Option é nomeado pela posição física, o mesmo nome na captura e no matcher.

## 2026-11-06 — Risco de PR: correções da revisão independente (rodada 1)

Revisão (Pi, `openai-codex/gpt-6.1-terra`, `thinking=medium`) sobre `7a352d5`: três achados altos, quatro médios e dois baixos, todos reais.

- **O gate executava código do próprio PR.** `actions/checkout` num `pull_request` traz o merge commit, então `scripts/ci-risk-tier.sh` e `ci-risk-gate.sh` vinham do PR: um PR que mexesse neles se classificava como baixo ou se aprovava. Agora os dois são extraídos de `origin/<base>` (`git show`) e executados de lá, com o workspace como repositório git; o agregador `ci` faz o mesmo com `ci-aggregate.sh`. Quando a base ainda não tem o arquivo — o PR que o introduz — a cópia do PR roda e o job emite `::warning::` e uma linha "Bootstrap" no resumo. Não é buraco silencioso: é a única janela em que o código é novo, e ela é `risk:high` por construção.
- **A aprovação passou a ser ancorada no commit, não no relógio.** `committer.date` é controlável por quem escreve o commit: um commit novo com data antiga herdava a aprovação de uma revisão que ninguém viu. A regra agora é `commit_id` da review **igual** ao head do PR. Isso também apaga a chamada à API de commits (uma chamada a menos) e o problema de comparar timestamps com offsets diferentes.
- **O estado mais recente de cada revisor é o que conta.** Um `APPROVED` antigo sobrevivia a um `CHANGES_REQUESTED` posterior. Agora a redução é por revisor, pelo `submitted_at` mais recente; aprovação retirada (mudança de ideia ou `DISMISSED` de push) não ressuscita.
- **Manifesto comparado no merge base, não na ponta da base.** Se a base e o PR acrescentassem a mesma dependência, os dois lados concordavam e a mudança virava "sem mudança de dependência" (médio). O `git show` do manifesto agora usa o mesmo merge base do diff de caminhos.
- **A extração de tabela de dependência do TOML virou conservadora.** `[target.'cfg(...)'.dependencies.foo]` era ignorado (adicionar dependência ali dava médio); agora qualquer tabela cujo nome mencione `dependencies` conta, mais `patch`/`replace`. E uma linha que começa como cabeçalho mas não é (valor multilinha) faz o script **falhar fechado** para alto, em vez de descartar o resto da tabela e dizer "sem mudança".
- **`--paginate` do `gh api` imprime um array por página.** O script lia só `$reverts[0]`: um mês com mais de 100 commits via só a primeira página. A saída passa por `jq -s 'add // []'`, e o mesmo tratamento normaliza os fixtures (um teste com dois arrays concatenados prova as duas páginas).
- **A busca de PRs mergeados ganhou limite superior e falha explícita no teto.** Antes era `merged:>=YYYY-MM-01` com teto de 500: com histórico grande, PRs mais novos empurravam o mês para fora. Agora o intervalo fecha no último dia real do mês (com bissexto) e bater o teto é erro, não truncamento silencioso.
- **Falha de pipeline mascarada.** `gh api ... | sort | tail -1` devolvia o status do `tail`: um erro de API passava como "lista vazia". A saída é capturada e checada antes de ordenar.
- **Falha do auto-merge não afirma mais a causa.** O aviso dizia "o repositório precisa de `allow_auto_merge`" para qualquer erro, inclusive token read-only. Agora a saída real do `gh` vai para o `::warning::` e para o resumo, com a sugestão do setting como hipótese, não como diagnóstico.
- **Mês posicional.** O ticket escreveu `revert-rate.sh [mês]`; o script só aceitava `--month`. As duas formas valem.
- **O que continua aberto, e é decisão do Helder.** Com `pull_request`, o YAML que roda é o do merge commit: um PR que apague o passo do gate do próprio `ci.yml` ainda pode ficar verde. Tirar a decisão das mãos do PR (feito aqui) e deixar a brecha residual num diff de YAML é o limite desta frente; fechar de vez pede `pull_request_target` para o gate e um terceiro check obrigatório no ruleset — settings, portanto issue filha.

## 2026-11-06 — `allow_auto_merge` ligado: o tier baixo passa a entrar sozinho

- **Fato novo, decisão do Helder.** O setting `allow_auto_merge` do repo foi ligado (`gh api repos/Heldinhow/openridr --jq .allow_auto_merge` → `true`). A decisão de 2026-11-06 sobre o gate de risco dizia que o job avisaria enquanto o setting estivesse desligado; agora ele arma o merge de verdade no tier baixo.
- **O job não muda de contrato.** `gh pr merge --auto --squash` continua sendo `::warning::` quando falha, nunca reprovação: o setting é configuração do repo, e um PR de documentação não deve ficar vermelho por causa dela.
- **O que passa a depender do setting é o resultado, não o check.** Com ele ligado, `risk:low` entra em squash quando `ci` e `security` ficam verdes; desligado, o mesmo PR espera alguém. O check `ci` é verde nos dois casos.
- **Estado documentado em um lugar só:** `docs/ci-main-ruleset.md` (tabela de settings) e `AGENTS.md` (o que a sessão espera). Nenhum PR `risk:low` passou por esse caminho ainda — está registrado como não verificado em `docs/evidence/risk-tiers.md`.

## 2026-11-06 — Risco de PR: correções da rodada 2 (o bootstrap não executa código do PR)

Re-check (Pi, `openai-codex/gpt-6.1-terra`, `thinking=medium`) sobre as correções da rodada 1: dois altos e três médios, todos reais.

- **O bootstrap era uma porta dos fundos.** Quando a base não tinha os scripts, o job rodava a cópia do PR — com um `::warning::`, que não é gate. Agora, sem script na base, o job usa uma **política fixa escrita no próprio YAML**: o tier é forçado a `high` (o classificador do PR nunca roda) e a aprovação é conferida por um filtro `jq` inline que implementa a mesma regra do script. O agregador tem o mesmo tratamento: sem script na base, roda a regra inline equivalente (needs vazio, job PR-only presente, job não-`success`). Nenhum caminho executa código do PR; o que sobra é o YAML, que é a brecha já registrada na #139.
- **Cabeçalho de tabela TOML pode ser indentado.** A gramática permite espaço antes de `[tabela]`; o extrator só reconhecia coluna zero, então bastava indentar `[dependencies]` e mudar uma entrada para a mudança virar "sem mudança de dependência" (médio). Agora o reconhecimento aceita indentação, e o teste tem o par de branches em que a única diferença mora dentro da tabela indentada (falha-antes: o script anterior dá `medium`).
- **`COMMENTED` não retira aprovação.** O GitHub não desfaz uma aprovação porque o revisor comentou depois; a redução por revisor agora considera só os estados que mudam a decisão (`APPROVED`, `CHANGES_REQUESTED`, `DISMISSED`) e ignora `COMMENTED`/`PENDING`. Sem isso, um comentário posterior tornava o PR impossível de mergear até uma nova aprovação.
- **`gh api | jq` mascarava falha do `gh`.** Mesmo problema do `| sort | tail` da rodada 1, num lugar que eu tinha deixado passar: o status do pipeline é o do `jq`. As páginas são capturadas em arquivo, o `gh` é checado, e só então o `jq` normaliza.
- **Dois fixtures com o mesmo nome se sobrescreviam.** A normalização derivava o caminho do `basename` da entrada: `a/same.json` e `b/same.json` viravam o mesmo arquivo. Agora são dois caminhos fixos e distintos (`merged.normalised.json`, `reverts.normalised.json`), com caso de teste.

## 2026-11-06 — Risco de PR: correções da rodada 3 (o bootstrap vira código testado)

Re-check (Pi, `openai-codex/gpt-6.1-terra`, `thinking=medium`) sobre as correções da rodada 2: **nenhum alto** — os dois altos foram confirmados corrigidos — e três médios, todos reais e corrigidos.

- **A regra inline de aprovação rodava por página.** `gh api --paginate --jq '<filtro>'` aplica o filtro a cada página e imprime um número por linha; com mais de 30 reviews o teste numérico recebia `"1\n0"` e quebrava (falhava fechado, mas com erro ilegível). Agora as páginas são baixadas inteiras, `jq -s 'add // []'` junta e o filtro roda uma vez. A regra inline também passou a respeitar o tier: `low`/`medium` saem 0, como no script — antes, se só o gate faltasse na base, um PR de documentação passaria a exigir aprovação.
- **O guard do TOML validava o cabeçalho com a indentação grudada.** `  [dependencies] # comment` é TOML válido, mas virava "ilegível" e fazia **todo** PR de dependência ser alto. Agora o espaço à esquerda sai antes de julgar a forma, e o nome da tabela é extraído limpo. Caso novo: cabeçalho indentado com comentário e dependência inalterada = `medium` (antes: `high`).
- **`git show` de blob vazio "dava certo".** Um arquivo vazio na base fazia `sh` sair 0 — o agregador ficaria verde sem olhar `needs`, o único fail-open dos três. A extração agora exige blob de modo `100644`/`100755` e tamanho maior que zero; symlink, árvore, vazio ou ausente caem na regra inline. Mesmo tratamento para classificador e gate.
- **A política de bootstrap virou código testado.** Ela mora no YAML (não pode vir do PR), então `tests/shell/ci-workflow-bootstrap.test.sh` extrai os três blocos marcados do `ci.yml` e os executa de verdade, com `gh` de mentira e fixtures. São 25 casos, e três mutações provam que o teste morde: filtro por página derruba `gate-approval-on-second-page`; sem o `case "$TIER"` caem os três casos de tier; sem a checagem de blob cai `blob-symlink`.
## 2026-11-06 — Projeto fora do diretório da solution entra no escopo do workspace (issue #52)

Decisões do grill da #52 (respondidas pelo orquestrador; Q2 é decisão do Helder).

- **Escopo, não raiz nova.** O workspace ganha um escopo ordenado: raiz (diretório da solution) + diretório canônico de cada projeto nomeado pelo `.sln`/`.slnx`, deduplicado por contenção, cap de 64 diretórios extras, raiz do filesystem recusada, recalculado a cada `evaluation.run`. A raiz continua a âncora de árvore, busca, cwd de build e terminal; o escopo é o que o host pode tocar.
- **Trust na solution cobre o escopo (Helder).** A Trust decision gravada para o caminho da solution autoriza os diretórios dos projetos que ela referencia: sem prompt novo, sem decisão por diretório, `trusted.json` no mesmo formato mapa. Justificativa: `restore.start` sem `project` já roda `dotnet restore <sln>` e o MSBuild já avalia/restaura esses projetos (medido: `obj/project.assets.json` criado no projeto de fora), então o que se concede de novo é leitura/escrita do host nesses diretórios — exatamente o que "analyzed" exige. Os diretórios vêm do `.sln` confiado, não de input do usuário, e "uma pergunta por projeto" continua valendo. O texto do trust passa a declarar o alcance.
- **Restricted mode não muda.** Sem avaliação, análise ou execução; leitura como texto dentro do escopo continua permitida, como hoje dentro da raiz.
- **Caminho absoluto fora da raiz.** Fora da raiz o caminho é absoluto e canônico na API e na UI; dentro continua relativo com `/`. `..` nunca atravessa a fronteira: relativo é sempre contra a raiz e recusado por forma; arquivo de fora é endereçado por absoluto.
- **Confinamento único com escopo.** A mesma função de confinamento (léxica primeiro, ancestral existente, alvo canonicalizado) passa a receber o escopo; todas as fronteiras de caminho a usam (`file.*`, tree, symlinks, símbolos, LSP, breakpoints, NuGet, problemas de build). Symlink que leva para fora continua recusado, exista ou não o alvo.
- **Limites declarados.** Busca Files/Text, git status/gutter/diff, "Show all files" e cwd do terminal continuam ancorados na raiz da solution; `ProjectReference` para projeto fora do `.sln` não é modelado; projeto em diretório ancestral da raiz coloca esse diretório inteiro no escopo; `.sln` que nomeia projeto por symlink coloca o diretório real no escopo.
- **Consequência.** `docs/SPEC-PROJECTS-OUTSIDE-SOLUTION.md`, tickets filhos da #52, evidência em `docs/evidence/projects-outside-solution.md`. PR é risco alto (trust) e pede aprovação do Helder.

## 2026-11-06 — Fetch é ação explícita; Refresh continua leitura local (issue #113)

- **Decisão (grill da #113, rodada 1: Q1a, Q2a, Q3, Q4b, Q5, Q6, Q7, Q8, Q9, aprovadas pelo Helder via orquestrador).** O painel Git ganha uma ação **Fetch** própria (`git.fetch`, botão no menu de branches e `Fetch…` no Find Action) e o **Refresh** que já existe — no menu de branches e no painel Commit — continua sendo leitura local do `git.status`. Motivo: o menu de branches já chama o refresh **ao abrir**, então fazer o Refresh buscar faria tráfego de rede escondido a cada abertura; e o rótulo continua dizendo a verdade — Refresh lê o local, Fetch fala com o remoto.
- **O remoto é o do upstream, com fallback para o primeiro configurado.** É o mesmo remoto que Update e Push usam, então o contador que muda é o do upstream da branch, que é o que a issue pede. Nunca `--all`: multiplica latência e falhas por remoto, e a UI não tem onde mostrar o resultado por remoto.
- **`--prune` faz parte do fetch.** A lista de branches remotas é metade do problema da issue: sem prune, uma branch apagada na contraparte fica listada para sempre e o `gone` do upstream nunca aparece por fetch. `--prune` só mexe em `refs/remotes/*` — nenhuma branch local, nenhum commit, nenhuma árvore — e o botão declara o comando no `title`.
- **Fetch tem prazo de 60 s; é a única operação de rede com prazo.** O `host_call` do shell serializa **todos** os dispatches num mutex global, então um `git` pendurado congela o app inteiro, não só o painel. O fetch mata o filho no estouro e responde em palavras do app ("the remote did not answer within 60 seconds. Nothing was changed."); push e pull ficam fora desta frente de propósito (escopo), e o mesmo risco neles vira issue própria — #136, *Related to #113*.
- **Fetch não exige árvore limpa e não é bloqueado por operação em andamento.** Ele move só refs: um merge em andamento não é atropelado por ele e uma árvore suja não é tocada. Exige repositório Git e remoto configurado (sem remoto, o botão fica desabilitado com motivo, como o Push hoje) e é trust-gated como todo `git.*`.
- **Erro de credencial vira palavra do app; o resto continua a cauda do stderr.** O host roda `git` com `GIT_TERMINAL_PROMPT=0` e `stdin` nulo, então um remoto privado falha com "could not read Username… terminal prompts disabled", que ensina a rodar comando no terminal sem explicar por quê. Essa classe é traduzida; qualquer outra falha fica como hoje, com o texto cru em `Operation.output`.
- **Sem diálogo, sem atalho, sem fetch automático.** Fetch não é destrutivo, então é ação direta com botão ocupado e mensagem factual no fim ("Fetched origin: main is 0 ahead, 1 behind."), seguida de status e lista de branches relidos. Nada de rede escondida no foco da janela nem polling: o `↓` continua valendo desde o último fetch, e o spec declara isso em vez de inventar selo de "stale". Atalho de teclado fica fora: `git.update` acabou de ser desamarrado do menu nativo (#111/#130) e o Rider também não tem default para Fetch.
- **Correções da revisão independente (rodada 1).** Quatro achados reais, todos corrigidos. (a) **O prazo matava só o `git`**: `git remote-http`/`git-remote-http` (e ssh, e o helper de credencial) herdam os pipes, e um sobrevivente deixaria a leitura bloqueada — o congelamento que o prazo existe para evitar. Agora o filho nasce em grupo de processos próprio e o estouro mata o grupo (`kill_group`), com a leitura dos pipes limitada por um segundo prazo curto como rede de segurança; o teste sobe um listener que aceita e não responde e reprova se algum descendente sobreviver (falha-antes medida: sem o grupo, os dois processos `git-remote-http` ficam vivos). (b) **`--prune` não se limita a `refs/remotes/*`**: ele apaga o que `remote.<nome>.fetch` apontar, então um repositório que espelha em `refs/heads/*` ou `refs/mirror/*` perderia refs locais. O fetch passa a nomear o refspec (`+refs/heads/*:refs/remotes/<remote>/*`) **e** a desligar o refmap configurado (`--refmap=`): só o refspec explícito ainda deixa o Git aplicar `remote.<nome>.fetch` como refmap adicional, então as refs espelhadas seriam criadas/atualizadas mesmo sem prune (medido: `refs/mirror/main` nasce com o refspec sozinho e não nasce com `--refmap=`). Tags continuam vindo pelos commits buscados. (c) **A mensagem de prazo prometia "nada mudou"**, o que um kill não pode garantir (pode cair depois de o Git atualizar refs); agora ela promete o que um fetch nunca toca — branch, index e árvore de trabalho — e diz que refs já recebidas ficam. (d) **O remoto vinha do split de `origin/main`**, que erra com remoto de nome com `/` e com upstream local (`.`); passa a vir de `branch.<nome>.remote`, com fallback para o primeiro remoto configurado.
- **Também da revisão.** A drenagem dos pipes passou a guardar a **cauda** do stderr (era o começo, e a mensagem de falha mora no fim); o botão ganhou guarda de reentrância por ref (o Find Action podia disparar um segundo fetch durante o primeiro) e `aria-busy` com o rótulo `Fetching…`; e entraram testes para mascaramento de credencial na URL de um fetch que falha e para o refspec hostil. A drenagem também passou a marcar `Output.truncated` quando descarta bytes.
- **Consequência.** `docs/SPEC-GIT-FETCH.md`; testes de host com o remoto bare local real (mesmos helpers do #112) e prova no app Tauri real com `agent-desktop`; testids novos `branch-fetch`, `branch-fetch-error`, `branch-fetch-reason`, nenhum existente renomeado; `GLOSSARY.md` ganha **Fetch**.

## 2026-11-07 — Push e o fetch do Update têm prazo; o lock global fica (issue #136)

- **Decisão (grill da #136, rodada 1: Q1–Q9 aprovadas pelo Helder via orquestrador).** Toda operação de Git que fala com o remoto passa a ter prazo: **push 120 s** e **o fetch do pull 60 s**, o mesmo do fetch da #113. O merge do pull continua local e **sem prazo**: ele não espera rede, e matá-lo no meio deixaria um merge pela metade — pior que o congelamento que o prazo evita. Como o fetch vem antes, um Update interrompido nunca começa um merge, então a mensagem pode dizer "Nothing was merged" sem mentir.
- **O lock global do `host_call` não muda.** Ele é o que garante a ordem de `lsp.didChange`/`session.write`, e tirá-lo do caminho longo está fora desta frente. O limite honesto é "congela no máximo 2 minutos", não "nunca congela"; spec, evidência e retro dizem isso em voz alta.
- **Dois números, com motivo.** Push sobe objetos (uplink assimétrico) e espera hook do servidor (`pre-receive`), então um prazo igual ao do download mataria push legítimo; o custo aceito é um teto de 2 min em vez de 1.
- **O prazo é teto, não promessa de mensagem.** Contra remoto que descarta pacotes o próprio Git desiste antes (75 s medidos contra `192.0.2.1`) e o diálogo mostra o erro dele; contra servidor que aceita a conexão e não responde, o prazo do app dispara com a frase do app. Os dois casos são limitados — que é o que a issue pede — e nada promete que todo push morto termina na nossa redação. Por isso o `PUSH_DEADLINE` **não** foi baixado para ganhar a corrida do timeout de conexão do Git.
- **Mensagens prometem só o que é certo.** Push morto: `Push stopped: the remote did not answer within 120 seconds. The push may already have reached it: check the branch there before pushing again.` — sem afirmar sucesso, sem afirmar que o `--set-upstream` valeu, sem prometer o estado local (hook `pre-push` pode ter rodado) e sem nomear remoto (`git push` sem argumento resolve `branch.<nome>.pushRemote`/`remote.pushDefault`, que o app não lê). Update morto: `Update stopped: <remote> did not answer within 60 seconds. Nothing was merged; your branch, index and working tree were not touched, and refs already received stay.` — sem sugerir `merge --abort`, porque nenhum merge começou.
- **Mecanismo reusado, zero dependência.** `run_with_deadline` da #113 (grupo de processos próprio, `kill_group`, drenagem por canal com `recv_timeout`) passa a servir push e o fetch do pull; `push_with`/`pull_with` existem só para o teste injetar prazo curto, como `fetch_with`. Sem UI, sem `data-testid` novo, sem mudança em `git.diff`/`git.fileDiff`.
- **Cancelar pela UI fica fora, com issue filha.** O prazo já libera o host; um botão de cancelar exige estado de operação em voo no host.
- **Consequência.** `docs/SPEC-GIT-NETWORK-DEADLINE.md`, tickets #147 (host) e #148 (evidência/registro), evidência em `docs/evidence/git-network-deadline.md`, retro em `docs/RETRO-GIT-NETWORK-DEADLINE.md`.
## 2026-11-07 — Risco alto: o dono do repositório aprova o próprio PR (opção (b) da #149)

- **Decisão do Helder.** No tier alto, o gate passa quando o **autor do PR** tem `author_association == OWNER`, sem exigir review. Qualquer outro autor continua precisando da aprovação na revisão atual, com a regra que já existia (`state == APPROVED`, `commit_id == head`, associação de escrita, estado mais recente daquele revisor).
- **Por que.** O desenho do #125 pede uma review `APPROVED` de outra identidade com write access, e o GitHub recusa aprovação no próprio PR (HTTP 422). O repo tem um mantenedor só e o ruleset da `main` está com `bypass_actors: []`, então o `risk:high` nunca ficava verde e o agregador `ci` — que depende dele — travava todo PR de risco alto. Foi o que aconteceu com o próprio #131.
- **Custo aceito, escrito antes de implementar.** O tier alto deixa de significar "outra pessoa olhou": quem escreve se aprova, e o gate vira registro, não revisão. O que continua valendo no alto é o resto: ele não arma auto-merge, o merge é manual do mantenedor/orquestrador, e o `/code-review` com outro modelo segue sendo a segunda opinião real — só não está no GitHub.
- **Onde a regra mora, e por quê.** Em dois caminhos, os dois que já existiam: `scripts/ci-risk-gate.sh` (executado da revisão base) e a regra inline de bootstrap do `.github/workflows/ci.yml`. A associação do autor vem de `gh api repos/<repo>/pulls/<n> --jq .author_association`, **nunca** de um input do workflow: num `pull_request` o YAML que roda é o do merge commit, então um valor que o PR pudesse escrever seria uma autoaprovação. A checagem é a primeira coisa do gate depois do tier, e o curto-circuito é total: no PR do dono a lista de reviews nem é lida.
- **O que não muda.** `low`/`medium` continuam passando sem ler nada; o auto-merge continua armado só no `low` (`if: steps.tier.outputs.tier == 'low'`); o ruleset da `main` e os settings do repo não foram tocados — os dois checks obrigatórios seguem `ci` e `security`.
- **Alternativas recusadas na #149.** (a) segunda conta com write access: preserva o desenho, custa convite + login por PR alto; (c) label/comentário `/approve`: o grill do #74 já tinha recusado por transformar "quem aprovou e quando" num rótulo que qualquer write põe; (d) admin override: exigiria reabrir o ruleset que o #56 entregou; (e) baixar `scripts/ci-*` e `.github/workflows/**` para médio: contraria o corpo do #74.
- **Prova.** `tests/shell/ci-risk-gate.test.sh` (23 casos) e `tests/shell/ci-workflow-bootstrap.test.sh` (27 casos) ganharam os casos do dono autor, do autor que não é dono e do PR ilegível; o código testado roda sob `sh` e sob `dash` (`GATE_SHELL=dash`). Falha-antes medida: com o script e o YAML anteriores a esta decisão, os casos do dono reprovam (`expected exit 0, got 1`). O run real do #131 é a prova de ponta: o `risk` fica verde com o autor `OWNER` e sem nenhuma review.
- **Correções da revisão independente (rodada 1, #149).** Dois achados, ambos reais e corrigidos. (a) **Médio: `dash <teste>` não provava nada sobre o gate.** O harness chamava o código testado com `sh` fixo, então rodar o teste sob `dash` só provava que o *teste* era compatível — a regra 3 da #149 pede o contrário. Agora `GATE_SHELL` escolhe o interpretador do gate e do bloco inline, e a prova é `GATE_SHELL=dash sh tests/shell/…` (medido: um `${var^^}` injetado no gate morre com `Bad substitution` no `dash` e passa no `bash`). (b) **Baixo: a evidência afirmava "nem lê a lista de reviews" sem teste.** Caso novo `owner-author-does-not-read-reviews` roda o PR do dono com a chamada da lista falhando (`FAKE_FAIL=1`) e exige verde; a mutação que tira o `exit 0` do curto-circuito o reprova.

## 2026-11-07 — Download de ferramenta é limitado por inatividade, não por prazo total (issue #161)

- **Decisão (grill da #161, rodada 1: Q1–Q6 aprovadas pelo Helder via orquestrador).** O download do pacote do csharp-ls deixa de ter prazo **total** de 120 s e passa a falhar só por **inatividade**: 60 s sem um byte. Um link lento que continua entregando bytes termina, por mais que demore; uma conexão que para de mandar falha. O `ureq` não deixa combinar os dois (`.timeout()` total tem precedência sobre `.timeout_read()`), então é um ou outro — e o total é exatamente o que reprovou uma transferência de 151 s que estava progredindo.
- **Dois números, com motivo.** `timeout_connect` 15 s (o mesmo que o download do netcoredbg já usa) fecha a conexão que nunca abre; `timeout_read` 60 s é folgado para o primeiro byte de um proxy que bufferiza o objeto antes de responder, e 30 s foi recusado por ser curto para esse caso.
- **Sem teto total, custo aceito.** Um servidor que pinga um byte a cada 59 s mantém o download vivo até o limite de 32 MiB do pacote, e a UI não tem botão de cancelar (só `Try again`). O teto que existe é o de bytes. Alternativa recusada: watchdog manual para um prazo total, que é código novo para um caso que ninguém viu.
- **A política mora no cliente HTTP do host**, junto do roteamento de proxy e da guarda de endereço — não no instalador. O instalador passa a janela de stall. O download do netcoredbg tem a mesma forma com prazo total de 300 s e fica na filha **#162**, para o diff desta frente ser revisável.
- **Mensagem do stall.** `Could not download csharp-ls: <url>: the connection stalled — no data arrived for 60 seconds. Nothing was installed or replaced; check the network and retry the setup.` O mesmo texto para o stall antes dos headers e no meio do corpo: para quem usa é o mesmo defeito. "Nothing was installed or replaced" é verdade porque o destino só é tocado depois do download verificado.
- **Detecção por tipo, nunca por texto.** Corpo: `io::ErrorKind::TimedOut` na leitura. Headers: o erro do `ureq` carrega um `io::Error` `TimedOut` na cadeia de `source`.
- **Teste com ambiente injetado, sem seam novo de ambiente.** O cliente do download ganha a variante que recebe o ambiente (o par que já existe no roteamento de proxy) e o teste sobe um proxy local que serve o corpo em pedaços; o destino é um IP público literal, então não há DNS nem rede. Falha-antes medida: com o prazo total de 400 ms a mesma transferência morre em 16 KiB; com inatividade chega aos 61.440 bytes.
- **O alcance é menor do que a issue sugere, e isso ficou escrito na #161.** O download só existe **fora** de um `.app`: a build distribuída embute o csharp-ls e o runtime privado, e o plano de setup recusa baixar quando o par está presente (e falha fechado quando está danificado). Quem encontra o prazo é `tauri dev` e a ponte HTTP de dev, não o DMG.
- **Uma linha de UI com um `data-testid` novo.** O gate de setup mostrava só `setup.error`; um download que falhava aparecia como `Failed` sem motivo, e o gate é a única superfície enquanto o setup bloqueia o workspace (o painel de ferramentas fica atrás dele). O gate passa a mostrar o erro da ferramenta em parágrafo próprio, `data-testid="setup-gate-tool-error"` — **novo**, nenhum existente renomeado ou removido, nenhuma dependência nova.
- **Correções da revisão independente (rodada 1, #161).** Três achados médios, todos reais e corrigidos. (a) **Erro de conexão virava stall**: a detecção olhava só a cadeia de `source`, então um `timeout_connect` de 15 s era anunciado como "no data arrived for 60 seconds". Agora a detecção exige `ErrorKind::Io` antes da cadeia (o `ureq` reporta falha de conexão como `ConnectionFailed`) e há teste com TEST-NET-1 provando que a frase de stall não aparece. (b) **Escrita sem teto**: sem o prazo total, `timeout_write` ficava sem limite e uma escrita bloqueada penduraria o setup; passa a receber a mesma janela de stall. A resolução de nome continua não interrompível pelo `ureq` — e o prazo total anterior também não a limitava (é conferido entre operações), então isso fica documentado como limite conhecido do host, não como teto que a frente ganhou. (c) **A fronteira do instalador não tinha teste**: o mapeamento de leitura do corpo para a frase do download ganhou função própria e teste, em vez de só a função de mensagem ser testada. Baixos: a folga do teste de progresso subiu de 4× para 5× (intervalo de 200 ms, janela de 1 s) e a linha nova de `docs/evidence/test-stability.md` passou a dizer "hipótese: carga; causa não determinada" em vez de afirmar a causa.
- **Consequência.** `docs/SPEC-TOOLS-DOWNLOAD-STALL.md`, tickets #163 (política + teste), #164 (frase do stall + testes) e #165 (prova no app real + evidência); evidência em `docs/evidence/tools-download-stall.md`, retro em `docs/RETRO-TOOLS-DOWNLOAD-STALL.md`.

## 2026-11-07 — O diretório neutro da ferramenta é criado em um lugar só (issue #93)

- **Decisão (grill da #93, rodada 1: Q1a, Q2a, Q3a, Q4a aprovadas pelo orquestrador).** `neutral_cwd_ready()` **cria** o diretório neutro e devolve o caminho; `verify_installed`, `verify_bundled` e o worker de install chamam essa função e perdem o `create_dir_all` próprio. `neutral_cwd()` continua só juntando o caminho, porque é o que o plano anuncia: a primeira versão criava ali e fazia `tools.plan` — chamada de leitura — escrever no `$TMPDIR` antes de qualquer confirmação (achado da revisão independente). Motivo: o `#90` corrigiu o sintoma em um chamador e deixou o passo em três lugares — a mesma armadilha (esquecer de criar) a um `create_dir_all` de distância. Quem pede o caminho recebe um caminho que existe.
- **O erro de criação é ignorado, como `verify_bundled` já fazia.** Custo declarado: o worker de install perde a mensagem "Could not create the neutral install directory". Só aparece com `TMPDIR` não gravável, e nesse caso o spawn falha logo depois com `ENOENT`. Aceito: um lugar único vale mais que uma mensagem melhor num caso raro; o caminho que a issue pede (máquina fria, `TMPDIR` gravável) não passa por ali.
- **A criação é testável sem mexer no ambiente do processo.** `neutral_cwd_in(root)` recebe a raiz e `neutral_cwd_ready()` chama com `std::env::temp_dir()`; o teste usa um `tempfile::tempdir()` e prova que o diretório nasce de um pai onde ele não existe. O teste que a issue cita só reprova em runner limpo (foi o que escondeu o bug até o CI), e `set_var` em teste paralelo é corrida — daí a indireção de três linhas.
- **Sem UI, sem `data-testid`, sem dependência.** Nada muda no plano de instalação, no payload ou nas mensagens de verificação; o app real entra como não verificado, porque não há superfície de UI para dirigir.
- **Consequência.** `docs/SPEC-STAGED-TOOLS-NEUTRAL-DIR.md`, ticket #153, evidência em `docs/evidence/staged-tools-neutral-dir.md`; a falha de rede dos testes de install real ficou registrada em `docs/evidence/test-stability.md`.

## 2026-11-07 — Diff de um arquivo de um commit pelo Git log (issue #101)

- **Decisão (grill da #101, rodada 1: Q1a, Q2a, Q3a, Q4 e Q5a aprovadas pelo orquestrador; Q6 = (b), decisão do Helder).** Clicar num arquivo do commit selecionado no Git log abre uma aba de diff read-only daquele arquivo naquele commit, no mesmo painel do working-tree diff. A aba é própria (`commit-file-diff-tab`), rotulada `<shortHash> · <arquivo>`, com hash completo e assunto no `title`; o painel mostra `parent · <parentShort>` à esquerda, `<commitShort>` à direita e a linha `<shortHash> · <subject>`. Um slot só, como o working-tree diff: abrir outro arquivo substitui o conteúdo da aba.
- **As duas abas especiais convivem.** Working-tree diff e diff de commit são slots independentes; o estado "aba ativa" do tab strip virou três valores (arquivo, diff da árvore, diff do commit). O `HEAD` do texto só existe no working-tree diff, porque o commit mostrado pode não ser o `HEAD`.
- **Base = primeiro pai, lida como blob.** `git.commitDiff` resolve o commit (`rev-parse --verify --quiet "<hash>^{commit}"`), o primeiro pai (`^`, exit 1 = raiz) e lê `parent:<originalPath ?? path>` e `<hash>:<path>`; nada toca a árvore de trabalho, então uma alteração não commitada não muda o diff. Commit raiz não tem pai: a base é a empty tree (`hash-object -t tree --stdin`) e o painel diz `No previous file`.
- **Rename exige os dois caminhos no pathspec.** Medido num repositório descartável: com só o caminho novo o Git devolve add+delete e o diff vira arquivo inteiro; `git diff <pai> <hash> -- <old> <new>` pareia o rename e mostra só a linha alterada. O `originalPath` vem do `git.log` (que já reporta R/C) e passa por `validate_path` como o `path`; limite declarado: um `originalPath` errado produziria uma base errada, e a UI só passa o que o log devolveu para aquele commit.
- **Hash não é aceito como texto solto.** `validate_commit` exige 4–64 dígitos hexadecimais antes de qualquer `git rev-parse`, então nem `--help` nem sintaxe de revisão alcançam o comando. Hash que não resolve (amend/rebase, log velho) vira `failed` com "Commit not found", nunca um diff vazio.
- **Um tail só para os dois diffs.** A montagem do payload (lados, `full`/`fullReason`, ranges de UTF-16 sobre o texto decodificado, `unaligned`, cap do patch, o "No diff" que não mente quando um lado existe e não pôde ser lido) saiu do `file_diff` para `assemble`, usada pelos dois métodos; `file_diff`/`git.diff` mantêm comportamento e testes.
- **Sem E2E novo (Q6b, decisão do Helder).** A issue pedia um spec contra repositório descartável; a #67 diz que Playwright é só smoke noturno e nunca verificação de UI de sessão. A prova é host (5 testes novos, 59 no módulo) + unidade (15) + app Tauri real com `agent-desktop`, e a troca está registrada na spec e na evidência.
- **Navegação e testids.** `Previous/Next changed file` e a contagem `n/m files` andam pela lista de arquivos do commit; nenhum `data-testid` existente foi renomeado. Novos: `commit-file-diff-tab` e `git-log-file-<path>`; o painel reusa `commit-diff`, `commit-diff-mode`, `commit-diff-commit`, `commit-diff-note`, `commit-diff-left-label`, `commit-diff-right-label`, `commit-diff-count`, `commit-diff-file-count` e `commit-diff-unified`.
- **Consequência.** `docs/SPEC-GIT-COMMIT-DIFF.md`, tickets #169 (host), #170 (UI) e #171 (app + registro), evidência em `docs/evidence/git-commit-diff.md`; `GLOSSARY.md` ganha **Commit diff**.
- **Correções da revisão independente (rodada 1).** Quatro achados reais, todos corrigidos. (a) **Médio: caminho era tratado como pathspec.** `a[1].txt` é classe de caractere para o Git e o diff também trazia `a1.txt`; `--literal-pathspecs` passou a valer em `commit_diff_output`, `diff_output` e `is_tracked` (o defeito também existia em `git.diff`/`git.fileDiff`), com teste que reprova sem a flag (2 hunks em vez de 1); em `is_tracked` a flag é intenção declarada — o Git tenta o nome literal antes do padrão, então o caso não reprovava. (b) **Médio: metadados sem limite.** `commit_ref` usava `run` (stdout inteiro); agora usa `spawn_bounded` com 32 KB, e um assunto absurdo deixa o diff sem a linha do commit em vez de alocar. (c) **Médio: fechar aba inativa trocava a aba visível.** Os dois botões de fechar limpavam `specialTab` sempre; agora só quando a aba fechada é a ativa. (d) **Baixo: hash curto divergente.** `git.log` cortava o hash em sete caracteres e o painel usava o `%h` do Git; o log passou a pedir `%h` também, e o teste compara com `rev-parse --short`. O `too_large` do diff de commit ganhou teste próprio (o achado baixo da evidência) e a evidência deixou de atribuí-lo ao teste do #48.
## 2026-11-07 — Merge em andamento se conclui ou aborta pela UI; o Update deixa de abortar (issue #114)

- **Decisão (grill da #114, rodada 1: Q1–Q8 aprovadas pelo Helder via orquestrador).** Um merge em andamento deixa de ser beco sem saída: o menu de branches ganha **Complete merge** e **Abort merge…**. Concluir stageia só os caminhos que estavam conflitados (`git add -u -- <paths>`, nunca um `add -u` global) e cria o merge commit com a mensagem que o Git preparou (`git commit --no-edit`); abortar roda `git merge --abort` atrás de confirmação. Escopo é **só merge**: rebase, cherry-pick e revert são recusados pelo host e a nota do menu diz que esses casos pedem terminal — a #112 já tinha deixado rebase para frente própria.
- **Conflito com marcador recusa a conclusão.** Antes de stagear, o host roda `git diff --check HEAD` e `git diff --check --cached HEAD` e recusa listando os caminhos quando aparece `leftover conflict marker`. É o critério do próprio Git (pega `<<<<<<<`, `=======` e `>>>>>>>`), sem heurística de leitura de arquivo; erros de whitespace do mesmo comando são ignorados. Os dois checks existem porque o commit usa o index: marcador staged e removido do worktree só aparece no `--cached`, e caminho unmerged só no do worktree. Saída truncada **recusa** (o teto é 1 MiB, rede de segurança), nunca aprova por falta de prova.
- **O Update deixa de abortar o próprio conflito.** O merge conflitante fica em andamento e a mensagem diz onde terminar: `Update stopped: <N> file(s) changed on both sides (<paths>). The merge is still in progress: resolve the conflicted files in the editor, then finish or abort it from the branch menu.` Com as ações existindo, o abort provisório da #112 (que existia justamente porque um merge aberto era outro beco) perde a razão: `merge_head` e `abort_conflicting_merge` saem do host, e a posse do abort deixa de ser necessária porque o app não aborta mais nada que não seja pedido explícito. Merge de outro processo continua recusado antes de qualquer comando pelo `require_clean`.
- **Commit bloqueado durante qualquer operação em andamento.** O Git proíbe commit parcial durante um merge (`cannot do a partial commit during a merge`), então `git.commit` recusa com `git_operation_in_progress` e mensagem do app antes de rodar nada, e a UI desabilita o **Commit** com o mesmo motivo. Vale para rebase/cherry-pick/revert também, que antes vazavam o erro cru.
- **Duas ações no menu de branches, ponteiro no Commit tool.** O Commit tool substitui a Solution Explorer no painel esquerdo, então duplicar as ações ali exigiria sincronizar dois estados; ele passa a apontar o editor e o menu. O menu de branches é a única superfície Git sempre visível.
- **Concluir não pede diálogo; abortar pede.** O abort descarta o merge, as resoluções em andamento e o que foi staged durante o merge (é o `git reset --merge` que o `merge --abort` usa), então o modal diz isso; concluir é o caminho feliz e o resultado vai na mensagem da operação. A mensagem de sucesso promete só o que o Git garante: `Merge aborted; the merge is undone and your commits stay.` — mudanças não commitadas anteriores ao merge podem não voltar, e o modal avisa.
- **Sem dependência, sem comando novo.** Tudo é Git de linha de comando pelo host (`add -u`, `diff --check`, `ls-files -u`, `commit --no-edit`, `merge --abort`); sem libgit2, sem atalho de teclado, sem push automático, sem UI de resolução de conflito (a resolução é no editor, com os marcadores do Git).
- **Consequência.** `docs/SPEC-GIT-MERGE-IN-PROGRESS.md`; tickets #154 (abort), #155 (concluir + commit bloqueado) e #156 (Update deixa o merge aberto, bloqueada por #154/#155); `docs/SPEC-GIT-UPDATE-RECONCILE.md` marca o abort como substituído; `GLOSSARY.md` ganha **Merge in progress** e a entrada **Update** deixa de dizer que o conflito aborta; evidência em `docs/evidence/git-merge-in-progress.md`.

## 2026-11-07 — App de dev sobe em um comando e o workspace entra por argv (issue #91)

- **Decisão (grill da #91, rodada 1: Q1–Q8 aprovadas pelo orquestrador).** `scripts/dev-app.sh [sln]` resolve o par embutido (os três `OPENRIDR_BUNDLED_*` já exportados > `src-tauri/resources` vendorizado > `$OPENRIDR_APP` > `/Applications/OpenRidr.app`), valida com as mesmas condições que o host usa para chamar o par de completo, exporta os três overrides e roda `tauri dev --config '{"bundle":{"resources":null}}'`, com o workspace opcional em argv. Motivo: numa worktree nova o `tauri-build` morre em `resource path 'resources/dotnet' doesn't exist` antes de compilar, e mesmo com os resources no lugar o binário de dev não está num `.app`, então `resources_root()` não existe e o par só aparece pelos overrides. Duas sessões seguidas pagaram ~20 min cada.
- **O `--config` remove `bundle.resources` com `null`, não com `{}`.** `TAURI_CONFIG` passa por merge patch (RFC 7386) no `tauri-build`; objeto vazio é no-op e o build continua falhando. Medido: `{"bundle":{"resources":null}}` compila em 6,2 s e o app abre o `.sln` do argv.
- **Nenhuma cópia de resources no repositório.** A alternativa (copiar/simlinkar o par para `src-tauri/resources`) faria o `tauri-build` copiar ~150 MB para `target/debug` a cada mudança de resource e continuaria precisando dos overrides; o bundle de debug (contorno do RETRO-GIT-DIFF) perde o hot reload. O script também não vendoriza nem baixa nada: sem par, a mensagem diz o que falta e as duas saídas (`bun run app:install`, `scripts/vendor-*.sh`).
- **O script recusa subir com outro OpenRidr vivo.** O plugin de instância única entrega o argv ao app instalado e o binário de dev sai na hora; sem o guard o sintoma é "o app de dev nunca aparece".
- **`OPENRIDR_HOME` não é isolado pelo script.** Isolar é trabalho do `desktop.sh` da skill de verificação; o script respeita o ambiente do chamador.
- **Abrir workspace no app real é por argv.** O campo do Welcome não recebe digitação (`delivered_unverified`, valor vazio) e "Open" com o campo vazio abre o seletor nativo, que deixa o app em "Opening…". A receita fica na skill `verify-openridr` (superfície C) e em `test-app-agent-desktop`; o `AGENTS.md` ganha só a linha do comando, para a mudança de processo não entrar de carona.
- **Sem dependência nova, sem UI, sem `data-testid`.** O app distribuído não muda: o `--config` é por invocação e só no dev.
- **Consequência.** `docs/SPEC-DEV-APP.md`, tickets #172 (script + self-check) e #173 (docs), evidência em `docs/evidence/dev-app.md`; `scripts/dev-app-selfcheck.sh` roda local, não no CI (nenhum self-check entra hoje e workflow é `risk:high`).

## 2026-11-07 — Launch profile: argv do .NET, expansão de variáveis e navegador (issue #53)

- **Decisão (grill da #53, rodada 1: Q1–Q9 aprovadas pelo Helder via orquestrador, com duas exigências de segurança).** O perfil importado de `launchSettings.json` passa a se comportar como o `dotnet run` no que ele faz e como o Rider no que ele não faz: argumentos pela gramática do `CommandLineToArgvW`, expansão de `%VAR%`/`$(VAR)` em `environmentVariables` e `applicationUrl`, e abertura do navegador com `launchBrowser`/`launchUrl` no Run e no Debug.
- **O que o CLI mede, e o que ele não faz (medido, não suposto).** `dotnet run` 8.0.408 e 10.0.100 neste Mac expande `%VAR%` **só** nos valores de `environmentVariables`, contra o ambiente do processo, uma passada e sem reexpandir o substituído; `%NOPE%` indefinido e `%%` ficam literais; `$(VAR)` **nunca** expande (o próprio SDK comenta "MSBuild variables are not expanded like they are in VS"); `commandLineArgs`, `applicationUrl` e `workingDirectory` não expandem. `launchBrowser`/`launchUrl` são lidos e **ignorados** pelo `dotnet run` (nenhum caminho de código; testado com shim de `open`), e `workingDirectory` também é ignorado. Quem honra `launchBrowser` é o `dotnet watch` e o Rider/VS.
- **Gramática do argv = `CommandLineToArgvW`, com matriz medida.** O SDK entrega `commandLineArgs` como `ProcessStartInfo.Arguments` e o filho divide com as regras do Windows: espaço/tab separam fora de aspas; `\` só é especial antes de `"` (2n `\` + `"` → n `\` e a aspa delimita; 2n+1 → n `\` + `"` literal); `""` dentro de aspas é `"`; aspa não fechada é tolerada; `#` e `'` são literais. A matriz de 40 casos foi capturada do `dotnet run` real (SDK 8.0.408) e é o teste; 16 desses casos falhavam no divisor antigo (medido).
- **Expansão: `%VAR%` e `$(VAR)` em `environmentVariables` e `applicationUrl`.** `$(VAR)` é extensão de Visual Studio/Rider e entra porque a issue pede e o perfil importado do IDE usa; `commandLineArgs` e `workingDirectory` ficam literais para o argv e o cwd não divergirem do CLI. Nome vazio ou indefinido mantém o texto (`%NOPE%`, `%%`, `$(NOPE)`), e um `%` de porcentagem sobrevive porque só um par `%nome%` definido muda.
- **Navegador: host, URL validada, abridor direto, espera com prazo.** A URL é a primeira entrada não vazia do `ASPNETCORE_URLS` efetivo (o `environmentVariables` do perfil vence `applicationUrl`, como já era) combinada com `launchUrl`: absoluta `http(s)` vence, relativa é concatenada. Só abre `http`/`https` com host não vazio; `file:`, `javascript:`, esquema customizado, host vazio, credenciais e porta `0` (porta dinâmica que o host não pode conhecer) não abrem e o log diz o motivo. O abridor é chamado **direto com a URL como argumento único** — `open` no macOS, `xdg-open` no Linux, nunca shell nem `cmd /c` com string montada; Windows fica fora e é declarado não suportado. A abertura espera até 15 s o host:porta aceitar conexão TCP, numa thread separada, e nunca falha o lançamento: o log do Run (`run-log`) ou do DAP (`dap-log`) recebe `Opened <url> in the default browser.` ou `Launch browser skipped: <motivo>.`
- **Trust não ganha gate novo.** Run e Debug já exigem workspace confiado antes de spawnar; a abertura acontece depois desse gate, então modo restrito continua sem executar nada — e um `launchSettings.json` de outra pessoa não consegue abrir nada além de uma URL `http(s)` no navegador padrão.
- **Zero dependência, zero contrato novo, zero UI nova.** A decisão entra em `LaunchOptions` (o mesmo caminho de Run e Debug), o abridor usa `std::process::Command` e o log existente mostra o resultado; nenhum `data-testid` novo. `workingDirectory` do perfil continua aplicado (divergência existente com o CLI, fora desta issue), e `DOTNET_LAUNCH_PROFILE` continua fora.
- **Consequência.** `docs/SPEC-LAUNCH-PROFILES-EXTRAS.md`, tickets #157/#158/#159 (sub-issues de #53), evidência em `docs/evidence/launch-profiles-extras.md`; `GLOSSARY.md` ganha **Launch profile** e **Launch browser**.

## 2026-11-07 — O download do netcoredbg usa a mesma política de inatividade da #161 (issue #162)

- **Decisão (grill da #162, rodada 1: Q1a, Q2a, Q3b, Q4a, Q5a, Q6 aprovadas pelo orquestrador).** O download do netcoredbg (`Debugger::download`) perde o prazo **total** de 300 s e passa a ser limitado por inatividade com a política que a #161 entregou: `timeout_connect` 15 s, `timeout_read`/`timeout_write` 60 s, sem prazo total. A política fica num lugar só: `http::download_agent(url, stall)` monta o cliente e é usado pelo `http::fetch_with` (csharp-ls) e por este download. `DOWNLOAD_CONNECT` e `timed_out` passaram a `pub(crate)`.
- **O laço de redirect do netcoredbg fica.** Ele revalida o host oficial (`allowed_url`) a cada hop, o que `http::fetch` não faz; trocar por `fetch` exigiria um callback novo no caminho recém-entregue do csharp-ls. A política compartilhada é o cliente, não o laço.
- **A frase do stall é a da #161, palavra por palavra** (`http::stalled`), inclusive "retry the setup" — o botão do painel de Debug se chama "Retry adapter install". Uma frase só vale mais que duas; o descompasso fica escrito no spec e na evidência.
- **Stall antes dos headers e no meio do corpo são o mesmo defeito.** Antes: `http::timed_out(&ureq::Error)` (que exige `ErrorKind::Io`, então `timeout_connect` não é stall). Corpo: `http::is_stall(&io::Error)`. As duas viram `download_stalled(url, stall)`; qualquer outro erro de leitura mantém a frase do transporte.
- **A URL assinada sai das mensagens.** `loggable_url` alimenta os dois `describe_error` desta função (407 e transporte) e, depois da revisão, `redact_url` remove a query assinada também do texto que o `ureq` repete dentro do erro (o `Debug` de `Transport` traz a URL como `Url { query: Some(...) }`): sanitizar só o argumento não bastava. O asset do GitHub vem de um redirect assinado e o `debug.error` é copiável.
- **Seam de teste: a janela é parâmetro.** `Debugger::download` delega para `download_with(asset, path, stall)`; o produto passa `DOWNLOAD_STALL`, o teste passa 1 s — o mesmo recurso de `fetch_with` na #161, e o que faz o caso lento falhar com a política antiga (5.120 de 15.360 bytes) e passar com a nova.
- **Prova no host, não na tela.** Em `tauri dev` o gate de setup só libera com `debug.adapterState == "ready"` e `require_trust` também exige setup pronto; o adaptador é o que o download instala, então o `debug.install` não é alcançável pela UI nem pela ponte de dev (medido: `trust.grant` → `setup_required`). Com adaptador embutido, `begin_install` recusa. Fica registrado como limite do shell de dev, sem issue de follow-up (na release não há download).
- **Consequência.** `docs/SPEC-DEBUGGER-DOWNLOAD-STALL.md`, tickets #178 (política + frase + testes) e #179 (prova no host + evidência); evidência em `docs/evidence/debugger-download-stall.md`, retro em `docs/RETRO-DEBUGGER-DOWNLOAD-STALL.md`.

## 2026-11-07 — Proxy do macOS System Settings entra como última fonte da rota, sem tocar na guarda (issue #132)

- **Decisão (grill da #132, rodada 1: Q1–Q8 respondidas pelo orquestrador).** O ajuste de proxy do System Settings do macOS passa a ser lido (`/usr/sbin/scutil --proxy`, leitura pura, sem elevação, ~10 ms) e entra como **último item** da lista ordenada que o #51 definiu: qualquer variável de proxy do ambiente — inclusive `HTTP_PROXY`, o último recurso entre as variáveis — vence o sistema, e o ajuste do macOS só decide quando o ambiente não nomeia proxy nenhum. Motivo: o app aberto pelo Finder/Dock nunca herda o ambiente do shell, então para o usuário corporativo o proxy só existe no System Settings.
- **`ExceptionsList` com semântica de `NO_PROXY`.** `NO_PROXY`/`no_proxy` do ambiente vence a lista do sistema; sem ele, as exceções do Mac valem — inclusive para um proxy vindo do ambiente, porque exceção só torna a rota direta e a guarda continua recusando destino privado. Entradas com CIDR (`169.254/16`, `10.0.0.0/8`) não casam, e não fazem falta: destino nessas faixas é recusado pela guarda de qualquer forma. `ExcludeSimpleHostnames` é ignorado (todo destino do host é nome completo).
- **`HTTPSProxy` ausente cai para `HTTPProxy`.** É o que o CFNetwork faz para URL https e o que a configuração corporativa costuma usar (só "Web Proxy (HTTP)" preenchido); porta ausente usa 80/443.
- **PAC ligado ⇒ nenhum proxy desta fonte; SOCKS ligado sem HTTP/HTTPS ⇒ rota direta.** Um arquivo PAC decide por URL e o cliente não interpreta PAC, então usar o proxy manual ao lado dele seria adivinhar o que o PAC manda. SOCKS é ambiente da máquina, não instrução para este app, e falhar fechado quebraria quem tem SOCKS ligado para o navegador e cuja rede aceita conexão direta. Os dois casos ficam nomeados no diagnóstico (#182), para o timeout não virar mistério. É deliberadamente diferente do `socks5://` no ambiente, que falha fechado porque ali o operador nomeou um proxy para o app.
- **Leitura por requisição, sem cache.** 10 ms num pedido de rede é irrelevante, e mudar o proxy no System Settings passa a valer na requisição seguinte, sem relançar o app. `scutil` ausente, com exit não-zero ou texto ilegível ⇒ sem proxy do sistema (direto, como hoje), sem timeout próprio para uma chamada local.
- **Guarda inalterada.** A rota vinda do sistema passa pelo mesmo `require_public_target` e pelo mesmo `GuardedResolver` do #51: destino privado/loopback/link-local é recusado antes de qualquer conexão e o endereço do proxy é isento. Nenhuma lista nova de endereço privado; o `probe_enabled` do seam de instalador virou função compartilhada em `netguard` em vez de duplicar o gate.
- **Seam só de desenvolvimento, com gate de release.** `OPENRIDR_SYSTEM_PROXY=off` desliga a fonte; um caminho de programa substitui o `scutil` **só** em build de debug (ou com `OPENRIDR_ENABLE_PROBE=1`, que em release não existe); em release só `off` é aceito e qualquer outro valor é ignorado. O harness de teste purga a variável e seta `off`, então `cargo test` deixa de depender da configuração de rede da máquina.
- **Fora de escopo, declarado.** Campo "Proxy" nas Settings, Windows (WinINET/registro) e Linux (`gsettings`), PAC/WPAD, SOCKS, credenciais do proxy do sistema via Keychain, CIDR nas exceções e cache da leitura. Sem dependência nova, sem UI nova, sem `data-testid` novo.
- **Consequência.** `docs/SPEC-SYSTEM-PROXY.md`; tickets #181 (leitor + rota + seam) e #182 (diagnóstico), sub-issues de #132; evidência em `docs/evidence/system-proxy.md`; `GLOSSARY.md` ganha **System proxy**.

## 2026-11-07 — O gatilho do menu de branches é um botão; o nome acessível é uma função pura (issue #168)

- **O `<summary>` sai.** O WKWebView expõe `<summary>` de `<details>` como `disclosure` com `interactive: false` e **sem `ref_id`** (medido no retro da #114 e na tabela de falhas do `test-app-agent-desktop`), então o único caminho para **Complete merge** e **Abort merge…** não existia para quem usa leitor de tela ou ferramenta dirigida por acessibilidade. O gatilho passa a ser `<button type="button">` com `aria-expanded`, o mesmo padrão de `RowMenu` e `SolutionToolbar`. Alternativa recusada: manter `<details>` e pôr `role="button"` no `<summary>` — depende de o WebKit honrar override de role em `<summary>` dentro de `<details>`, o que não foi medido; botão real é o que já aparece com `ref_id` nas receitas.
- **Sem `aria-haspopup`.** O popover não é um `menu`: tem busca, formulário de nova branch e lista. Declarar `menu` prometeria navegação por setas que não existe; `aria-expanded` é o que diz aberto/fechado.
- **O nome acessível é uma função pura do status do Git**, e a **mesma string** vai em `aria-label` e `title` (leitor de tela e tooltip não divergem). Durante merge ela acrescenta ` · merge in progress — complete or abort it here`, que é o que a issue pede: o estado e onde resolvê-lo. O prefixo `Current branch: …` fica, porque o teste E2E existente procura por ele.
- **O popover deixa de ser `<details>`.** O estado React que já existia controla a montagem; saem o `onToggle`, o `removeAttribute("open")` e a ref do `<details>`, e o caminho da command palette que abre o menu fica mais simples (nenhum DOM nativo para sincronizar). Escape e clique fora continuam fechando, agora com listener próprio do componente — o handler global do workspace só casa `details.toolbar-menu`.
- **Teste: função pura com `bun test`, prova de UI no app real.** Decisão do Helder: nada de cenário Playwright novo. O "a test covers it" da issue é atendido pelo teste da função (com e sem merge, falha-antes/passa-depois) mais a prova no app Tauri real por `agent-desktop` (gatilho como botão com `ref_id`, nome com merge, `aria-expanded` e as duas ações por papel e nome). Ordem de tabulação e leitor de tela real continuam não verificados: o `agent-desktop` só vê `webarea` no conteúdo web.
- **Os outros três `<details>` da toolbar ficam como estão** (projeto, workspace actions, run config): mesmo defeito de árvore de acessibilidade, mas não são caminho para ação exclusiva. Ficam numa issue filha de follow-up.
- **Consequência.** `docs/SPEC-BRANCH-MENU-A11Y.md`, ticket filho de #168, evidência em `docs/evidence/branch-menu-a11y.md`; nenhuma dependência nova, nenhum `data-testid` renomeado.

## 2026-11-07 — Os três menus restantes da toolbar são botões; o estado de abrir/fechar fica num componente só (issue #177)

- **O `<summary>` sai dos três.** Projeto (soluções recentes), workspace actions e run configuration tinham o mesmo defeito que a #168 mediu no menu de branches: o WKWebView expõe `<summary>` de `<details>` como `disclosure`, `interactive: false`, **sem `ref_id`**. O gatilho passa a ser `<button type="button">` com `aria-expanded`, e o popover é montado condicionalmente. Alternativa recusada: `role="button"` no `<summary>` — depende de o WebKit honrar override de role dentro de `<details>`, o que não foi medido.
- **Um componente compartilhado, não três cópias.** `ToolbarMenu` (molde do `RowMenu`) é o único sítio de `useState`/`useEffect`/refs; o popover vem como render prop que recebe `close()`, e é assim que os seis botões internos que hoje fazem `closest("details")?.removeAttribute("open")` passam a fechar. Motivo decisivo: o `RunConfigWidget` tem `return` antecipado no caso desabilitado, e estado no widget seria a **mesma armadilha de hook condicional** que deixou a janela em branco na #168 (retro daquela frente). Auto-fechar em qualquer `<button>` do popover foi recusado: os `<select>` de configuração/TFM e o `sdk-redetect` não fecham hoje e não passam a fechar.
- **`pointerdown`, popover condicional, sem Tab-close, sem `aria-haspopup`.** Os quatro pontos copiam a #168 para que os quatro menus da toolbar se comportem igual; `aria-haspopup` prometeria navegação por setas que o popover não tem (ele tem `<select>`, formulário e lista).
- **Nomes acessíveis e `data-testid` idênticos aos de hoje**, sem função pura nova: `Project: …`, `Workspace actions` (com `title="More actions"`, como está) e `Run configuration: …`. A issue é sobre o papel do elemento; mudar texto ou tooltip é mudança visível. Consequência: **nenhum teste automatizado novo** — não há seam puro, o repo não tem infraestrutura de DOM e a #168 já decidira não adicionar uma.
- **O handler global de clique-fora do workspace sai.** Ele só casava `details.toolbar-menu[open]` e, com o menu de branches já convertido na #168, passaria a casar zero elementos.
- **O `BranchWidget` não migra.** O `menuOpen` dele é aberto de fora pela command palette (`intent`) e o `closeMenu` limpa erro e formulário de nova branch; caber no componente exigiria modo controlado e hook de fechamento — API nova em código recém-provado, sem ganho.
- **Locators de E2E atualizados na mesma mudança, sem rodar Playwright.** Dez sítios usavam `getByTestId("workspace-actions-menu").locator("summary")`; passam a `getByRole("button", { name: "Workspace actions" })`. Atualização mecânica de contrato de DOM (lição do retro da #168), não cenário novo; Playwright é smoke noturno (#67), então fica como não verificado.
- **Consequência.** `docs/SPEC-TOOLBAR-MENUS-A11Y.md`, tickets filhos de #177, evidência em `docs/evidence/toolbar-menus-a11y.md`; nenhuma dependência nova, nenhum `data-testid` renomeado, nenhum `role="tab"` tocado.

## 2026-11-07 — O item Commit… do menu de workspace actions executa a ação que ele mostra (issue #190)

- **A fiação volta a ser por item, com o `close()` do `ToolbarMenu`.** O `onClick` de `git-commit-action` chama `close()` e `commitAction.execute()`, como os vizinhos que a #177 converteu. Auto-fechar em qualquer `<button>` do popover continua recusado pela decisão da #177 (os `<select>` de configuração/TFM e o `sdk-redetect` não fecham hoje), então esta frente não reabre o `ToolbarMenu`.
- **A ação não é duplicada.** `git.commit` continua definida uma vez na registry (`execute: () => openCommitPanel()`) e serve keymap, Find Action, menu nativo e agora o botão; o defeito era só a fiação, não a ação.
- **`sdk-redetect` e `close-workspace` ficam como estão.** O primeiro foi deixado aberto de propósito na #177 (mostra o inventário recém-detectado) e o segundo fecha o workspace inteiro, desmontando o menu com ele.
- **O critério "um teste cobre o caso habilitado" é substituído pela prova no app real** (decisão do Helder no grill): nada de cenário E2E novo, nada de Playwright — é smoke noturno desde a #67. A mudança é uma linha de fiação sem seam puro (não há função a extrair como o `branchTriggerLabel` da #168) e o repo não tem infraestrutura de DOM. A prova é `agent-desktop` no app Tauri real cobrindo os dois critérios de aceite visíveis (habilitado abre o painel e fecha o popover; restrito continua desabilitado com o mesmo `title`), registrada em `docs/evidence/commit-menu-action.md`.
- **Consequência.** `docs/SPEC-COMMIT-MENU-ACTION.md`, ticket filho de #190, nenhuma dependência nova, nenhum `data-testid` renomeado ou criado, nenhuma mudança de CSS ou de host.

## 2026-11-07 — Entrada de zip com caminho absoluto continua recusada depois do zip 9 (PR dos majors de dependência)

- **A decisão.** `extract_zip` recusa entrada cujo nome cru começa por componente `RootDir`/`Prefix` (`/etc/passwd`, `C:\…`), em vez de aceitar o nome reescrito que o `zip` 9 devolve. Motivo: o `zip` 2.6.1 devolvia `None` em `enclosed_name()` para entrada absoluta e o host recusava; o `zip` 9 descarta o `/` inicial e devolve o resto como caminho relativo, então a mesma entrada passaria a ser **extraída** com o nome reescrito. A contenção sempre esteve garantida pela guarda `starts_with(target)`, mas o contrato documentado no código e no teste `hash_version_corruption_and_traversal_are_rejected_without_promotion` é de recusa, e afrouxar isso de carona num bump de dependência não é decisão de sessão. Alternativa recusada: aceitar o comportamento novo do `zip` 9 (entrada absoluta vira arquivo dentro do staging) e mudar o teste para asserir só contenção.
- **Como.** O nome decodificado é lido uma vez por entrada — `name()` é falível no `zip` 9 (`ZipResult<Cow<str>>`) — e o teste de componente roda sobre ele; `enclosed_name()` continua sendo a checagem de `..` e de nome que não decodifica. As mensagens de recusa passam a mostrar o nome decodificado em vez de `Ok("…")`.
- **Consequência.** `crates/host/src/debugger.rs` (`extract_zip`); o teste existente cobre o caso, sem teste novo. Nenhum `data-testid`, nenhum contrato novo do host, nenhuma dependência além do próprio bump.

## 2026-11-07 — Atualização dentro do app pelo updater do Tauri (issue #55)

- **Decisão (grill da #55, rodada 1: Q1–Q8 e Q10–Q13 aprovadas pelo orquestrador; Q9 com senha por decisão do Helder).** O app instalado passa a checar um manifesto assinado publicado junto das releases, avisar, e — depois da confirmação — baixar o `.app` novo, verificar a assinatura, trocar o bundle e reiniciar. O artefato sai do **mesmo** `scripts/release-macos.sh` que assina e notariza o DMG: o app atualizado é o app do DMG, não um segundo build.
- **Fonte do manifesto: release-canal de tag fixa.** `latest.json`, `OpenRidr.app.tar.gz` e `.sig` vivem na pré-release `updater-alpha`, re-uploadados com `--clobber` a cada versão; o endpoint é `https://github.com/Heldinhow/openridr/releases/download/updater-alpha/latest.json`. Motivo: toda release do projeto é pré-release, e `releases/latest/download/latest.json` só resolve a última **não**-pré-release. Recusados: consultar a API do GitHub em runtime (código novo, 60 req/h sem token) e publicar as alphas como não-pré-release (muda o significado da release).
- **Um canal, sem downgrade.** Semver do próprio plugin (`update > current`): `0.3.0-alpha.12 > 0.3.0-alpha.11` e, quando a estável sair, `0.3.0 > 0.3.0-alpha.12`. Sem canal stable/beta, sem escolha na UI, sem downgrade.
- **`requireSignedVersion: true`.** O plugin 2.14 recusa manifesto cuja `version` não é a versão gravada no trusted comment da assinatura; sem isso um manifesto adulterado pareia versão inflada com o artefato antigo genuíno. Não havia usuário do updater antes desta frente, então nenhuma release legado sem versão assinada precisa ser rejeitada; a assinatura manual passa `--app-version`.
- **Uma dependência Rust, zero JS.** `tauri-plugin-updater` 2.14.1, que casa com `tauri` 2.12.5 e `reqwest` 0.13.5 já no lock (sem reqwest duplicado). Nada de `@tauri-apps/plugin-updater` nem `plugin-process` (o reinício é `AppHandle::restart()`) e nada de `tauri-plugin-dialog` (a confirmação é a UI do app).
- **Três comandos no shell, num módulo próprio.** `src-tauri/src/update.rs`: `update_check`, `update_install` e `restart_app`. O host não participa (o plugin precisa do `AppHandle`) e a checagem **não** passa por trust/restricted mode: é chamada de rede do shell, não código do projeto.
- **Gate de build empacotado.** Fora de `<Algo>.app/Contents/MacOS/` a checagem responde `unsupported`: `tauri dev` e a ponte HTTP de dev rodam binário solto, e o plugin trocaria um diretório que não é bundle. Função pura sobre o caminho, com teste (falha-antes medido: sem o gate, o caso do binário solto reprova).
- **Sem prazo total, só teto de conexão.** O timeout do plugin vale para a requisição **e** para o download (o mesmo `Update` carrega o valor), então um prazo total mataria um download legítimo de ~63 MB em link lento. Fica `connect_timeout` de 15 s. Limite declarado: servidor que aceita a conexão e para de mandar deixa a checagem pendente e o download parado no último percentual; não há botão de cancelar, e fechar o app cancela.
- **Confirmação na UI, com progresso.** Modal com versão, notas roláveis, **Download and install** / **Later**, barra de progresso (evento `openridr-update-progress` com `{downloaded,total}`, no máximo a cada 200 ms e uma vez no fim) e, ao terminar, **Restart now** / **Later**. Checagem silenciosa ~3 s depois do primeiro paint (falha só no log) e o item nativo **Check for Updates…** no menu do app, que checa na hora e mostra "já é a mais nova", "não disponível neste build" ou o erro.
- **Reiniciar reaproveita o fluxo de quit.** "Restart now" pede o mesmo quit do ⌘Q (`restartOnQuit` na UI): com buffer sujo aparece o prompt de salvar/descartar, e o `quit_app` final vira `restart_app`, que fecha o workspace do host, aprova a saída e chama `AppHandle::restart()` (no macOS relança pelo LaunchServices, `open -n`, então o processo novo lê o bundle já trocado). `app.restart()` direto descartaria buffer não salvo em silêncio.
- **Artefato depois do grampo.** `release-macos.sh` empacota o `.app.tar.gz` a partir do `.app` **já grampeado** (o ticket de notarização é o arquivo `Contents/CodeResources`, não um xattr), assina com `tauri signer sign --app-version` e verifica extraindo: `codesign --verify --deep --strict`, `spctl --assess` e `stapler validate` no app extraído. `bundle.createUpdaterArtifacts` fica desligado porque arquiva no build, **antes** da notarização.
- **Bug que a prova real achou: AppleDouble no arquivo.** O `tar` do macOS gravava `._OpenRidr.app` para os xattrs do bundle e o leitor tar do plugin **não** funde essa entrada de volta — o install morria em `failed to unpack '._OpenRidr.app'`. O `bsdtar` esconde essas entradas ao listar e ao extrair, então `tar -tzf` mostrava o arquivo limpo. Correção: `tar --no-mac-metadata` no empacotamento e, na verificação, ler o primeiro header do fluxo cru e recusar a release quando ele começa com `._`; `release-selfcheck.sh` planta um xattr no bundle do fixture e falha sem a flag (medido).
- **O pubkey é conferido, não digitado.** O valor do `tauri.conf.json` foi copiado do arquivo da chave por comando, e `scripts/updater-pubkey-selfcheck.sh` compara o conf com `~/.tauri/openridr-updater.key.pub` e com a cópia commitada no script (fallback para CI/máquina sem a chave). Um pubkey errado não reprova build nem release: faz todo app recusar todo update em silêncio. Falha-antes medido com uma pública válida de outro par.
- **Chave própria, com senha, fora do repo.** Par em `~/.tauri/openridr-updater.key` + `.password` (0600); só a pública vai no `tauri.conf.json`. O workflow recebe `TAURI_SIGNING_PRIVATE_KEY` e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; o preflight do `release-macos.sh` exige os dois nomes (nomes apenas) e o `release-selfcheck.sh` cobre a ausência (falha-antes medido contra o script anterior, que publicava sem a chave). Perder a chave ou a senha significa que nenhum update futuro funciona.
- **Proxy e TLS herdados do reqwest.** `rustls-platform-verifier` (raízes do Keychain, CA corporativa instalada funciona) e `system-proxy` (variáveis de ambiente **e** Proxies do System Settings do macOS, via hyper-util). A guarda `netguard` do host não se aplica: o updater só fala com github.com e com o host do artefato na mesma release. Sem ponte, sem configuração nova.
- **Publicação em ordem.** O workflow publica a release da versão primeiro e o canal depois: artefato, assinatura e `latest.json` por último, para um cliente nunca ler uma URL que ainda não existe.
- **Testids novos, nenhum renomeado.** `update-dialog`, `update-version`, `update-notes`, `update-progress`, `update-install`, `update-later`, `update-restart`, `update-error`, `update-check-result`.
- **Correções da revisão independente (rodada 1).** Cinco achados, todos reais. (a) **Alto: nada provava que o artefato foi assinado pela chave que o app confia.** O release assinava e verificava a assinatura Apple, mas um secret registrado com outra chave publicaria um artefato que todo app instalado rejeita — com o DMG perfeitamente válido. `verify_updater_signature` passou a comparar o *key id* minisign (8 bytes no offset 2 do blob, tanto na pública quanto na assinatura) com o da `pubkey` do `tauri.conf.json`; o `release-selfcheck.sh` assina com a chave certa e com uma de outro par, e reprova a segunda (falha-antes medida desligando a comparação). (b) **Médio: o arquivo do canal era mutável.** Trocar `OpenRidr.app.tar.gz` no canal antes do manifesto deixava o manifesto antigo apontando para bytes novos com assinatura antiga. O artefato passou para a release da versão (URL imutável) e só o `latest.json` mora no canal. (c) **Médio: checagens sobrepostas podiam trocar o update depois da confirmação.** A UI serializa as operações (`busy`) e `update_install` recebe a versão mostrada e recusa outra. (d) **Baixo: o erro da checagem de startup não deixava rastro.** O comando agora escreve a frase no stderr; a spec deixou de prometer "log" que não existia. (e) **Baixo: a spec pedia o tamanho antes da confirmação.** O tamanho só existe quando o `Content-Length` chega; a spec passou a dizer isso em vez de prometer um número que não há.
- **Re-check da revisão (rodada 2).** Cinco achados, quatro corrigidos e um declarado. (a) Fechar o modal durante uma checagem liberava a trava e uma segunda checagem podia sobrepor — `checking`/`installing` passaram a ser refs separadas, e só o fim da requisição limpa a primeira. (b) O erro ao construir o updater (config inválida) não era logado — as duas falhas passam pela mesma closure que escreve no stderr. (c) O modal mostrava só a porcentagem quando o total era conhecido — agora mostra `% de <tamanho>`. (d) O comentário da conferência de chave prometia verificação criptográfica; passou a dizer o que ela é (identidade pelo key id; a verificação de verdade é a do app na instalação). (e) **Declarado, não corrigido:** republicar uma versão já publicada re-uploada o artefato com `--clobber` (a mesma política do DMG) e, entre esse upload e a troca do manifesto, o manifesto antigo aponta para bytes novos com assinatura antiga — quem checar nessa janela baixa e recusa. Aceito e escrito na spec e em `release-signing.md`: a alternativa (versão publicada imutável) mudaria a política de republicar tag que o projeto já tem, e a assinatura continua sendo o que decide o que instala.
- **Consequência.** `docs/SPEC-IN-APP-UPDATER.md`, tickets #195 (shell), #196 (release), #197 (UI) e #198 (prova + evidência), evidência em `docs/evidence/in-app-updater.md`, retro em `docs/RETRO-IN-APP-UPDATER.md`; `GLOSSARY.md` ganha **Update channel**, **Update manifest** e **Update artifact**.

## 2026-11-07 — A superfície de verificação de UI segue o que mudou; o E2E usa portas por sessão (issue #210)

- **Decisão do Helder (2026-11-07).** Mudança de interface pura se verifica com **Playwright em modo web** — só os specs da área, escrevendo/atualizando o spec do cenário. O **app Tauri real com `agent-desktop`** fica para o que é nativo do shell: menu nativo, atalhos do sistema, updater, seletor nativo, árvore de acessibilidade do WebKit, single-instance e DMG. O **host isolado** (`web.sh`) continua para o que não tem spec ou precisa de captura. Motivo: a regra antiga mandava abrir o app real para provar uma mudança de CSS e empurrava toda verificação de UI para a superfície mais cara e menos determinística.
- **O que a #67 decidiu continua valendo.** O smoke noturno é o único E2E automatizado, nunca roda em PR, e a quarentena segue com motivo por spec (`docs/evidence/e2e-quarantine.md`). A decisão nova muda quem roda Playwright na sessão, não o que o CI roda.
- **Convenção de portas, sem helper.** Default 1420 (UI) / 4317 (host); sessão que roda em paralelo, ou junto com `bun run dev`, escolhe outro par (faixa 15xx/44xx) e confere com `lsof -nP -iTCP:<porta> -sTCP:LISTEN`. Porta ocupada falha na hora pelo `strictPort` do Vite de teste. Nada de script que "acha porta livre": escolher duas portas é a única decisão da sessão. A faixa é do par inteiro: cada spec que sobe host próprio usa `hostPort + 1` até `hostPort + 7`, então o par escolhido precisa de 10 portas de host livres acima dele. *(Substituído em 2026-11-08 pela #246: as auxiliares são efêmeras; só o par do harness é escolhido.)*
- **Consequência.** `docs/SPEC-E2E-PORTS.md`; issues #211 (app home derivado da porta da UI), #212 (esta documentação) e #213 (os quatro sítios de `workspace.spec.ts` que a #177 deixou vermelhos); evidência em `docs/evidence/e2e-ports.md`; `AGENTS.md`, `.agents/skills/verify-openridr/SKILL.md`, `scripts/e2e-smoke.txt` e `docs/evidence/e2e-quarantine.md` deixam de repetir a regra antiga. Nenhuma dependência nova, nenhum `data-testid`, nenhuma mudança de código.

## 2026-11-07 — Três buracos medidos do CI fechados sem dependência nova e sem tocar no ruleset (issue #142)

- **A decisão.** A #142 pedia quatro frentes; três tinham buraco medido e entram, a quarta já estava
  resolvida. Nada de linter de TS, nada de `knip`, nada de filtro de job por caminho, nenhuma mudança
  em `.github/rulesets/**` ou em required check. O que entra é o menor artefato que fecha cada buraco,
  com o número que o justifica.
- **Type check estrito em vez de linter (decisão do Helder).** `noUnusedLocals` e
  `noUnusedParameters` entram no `tsconfig.json`: 17 achados reais hoje (14 variáveis e 3 parâmetros),
  zero dependência nova e **zero deriva possível entre local e CI**, porque as flags moram no arquivo
  que o `tsc --noEmit` do CI já lê — nenhum passo novo no workflow, nenhum script novo no
  `package.json`. Recusado agora: Biome, com o baseline medido para a issue filha — 175 achados com
  recorte de regras de bug real (138 deles `useExhaustiveDependencies`; 37 sem ela), 5
  `useHookAtTopLevel` que são **todos falso positivo em spec Playwright** (Biome confunde o callback do
  `test()` com componente React), 1,1 MB de JS + 81 MB de binário de plataforma, e a config default
  inútil (596 erros + 407 warnings, mais 1127 `noReactNativeRawText`). `noUncheckedIndexedAccess`
  fica registrado como próximo degrau com o número: 120. Cinco dos 17 achados estão em specs de E2E.
- **`src-tauri` em job novo e paralelo, não em passo do job Rust.** O job Rust é o caminho crítico
  (3:38–4:21 de uma parede de ~4:20); somar 2–4 min a ele levaria todo PR a ~7 min, enquanto um job
  paralelo não muda a parede. Custo aceito: ~35 linhas de YAML, uma entrada no `needs` do agregador e
  um `rust-cache` próprio. **O `cargo check` de `src-tauri` falha em checkout limpo** — o `tauri-build`
  valida os resources do bundle e `resources/csharp-ls/` e `resources/dotnet/` são vendor dirs
  gitignored —, então o job cria as duas pastas vazias e comenta por quê. Recusado: mexer no
  `tauri.conf.json` para tornar os resources condicionais (mudança de empacotamento por causa de CI) e
  rodar os vendor scripts no CI (centenas de MB e rede para satisfazer uma checagem de existência).
- **Fase informativa com critério escrito, não com memória.** `continue-on-error: true` no job faz o
  `result` dele virar `success`, então o agregador `ci` continua verde e **o ruleset não muda**. A
  promoção é remover o `continue-on-error` num PR próprio, depois de 5 execuções verdes consecutivas
  (uma delas com cache frio), duração estável abaixo de 5 min e nenhum flake — e continua sem exigir
  mudança de setting, porque o check obrigatório já é o agregador.
- **Auditoria de dependências semanal, fora do caminho do PR.** Um advisory publicado hoje não pode
  parar o PR de amanhã, e gitleaks (segredo) e Dependabot (bump) não são auditoria. O workflow não
  entra no `needs` do agregador, então não bloqueia merge nenhum. `bun audit` já vem no bun 1.3.9 que
  o CI fixa (0,4 s, zero dependência); `cargo audit` 0.22.2 com versão fixa roda duas vezes, porque a
  raiz e `src-tauri` são workspaces com lockfiles separados. Baseline medido: **0 vulnerabilidades**
  nos dois; 1 aviso unmaintained na raiz (`rustls-pemfile`) e 6 em `src-tauri` (glib/zvariant, deps de
  Linux), que são impressos e não falham — transformar aviso em gate faria o job nascer vermelho.
- **Falha de scanner é vermelho, nunca "limpo".** Sem `continue-on-error`, sem `|| true`, sem cache do
  advisory-db. Medido: `bun audit` com a rede fora sai 1 com "audit request failed"; `cargo audit` com
  banco inacessível e `--no-fetch` sai 1 com "error loading advisory database". Exceção é `--ignore`
  com motivo e data de revisão na mesma linha, e **nada executa essa data** — limitação declarada, não
  promessa.
- **Gatilho de PR filtrado por caminho no workflow da auditoria.** `schedule` e `workflow_dispatch` só
  rodam a partir da branch default, então sem um terceiro gatilho o PR que introduz o workflow não
  teria como prová-lo (e a #142 exige run registrado). O filtro é `bun.lock`, os dois `Cargo.lock` e o
  próprio arquivo, então o custo em PR normal é zero e a dependência nova é auditada no PR que a
  introduz.
- **Item 4 da #142 é verificar, não construir.** Caminho desconhecido, falha de classificação, arquivos
  sensíveis, alteração do próprio gate e mudança do head depois da aprovação já estão cobertos pelos
  quatro testes de shell, que passam. O residual é a #139 (o YAML do PR roda), que exige ruleset.
  **Sem filtro de job por caminho**: medido, o job Rust é 3:38–4:21 de 4:20 de parede, então filtrar
  não economiza nada num PR que toca Rust e cria risco de pular validação em silêncio.
- **Consequência.** `docs/SPEC-CI-PRACTICES.md`, tickets filhos de #142, evidência em
  `docs/evidence/ci-practices.md`; `GLOSSARY.md` ganha **Job informativo** e **Auditoria de
  dependências**. Nenhuma dependência nova, nenhum `data-testid`, nenhuma mudança de produto.

## 2026-11-07 — O host auxiliar do E2E prova a posse pelo `pid`; porta efêmera fica como follow-up (issue #216)

- **A decisão.** Os sete specs que sobem host próprio (`trust-prompt`, `welcome`, `medium-editor-session`, `debug-advanced`, `tools-install-ux`, `analysis-gate`, `setup-gate-layout`) passam a obter o host por `tests/e2e/isolated-host.ts`, e o helper só aceita o `/health` cujo `pid` é o do filho que ele mesmo subiu. Porta ocupada por outro processo deixa de ser sucesso silencioso: vira falha imediata com a porta e o pid estranho na mensagem. O helper também assume o que os sete repetiam — o `page.route` de `/__host/(call|health)` e o ciclo de vida do `OPENRIDR_HOME` temporário (`stop()` mata o filho e apaga o home).
- **Por que `pid` e não porta efêmera.** O `pid` responde a pergunta que o spec faz ("o processo que respondeu é meu?") sem tocar em produto: nenhuma linha de `crates/`, `src/` ou `src-tauri/`, nenhuma dependência nova, e a convenção de `hostPort + 1` … `hostPort + 7` de `AGENTS.md`/`SPEC-E2E-PORTS` continua verdadeira. Porta efêmera (`--port 0`) mataria a classe do problema, mas exigiria o `host-http` imprimir a porta concedida (`server_addr()`), reescrever a convenção de 10 portas em três docs e tornaria indemonstrável o critério "porta ocupada falha nomeando a porta" — e o PR deixaria de ser `risk:low`. Fica como follow-up da retro, sem issue agora. *(Virou a #246, feita em 2026-11-08: o host auxiliar sobe em porta efêmera.)*
- **O `cargo run` faz `exec`: `kill` no filho mata o host.** A nota de "host órfão" em `implementation-notes.md` (welcome-trust, 2026-11-01) estava errada e foi corrigida nesta frente: medido, o `/health` devolve o mesmo pid do `spawn` e `kill()` no filho derruba o host. Não há `lsof -ti tcp:… | xargs kill` a fazer entre rodadas.
- **Três falhas explícitas, sem retry.** Filho saiu antes de responder (código de saída, com o pid estranho se um `/health` respondeu), `/health` com pid diferente do filho (os dois pids) e timeout de 300 s (a porta). Em qualquer uma o helper mata o filho e apaga o home antes de lançar; esperar a porta "ficar livre" foi recusado porque mascara a colisão em vez de nomeá-la.
- **O teste da garantia é um decoy, fora do smoke.** `tests/e2e/isolated-host.spec.ts` sobe um decoy que responde `/health` com pid alheio e assere a recusa com a porta na mensagem, além do caminho feliz e do `stop()`. Não entra em `scripts/e2e-smoke.txt`: a lista é curada pela #67 e acrescentar spec lá é outra decisão.
- **O que o `pid` não prova.** Que o host está saudável — um host do próprio spec que subiu e travou responde `/health` igual. A prova de saúde continua sendo o teste do spec.
- **Consequência.** `tests/e2e/isolated-host.ts` e `isolated-host.spec.ts` (#233), os sete specs migrados (#234), esta documentação (#235), evidência em `docs/evidence/e2e-host-ownership.md`. Nenhum `data-testid`, nenhum contrato do host, nenhuma dependência nova.
## 2026-11-07 — Cancelar push, update e fetch: o Git de rede entra no padrão assíncrono do Build (issue #150)

- **Decisão (do Helder, depois de quatro rodadas de review reprovarem o desenho síncrono).** `git.push`, `git.pull` e `git.fetch` deixam de ser chamadas síncronas e passam a ser **start**: validam só o que é barato (trust, workspace aberto, `confirm: true` no push), registram a operação com um **id emitido pelo host** e devolvem `{ "id": "git:<n>" }` na hora; o trabalho que depende do repositório roda numa thread com o prazo da #136 (push 120 s, fetch e o fetch do Update 60 s). O estado publicado vai no **snapshot** que a UI já consulta a cada 400 ms (`git: { id, kind, state: idle|running|ok|failed|cancelled, message }`), como `build`/`run`/`tests`. `git.cancel { id }` cancela **só** a operação que o host emitiu com aquele id, e só enquanto o passo de rede dela está em voo. O escopo inclui o **Fetch**: é a mesma espera de 60 s e o mesmo caminho no host.
- **Por que o desenho anterior caiu.** Ele mantinha o Git de rede síncrono e abria duas exceções para o cancelamento caber: um slot com o `opId` inventado **pelo cliente** e uma lista de métodos que furam o `HOST_CALL`. As quatro rodadas acharam sempre a mesma classe de bug — cancel aceito sem efeito, ou cancel que atinge a operação errada — porque a identidade da operação nascia no cliente e o host não distinguia "ainda não começou" de "já terminou". Cada correção (retry, slot no pre-flight, transição atômica, `opId`) mudou a corrida de lugar em vez de eliminá-la. Com o id nascendo no host e o estado publicado por ele, o cancel só pode nomear uma operação que existe, e a pergunta "ainda dá para cancelar?" tem uma resposta só, no host.
- **Nenhum mecanismo novo.** É o padrão que o Build, o Run e o Tests já usam (start devolve id, estado consultável, cancel nomeia o id); o Git de rede passa a ser mais um consumidor dele. Alternativa recusada: um `git.wait { id }` para a UI continuar com `await` — ou segura o lock de novo (o problema original) ou precisa da mesma exceção que acabou de ser reprovada.
- **O fim do passo de rede e o cancelamento são uma transição só.** O registro nasce no start (antes de qualquer trabalho, então um cancel durante o `git status` de um repositório grande não responde "nada para cancelar" com a operação por começar) e o flag de cancelado é lido e limpo sob o **mesmo mutex** que o `cancel` usa, no instante em que o filho do `git` termina. Assim o cancel ou chega antes (a operação publica `cancelled`; no Update, antes de qualquer merge) ou depois (o host responde `cancelled: false`, porque o passo de rede já não está em voo). É o que faz o merge local do Update não ser cancelável sem uma regra à parte.
- **O kill é do laço que já tem o `Child`.** O cancel só levanta o flag; `run_with_deadline` observa a cada `FETCH_POLL` (20 ms) e chama o mesmo `kill_tree` do prazo. Sinalizar o pid de outro dono reabriria a corrida de reuso de pid que o comentário do `kill_tree` resolve ("o líder não foi colhido"), e o custo de não fazer isso é uma latência de até 20 ms.
- **`run_with_deadline` continua devolvendo o motivo, não um booleano.** `(Output, Stop)` com `Stop::{No, Deadline, Cancelled}`: a frase do prazo ("did not answer within N seconds") seria mentira num cancelamento.
- **A exceção do lock do shell e a fila da ponte de dev saem por serem código morto.** Lido o código: `host_call` toma `HOST_CALL` durante o `dispatch` inteiro, e o `git.push` passava até 120 s lá dentro — daí a fila de todo mundo e a necessidade de um caminho fora do lock. Com o start, o `dispatch` dura o registro da operação e o trabalho de rede roda numa thread que ninguém enfileira; o `git.cancel` é chamada comum, sem nada para ultrapassar. A fila FIFO com um worker da ponte `host-http` existia pela mesma razão (furar a fila sem perder a ordem de `lsp.didChange`/`session.write`); sem chamada longa, o laço que atende uma requisição por vez preserva a ordem por construção. Saem `UNSERIALIZED_METHODS`/`bypasses_host_lock` (e o teste que os fixava), o ramo do lock no shell Tauri e a fila/overtake da ponte.
- **O start valida só o barato.** Trust, workspace e `confirm: true` continuam síncronos (contrato de segurança e de API, falham sem tocar a rede); abrir o repo, `git status`, achar o remoto e a árvore suja rodam na thread e aparecem como `failed` com a mensagem do app. Motivo: o `git status` de um repositório grande é um dos passos que travava o app e não pode segurar o lock. Um segundo start com operação em voo é recusado (`invalid_state`, nomeando a operação), como no `build.start`.
- **Mensagens: mesmas promessas da #136, motivo diferente.** `Push cancelled: the push may already have reached the remote. Check the branch there before pushing again.` / `Update cancelled: nothing was merged; your branch, index and working tree were not touched, and refs already received stay.` / `Fetch cancelled: your branch, index and working tree were not touched; refs already received stay.` O `cancelled` é estado do registro publicado; o contrato de `Operation` não ganha estado novo.
- **O merge local do Update não é cancelável, e a UI diz isso.** Um cancel nessa janela devolve `cancelled: false` e o diálogo mostra `Nothing to cancel: the update is already merging.` (push: `…the push already finished.`, fetch: `…the fetch already finished.`). Matar o merge deixaria merge pela metade (decisão da #136).
- **O diálogo em voo não fecha mais.** Dispensar (`push-cancel`/`update-cancel`) fica desabilitado durante a operação e Escape cancela. O diálogo de Abort merge não muda (abortar é local).
- **Consequência.** `docs/SPEC-GIT-NETWORK-CANCEL.md`; tickets #206 (push + mecanismo), #207 (Update) e #208 (Fetch), sub-issues de #150; evidência em `docs/evidence/git-network-cancel.md`. Nenhuma dependência nova, nenhum `data-testid` renomeado (`push-stop`, `update-stop`, `branch-fetch-stop` são novos).

## 2026-11-07 — Três dependências importadas sem declarar passam a ser declaradas; o fontsource é falso positivo do knip (issue #223)

- **Decisão.** `@lezer/highlight`, `@codemirror/search` e `@codemirror/autocomplete` entram em
  `dependencies` com a faixa `^` da versão que o `bun.lock` já resolve (1.2.5, 6.7.2, 6.20.4). Nenhuma
  versão nova, nenhum pacote novo: as três já estão no lock por transitividade de `codemirror`.
- **Por que declarar, e não trocar o import.** `codemirror` não reexporta o que o editor usa: ele importa
  `searchKeymap`/`highlightSelectionMatches` e `autocompletion`/`closeBrackets`/`completionKeymap` para
  montar o `basicSetup` e não exporta nenhum deles; `tags`, `getSearchQuery`, `gotoLine`,
  `searchPanelOpen`, `insertCompletionText`, `pickedCompletion` e os tipos `Completion*` só existem nos
  pacotes originais. Trocar o import exigiria reescrever o editor e ainda assim declarar dois dos três.
- **O `@fontsource-variable/*` fica, e o knip está errado.** Os dois pacotes são usados em
  `src/styles.css` por `url()` dentro de `@font-face`; o knip não segue `url()` de CSS. Remover quebraria
  as fontes da UI. A exceção é registrada aqui, não em config: o knip não é dependência do repositório e a
  adoção (com a config junto) é a frente adiada de `docs/SPEC-CI-PRACTICES.md`.
- **Sem guarda de regressão nesta issue.** Um teste que varresse os imports de `src/` e comparasse com o
  `package.json` fecharia a classe do bug sem o knip, mas seria um parser de import por regex e a guarda
  de verdade é o knip. A lacuna fica registrada no retro.
- **Consequência.** `docs/SPEC-DECLARED-DEPS.md`, evidência em `docs/evidence/declared-deps.md`. O PR toca
  `package.json` e `bun.lock`, então cai em `risk:high` pela tabela de risco — inevitável para a issue.
  Nenhum `data-testid`, nenhuma mudança de produto.

## 2026-11-07 — O smoke noturno reporta numa issue de rastreio: vermelho abre/atualiza, verde fecha (issue #215)

- **O problema era leitor, não cobertura.** `workspace.spec.ts` ficou vermelho na `main` da #177 à #210
  com o spec na lista do smoke, e no dia desta decisão o smoke estava vermelho de novo
  (`solution-git-status.spec.ts:111`, run 42364384217) sem ninguém ver. A página de Actions não é onde o
  mantenedor olha; a lista de issues abertas é.
- **Uma issue, achada pelo título exato `[nightly] e2e-smoke is red`.** Sem label nova: label é setting
  do repositório, e a sessão não cria setting. A busca é `gh issue list --search 'e2e-smoke in:title'`
  (o marcador com colchetes é sintaxe de busca do GitHub) e o casamento exato do título acontece no
  `jq`. Se mais de uma casar, a de menor número vale.
- **Vermelho reescreve o corpo; não comenta por noite.** O corpo é o último resultado (run, `ref`, SHA,
  data, specs, testes); o histórico de runs já está na página do workflow, e a data de criação da issue
  vira o "vermelho desde".
- **Verde comenta o link e fecha.** Uma run verde com a issue aberta é o mesmo silêncio que a frente
  existe para matar, então falha em fechar deixa o job vermelho.
- **Um job, dois passos (`if: failure()` / `if: success()`), com guarda `main || workflow_dispatch`.**
  Cobre falha de teste, de instalação e de bootstrap, porque um passo com `if: failure()` roda mesmo
  assim. O `workflow_dispatch` reporta de propósito: é o que permite provar os dois caminhos sem esperar
  a noite. Limite aceito e escrito como `shortcut:` no script: timeout de 60 min e cancelamento do job
  não reportam (o plano B é um job `needs: smoke` com `if: always()` e um output).
- **A decisão mora num script, não no YAML.** `scripts/e2e-smoke-issue.sh` decide criar/reescrever/fechar
  e recebe tudo por env (`RESULT`, `RUN_URL`, `REF`, `SHA`, `REPO`, `REPORT`, `GH`), com `GH` apontável
  para um stub — o mesmo seam de `ci-risk-gate.sh`. O YAML fica com a montagem do relatório e a chamada.
- **O relatório é `--reporter=list,json` na linha do CI, não no config.** O log continua legível e o
  script ganha arquivo; o `playwright.config.ts` não muda. Duas armadilhas medidas num relatório real
  (Playwright 1.64.1): suites **aninhadas** também trazem `file`, com valor errado, então o filtro herda
  o `file` da suite de topo; e o `file` é relativo ao `testDir` (`workspace.spec.ts`, não
  `tests/e2e/workspace.spec.ts`), então o corpo diz "relativo a `tests/e2e/`" em vez de o script
  inventar prefixo.
- **`issues: write` no job, `contents: read` repetido nele.** Um bloco de permissões no job substitui o
  de topo em vez de somar. Nenhuma expressão `${{ }}` dentro de `run:` (zizmor), nenhuma action nova,
  nenhuma dependência nova.
- **Prova com runs reais, em branch descartável.** Duas runs vermelhas (uma criou a #244, a outra
  reescreveu o corpo dela sem criar segunda) e uma verde (comentou e fechou), em `workflow_dispatch` na
  `ci/issue-215-proof`, apagada no fim. A primeira noturna agendada depois do merge é a primeira
  execução real do `schedule`.
- **Consequência.** `docs/SPEC-NIGHTLY-SMOKE.md`; tickets #240 (script + teste), #241 (workflow +
  contrato do YAML + `AGENTS.md`) e #242 (prova), sub-issues de #215; evidência em
  `docs/evidence/nightly-smoke.md`. O smoke real continua vermelho por `solution-git-status.spec.ts:111`
  (issue #245), que é spec de UI e não desta frente. Nenhum `data-testid`, nenhuma mudança de produto.

## 2026-11-07 — A captura do E2E sai do repositório por default; o caminho versionado vira opt-in (issue #217)

- **Decisão.** O default do diretório das capturas deixa de ser `docs/evidence/screenshots` e passa a ser
  `<tempdir>/openridr-e2e-screenshots-<porta da UI>`, derivado da porta como o E2E app home da #210;
  `OPENRIDR_SCREENSHOT_DIR` continua vencendo, com valor relativo resolvido contra o diretório atual. O
  caminho versionado só recebe captura quando a variável aponta para ele de propósito.
- **Por que o default era um defeito, e não um incômodo.** O diretório está no `.gitignore`, mas **86 PNGs
  dele estão versionados** — o `.gitignore` não vale para arquivo já rastreado. Uma execução que falhava
  reescrevia evidência versionada (aparecia em `git status`) ou deixava arquivo não rastreado dentro do
  repositório, que é o que faz `git worktree remove` recusar a worktree (o sintoma registrado na #210).
  A saída era lembrar de exportar a variável antes de rodar, e a documentação precisava repetir o aviso.
- **A chave é a porta, não o worktree.** Mesmo critério do E2E app home: duas sessões em portas diferentes
  não se sobrescrevem, e duas na mesma porta não coexistem de qualquer forma (`strictPort` do Vite de
  teste). `mkdtemp` por execução foi recusado porque ninguém acha a captura depois sem o log.
- **Um seam só.** O caminho é calculado no módulo de ambiente do E2E, que já é por onde config,
  `globalSetup` e specs leem portas, home e URLs; os specs importam de lá, inclusive os dois que gravavam
  caminho literal e ignoravam a variável. Specs que só capturam quando pedido usam o mesmo módulo, sem
  ganhar default.
- **`outputDir` e `OPENRIDR_EVIDENCE_LOG_DIR` ficam onde estão.** `test-results/` e `playwright-report/`
  já são ignorados e não sujam `git status`; `docs/evidence/logs` é ignorado e não tem arquivo versionado.
  Mover não resolveria problema nenhum — decisão registrada para não ser reaberta.
- **Consequência.** `docs/SPEC-E2E-EVIDENCE-OUTPUT.md`; tickets #248 (código + guarda) e #249 (docs
  vivos), sub-issues de #217; evidência em `docs/evidence/e2e-evidence-output.md`. A guarda em
  `tests/unit` roda no CI e reprova qualquer spec que volte a nomear o caminho versionado. Nenhuma
  dependência nova, nenhum `data-testid`, nenhuma mudança de produto.

## 2026-11-07 — O validador de porta do E2E recusa porta restrita do Chrome (issue #258)

- **Decisão.** A validação de `OPENRIDR_E2E_UI_PORT`/`OPENRIDR_E2E_HOST_PORT` passa a recusar, no load do
  módulo de ambiente, as portas da `kRestrictedPorts` do Chromium (`net/base/port_util.cc`), com a lista
  copiada inteira e a fonte/data no comentário. A recusa cobre também a faixa derivada `hostPort + 1` até
  `+ 7` que os specs que sobem host próprio usam. *(A faixa derivada saiu em 2026-11-08 com a #246: a
  recusa vale só para as duas portas escolhidas.)* A mensagem mantém o prefixo `Invalid E2E port` e
  acrescenta que o Chrome bloqueia a porta.
- **Por que o erro precisa ser de configuração, não de rede.** O `ERR_UNSAFE_PORT` só aparece no
  `page.goto`, com host e Vite já de pé, e não diz qual das duas variáveis errou — a sessão perde a rodada
  até reconhecer a mensagem (medido com 1720/4617 na sessão da #217). Falhar no load do módulo de ambiente
  transforma isso num erro de configuração no primeiro segundo, antes de qualquer processo subir.
- **Lista exata, não faixa.** A issue dizia "1719–1723"; o Chromium restringe 1719, 1720 e 1723, e aceita
  1721 e 1722. Bloquear a faixa inteira seria mais curto e faria a mensagem mentir, então o teste pina
  1721/1722/1725 como aceitas — é o que prova que a lista foi copiada em vez de inventada.
- **As derivadas entram.** Sete specs usam `hostPort + 1` até `+ 7` e o navegador alcança essas portas
  direto: um `hostPort` de 1716 quebra no `+ 3` exatamente como a issue descreve, só que no meio do run.
  Recusar o par na hora custa poucas linhas e fecha a classe. *(Em 2026-11-08 a #246 tirou as derivadas
  do mapa: o host auxiliar é efêmero e o navegador não conecta nele — quem fala com ele é o Node.)*
- **Teste no CI de PR, não no Playwright.** O teste mora em `tests/unit` (job `typescript`), porque
  Playwright não roda em PR; um subprocesso por variável prova o wiring das duas, e o critério "erro no
  começo do run" fica como evidência executada (`playwright test --list` com porta restrita), não como teste
  permanente.
- **Consequência.** `docs/SPEC-E2E-RESTRICTED-PORTS.md`; ticket #261, filho de #258; evidência em
  `docs/evidence/e2e-restricted-ports.md`. Uma cláusula no skill `verify-openridr`; `AGENTS.md` intocado.
  Nenhuma dependência nova, nenhum script novo, nenhum `data-testid`.

## 2026-11-07 — O doctor mede o espaço livre do `$TMPDIR` com piso de 3072 MiB (issue #259)

- **Decisão.** O `doctor.sh` do skill `verify-openridr` passa a imprimir uma linha com o espaço livre em
  MiB no volume de `${TMPDIR:-/tmp}`, medido com `df -Pk` (coluna `Available`, 1024-blocos ÷ 1024):
  `ok` a partir de 3072 MiB, `note` entre 1024 e 3071, `FAIL` (exit 1) abaixo de 1024. Medição sem
  número vira `note`, nunca `ok`. O script continua somente leitura.
- **Por que o volume do `$TMPDIR`.** É onde o E2E escreve: um app home por porta da UI e o diretório de
  capturas. O volume do repositório (`target/`, `node_modules`) é outra pergunta e ficou fora.
- **Por que 3072/1024 MiB.** Até quatro sessões em paralelo, cada uma com ~200 MiB de app home mais
  caches e builds; o orquestrador já manda sessões pararem abaixo de 3 GB. Os dois números e o motivo
  ficam escritos ao lado do check no script e no parágrafo do Doctor do `SKILL.md`, para a próxima
  sessão não redescobrir de onde vieram.
- **Por que o doctor e não o harness.** O `ENOSPC` aparece numa cópia de arquivo, minutos depois do
  começo do run (medido na #217: 127 MiB livres, `globalSetup` morto no download do csharp-ls). O doctor
  é o passo que existe para descobrir isso antes, e agora ele mostra o número em vez de deixar a sessão
  adivinhar.
- **Teste sem encher disco.** O `df` resolvido pelo `PATH` é o seam: um `df` falso primeiro no `PATH` e
  stubs das outras ferramentas deixam o exit code depender só do caso. Nenhuma variável só-de-teste no
  script.
- **Consequência.** `docs/SPEC-DOCTOR-DISK-SPACE.md`; ticket #267, filho de #259; evidência em
  `docs/evidence/doctor-disk-space.md`; teste em `tests/shell/doctor.test.sh` (rodado à mão, como os
  outros de `tests/shell/`). Nenhuma dependência nova, nenhum `data-testid`, nenhuma UI.

## 2026-11-07 — O teste do conflito de disco segura o poll no bridge do host (issue #273)

- **Decisão.** O teste `workspace.spec.ts` que prova a revalidação do `conflict-overwrite` passa a segurar
  no bridge do host (`POST /__host/call`, via `page.route`) a chamada `file.read` do arquivo ativo, esperar
  o poll do disco bater nela e só então escrever a segunda versão em disco e clicar. A leitura segurada
  nunca chega ao host, então o conflito pendente não pode ser substituído entre a escrita e o clique.
- **Por que no bridge e não no app.** O seam já existe e é por onde todo spec fala com o host; um intervalo
  configurável em produção (`window.__…`, variável de ambiente) viraria contrato novo para resolver um
  problema de teste. O mesmo padrão já segura `trust.grant` em `gates.spec.ts`.
- **Por que esperar o poll bater no gate.** Sem essa espera o teste passa mesmo com o gate removido: a
  janela é de ~100 ms contra um tick de 1500 ms (~7% por rodada, medido: 20/20 verdes sem ajuda). Com a
  espera, remover o gate derruba o teste sempre — o gate vira prova, não enfeite.
- **O release termina o handler antes do `unroute`.** `page.unroute` com o handler parado no gate faz o
  `route.continue()` estourar `Route is already handled!` (medido); o release resolve o gate, espera o
  handler e só então remove a rota.
- **Consequência.** `docs/SPEC-E2E-DISK-CONFLICT.md`; ticket filho de #273; evidência em
  `docs/evidence/e2e-disk-conflict.md`; ticket #278. Nenhuma linha de `src/`, nenhum `data-testid`, nenhuma dependência.

## 2026-11-07 — O pre-commit decide pelo diff do commit em andamento (issue #265)

- **Decisão.** O hook de pre-commit deixa de rodar `bun run check` em todo commit e passa a rodar só o
  que os caminhos daquele commit pedem: documentação (`*.md`, `docs/**`) nada; Rust (`crates/**`,
  `src-tauri/**`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain*`) `cargo check --workspace --all-targets`;
  qualquer outro caminho `tsc --noEmit`; misto, os dois. `bun run check` fica intocado como o check
  completo manual e do CI: o hook filtra, não substitui.
- **Por que `git diff --cached --name-only`, não `git diff --name-only HEAD`.** Com `git commit --only`
  (o do `scripts/commit-only.sh`) o git aponta `GIT_INDEX_FILE` para um index temporário, então o diff do
  index lista exatamente os caminhos do commit; o diff contra `HEAD` lista também o que está sujo na
  árvore de trabalho, que é o custo de hoje. Verificado num repositório descartável: `--only`, `-a` e
  amend só de mensagem (diff vazio) se comportam como a classificação precisa.
- **Por que caminho ambíguo roda só `tsc`.** O que sobra é `scripts/**`, `fixtures/**`, `.githooks/**`,
  `.github/**`: `tsc` custa 0,8 s e pega config da UI quebrada; `cargo check` ali não tem o que checar. O
  check completo só quando o commit tem Rust e não-Rust.
- **Por que não há limiar de diff grande.** O check é por linguagem, não por tamanho: diff grande só de
  TS não tem Rust para checar e o misto já roda os dois. Um limiar seria um número inventado pagando
  `cargo check` sem mudança de Rust.
- **Três guardas do code review.** O diff sai com `core.quotePath=false` e `--no-renames`: sem o
  primeiro, um nome não-ASCII em `crates/` vira `"crates/a\303\247..."` e não casa com o padrão de Rust
  (o `cargo check` seria pulado); sem o segundo, mover um `.rs` para `docs/` lista só o destino e o
  commit passa como documentação. Caminho que ainda sai entre aspas roda os dois checks. E os caminhos
  são lidos um por linha, nunca com `for path in $staged`: a divisão por espaço corta `crates/a.md b.rs`
  em `crates/a.md` (documentação) e `b.rs` (TypeScript), e o `cargo check` não roda.
- **`node_modules` só no ramo do TS.** O gate é `node_modules/.bin/tsc` e a mensagem diz `bun install`.
  Commit só de documentação numa worktree nova passa — antes ele morria com `tsc: command not found`.
- **Consequência.** `docs/SPEC-PRECOMMIT-PROPORTIONAL.md`; ticket #271, filho de #265; teste em
  `tests/shell/pre-commit.test.sh` (rodado à mão, como os outros; ligar `tests/shell/` no CI é a #269);
  uma linha do `AGENTS.md` corrigida, o que faz o PR ser `risk:high`. Nenhuma dependência nova, nenhum
  `data-testid`, nenhuma mudança em `bun run check`.

## 2026-11-07 — O risk tier lê o caminho cru que o git cita (issue #276)

- **Decisão.** O classificador de risco pede o diff com `git -c core.quotePath=false diff --name-only
  --no-renames "$base...$head"`, e o caminho que o git **ainda** cita (tab, aspas ou quebra de linha no
  nome) é `high`, com o motivo `git quoted the path`. Nada mais muda: a tabela de caminhos, o gate, o
  agregador e o workflow ficam como estão.
- **Por que.** `core.quotePath` é `true` por default, então `docs/evidência.md` chegava como
  `"docs/evid\303\252ncia.md"`, não casava com `docs/*` e caía no default (`medium`): um PR só de
  documentação não recebia `risk:low`, o job `risk` não armava o auto-merge e o mantenedor era chamado à
  toa. Onde o tier certo já era `medium` o defeito era invisível, porque o default coincidia.
- **Por que falhar fechado no que continua citado.** `core.quotePath=false` resolve o não-ASCII, mas o git
  cita de qualquer jeito um nome com tab, aspas ou quebra de linha (medido: `"docs/tab\there.md"`).
  Despir as aspas seria adivinhar; `high` é a mesma política do hook de pre-commit (#265), que nesse caso
  roda os dois checks. Custo aceito: um PR que toque um desses nomes passa a exigir o mantenedor.
- **Por que não `-z`.** `git diff -z` é imune a citação por construção, mas a leitura NUL-delimitada exige
  `read -d ''`, que o `dash` — o `/bin/sh` do runner — não tem. A flag resolve o caso que existe sem
  trocar a forma de ler.
- **Por que o loop de leitura ficou como está.** `IFS` com quebra de linha, `for path in $paths` e
  `set -f` já leem um caminho por linha, sem divisão por espaço e sem glob; medido, os dois estilos se
  comportam igual. O caso de nome com espaço entrou no autoteste como prova de regressão, não como
  mudança.
- **Consequência.** `docs/SPEC-RISK-TIER-QUOTED-PATHS.md`; critérios em `docs/SPEC-RISK-TIERS.md`; ticket
  #276; autoteste de 45 para 51 casos (`unicode-docs`, `unicode-product`, `unicode-product-reason`,
  `space-in-name`, `quoted-tab`, `quoted-tab-reason`); `AGENTS.md` intocado (ele não afirmava nada falso sobre citação). Nenhuma
  dependência nova, nenhum script novo, nenhum `data-testid`.

## 2026-11-07 — O fluxo do repositório é o main flow: spec na issue-mãe, um PR por frente, sem retro (issue #290)

- **Decisão.** O processo escrito passa a ser o main flow das skills: `grill-with-docs` → `to-spec` →
  `to-tickets` → `implement-spec` → `code-review`, **sem `/retro`**. A spec de uma frente é o **corpo da
  issue-mãe**; os tickets são sub-issues nativas dela, com bloqueio nativo; a implementação roda numa
  **branch de integração por frente**, com um implementer por ticket usando `tdd`, e fecha **um PR por
  frente** — draft depois do primeiro merge na branch de integração, pronto para review no fim, com
  `Closes` da mãe e de cada ticket.
- **O que substitui.** (a) O "code review em subagente único" de 2026-11-06 que o `AGENTS.md` registrava:
  agora é a skill `code-review`, dois eixos (Standards e Spec) em subagentes paralelos, modelo
  `openai/gpt-6.1-terra` com `thinking=medium`, fixed point `git merge-base origin/main HEAD`. (b) O
  bullet "Spec, RETRO e evidência continuam em `docs/`" da entrada de 2026-11-06 (suíte hermética): só a
  evidência (`docs/evidence/<tema>.md`) e as decisões continuam em `docs/`; a spec passa a ser a
  issue-mãe. (c) O `/retro` e o `docs/RETRO-<tema>.md` do fluxo: os retros existentes ficam como
  histórico e nenhum é criado para frente nova.
- **Por que.** O fluxo antigo foi escrito para o mantenedor trabalhando sozinho com um agente. A partir
  do Super Agent toda frente é conduzida pelas skills, e um `AGENTS.md` que manda criar `docs/SPEC-*`,
  abrir um PR por ticket e fechar com retro manda o agente fazer o contrário do que o orquestrador pede.
  Documento de processo contraditório é pior que documento ausente: o agente segue o que leu.
- **O que não muda.** Risk tiers e o gate de PR, a verificação pela superfície (`verify-openridr`),
  `docs/evidence/<tema>.md`, `data-testid` como contrato, stage explícito, commit convencional um por
  ticket, e o merge continuar fora da sessão.
- **Consequência.** `AGENTS.md` ("Leia antes de mexer", "Pull request e CI", "Verificação antes de
  entregar" e a regra "Ticket"), `docs/agents/issue-tracker.md` ("O que continua em `docs/`"),
  `docs/agents/domain.md` (árvore de layout) e `docs/README.md` (convenção de trabalho novo). Nenhuma
  linha de `src/`, `crates/` ou `src-tauri/`, nenhum `data-testid`, nenhuma dependência nova.

## 2026-11-07 — Nível de listagem que falha é repetido com limite e não vira pasta vazia (issue #282)

- **Decisão.** O cache de um nível da árvore "Show all files" (`src/solutionListing.ts`) repete a
  leitura que falhou com a política das consultas de marcador da árvore (#264): a original mais duas,
  com 200 ms e 400 ms (`LISTING_RETRY_DELAYS_MS`). Depois do último fracasso o nível **não** é
  guardado no cache; a repetição para na hora se um `reload()` já substituiu o pedido (epoch).
- **Por que não cachear o vazio.** Era o desenho antigo: `catch` gravava `dirs[dir] = []` e o guarda
  de entrada aceitava o array vazio (vazio é *truthy*), então a pasta expandida ficava sem filhos até
  algo alheio forçar `reload()`. Com o retry, o vazio só sobra quando a falha é definitiva — e ali
  gravar `[]` reproduziria o sintoma. Não cachear faz a próxima expansão perguntar de novo, sem
  inventar "pasta sem filhos" no cache.
- **Por que o retry fica no cache e não no componente.** O cache é quem decide o que é acerto de
  cache; a política pertence a ele e o teste unitário já é dele (`tests/unit/solutionListing.test.ts`).
  O `SolutionTree` não mudou, o que também mantém esta correção fora do arquivo que a #264/#281 mexe.
- **Consequência.** `src/solutionListing.ts`, `tests/unit/solutionListing.test.ts` (8 casos),
  `tests/e2e/solution-explorer.spec.ts` (spec novo que falha a primeira chamada de `workspace.tree`),
  evidência em `docs/evidence/solution-listing-retry.md`. Nenhum `data-testid`, nenhuma mudança no
  host, nenhuma dependência nova.

## 2026-11-08 — O host auxiliar do E2E sobe em porta efêmera e a faixa de 10 portas sai (issue #246)

- **A decisão.** `openridr-host-http --port 0` passa a imprimir o endereço que o kernel concedeu
  (`server.server_addr()`), e `tests/e2e/isolated-host.ts` sobe o host com `--port 0` e lê a porta dessa
  linha em vez de receber uma. Os sete specs que sobem host próprio e `isolated-host.spec.ts` deixam de
  declarar `const port = hostPort + N`. Só o par do harness (UI/host) continua escolhido pela sessão; as
  auxiliares são do kernel.
- **Por que a porta e não só o `pid`.** A #216 fez a colisão gritar (o helper recusa um `/health` cujo
  `pid` não é o do filho que ele subiu), mas a colisão continuava possível: quem roda duas sessões
  precisava de 10 portas de host livres acima do par, e um par vizinho ainda se cruzava. Porta efêmera
  mata a classe: não existe número para escolher errado.
- **O `pid` continua sendo a prova de posse.** A porta não prova que o processo que respondeu é o meu —
  ela só diz onde ele está. O helper continua comparando o `pid` do `/health` com o do filho que subiu, e
  o teste do decoy passou a provar isso por dentro do caminho novo: um filho de mentira imprime a porta
  de um decoy alheio, e o helper recusa nomeando a porta e o `pid` estranho.
- **O que deixa de ser demonstrável.** O critério da #216 "porta ocupada falha nomeando a porta" deixa de
  valer **pelo helper**: ele sempre pede 0, e o kernel não concede porta ocupada. Uma porta fixa pedida à
  mão (`--port 4317`) continua falhando no bind e nomeando o endereço. O que sobra provando no caminho do
  helper é o `/health` de `pid` estranho — que é o caso real que a posse protege.
- **A faixa sai dos três docs.** `AGENTS.md`, `docs/SPEC-E2E-PORTS.md` e esta entrada dizem a mesma
  regra: o par do harness é escolhido, as auxiliares são efêmeras. `.agents/skills/verify-openridr/SKILL.md`
  repetia a faixa e sai junto. `tests/e2e/environment.ts` perde a validação da faixa derivada
  (`hostPort + 1` … `+ 7`), e o teste unitário que a pinava vira o contrário: um par vizinho de porta
  bloqueada do Chrome é aceito.
- **Por que a porta efêmera não precisa do filtro do Chrome.** O navegador não conecta na auxiliar: o
  spec fala com ela por `route.fetch`/`fetch` do Node, e a faixa efêmera do macOS (49152–65535) não contém
  nenhuma porta da `kRestrictedPorts`. A recusa das duas portas escolhidas continua no load do módulo de
  ambiente.
- **Consequência.** `crates/host-http/src/main.rs` e `crates/host-http/tests/ephemeral_port.rs` (teste
  novo: `--port 0` imprime a porta concedida e o `/health` responde o `pid` do filho), o helper e o spec
  dele, os sete specs, `environment.ts`, `tests/unit/e2eRestrictedPorts.test.ts`, os docs vivos e a
  evidência em `docs/evidence/e2e-ephemeral-port.md`. Nenhuma dependência nova, nenhum `data-testid`,
  nenhum contrato do host e nenhuma mudança na UI.
## 2026-11-07 — O E2E recusa a rodada abaixo do piso duro de disco (issue #270)

- **Decisão.** O `playwright.config.ts` mede o espaço livre no volume de `os.tmpdir()` **no load do
  config**, antes de montar plugins e servidores. Abaixo de 1024 MiB a rodada é recusada com exit
  não-zero e uma mensagem com o número medido, o piso e o diretório; entre 1024 e 3071 MiB o harness
  imprime uma linha `note` e segue; acima de 3072 MiB nada muda. Medição sem número (`df` ausente ou
  saída estranha) é `note` e a rodada segue. Os dois pisos vêm do `doctor.sh`, e um teste de
  `tests/unit` compara o script com as constantes do harness.
- **Por que não no `globalSetup`, como a issue pedia.** Medido em Playwright 1.65: o `webServer` é um
  plugin e `createGlobalSetupTasks` executa os plugin setups **antes** dos global setups
  (`node_modules/playwright/lib/runner/index.js:6509-6515` e `:7130`). Num projeto mínimo com
  `webServer` + `globalSetup`, o `globalSetup` fez `fetch` na URL do `webServer` e recebeu
  `server already up = true`: no `globalSetup` o host já subiu e o `cargo run` já compilou. O load do
  config é o único ponto anterior a qualquer servidor — a mesma decisão que a #258 tomou para portas
  restritas.
- **Por que fail-open na medição que falha.** O doctor já trata "sem número" como `note` e sai 0. Recusar
  por falta de `df` trocaria um risco medido por um bloqueio certo, e `df` não é requisito do E2E.
- **Por que sempre `os.tmpdir()`.** É onde o harness põe o app home por porta da UI e o diretório de
  capturas por default. Um `OPENRIDR_E2E_HOME`/`OPENRIDR_SCREENSHOT_DIR` noutro volume é escolha da
  sessão; a mensagem imprime o diretório medido, então o número não mente.
- **Consequência.** `docs/SPEC-E2E-DISK-FLOOR.md`; ticket #287, filho de #270; seam é o `df` falso no
  `PATH` (nenhuma variável só-de-teste no harness); `playwright test --list` também recusa abaixo do
  piso. O PR é `medium` porque `playwright.config.ts` é caminho que a tabela de risco não conhece.
  Nenhuma dependência nova, nenhum `data-testid`, nenhuma mudança no `doctor.sh`.

## 2026-11-07 — O `.tmp` do `write_json_atomic` é nomeado por contador do processo, não pelo relógio (issue #294)

- **A decisão.** `crates/host/src/fsutil.rs` nomeia o temporário com `pid` + um contador atômico do
  processo (`static TEMP_SEQUENCE: AtomicU64`). O relógio sai do nome; `SystemTime`/`UNIX_EPOCH` saíram
  do arquivo.
- **Por que o relógio não servia.** Este macOS repete `SystemTime::now()` entre threads (a issue mediu
  15.506 de 16.000 timestamps duplicados com 8 threads), então duas escritas concorrentes no mesmo
  destino escolhiam o mesmo `.tmp`: uma renomeava e a outra falhava em `rename` com `No such file or
  directory`, e o `fs::write` de uma podia truncar o arquivo que a outra renomeava — `trusted.json`
  parcial lido como `trusted_unreadable`. O pid separa processos, o contador separa escritas dentro do
  processo; nenhum dos dois depende da resolução do relógio.
- **Consequência.** Teste novo em `crates/host/src/fsutil.rs` (8 threads × 200 escritas no mesmo
  destino; cada escrita tem de voltar `Ok` e o arquivo relido tem de ser JSON válido) — vermelho com o
  relógio, verde com o contador. Assinatura, formato e local de `trusted.json` não mudam; nenhuma
  dependência nova. Evidência: `docs/evidence/atomic-tmp-name.md`.

## 2026-11-07 — Build explícito recusa enquanto a automação do app constrói (issue #275)

- **Decisão.** `build.start`, `build.solution` e o build implícito de Run/Debug recusam com
  `invalid_state` e motivo nomeado enquanto a descoberta automática de testes roda ou enquanto a análise
  está carregando a solução. A recusa é a mesma do restore ("wait for it to finish before building"), e o
  motivo diz qual automação está construindo e qual é a saída (cancelar a descoberta, parar a análise). A
  simetria também vale nas duas portas da automação: `tests.discover` recusa enquanto um build roda, e
  `lsp.start`/`lsp.restart` recusam enquanto um build roda — senão a análise reabriria a mesma colisão
  pelo outro lado.
- **Por que recusar, não enfileirar.** Enfileirar (o clique não se perde) exigiria estado novo no build,
  gatilho no fim da descoberta, cancelamento e invalidação por workspace/contexto/trust/restore: um build
  enfileirado que sobrevive a uma troca de workspace é um bug novo. O ganho sobre recusar é o clique que
  não se repete, e o custo é superfície nova num caminho que hoje é só uma checagem.
- **Por que a análise só bloqueia enquanto carrega.** O estado da análise vai a `loading` no `initialize`
  aceito e sai no `Finished loading solution` do csharp-ls (~5 s medidos no OrderDesk, 3 projetos, nesta
  máquina). Depois disso o `BuildHost` continua vivo mas não constrói: bloquear mais seria bloquear sem
  causa. A recarga que o csharp-ls faz sozinho depois de carregada não é observável pelo host e fica como
  risco residual escrito na evidência.
- **Por que a tabela de motivos é uma função pura.** A ordem (build > restore > descoberta > análise
  carregando) é contrato, não detalhe: quem vem primeiro decide qual motivo o usuário lê. Fora do corpo
  de `start_build_locked`, ela é testável sem csharp-ls real e sem workspace.
- **Consequência.** issue #275 (a spec é o corpo da issue-mãe); evidência em
  `docs/evidence/concurrent-builds.md`; o gate de UI em `executionGates.ts` ganha teste de unidade em
  `tests/unit/` (o E2E não roda em PR); os specs que afirmam "enabled" logo depois de restaurar passam a
  esperar a descoberta com o `settleDiscovery` da #251. Nenhum `data-testid` novo, nenhuma dependência
  nova, nenhum estado novo na UI.

## 2026-11-08 — A análise de workspace de projeto carrega por uma solução gerada (issue #299)

- **Decisão.** Para um workspace de projeto (`.csproj` aberto direto), o host escreve uma solução mínima
  em `<app home>/analysis/<nome>-<16 hex do sha256 do caminho>.sln` nomeando o projeto aberto e lança o csharp-ls com
  `--solution <essa solução>` — o mesmo caminho do workspace de solução. Com isso o projeto sai de
  `loading` como a solução, e o gate da #275 volta a valer para os dois tipos: `start_build_locked` não
  olha mais o tipo do workspace e `analysisLoading` em `executionGates.ts` também não.
- **Por que não consertar a descoberta do csharp-ls.** A descoberta de projetos do próprio servidor trava
  dentro de `MSBuildWorkspace.OpenProjectAsync` e nunca retorna (0.28.0: log parado em `loading project`,
  processo a 0% de CPU, sem filho `dotnet`/BuildHost; o mesmo projeto via `--solution` carrega em ~4 s).
  O código é do upstream e o mesmo defeito está na `master` de hoje: o host não corrige o servidor, só
  escolhe o caminho que funciona.
- **Por que uma solução gerada e não o `.csproj` no `--solution`.** `--solution` passa por
  `SolutionFile.Parse`; um `.csproj` ali falha no parse, o servidor fica defunto e nunca emite
  `Finished loading solution` — a análise ficaria em `loading` do mesmo jeito.
- **Por que no app home.** O arquivo é do app, não do usuário: um `.sln` ao lado do `.csproj` poluiria o
  repositório dele. A solução nomeia o projeto por caminho absoluto, então a distância entre os dois não
  importa, e o arquivo é reescrito a cada `lsp.start`. O digest do caminho no nome separa dois projetos de
  mesmo nome em pastas diferentes, e um caminho com aspa ou quebra de linha é recusado com
  `operation_failed` — o `.sln` não tem como representá-lo, e recusar é melhor que escrever um arquivo que
  o csharp-ls não consegue ler.
- **Consequência.** `lsp.solution` passa a ser preenchido também no workspace de projeto (caminho da
  solução gerada; a UI não o renderiza). Evidência em
  `docs/evidence/project-workspace-analysis.md`; `docs/evidence/concurrent-builds.md` deixa de registrar a
  exceção. O leaf `build_is_refused_while_the_analysis_is_loading_the_solution_isolated` cobre o projeto
  (recusa durante `loading`, build real depois) e o teste de unidade do gate perde o caso "projeto não
  espera".
## 2026-11-08 — Consulta de árvore que falha é repetida com limite, não tratada como resposta vazia (issue #264, ticket #281)

- **Decisão.** As três consultas de marcador da Solution Explorer (`git.ignored`,
  `workspace.symlinks`, `workspace.symbolKinds`) repetem o lote que falhou até três tentativas no
  total — a original mais duas, com 200 ms e 400 ms (`TREE_QUERY_RETRY_DELAYS_MS`, em
  `src/treeQueryRetry.ts`). A repetição não acontece quando o pedido já foi substituído (workspace,
  revisão do Git ou caminhos visíveis mudaram), nem antes nem depois da espera. Depois do último
  fracasso o laço **continua** para o lote seguinte em vez de abandonar o resto; o que os lotes
  anteriores provaram fica pintado e o lote perdido fica sem marcador, como hoje.
- **Por que repetir e não manter a resposta anterior.** Manter o marcador antigo contrariaria a
  decisão já registrada no código ("a missing marker is safer than a wrong one") e não ajudaria a
  primeira consulta de um workspace, que não tem resposta anterior. A repetição cobre os dois casos:
  a resposta que não chegou e a que chegaria errada.
- **Por que o helper é um módulo e tem teste unitário.** A spec pedia helper local e prova só no E2E,
  porque o repo não tem teste de componente React. A política é compartilhada pelas três consultas e
  o unitário (`tests/unit/treeQueryRetry.test.ts`) é a prova determinística dela — três tentativas,
  as esperas, o aborto por substituição — sem host, dotnet ou navegador; o E2E fica com o contrato
  visível.
- **O efeito segue o id do pedido, não o objeto.** Achado da revisão: o poll de `workspace.state`
  (400 ms) substitui o snapshot, `views` é recalculado e o memo do pedido devolve um objeto novo com o
  mesmo `treeQueryId`; o efeito re-rodava, cancelava a repetição que esperava e descartava os lotes
  que já tinham respondido — o critério "os marcadores já provados ficam pintados" não valia na
  prática. Com a dependência no `id` (string) e os caminhos lidos de um ref, a consulta roda uma vez
  por identidade, que é o contrato que os comentários do arquivo já prometiam. Isso fecha também a
  [#300](https://github.com/Heldinhow/openridr/issues/300) (a árvore re-consultava a cada poll), que
  entrou nesta frente porque sem ela a repetição não sobrevive ao render seguinte. A limpeza de
  estado na troca de identidade do pedido continua como estava.
- **A resposta malformada de um lote é lote perdido.** Array que não é array, ou elemento que não é
  objeto/string, é descartado em vez de virar exceção não observada fora do `try` do helper — o
  processamento da resposta deixou de estar dentro do `catch` que existia antes do retry.
- **Consequência.** `src/treeQueryRetry.ts` (novo), `src/SolutionTree.tsx`, 6 testes unitários e 4
  E2E em `tests/e2e/solution-git-status.spec.ts` (três da repetição, um da #300 contando chamadas por
  identidade); evidência em `docs/evidence/tree-query-retry.md`. Nenhum `data-testid` renomeado,
  nenhuma dependência nova, nenhuma mudança no host.

## 2026-11-07 — Sem `global.json`, o projeto resolve o maior SDK instalado que atenda ao TargetFramework (issue #289)

- **A decisão.** `sdk.project` deixa de responder "o que o `global.json` pede" e passa a responder "qual
  **SDK requirement** o projeto tem e se algum **Project SDK** instalado atende". O requisito vem do pin
  quando existe; sem pin, ou com `global.json` sem `sdk.version`, vem do TargetFramework declarado:
  `netN.0` com N≥5 pede SDK N.x ou maior, ignorando o sufixo de plataforma (`net10.0-windows` → 10);
  `netstandard*`, `netcoreapp*`, `net4x` e TFM desconhecido não afirmam requisito nenhum. Multi-TFM usa
  a maior linha declarada.
- **Sem pin, a resolução escolhe o maior SDK estável instalado que satisfaça o requisito** — o mesmo que o
  `dotnet` faz sem `global.json`. Prerelease só quando é o único que satisfaz, e a mensagem diz que é
  prerelease. Sem requisito, o maior estável instalado. As raízes são as que `discover_roots` já cobre;
  nenhuma raiz nova e nenhuma pasta gerenciada obrigatória.
- **Substitui parcialmente** "Pedido de pré-requisito por projeto" (2026-11-04): o bullet "sem
  `global.json`/versão, a UI diz que o TargetFramework declarado decide e **não resolve um SDK por conta
  própria**" deixa de valer. Os critérios 3 e 4 da #289 (Helder, 2026-11-07) pedem o contrário e são a
  origem desta entrada. **Continuam valendo** daquela entrada: runtime ≠ SDK na tela, a UI não recalcula
  regra (requisito, estado, versão e URL vêm do snapshot) e os gates de
  evaluation/restore/build/run/test/analysis exigem `resolved`.
- **Por que.** O estado `no_global_json` bloqueava tudo (`state != "resolved"`): nesta máquina, com SDK
  10.0.403 instalado, um projeto `net10.0` sem `global.json` não avaliava, e a barra de execução dizia
  "No console project". A mensagem sobre `global.json` era sintoma; o gate era a causa.
- **Estados.** `no_global_json` e `not_pinned` deixam de ser estados de bloqueio: viram
  `requirement.source = "target_framework"` com `globalJsonPath`/`requestedVersion` nulos. `sdk_missing`
  continua sendo o único estado de falha e ganha `requirement`, `detected` e `download`; "nenhum SDK
  instalado" e "os instalados não servem" se distinguem pela mensagem, não por um estado novo.
  Quando nenhum TFM afirma requisito (só `netstandard2.0`, `net472`, TFM desconhecido), `source` é
  `none` e `line` é nulo: continua sendo o caso sem pin, e a distinção é o que deixa a tela dizer
  "o projeto não declara requisito" em vez de inventar uma linha.
- **Link sem rede.** O host monta a URL: patch exato
  (`https://builds.dotnet.microsoft.com/dotnet/Sdk/<versão>/dotnet-sdk-<versão>-<os>-<arch>.<ext>`) quando
  o requisito nomeia uma versão; senão o `aka.ms` da linha
  (`https://aka.ms/dotnet/<linha>/dotnet-sdk-<os>-<arch>.<ext>`) **e** a página oficial da linha
  (`https://dotnet.microsoft.com/download/dotnet/<linha>`) como alternativa; sem requisito, a página geral
  de SDKs. Nada de `releases.json`: rede no caminho da tela, e o patch mais novo da linha não satisfaz um
  pin `disable` de qualquer jeito. **Medido em 2026-11-07:** `aka.ms/dotnet/10.0/dotnet-sdk-osx-arm64.pkg`
  → 301 para 10.0.403; o patch 8.0.408 → 200; 8.0.999 → 404; linha inexistente (11.0) → Bing, e é por
  isso que o link de linha vem acompanhado da página da linha.
- **Redetecção.** O bloco de erro ganha "Check again", que chama o `sdk.redetect` existente. Quando o
  redetect leva o estado de `sdk_missing` para `resolved`, a UI dispara a avaliação; build/run/testes já
  ficam `stale` pelo host. Nada observa o disco nem o foco da janela.
- **Consequência.** `crates/host/src/sdk.rs` (requisito, resolução sem pin, montagem de URL),
  `crates/host/src/lib.rs` (`inspect_project_sdk`), `src/welcome/ProjectSdkBody.tsx`, a nota da árvore e a
  barra de status em `src/WorkspaceView.tsx`, `src/host/client.ts`; testes unitários de `sdk.rs`, de
  fronteira em `crates/host/tests/host.rs` e `tests/e2e/sdk-prerequisites.spec.ts`. Nenhuma dependência
  nova, nenhum `data-testid` removido, nenhuma instalação automática e nenhuma escrita em `global.json`
  ou no PATH.

## 2026-11-08 — O snapshot pergunta ao muxer qual SDK ele resolve, em vez de prever (issue #310)

- **A decisão.** Sem pin, `sdk.project.resolvedVersion` deixa de ser o resultado da regra do host
  (`resolve_unpinned`) e passa a ser a resposta do **mesmo muxer que executa o MSBuild**:
  `<raiz>/dotnet --version` com `cwd` na raiz do workspace e o mesmo `DOTNET_ROOT`/
  `DOTNET_MULTILEVEL_LOOKUP=0` dos outros filhos. A regra continua existindo só para escolher **qual
  raiz** é consultada; quem responde é o muxer.
- **Motivo.** As duas regras concordam no caso comum e divergem em dois: um `global.json` **acima** da
  raiz do workspace (o host só lê `<raiz>/global.json`, o muxer sobe a árvore) e um prerelease mais
  novo que o maior estável instalado na mesma raiz. Medido nesta máquina (`~/.dotnet` com 8.0.408 e
  10.0.403): com `global.json` em `/tmp/X` fixando `8.0.408` e um projeto `net10.0` em `/tmp/X/app`
  sem `global.json` próprio, `dotnet --version` em `app/` responde `8.0.408`, a avaliação (`-getProperty`)
  **passa** e o `dotnet build` falha com `NETSDK1045` — a tela dizia `10.0.403` e o build rodava `8.0.408`.
- **A resposta do muxer também decide o estado.** Se ela não satisfaz a linha do TargetFramework, o
  estado é `sdk_missing` com a versão que o muxer resolve na mensagem, em vez de `resolved` seguido de
  build quebrado. Era a metade "ou falha, mesmo com `state == \"resolved\"`" do problema.
- **Substitui parcialmente** "Sem `global.json`, o projeto resolve o maior SDK instalado que atenda ao
  TargetFramework" (2026-11-07, #289): o bullet "sem pin, a resolução escolhe o maior SDK estável
  instalado que satisfaça o requisito" deixa de ser o que a tela reporta — a regra passa a escolher só
  a raiz consultada e quem responde é o muxer. **Continuam valendo** daquela entrada: o requisito vem
  do pin ou do TargetFramework, `sdk_missing` é o único estado de falha, a URL é montada no host sem
  rede e a UI não recalcula regra.
- **O que não muda.** O caminho com pin (`<raiz>/global.json` com `sdk.version`) continua resolvendo
  pela regra do host e o pin continua tendo precedência; `evaluation.rs` continua chamando
  `dotnet msbuild` (nenhuma mudança em como o MSBuild é invocado); nenhuma leitura de arquivo fora da
  workspace, nenhuma escrita em `global.json`/PATH, nenhuma instalação, nenhuma rede, nenhuma
  dependência nova. `sdk.project` continua sendo um valor por workspace.
- **Custo.** Uma chamada `dotnet --version` (~0,2 s medidos) por trust e por `sdk.redetect`, no lugar
  de uma previsão. Se o muxer não inicia ou não responde uma versão, o estado é `sdk_missing` com o
  motivo: sem muxer não há build de qualquer forma.
- **Limite conhecido (registrado como `shortcut:` em `resolve_without_pin`).** O muxer é consultado na
  raiz do workspace, que é o `cwd` de restore/build/run/test; a avaliação roda no diretório do projeto,
  então um `global.json` **entre** a raiz e o diretório do projeto ainda diverge. Fechar isso exige
  mudar o `cwd` da avaliação (decisão de 2026-10-28), então ficou na issue #313.
- **Consequência.** `crates/host/src/sdk.rs` (`muxer_version`, `satisfies_line`, remoção do `shortcut:`
  de `resolve_unpinned`), `crates/host/src/lib.rs` (`resolve_without_pin`), dois testes de fronteira em
  `crates/host/tests/host.rs` e um unitário de `satisfies_line`. Nenhum `data-testid` tocado, nenhuma
  mudança de UI.

## 2026-11-08 — A avaliação roda o muxer com `cwd` na raiz da workspace (issue #313)

- **A decisão.** `evaluation::invoke` passa a rodar `<raiz>/dotnet msbuild <csproj> …` com `cwd` na
  **raiz da workspace**, o mesmo `cwd` de restore/build/run/test. O muxer procura `global.json` do `cwd`
  para cima, então avaliação e snapshot passam a ler o mesmo arquivo por construção: uma resposta só
  por workspace, sem contrato novo.
- **Substitui** "avaliação por projeto, sem restore" (2026-10-28, ticket 03) no ponto em que ela fixou
  `cwd` no diretório do projeto. **Continuam valendo** daquela entrada: um `dotnet msbuild` por
  `.csproj` com `-getProperty`/`-getItem`, nenhum MSBuild para a solução, `-p:Configuration`/
  `-p:TargetFramework` explícitos, `TargetPath` sempre avaliado e multi-TFM sem seleção pedindo TFM.
- **Fecha** o limite que a entrada do #310 registrou como `shortcut:` em `resolve_without_pin`: com um
  `global.json` **entre** a raiz e o diretório do projeto, a avaliação usava o SDK pinado e o snapshot
  nomeava outro. Medido antes da mudança nesta máquina (`~/.dotnet` com 8.0.408 e 10.0.403): raiz sem
  pin, `src/global.json` fixando `8.0.408` (`rollForward: disable`), projeto `net10.0` — o snapshot
  dizia `10.0.403` e o `-getProperty:NETCoreSdkVersion` da avaliação respondia `8.0.408`, **sem falhar**.
  A divergência era silenciosa; só o build (que já rodava na raiz) usava o SDK do snapshot.
- **O que não muda.** `Directory.Build.props`, `obj/`, `bin/` e `TargetPath` são resolvidos pelo caminho
  do projeto, não pelo `cwd`. O que o `cwd` decide é o `global.json` que o muxer encontra e qualquer
  expressão do próprio projeto que leia o diretório de trabalho (`$(MSBuildStartupDirectory)` ou uma
  função de propriedade equivalente) — nenhuma fixture faz isso e a suíte inteira continua verde, mas
  é um jeito de o resultado mudar que este teste não cobre. Nenhuma leitura fora da workspace, nenhuma
  escrita, nenhuma rede, nenhuma dependência, nenhum subprocesso a mais.
- **Projeto fora do diretório da solução muda de SDK.** Antes ele era avaliado no próprio diretório e o
  muxer resolvia o maior SDK estável; agora é avaliado sob o `global.json` da raiz, como restore/build/
  run/test já faziam para ele. É a mesma resposta por workspace que a decisão compra, e o escopo do
  #289 (que admite o diretório do projeto de fora) não muda.
- **O log passa a nomear o SDK.** O comando da avaliação ganhou `-getProperty:NETCoreSdkVersion`, então
  o log de cada projeto diz qual SDK o avaliou — a pergunta que #310 e #313 não conseguiam responder
  olhando o resultado. Vale enquanto o log não é truncado: `cap_log` guarda os **últimos** 32k
  caracteres, então num projeto com muitos itens a lista de `Compile`/`PackageReference` pode empurrar
  as propriedades para fora do log. Nenhuma propriedade nova no modelo e nenhuma mudança de UI.
- **Consequência.** `crates/host/src/evaluation.rs` (`invoke` recebe a raiz), `crates/host/src/lib.rs`
  (sai o `shortcut:` de `resolve_without_pin`) e um teste de fronteira em `crates/host/tests/host.rs`.
  Nenhum `data-testid` tocado.

## 2026-11-08 — Tela de boas-vindas respeita a plataforma (#319)

O botão **Open** deixa de ser um beco sem saída fora do macOS. `workspace.pick` continua sendo o único contrato: o host responde `{ path }`, `{ path: null }` no cancelamento ou o erro `pick_unavailable`. A UI trata `pick_unavailable` como **estado**, não como erro — o campo de caminho aparece com foco e uma linha de status discreta, sem banner vermelho. O campo é permanente no Linux e no Windows e continua escondido no macOS (a aparência verificada não muda); o build de browser/E2E, que não tem seletor nativo, sempre o mostra. `~` e `~/…` são expandidos pelo host (`expand_home`, contra `HOME`/`USERPROFILE`) e todo o resto é tomado como digitado, inclusive `~user` e um til no meio do caminho.

O atalho vem do keymap: `shortcut("o", { primary: true })` com `formatShortcut` dá `⌘O` no macOS e `Ctrl+O` fora dele, e o handler usa `matches`, não `metaKey || ctrlKey`. Nada de texto fixo no JSX.

No Linux a janela não recebe menu: `Builder::menu` não é chamado. O menu GTK padrão (Edit, Window, Help) não é a UI do app, e os itens predefinidos de recortar/copiar/colar/selecionar tudo do `muda` carregam **só o rótulo do acelerador** (`muda-0.20.0/src/platform_impl/gtk/mod.rs`: "These items do not need an accelerator as GTK automatically have them"), com a ação real atrás de `libxdo`, que é X11. A barra nunca foi dona dessas teclas, então removê-la não as tira; quem responde por elas é o webview. Nesta sessão `Ctrl+A` e `Ctrl+V` não responderam no WebKitGTK **com e sem** a barra (ver `docs/evidence/welcome-linux.md`), então o que falta ali é anterior a esta mudança e precisa da confirmação do Helder com teclado real. Windows mantém `Menu::default`.

O seletor nativo no Linux (portal XDG `org.freedesktop.portal.FileChooser`) ainda não existe: `gdbus` sozinho não serve (a resposta do portal é unicast para a conexão que chamou e o portal recusa `Request.Close` de outra conexão), e cada caminho sem dependência nova depende de um runtime do sistema (`gjs`) ou de código de protocolo D-Bus escrito à mão. Fica registrado como pendente de decisão de dependência na #319; até lá o Linux usa o campo de caminho.

Consequência: `crates/host/src/lib.rs` (`expand_home`, `home_dir`, `pick_unavailable`), `src/SetupScreen.tsx`, `src/welcome/{ProjectsView,PathForm}` e `src/welcome/openTarget.ts`, `src/styles.css`, `src-tauri/src/main.rs`, `tests/unit/welcomeOpen.test.ts` e `tests/e2e/welcome.spec.ts`. Nenhum `data-testid` renomeado ou removido; `welcome-keys` e `workspace-pick-notice` são novos.

## 2026-11-08 — O build do Linux embute o netcoredbg e o setup fica pronto (issue #318)

- **A decisão.** O build do Linux passa a embutir o netcoredbg `3.2.0-1092`, como o macOS: o
  `.tar.gz` oficial da arquitetura é verificado por SHA-256 em tempo de build
  (`scripts/vendor-netcoredbg.sh`; pinos `065ff49b…` para `linux-arm64` e `080eb3b2…` para
  `linux-amd64`, os mesmos do metadata da release) e extraído para
  `src-tauri/resources/netcoredbg/`. O runtime nunca baixa nem descompacta o `.tar.gz`, e o
  instalador `debug.install` continua recusado onde o adaptador é embutido.
- **Onde o app procura.** `bundled::resources_root()` ganhou o layout do Linux, relativo ao
  binário e nunca ao `cwd`: `<dir do exe>/../lib/<nome do binário>` — o mesmo que o Tauri resolve
  em `resource_dir()` — com `/usr/lib/<nome>` como instalação de sistema. `debugger::bundled_binary()`
  usa a plataforma real do processo em vez do `osx-arm64` fixo.
- **O gate.** O gate de Linux deixou de ser "unavailable / fora do P0": passa a `installable: false`
  (o instalador nunca roda) e `bundled: true` (o adaptador embutido satisfaz o gate). `ready`
  continua exigindo `--version` com `3.2.0` no binário real, no `ready_adapter`; o snapshot
  reporta `ready` pela presença do executável, como no macOS. Adaptador ausente bloqueia o setup
  com a mensagem de reinstalação e sem download.
- **O que não muda.** macOS mantém o mesmo asset, pino, assinatura e layout: `gate.bundled` só é
  verdadeiro lá com o opt-in experimental, que é o que já bloqueava. `is_setup_ready`, os gates de
  trust e o `setup_override` de teste não foram tocados. Windows e macOS Intel mantêm o
  comportamento atual.
- **Build.** `src-tauri/tauri.linux.conf.json` (merge RFC 7396 sobre `tauri.conf.json`) troca o
  `beforeBuildCommand` para não chamar `vendor-analysis.sh` (csharp-ls e runtime .NET continuam
  fora do escopo no Linux) e remove `resources/csharp-ls/` e `resources/dotnet/` de
  `bundle.resources` com `null`. O macOS continua com o `beforeBuildCommand` e os três resources.
- **Proteções da extração de build.** O script valida a listagem do `tar` antes de escrever:
  caminho absoluto, segmento `..`, symlink, hardlink e arquivo especial são recusados, com os
  mesmos limites do `extract_zip` (200 entradas, 200 MiB). A extração Linux fica em
  `scripts/vendor-netcoredbg-linux.sh`, sourcing do script principal, para
  `tests/shell/vendor-netcoredbg.test.sh` chamar as checagens com um arquivo controlado; o build
  sempre passa o arquivo e o digest fixados na tabela do script, então nenhum asset é
  substituído por ambiente.
- **Consequência.** `crates/host/src/bundled.rs`, `crates/host/src/debugger.rs`,
  `crates/host/tests/debugger.rs`, `src/host/client.ts` (`availability: "bundled"`),
  `scripts/vendor-netcoredbg.sh`, `src-tauri/tauri.linux.conf.json`, os dois `.tar.gz` em
  `vendor/netcoredbg/` e `tests/shell/vendor-netcoredbg.test.sh`. Nenhum `data-testid` tocado,
  nenhuma dependência nova, `AGENTS.md` intocado.

## 2026-11-09 — Seletor nativo no Linux pelo portal XDG com `zbus` direto (#319)

- **A decisão.** `src-tauri` passa a depender de `zbus` (`[target.'cfg(target_os = "linux")'.dependencies]`, 5.19.0, a mesma versão que já entra transitivamente pelo `tauri-plugin-single-instance`) e `src-tauri/src/picker.rs` implementa o seletor pelo portal `org.freedesktop.portal.FileChooser`: `OpenFile` com filtros `*.sln`, `*.slnx` e `*.csproj`, `{ "path": null }` no cancelamento. Aprovado pelo Helder em 2026-11-09.
- **O host continua dono do contrato.** `Host::set_file_picker` registra o seletor da shell e `workspace.pick` chama o hook antes do padrão da plataforma; sem hook, o macOS mantém `osascript` e os demais devolvem `pick_unavailable`. O `main` registra `picker::pick_workspace` só no Linux. O `host-http` (dev/E2E) não registra nada, então o Playwright continua vendo `pick_unavailable` e o campo de caminho — que é o fallback que a issue exige.
- **Por que não `gdbus`.** O portal emite `Response` **unicast** para a conexão que chamou (`request->sender`) e recusa `Request.Close` de outra conexão (`AccessDenied: Portal operation not allowed: Unmatched caller`). Um `gdbus call` sai, a conexão morre e o pedido vai com ela; as duas chamadas estão medidas em `docs/evidence/welcome-linux.md` (PR #320).
- **Por que não `gjs`.** Resolveria sem dependência de repositório, mas exige um runtime do sistema que não está garantido (nesta máquina ele só existe por causa do `sushi`) e transforma o seletor num subprocesso cujo ciclo de vida teria de ser casado com o pedido.
- **Por que não `tauri-plugin-dialog`.** É a solução de GUI do Tauri, mas no Linux também depende de um backend de diálogo do sistema e adiciona uma árvore de dependências maior que o `zbus` já presente; o contrato `workspace.pick` do host continuaria precisando de um seam igual.
- **O que não muda.** `workspace.pick` continua devolvendo `{ path }`, `{ path: null }` no cancelamento ou `pick_unavailable`; o caminho escolhido entra no mesmo `workspace.open` (expansão de `~`, canonicalize, confinamento, trust). Nenhum `data-testid` tocado e nenhuma mudança de UI: o campo de caminho continua permanente no Linux/Windows e escondido no macOS.
- **Limite conhecido.** O diálogo é aberto sem `parent_window` (o Tauri/tao não exporta um handle `xdg-foreign`), então ele não é modal em relação à janela do app; no Hyprland ele aparece como uma janela própria do backend GTK do portal. `pick_unavailable` cobre "sem barramento de sessão", "sem portal" e "sem backend de file chooser"; o detalhe vai para o stderr, não para a tela.
- **Consequência.** `src-tauri/Cargo.toml`, `src-tauri/src/{main,picker}.rs`, `crates/host/src/lib.rs` (`FilePicker`, `set_file_picker`, `pick_workspace` com hook) e um teste de fronteira em `crates/host/tests/host.rs`. Evidência em `docs/evidence/picker-linux.md`.

## 2026-11-09 — Workspace escuro contínuo inspirado no T3 (#348, #349)

A primeira fatia substitui as ilhas arredondadas por uma superfície escura contínua: o Solution Explorer atravessa a altura do editor e da tool window inferior, que passa à coluna do editor. Barra, árvore, abas e código usam uma hierarquia neutra; seleção, sintaxe e diagnósticos continuam distintos. Isto revisita a geometria da decisão UI shape de 2026-10-31, sem remover controles ou migrar preferências: larguras e alturas continuam nos mesmos slots. Referência e limites: [evidence/t3-workspace-349.md](evidence/t3-workspace-349.md). Sem copiar marca, imagens ou funções de agentes; expansão aguarda aprovação visual.
