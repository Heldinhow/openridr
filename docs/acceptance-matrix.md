# Matriz de aceitação P0 — OpenRidr

Data: 28/10/2026. Base de código: `4b8885c` + o commit `feat(acceptance)` desta entrega.
Escopo: fatia P0 (tickets 01–17) e consolidação do ticket 18. Documento em PT-BR; UI em inglês.

**Decisão desta matriz: release NÃO aprovado.** A matriz registra o que foi realmente
executado, o que está bloqueado e o que não foi verificado. Ela não transforma ausência de
prova em aprovação, não cobre Linux/outras arquiteturas e não é parecer jurídico.

## Como ler os estados

| Estado | Significado |
|---|---|
| `approved` | A cláusula foi exercitada naquele ambiente e a evidência observada está registrada. Não vale para outro ambiente. |
| `blocked` | A cláusula não pode ser satisfeita com a evidência atual, ou um gate obrigatório de release está aberto. |
| `not_verified` | Não foi executado naquele ambiente. Ausência de prova; nunca “passou”. |

Evidência: `E01`…`E17` = `docs/evidence/NN-*.md`; `A18` = `docs/evidence/18-aceitacao-e-release.md`;
`L18-*` = `docs/evidence/logs/18-*`; `S18-*` = `docs/evidence/screenshots/18-*`.
Os logs de cada ticket (comandos, saídas e exits) estão em `docs/evidence/logs/`.

## Ambientes

| Ambiente | SO / arquitetura | Versões observadas | SDK host | SDK do projeto | TFM | Ferramentas/assets | Executado |
|---|---|---|---|---|---|---|---|
| macOS ARM64 (esta máquina) | macOS 27.2 (26B118), Apple M2, arm64; Xcode 27.1 | Bun 1.3.9; Node 26.10.0; Rust/cargo 1.99.0; Chrome 155.0.8059.85; Playwright 1.64.1 | .NET SDK 10.0.100 (`/usr/local/share/dotnet`) | 8.0.408 em `~/Library/Application Support/OpenRidr/sdks/8.0.408`, `global.json` `rollForward: disable` | net8.0 | csharp-ls 0.28.0; netcoredbg 3.2.0-1092 (`osx-arm64`, community supported) | Sim, jornada real + suítes por ticket |
| macOS Intel | Não executado | — | — | — | — | netcoredbg 3.2.0-1092 não publica `osx-x64`; nenhuma versão substituta foi baixada | Não: sem host Intel nesta sessão |
| Windows x64 | Não executado | — | — | — | — | Asset `netcoredbg-win64.zip` existe no metadata, mas não foi instalado/executado | Não: sem host/CI Windows; UI desktop e DAP exigem smoke próprio |

O repositório é local (sem remote); não há CI Windows configurado. CI que só exercite CLI/contratos
não comprova a UI desktop e não fecharia o gate. Nenhum mínimo de macOS/Windows foi inventado;
as versões acima são as reais desta máquina.

## Matriz AC-01…AC-34 × ambiente

