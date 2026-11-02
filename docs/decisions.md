# Decisões de implementação (registro incremental)

Formato: data, decisão, motivo, consequência. Só entram decisões pequenas tomadas durante a implementação; decisões grandes continuam na [SPEC](SPEC.md).

## 2026-10-27 — Base própria, sem código do OpenIdea

O shell é escrito do zero (Tauri 2 + Rust + React/TS). Nenhum arquivo do OpenIdea foi clonado ou incorporado; a "auditoria da base" resulta em: nada reutilizado, nenhuma licença de terceiro herdada além das dependências declaradas. Avisos e identidade OpenRidr são preservados. Motivo: não há checkout local e a SPEC exige auditar antes de reutilizar; copiar sem auditoria seria pior.

## 2026-10-27 — Host separado da UI, com ponte HTTP só de desenvolvimento

A lógica de host vive em `crates/host` (`openridr-host`), sem dependência de Tauri. A janela Tauri (`src-tauri`) e a ponte HTTP (`crates/host-http`) chamam a mesma função `dispatch(method, params)`. A ponte existe apenas para Playwright exercitar a UI real contra o host real e ferramentas reais; não é empacotada no app, escuta em 127.0.0.1 e não tem autenticação (portanto nunca é distribuída). Motivo: Tauri não é automatizável por Playwright no macOS; sem a ponte, E2E de UI viraria só contrato ou clique manual.

## 2026-10-27 — Abertura de workspace por caminho digitado/colado

O diálogo nativo de arquivos fica para depois. A tela inicial aceita caminho absoluto de `.sln`, `.csproj` ou diretório; o host lista candidatos e exige escolha quando há mais de uma solução/projeto. Custo menor e testável por teclado; trocar por seletor nativo não muda o contrato do host.

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

`dotnet test` não expõe um fluxo de debug reutilizável pelos métodos atuais: `debug.start` lança o assembly do projeto, não um teste, e debugging de teste é P1 em `docs/README.md`. A tool window Tests e a quick list mostram "Debug selected" desabilitado com o motivo, em vez de mapear para `debug.start` e prometer algo que não é. Run, rerun failed e stop continuam reais. Consequência: a F5 não entrega debug de teste e o AC correspondente foi corrigido no ticket.

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
