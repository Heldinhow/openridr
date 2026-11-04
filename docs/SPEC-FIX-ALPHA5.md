# SPEC — Correções do E2E alpha.5 e camada SDK/toolchain

Status: ready-for-agent
Fonte: `docs/evidence/e2e-alpha5/REPORT.md` (20 achados: 1 P0, 1 P1, 11 P2, 7 P3) + decisões do Helder de 2026-11-03/04.
Alvo: macOS Apple Silicon. Windows e Intel continuam fora do que foi executado.

## Problem Statement

Quem usa o OpenRidr 0.3.0-alpha.5 perde trabalho ao sair com Cmd+Q, vê o editor mostrar código que já não está no disco, tem a árvore parada em um retrato antigo do projeto, e às vezes não consegue depurar depois de uma mudança externa — mesmo com build verde. Arquivos CRLF mudam de formato ao salvar e arquivos UTF-16 nem abrem. A instalação das ferramentas de análise e depuração é opaca: o Debug baixa um binário sem mostrar progresso nem dizer o que ficou instalado, e uma falha de rede não tem retry.

Na camada de toolchain, um projeto preso ao SDK 8 pelo `global.json` esbarra numa exigência de SDK 10 que só existe porque o csharp-ls precisa do runtime 10. O app também deixa de encontrar SDKs instalados fora do primeiro `dotnet` do PATH, então pede uma instalação que o usuário já tem.

## Solution

Corrigir os 20 achados por prioridade, sem criar obrigação nova para o usuário e sem mudar o que já funciona:

1. **P0** — Cmd+Q, menu Quit e o Close da janela passam pelo mesmo diálogo de alterações não salvas que o botão Close já usa. Quit pelo Dock e via AppleScript (`terminate:` nativo) ficam declarados como limitação conhecida, não como corrigidos.
2. **P1** — o gate de build deixa de bloquear o Debug por um arquivo que já não existe; um rebuild verde libera a sessão.
3. **P2/P3** — o editor recarrega buffer limpo quando o disco muda, preserva CRLF e lê UTF-16 com BOM; a árvore reflete mudanças externas sozinha e ganha criar/renomear/apagar; o Debug mostra progresso, verifica SHA-256, tem retry e registra origem/versão do netcoredbg; mensagens de abertura e avaliação dizem a verdade; rótulos de acessibilidade são inequívocos; a skill `verify-openridr` para de mentir e de colidir entre sessões.
4. **Camada SDK/toolchain** — o csharp-ls exige **runtime** .NET 10, nunca SDK 10; o projeto usa o SDK que o `global.json`/TargetFramework pedem; a descoberta olha DOTNET_ROOT, `/usr/local/share/dotnet`, `/opt/homebrew`, `~/.dotnet`, os SDKs gerenciados e todas as entradas do PATH; a UI pede só o que falta para aquele projeto, com link vivo e ajuda de instalação; o download do csharp-ls na primeira abertura de projeto C# tem progresso, retry e erro claro. O netcoredbg continua sendo baixado sozinho no Debug (decisão do Helder), com versão fixa, checksum, progresso e retry.

## User Stories