| AC | Cláusula (resumo) | macOS ARM64 | macOS Intel | Windows x64 |
|---|---|---|---|---|
| AC-01 | Abrir não confiável: caminho completo, trust desmarcado, edição textual e ausência de MSBuild/restore/LSP/tarefas/build/run/debug/runner/PTY | **approved** — E01/E02 + A18: restrito, 12 chamadas diretas `trust_required`, edição/save, sem `bin/obj`; L18-acceptance-summary | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows; UI desktop não executada |
| AC-02 | Inventário fora do workspace; SDK do projeto separado do host 10 e do TFM; SDK faltante não reescreve `global.json` | **approved** — E02/E05 + A18: host 10.0.100, projeto 8.0.408, TFM net8.0, `global.json` intacto | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-03 | Restore explícito; build/run/test não restauram; falha mantém contexto parcial, log e retry | **approved** — E04/E11/E12/E16/E17 + A18: restore exit 0, build/test bloquearam antes do restore, falha/cancel/retry registrados | **not_verified** — sem host Intel | **not_verified** — restore/build/test Windows não executados |
| AC-04 | Cancelar plano não baixa/executa; confirmar mostra versão/fonte/asset/arquitetura/destino/operação/permissões; trust e opt-in não substituem confirmação | **approved** — E05/E13 + A18: planos reais csharp-ls e netcoredbg; flag/trust separados | **not_verified** — instalação Intel não executada; asset osx-x64 ausente | **not_verified** — instalação/plano Windows não executados |
| AC-05 | downloading/installing/verifying, sucesso, cancelamento, falha, retry; log sobrevive; parcial não vira ready; executável/versão verificados; sem SDK de sistema | **approved** — E05/E13 + A18: csharp-ls 0.28.0 e netcoredbg 3.2.0-1092 reais; versões verificadas; nenhum SDK instalado pelo app | **not_verified** — sem host Intel | **not_verified** — caminho Windows existe no código, não executado |
| AC-06 | Asset incompatível/ausente, origem não validada ou verificação falha bloqueiam; sem downgrade silencioso/global; percentual só medido | **approved** — E05/E13 + testes de fronteira: hash/redirect/zip/traversal/versão; limitação: NuGet imprime “Skipping NuGet package signature verification”; SHA do netcoredbg vem do digest do GitHub, não de assinatura/notarização | **not_verified** — ausência de `osx-x64` é bloqueio documental, não execução em Intel | **not_verified** — asset win64 não instalado/validado |
| AC-07 | Revogar trust bloqueia novas execuções, encerra serviços e solicita término; remanescentes visíveis; sem alegação falsa de limpeza | **approved** — E02/E04/E05/E06/E11/E12/E15/E16/E17 + A18: revogação com buffer preservado, chamada direta `trust_required`, LSP parado, processos observados | **not_verified** — sem host Intel | **not_verified** — término Windows não executado |
| AC-08 | Abrir sln/projeto; ambiguidade pede escolha; console/biblioteca/testes com referências/TFM/pacotes; bin/obj ocultos; restrito só contorno textual | **approved** — E01/E03 + A18: três projetos avaliados, TargetPath, árvore sem bin/obj | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-09 | Projeto com falha de avaliação permanece visível com abrir/log/retry; restore incompleto explícito | **approved** — E03/E04: MSB4019, restore parcial, retry | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-10 | Mudança de contexto invalida avaliação; TargetPath corresponde ao contexto MSBuild, nunca caminho inferido/obsoleto | **approved** — E03/E04/E11/E12/E14 + A18: revisões 1→2→3, TargetPath Debug/net8.0 | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-11 | Sem solução/host/LSP: editar/salvar; abas/breadcrumb/modificado; fechar/conflito com decisão explícita | **approved** — E01/E07 + A18: edição restrita/save, fechamento com descarte, conflito coberto por E07 | **not_verified** — sem host Intel | **not_verified** — CRLF/paths/permissões Windows não executados |
| AC-12 | Completion/definição reais; rename console↔biblioteca com prévia; cancelar não altera; mudança concorrente invalida | **approved** — E08/E09 + A18: completion `Id`, definição `Order.cs:4:11`, rename 3 arquivos/6 edições com cancelamento inerte | **not_verified** — sem host Intel | **not_verified** — UTF-16/CRLF Windows não executados |
| AC-13 | Code action só se retornada no contexto; aplicar não marca diagnóstico corrigido; ausência não fabrica quick fix | **approved** — E10 + A18: 6 ações reais listadas, prévia cancelada; L18-code-actions | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-14 | Problems mantém severidade/código/mensagem/posição/origem/categoria/revisão; LSP e Build não se fundem | **approved** — E06/E11 + A18: `CS0246` em LSP e Build separados, origem/categoria/revisão | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-15 | Editar buffer torna diagnóstico/build anterior obsoleto; publicação vazia só cobre documento/revisão; restore/analyzers não viram garantia global | **approved** — E06/E11 + A18; limitação: `analyzerCoverage: not_verified`; publicação vazia é document-scoped | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-16 | LSP: carga, falha, log, reinício; coexistência host 10 + projeto 8; falha mantém edição e log | **approved** — E05/E06 + A18: csharp-ls 0.28.0.0, hostroot runtime 10 + SDK 8; reavaliação posterior deixou cobertura `partial` explicitamente | **not_verified** — sem host Intel | **not_verified** — symlink/hostroot/CRLF Windows não executados |
| AC-17 | Startup/config/TFM explícitos; biblioteca/ambiguidade não inicia; comando/contexto/stdout/stderr/estado visíveis | **approved** — E11/E12 + A18: startup console, Debug/net8.0, comando/cwd/SDK/log | **not_verified** — sem host Intel | **not_verified** — Shift+F10/processos Windows não executados |
| AC-18 | Build falho bloqueia Run/Debug; erro com posição abre fonte; texto não reconhecido fica no log; corrigir/salvar/recompilar permite executar | **approved** — E11/E12/E14 + A18: falha CS0246 exit 1, navegação, rebuild exit 0, Run OK | **not_verified** — sem host Intel | **not_verified** — build/erro/cancelamento Windows não executados |
| AC-19 | Buffer salvo depois do build não é fonte verificada; execução identifica revisão salva e exige decisão | **approved** — E11/E12: `savedNote`, hashes de fontes, aviso “Sources were saved after this build”; A18 executou o build salvo | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-20 | Cancelar build/run pede término real da árvore; exit conhecido preservado, ausente continua desconhecido, falha explícita | **approved** — E11/E12: cancelamentos reais, requested/observed separados; limitação: `exit:null` foi provado por unidade/boundary, o cancelamento real observou 143 | **not_verified** — sem host Intel | **not_verified** — TerminateProcess/árvore Windows não executados |
| AC-21 | Onde asset/plataforma habilitada: build Debug, launch da saída avaliada, breakpoint pendente→verificado, pausa, continue, step over/into, frames, locals, stop e logs | **approved (experimental opt-in)** — E14 + A18: netcoredbg 3.2.0-1092 `--interpreter=vscode`, breakpoint verificado, pausa, F8, locals, continue exit 0, stop observado; async/multi-thread em E14 | **not_verified** — debug indisponível: sem asset osx-x64; núcleo Intel não executado | **not_verified** — DAP Windows exige smoke próprio; asset win64 não executado |
| AC-22 | Build falho/adaptador ausente bloqueiam launch; breakpoint sem símbolos fica pendente; locals antigos não são atuais; exit ausente não vira zero nem prova limpeza | **approved** — E14/E13/E11 + A18: bloqueio antes do build, breakpoint pendente, locals limpos no continue, exit/termination; limitações: `runInTerminal` real não emitido, variáveis aninhadas não expandidas, `exit:null` por boundary | **not_verified** — sem host Intel | **not_verified** — DAP Windows não executado |
| AC-23 | Mac ARM64 inicia flag OFF/aviso experimental, Debug bloqueado, Run independente; opt-in não instala/prova; retirada exige gate DAP/async/multi-thread | **approved** — E13/E14 + A18: OFF por padrão, opt-in de sessão, instalação/launch separados; prova DAP real existe, mas a flag foi mantida; #256 segue risco não reproduzido | **not_verified** — N/A fora de macOS ARM64 | **not_verified** — N/A fora de macOS ARM64 |
| AC-24 | Mac Intel sem asset 3.2 mostra indisponibilidade e não baixa versão divergente; Windows x64 exige prova DAP própria; limitações não bloqueiam análise/run/testes | **blocked** — E13/E14 + A18: a parte Mac ARM64 (limitações não bloqueiam análise/Run/testes) está provada, mas a cláusula exige Intel e Windows; Intel sem `osx-x64` e Windows sem smoke DAP | **not_verified** — sem hardware Intel; indisponibilidade é decisão documental, não execução | **not_verified** — DAP Windows obrigatório e não executado |
| AC-25 | Após trust, um PTY com shell/cwd reais; entrada, output e Ctrl+C nos dois SOs; antes de trust não abre shell | **approved** — E15 + A18: `/bin/zsh`, cwd com espaço, `pwd`, `echo`, Ctrl+C em `sleep 30`, `dotnet --version` 8.0.408 | **not_verified** — sem host Intel | **not_verified** — ConPTY/input/Ctrl+C Windows não executados |
| AC-26 | Fechar shell ativo oferece manter/encerrar; saída/código são do shell; prompt não é shell encerrado; término e remanescentes observados | **approved** — E15 + A18: diálogo Keep/Terminate, `SIGTERM then SIGKILL`, `killedAfterGrace:true`, exit null; processo observado | **not_verified** — sem host Intel | **not_verified** — término/ChildKiller Windows não executados |
| AC-27 | Descoberta VSTest real da fixture xUnit; projeto/TFM/seleção explícitos; Run all/selected respeitam escopo | **approved** — E16/E17 + A18: 4 identidades, TFM net8.0, selected com filtro, all com 4 resultados | **not_verified** — sem host Intel | **not_verified** — VSTest/CRLF/cancelamento Windows não executados |
| AC-28 | passed/failed/skipped reais; mensagem/stack/output; navegar só com posição fornecida, senão log | **approved** — E17 + A18: 2 passed/1 failed/1 skipped, detalhes/stack, posição só quando fornecida | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-29 | Cancelar descoberta/execução preserva parciais e marca cancelled; ausentes não viram passed/skipped; infra ≠ asserção, com logs/retry | **approved** — E16/E17 + A18: cancelamento real de `Task.Delay(60000)`, 0 results parciais, exit 143, sem passed inventado | **not_verified** — sem host Intel | **not_verified** — cancelamento Windows não executado |
| AC-30 | Vazio, adapter ausente e descoberta falha distintos; nenhuma ação instala/edita packages; contagens pertencem à execução/revisão | **approved** — E16/E17 + A18: 4/5 identidades por descoberta, contagens da execução, sem alteração de packages | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-31 | MTP recebe mensagem exata e fallback de terminal; nenhum VSTest/conversão; TRX não descobre | **approved** — E16/E17: mensagem exata, comando vazio, sem processo; TRX só resultados | **not_verified** — sem host Intel | **not_verified** — sem host/CI Windows |
| AC-32 | Sete telas/tema/strings inglesas, foco visível, nomes acessíveis, teclado e estados textuais; atalhos por SO respeitam gates | **approved com limitações** — E01–E17 + A18: S18-screen-01…07 a 1440×900, auditoria heurística de nomes/foco/contraste/overflow, layout 1100×700; limitações: sem leitor de tela, janela Tauri nativa não dirigida, teclado Windows só por código | **not_verified** — sem host Intel | **not_verified** — UI desktop/teclado Windows não executados |
| AC-33 | Registrar matriz macOS Intel/ARM64/Windows x64 do núcleo; publicar Mac e Windows juntos com debug separado e limitações | **blocked** — matriz e decisão registradas (este documento + `docs/release-decision.md`), mas a publicação conjunta está retida: Windows/Intel não verificados; Mac sozinho não é entrega reduzida | **not_verified** — ambiente não executado | **not_verified** — ambiente não executado |
| AC-34 | Auditar licenças/avisos de componentes/assets incorporados ou baixados, preservar obrigações e identidade; auditoria não é liberação jurídica | **blocked** — auditoria feita (A18), mas faltam `LICENSE`/`NOTICE`/`THIRD_PARTY_NOTICES`, o ícone é o default do Tauri e não há revisão jurídica; MPL-2.0 transitivo exige avisos | **not_verified** — grafo Intel específico não executado | **not_verified** — grafo Windows específico não auditado em runtime |

