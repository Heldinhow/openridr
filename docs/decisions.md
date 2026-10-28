# Decisões de implementação (registro incremental)

Formato: data, decisão, motivo, consequência. Só entram decisões pequenas tomadas durante a implementação; decisões grandes continuam na [SPEC](SPEC.md).

## 2026-10-27 — Base própria, sem código do OpenIdea

O shell é escrito do zero (Tauri 2 + Rust + React/TS). Nenhum arquivo do OpenIdea foi clonado ou incorporado; a "auditoria da base" resulta em: nada reutilizado, nenhuma licença de terceiro herdada além das dependências declaradas. Avisos e identidade OpenRidr são preservados. Motivo: não há checkout local e a SPEC exige auditar antes de reutilizar; copiar sem auditoria seria pior.

## 2026-10-27 — Host separado da UI, com ponte HTTP só de desenvolvimento

A lógica de host vive em `crates/host` (`openridr-host`), sem dependência de Tauri. A janela Tauri (`src-tauri`) e a ponte HTTP (`crates/host-http`) chamam a mesma função `dispatch(method, params)`. A ponte existe apenas para Playwright exercitar a UI real contra o host real e ferramentas reais; não é empacotada no app, escuta em 127.0.0.1 e não tem autenticação (portanto nunca é distribuída). Motivo: Tauri não é automatizável por Playwright no macOS; sem a ponte, E2E de UI viraria só contrato ou clique manual.

## 2026-10-27 — Abertura de workspace por caminho digitado/colado

O diálogo nativo de arquivos fica para depois. A tela inicial aceita caminho absoluto de `.sln`, `.csproj` ou diretório; o host lista candidatos e exige escolha quando há mais de uma solução/projeto. Custo menor e testável por teclado; trocar por seletor nativo não muda o contrato do host.

## 2026-10-27 — Trust só em memória

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