1. Como pessoa editando um arquivo, quero que Cmd+Q, o menu Quit e o Close da janela perguntem antes de fechar com alterações não salvas, para não perder meu trabalho. (Quit pelo Dock e via AppleScript ficam de fora: limitação declarada.)
2. Como pessoa que escolheu "Keep editing" no diálogo de saída, quero que o app continue aberto com o buffer intacto.
3. Como pessoa que escolheu "Save all" no diálogo de saída, quero que os arquivos sejam gravados antes de o app encerrar.
4. Como pessoa que apagou um arquivo por fora e fez rebuild, quero que o Debug inicie em vez de acusar um arquivo que já não existe.
5. Como pessoa com uma aba limpa aberta, quero que o editor recarregue sozinho quando o arquivo muda no disco, para não ler código velho.
6. Como pessoa com uma aba suja e o arquivo mudado no disco, quero continuar vendo o diálogo de conflito atual (Reload / Overwrite), sem perda silenciosa.
7. Como pessoa que criou, renomeou ou apagou arquivos por fora, quero ver a árvore refletir isso sem clicar em Re-evaluate.
8. Como pessoa que salvou um arquivo CRLF, quero que ele continue CRLF no disco.
9. Como pessoa que abriu um arquivo UTF-16 com BOM, quero vê-lo no editor, sem mensagem de "não é UTF-8".
10. Como pessoa que salvou um UTF-16, quero que o arquivo continue UTF-16 (mesmo encoding e BOM).
11. Como pessoa que abre um `.cs` ou `.txt` pelo terminal/Finder, quero uma mensagem dizendo que preciso de `.sln`, `.slnx`, `.csproj` ou pasta, em vez de cair no Welcome sem explicação.
12. Como pessoa que abriu um `.sln` inválido, quero uma mensagem sobre o arquivo, não sobre o SDK.
13. Como pessoa cuja primeira avaliação falhou por TFM inválido, quero não ler "valores da avaliação anterior" quando nenhuma avaliação anterior existiu.
14. Como pessoa que clica em Debug, quero ver o download do netcoredbg progredir, com checksum verificado, retry em falha e um registro em Settings do que foi instalado, de onde e em que versão.
15. Como pessoa que usa leitor de tela, quero nomes acessíveis distintos para as três ações "Run", as duas "Save" e para o menu de contexto do projeto.
16. Como pessoa usando a árvore, quero criar, renomear e apagar arquivos pela UI, confinado ao workspace, com confirmação no delete.
17. Como pessoa que roda o binário direto com o app aberto, quero que a segunda execução reutilize a instância existente em vez de abrir outra.
18. Como pessoa que usa a skill `verify-openridr`, quero que `down` só diga "stopped" quando o processo morreu, que estados de sessões diferentes não se apaguem, e que o doctor valide a cópia sob teste.
19. Como pessoa seguindo as receitas da skill, quero que elas digam quando `type`/`press` exigem `--headed` e `focus-window`.
20. Como pessoa lendo os docs da skill, quero que descrevam os diálogos que existem (e só esses).
21. Como pessoa com `OrderDesk` preso ao SDK 8.0.408, quero abrir, avaliar, buildar, rodar e analisar sem o app exigir SDK 10.
22. Como pessoa com SDKs em `/usr/local/share/dotnet` ou `~/.dotnet` e no PATH, quero que o app os encontre sem eu configurar variável.
23. Como pessoa com um projeto multi-SDK, quero que o app peça só o que falta para aquele projeto, com mensagem e link de instalação que funcionem.
24. Como pessoa que instala a análise, quero progresso, retry e erro claro quando a rede falhar, sem perder o estado do workspace.
25. Como pessoa que já tem o csharp-ls instalado, quero que o app o reaproveite após confirmação, sem baixar de novo.
26. Como mantenedor, quero uma proposta medida de embutir o netcoredbg (~3.4 MB), sem implementá-la nesta rodada.
27. Como mantenedor, quero que cada correção tenha prova antes/depois em `docs/evidence/fix-alpha5/` e status no relatório.

## Implementation Decisions

### Saída e integridade de dados