**Contagem das 102 células AC × ambiente:** 31 `approved`, 3 `blocked`, 68 `not_verified`.
Os 3 bloqueios Mac são AC-24, AC-33 e AC-34; todas as células Intel/Windows são `not_verified`.
Não há percentual de cobertura nem contagem de arquivos inventada.

## Gates compartilhados G1–G5

| Gate | macOS ARM64 | macOS Intel | Windows x64 |
|---|---|---|---|
| G1 — trust e fronteira | **approved** — chamadas diretas negativas em 12 métodos, consumidores reais, revogação e processos observados (E02/E04/E11/E12/E15/E16/E17 + A18) | **not_verified** | **not_verified** |
| G2 — contexto e integridade | **approved** — `contextId`, respostas tardias, revisões, paths canônicos, exit `null` distinto de 0 (E03–E07/E11/E12/E14/E16/E17 + A18) | **not_verified** | **not_verified** |
| G3 — consentimento e instaladores | **approved com limitações** — planos/confirmação/cancelamento/hash/extração reais; sem assinatura NuGet e sem notarização do netcoredbg (E05/E13 + A18) | **not_verified** — sem asset osx-x64 | **not_verified** — asset win64 não executado |
| G4 — UI e privacidade | **approved com limitações** — UI inglesa, tema, foco, nomes, teclado e logs locais; sem leitor de tela, janela nativa não dirigida, teclado Windows só por código (A18) | **not_verified** | **not_verified** |
| G5 — validação e proveniência | **approved** — matriz, logs, comandos, exits e limitações por ambiente; release bloqueado pelos gates abertos | **not_verified** | **not_verified** |

## Matriz de Debug (separada do núcleo)

| Plataforma | Asset da versão aprovada | Flag / opt-in | Prova DAP real | Decisão |
|---|---|---|---|---|
| macOS ARM64 | `netcoredbg-osx-arm64.zip` 3.2.0-1092, community supported; SHA-256 do digest oficial | Flag experimental **OFF por padrão**; opt-in de sessão explícito; ligar não instala nem prova DAP | Sim: E14 (launch, breakpoint, step, stop, async/multi-thread) + A18 (launch, breakpoint verificado, F8, locals, continue exit 0, stop com término) | **Experimental, flag mantida.** Não é suporte de release; #256 segue risco não reproduzido. A retirada da flag exigiria nova prova no build corrente; nenhuma remoção foi feita. |
| macOS Intel | Nenhum asset `osx-x64` na release 3.2.0-1092 | Sem substituição de versão; `unsupported_platform` | Não executado | **Indisponível/não verificado.** Debug bloqueado; não há promessa de cobertura universal Mac. Núcleo Intel também não foi executado. |
| Windows x64 | `netcoredbg-win64.zip` existe no metadata | Asset/instalação não executados | Não executado | **Gate aberto.** Exige smoke DAP próprio (launch, breakpoint, step, frames, locals, stop, processos) antes de qualquer aprovação Windows. |