- **Quit coberto**: menu Quit próprio (Cmd+Q) emite para a UI; `ExitRequested` do Tauri chama `prevent_exit` e pergunta à webview; a resposta chama um comando `quit_app` que marca a saída como autorizada e encerra. O diálogo existente ganha o caso "quit", reutilizando Save all / Keep editing / Discard and close app. AppleScript `quit` e o Quit do Dock (`terminate:` nativo) ficam documentados como limitação conhecida.
- **Encoding**: a leitura detecta BOM UTF-8, UTF-16LE e UTF-16BE e decodifica; a escrita reaplica o mesmo encoding e BOM. O contrato de conflito (`expected_hash`) passa a ser sobre o texto normalizado, não sobre os bytes crus. Codificação desconhecida continua erro claro.
- **CRLF**: o editor trabalha em LF; o host detecta o estilo do arquivo na leitura e o restaura na escrita. Arquivo misto preserva o estilo predominante (primeira ocorrência de `\r\n`).
- **Buffer limpo**: uma verificação periódica (ordem de 1–2 s) relê a aba ativa quando a janela está visível; foco e troca de aba continuam relendo. Aba suja mantém o diálogo de conflito.

### Árvore e abertura

- **Frescor**: os diretórios de fontes avaliados entram no watch do host; criar/renomear/apagar dispara uma re-avaliação com debounce (~500 ms), só com a janela visível. A seleção da árvore é preservada. O banner de stale continua existindo para falhas.
- **Criar/renomear/apagar**: três métodos novos no host, confinados ao workspace (o pai do caminho novo precisa existir dentro dele), com `is_evaluation_input` marcando stale. UI: itens no menu do projeto/pasta, um diálogo de nome, confirmação de delete. Sem drag-drop, templates, multi-select ou operações fora do workspace.
- **Mensagens de abertura**: `.cs`/`.txt` em argv produzem a mesma mensagem da pasta sem solução (o app já faz isso no HEAD); o script `openridr` passa a validar extensão e a dizer qual bundle abriu. `.sln` sem projetos diz "não lista projetos; não é uma solução válida" em vez de falar de SDK. "Avaliação anterior" só aparece quando `revision > 1`.
- **Acessibilidade**: rótulos visíveis não mudam; nomes acessíveis ganham contexto (ex.: "Run tool window", "Save settings", "Project context menu"). O keymap pode mostrar o grupo como texto secundário.

### Debug e ferramentas

- **Debug continua one-click** (decisão do Helder). O install automático do netcoredbg mantém versão fixa e SHA-256 já existentes no host; a UI ganha progresso visível, botão de retry e erro acionável. Settings/Workspace tools mostram origem, versão e checksum verificado do adaptador instalado.
- **Status honesto**: o painel do Debug deixa de dizer "blocked on this platform" quando o binário existe e a sessão pode rodar; "experimental · disabled" continua sendo a flag de sessão, com o motivo explícito.
- **`Install analysis` (csharp-ls) não muda de fluxo**: continua plano revisável + confirmação explícita.
- **Skill**: `down` valida o PID (comando e liveness), espera a morte, remove o pid e falha listando instâncias restantes; estado padrão por usuário/sessão com rotação em vez de `rm -rf`, caminho impresso e override por env; doctor aceita bundle por argumento/env e diz qual validou; extração do `accessibility` sem JSON cru; docs corrigidos para os diálogos reais e referência à skill `test-app-agent-desktop`.

### SDK/toolchain

- **Runtime ≠ SDK**: o host passa a identificar o runtime `Microsoft.NETCore.App` 10 via `--list-runtimes` para o csharp-ls e o root composto; o gate da análise e o plano do csharp-ls deixam de exigir um SDK 10 instalado. O SDK do projeto continua vindo do `global.json`/TargetFramework, com a resolução atual (incluindo `rollForward`).
- **Descoberta**: DOTNET_ROOT, `/usr/local/share/dotnet`, `/opt/homebrew` (e `libexec`), `~/.dotnet`, SDKs gerenciados pelo app e **todas** as entradas do PATH, deduplicados por raiz canônica. O contrato é a versão resolvida e o `sdkRoot` absoluto, não o caminho.
- **Root composto**: muxer, `host/fxr/10` e `Microsoft.NETCore.App/10` vêm da raiz que tem o runtime 10; `sdk/` e `packs` continuam vindo da raiz do SDK do projeto.
- **Mensagem por projeto**: quando falta o SDK do projeto, a mensagem cita a versão pedida e o `global.json`/TFM; quando falta o runtime 10, diz ".NET 10 runtime" (não SDK) e aponta o link vivo. Nada de exigir 10 para projeto 8.
- **csharp-ls**: download na primeira abertura de projeto C# com progresso observado, retry e erro claro; falha de rede não deixa instalação parcial "pronta". O plano revisável continua.
- **Links**: URLs verificadas por HEAD/GET antes de entrar na UI; a página de download do .NET 10 runtime substitui qualquer link morto.
- **netcoredbg embutido**: só proposta escrita, com o tamanho medido do artefato oficial (3.39 MB para osx-arm64) e o custo de assinatura/notarização. Nenhum código de embed nesta rodada.