Linux e outras arquiteturas Windows estão fora do P0; não há promessa.

## .NET: o que está comprovado

| Alegação | Estado | Evidência |
|---|---|---|
| Fixture `OrderDesk` net8.0 com console, biblioteca e testes xUnit | **comprovado** | E01/E03 + A18; TFM net8.0 nos três projetos |
| SDK do projeto 8.0.408 fixado por `global.json` com `rollForward: disable` | **comprovado** | E02/E03/E05 + L18-environment; nenhum arquivo reescrito |
| Host de análise SDK 10.0.100, separado do SDK do projeto | **comprovado** | E05/E06 + A18 (hostroot runtime 10, SDK 8 em `sdk/`) |
| Build/Run/Debug/VSTest no TFM net8.0 | **comprovado neste ambiente** | A18 (exit 1→0, Run exit 0, 4 resultados, DAP exit 0) |
| .NET 8+ em todos os TFMs/alvos | **não comprovado** | Só net8.0 foi exercitado; não herdar aprovação |
| net6 | **best effort, não verificado** | Nenhuma execução; sem garantia equivalente |
| Windows .NET / macOS Intel .NET | **não verificado** | Sem execução nesses ambientes |
| Versões mínimas de SO | **não declaradas** | Registradas apenas as versões reais desta máquina; sem mínimo inventado |
| Linux/outras arquiteturas | **fora do escopo** | Sem promessa |

## Comandos, contexto e exits observados

Contexto real: cópia temporária de `fixtures/OrderDesk` em diretório com espaço no nome; host
`openridr-host-http` em `127.0.0.1:4317`; Vite em `1420`; Chrome real; ferramentas reais.
Logs: `L18-acceptance-summary.json`, `L18-acceptance-pre-revoke.json`, `L18-acceptance-final.json`,
`L18-restore.json`, `L18-build-failed.json`, `L18-build-fixed.json`, `L18-run.json`,
`L18-tests-run-all.json`, `L18-tests-cancel.json`, `L18-terminal.json`, `L18-dap.json`,
`L18-debug-install.json`, `L18-lsp-session.json`, `L18-ui-audit.json`, `L18-playwright-*.log`.

| Operação | Comando/contexto | Resultado observado | Exit |
|---|---|---|---|
| Build frontend | `bun run build` | tsc + Vite; 62 módulos; aviso de chunk >500 kB | 0 |
| Host | `cargo test -p openridr-host` | 192 testes passaram, 0 falharam (somatório dos binários; doc-tests 0) | 0 |
| E2E completo | `bun run test:e2e` | 49 testes passaram | 0 |
| E2E aceitação | `bun run test:e2e tests/e2e/zz-acceptance.spec.ts` | 2 testes passaram | 0 |
| Tauri debug | `bun run tauri build --debug --no-bundle` | app em `src-tauri/target/debug/openridr` | 0 |
| Restore | `dotnet restore OrderDesk.sln -nologo -v:minimal` (argv separado) | três projetos `restored` | 0 |
| Build falho | `dotnet build OrderDesk.Cli.csproj -c Debug ... --no-restore` | CS0246, um problema navegável | 1 |
| Build corrigido | idem, fonte original salva | sucesso | 0 |
| Run | `dotnet <TargetPath>` do build Debug/net8.0 | stdout `Order A-104` | 0 |
| Testes selecionados | `dotnet test ... --filter FullyQualifiedName=...` | 1 passed | 0 |
| Testes todos | `dotnet test ...` (4 identidades) | 2 passed, 1 failed, 1 skipped | 1 (falha de asserção) |
| Teste longo cancelado | `dotnet test ... --filter ...Waits_for_cancellation` | `cancelled`, 0 results parciais, TRX missing; sem passed inventado | 143 observado |
| PTY | `/bin/zsh` no cwd do workspace; `pwd`, `echo`, `sleep 30` + Ctrl+C, `dotnet --version` 8.0.408 | shell/cwd reais; `^C`; prompt; `8.0.408` | shell `null` (unknown) após SIGTERM→SIGKILL; `killedAfterGrace:true` |
| DAP | `netcoredbg --interpreter=vscode`; launch do assembly Debug; breakpoint; F8; continue; stop | breakpoint verificado; pausa; locals; exit 0; stop com `termination` observada | debuggee 0; adapter 0 na primeira sessão; segunda sessão cancelada com término observado |

`exit:null` significa desconhecido; não foi convertido em 0. Falha de término continua visível.

## Limitações e lacunas que esta matriz não esconde

- **Windows x64**: nenhum núcleo, UI desktop, PTY, VSTest ou DAP executado; CI local não existe.
- **macOS Intel**: nenhum núcleo executado; sem asset osx-x64 do netcoredbg 3.2; sem substituição.
- **Janela Tauri nativa**: o fluxo foi dirigido no Chrome real contra Vite + host-http (mesmo
  `dispatch`); TCC bloqueou captura/automação da janela nativa no macOS. Smoke humano da janela
  nativa continua recomendado.
- **Acessibilidade**: auditoria heurística por Playwright (nomes, foco, contraste, overflow,
  strings), não WCAG formal; sem leitor de tela.
- **Analyzers/pull diagnostics**: `analyzerCoverage` permanece `not_verified`; só push diagnostics
  foi consumido; MTP é explicitamente não suportado.
- **Rede/segurança de suprimentos**: csharp-ls via NuGet com verificação de assinatura pulada pelo
  próprio `dotnet tool install`; netcoredbg usa SHA-256 do digest do GitHub, sem assinatura/
  notarização; o host não inventa hash/assinatura.
- **Sessões/contadores globais**: os testes por ticket assumem sessão/sequência inicial; o spec de
  aceitação roda por último (`zz-acceptance.spec.ts`) para não alterar essas premissas. Se a ordem
  for alterada, os testes de ticket precisam ser revistos.
- **Package/legal**: ver `docs/release-decision.md`; a auditoria é técnica, não parecer jurídico.