## Testing Decisions

Bom teste prova comportamento externo, não implementação: arquivo em disco, resposta do host, texto visível na UI, exit code. Não se testa detalhe interno de struct sem contrato.

- **Host Rust** (prior art: `crates/host/tests/host.rs`, `tests/debugger.rs`): encoding/CRLF round-trip, hash de conflito, gate de fonte ausente, frescor de avaliação, descoberta de SDK/runtime, resolução de projeto, plano de ferramenta, status do adaptador.
- **Playwright** (prior art: `tests/e2e/*.spec.ts` contra host real + `fixtures/OrderDesk`): recarga de buffer limpo, conflito, CRLF/UTF-16, árvore após mudança externa, criar/renomear/apagar, mensagens de `.sln`/TFM, rótulos acessíveis, plano de instalação, argv.
- **App Tauri real** (skill `verify-openridr` + `test-app-agent-desktop`): Cmd+Q com buffer sujo, segunda instância, Debug após delete externo, progresso/retry do netcoredbg, mensagem de SDK com projeto 8.0.408. Evidência antes/depois em `docs/evidence/fix-alpha5/`.
- **Skill**: rodar `doctor.sh`, `web.sh up/status/down`, `desktop.sh up/down` e conferir PID/estado; os próprios scripts são a prova.
- Falha conhecida do HEAD continua valendo; qualquer outra falha é regressão desta rodada.

## Out of Scope

- AppleScript `quit` e qualquer swizzle Objective-C.
- Windows, macOS Intel, Linux.
- Embutir o netcoredbg no app (só proposta).
- Drag-drop, templates, multi-select ou operações fora do workspace na árvore.
- Mudar o fluxo revisável do `Install analysis` (csharp-ls).
- Push, PR, merge, release ou substituir `/Applications/OpenRidr.app`.
- Reformatar o repositório (`cargo fmt`/clippy globais) ou pagar dívida alheia.

## Further Notes

- **BUG-007 não reproduz no HEAD**: o relatório usou `scripts/openridr` sem `OPENRIDR_APP`, que abre a alpha.4 instalada; as quatro capturas são md5-idênticas e mostram a mensagem correta. O conserto é no script + cobertura E2E, e o relatório precisa da correção.
- **netcoredbg**: o artefato oficial osx-arm64 3.2.0-1092 mede 3.39 MB; o relatório citou ~9 MB. A proposta usa a medição.
- **csharp-ls 0.28.0** traz `tools/net10.0/any` e runtimeconfig `Microsoft.NETCore.App 10.0.0`; `dotnet tool install` funciona com qualquer SDK, só a execução exige o runtime 10.
- **URLs verificadas em 2026-11-04**: `dotnet.microsoft.com/download/dotnet` 200, `/download/dotnet/10.0` 200, `api.nuget.org/v3/index.json` 200, release do netcoredbg 200. `nuget.org/packages/csharp-ls/0.28.0` responde 404 a HEAD mas 200 a GET.
- **Decisões novas** vão em `docs/decisions.md`; a evidência em `docs/evidence/fix-alpha5/`; o status de cada achado no próprio `REPORT.md`.
