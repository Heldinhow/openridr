# RETRO — Download de ferramenta limitado por inatividade (issue #161)

Rodada de 2026-11-07, `59f7eea` (+ `cae8954` da spec), macOS arm64. Spec: [SPEC-TOOLS-DOWNLOAD-STALL](SPEC-TOOLS-DOWNLOAD-STALL.md). Evidência: [evidence/tools-download-stall.md](evidence/tools-download-stall.md). Tickets: #163, #164, #165; filha #162 (netcoredbg).

## O que se sustentou

- **A semântica do `ureq` era a resposta, não um número novo.** `timeout_read` é aplicado **por leitura do socket** (`set_read_timeout`), que é exatamente "nenhum byte em N segundos". Trocar `.timeout(120s)` por `timeout_connect(15s)` + `timeout_read(60s)` foram duas linhas; o resto do diff é mensagem, teste e doc. Nada de watchdog, nada de thread.
- **A falha-antes foi medida três vezes, com números.** Com o prazo total de 400 ms a transferência de 1,5 s morria em 16 KiB e a de 2,4 s no primeiro pedaço; com inatividade as duas chegam ao fim (61.440 e 49.152 bytes). A mensagem: sem o mapeamento de `TimedOut` o usuário lê o dump do transporte. A conexão: sem a guarda de `ErrorKind::Io`, um `timeout_connect` de 15 s era anunciado como "no data arrived for 60 seconds".
- **O host isolado deu a prova longa antes de gastar o slot do app.** A ponte HTTP completou uma transferência de 146 s (26 s além do prazo antigo) com progresso medido e `ready` no fim, sem UI e sem build do Tauri.
- **A prova no app foi barata porque o caminho é de dev.** `tauri dev` + seams de debug (`OPENRIDR_ENABLE_PROBE`, `OPENRIDR_TOOLS_MUXER`, `OPENRIDR_TOOLS_PACKAGE_URL`/`_SHA512`) + servidor local: ~150 s de download com a barra subindo até `Verified`, e o stall com a frase do app e o `Retry setup` funcionando. Duas execuções, ~6 min no total.
- **O `Retry setup` que já existia bastou.** Nenhum estado novo, nenhum botão novo: o `failed` de hoje já oferecia o caminho de volta, e a mudança foi só a condição de falha e a frase.

## O que quebrou / custou

- **O gate não mostrava o erro da ferramenta — e o gate é a única superfície.** `SetupGate` renderiza `error ?? setup.error`; `tool.error` (onde a frase do download mora) nunca aparecia. O stall na tela era `Setup failed` + `Failed`, sem motivo, e o painel de ferramentas que mostra `tool.error` fica **atrás** do gate, inalcançável enquanto o setup bloqueia o workspace. Só apareceu porque a prova foi feita na tela; teste de host nenhum pegaria isso. Corrigido com uma linha e o testid novo `setup-gate-tool-error`.
- **`tauri dev` numa worktree nova não builda sem os diretórios de recurso.** O `tauri-build` valida `resources/{netcoredbg,csharp-ls,dotnet}` do `tauri.conf.json` e falha com `resource path 'resources/csharp-ls' doesn't exist`; os recursos só são gerados no `beforeBuildCommand`. Para provar o download (que só existe **fora** de um `.app`) bastou criar os diretórios vazios — que são gitignored. Fica como ponteiro para a próxima sessão que precisar do shell de dev.
- **Dois testes de host falharam por carga no run completo, em runs diferentes.** `build_start_needs_the_current_evaluation…` (`restore_required` em vez de `invalid_state`, a assinatura já registrada na #112) e `initialize_failure_surfaces_the_sdk_message…`, ambos com o `tauri dev` e um servidor local rodando em paralelo; isolados 3/3 e 1/1 verdes, e o `--test host` completo verde na máquina ociosa. Registrados em `docs/evidence/test-stability.md` como hipótese de carga, causa não determinada.
- **Captura com caminho pessoal.** A linha "Analysis runtime · Found 10.0.14 at `/Users/<nome>/.dotnet`" obrigou a redigir a faixa antes de commitar as três imagens; a primeira tentativa de recorte errou a coordenada e deixou o caminho visível na revisão da própria imagem.
- **O `Setup failed` do topo continua no dev por causa do netcoredbg.** Fora de um `.app` o adaptador embutido não existe e a flag experimental está desligada; a linha do csharp-ls mostra `Verified` ao lado. Não é regressão desta frente, mas confunde quem lê a captura sem contexto — a evidência diz isso em voz alta.

## Revisão independente (rodada 1) — três médios e três baixos, todos tratados

- **Médio: erro de conexão virava stall.** `timed_out` olhava só a cadeia de `source`, e `TcpStream::connect_timeout` também produz `TimedOut`. Agora a detecção exige `ErrorKind::Io` (o `ureq` reporta falha de conexão como `ConnectionFailed`), com teste em TEST-NET-1 e teto de 300 ms provando que a frase de stall não aparece. Falha-antes medida.
- **Médio: escrita sem teto.** Sem o prazo total, `timeout_write` ficava sem limite e uma escrita bloqueada (peer que parou de ler) penduraria o setup. Passa a receber a mesma janela de stall. A **resolução de nome** continua não interrompível pelo `ureq` — e o prazo total anterior também não a limitava (é conferido entre operações, depois do resolvedor) — então virou limite documentado no spec e no código, não um teto fingido.
- **Médio: a fronteira do instalador não tinha teste.** O teste do corpo provava `is_stall` + a função de mensagem, e continuaria verde se `tools.rs` parasse de usar o mapeamento. O mapeamento virou `download_read_error` com teste próprio (timeout → frase do download; outro erro de leitura → a frase antiga).
- **Baixos.** A folga do teste de progresso subiu de 4× para 5× (intervalo de 200 ms, janela de 1 s); a linha nova de `test-stability.md` passou a dizer "hipótese: carga; causa não determinada"; a decisão e o spec deixaram de afirmar "sem testid novo" e passaram a nomear `setup-gate-tool-error`.

## Lições

- **O doc da biblioteca decide o desenho.** "`.timeout()` tem precedência sobre `.timeout_read()`" matou a alternativa "prazo total maior + progresso" antes de virar código: em `ureq` 2.x é um ou outro. Ler a fonte do cliente (e não só a doc) também foi o que separou `ConnectionFailed` de `Io`.
- **Mensagem de rede precisa distinguir "parou de mandar" de "nunca conectou".** As duas terminam em `TimedOut` na cadeia de `source`; só o tipo do transporte as separa. Frase de inatividade para conexão que nunca abriu é mentira verificável.
- **Testar a função de mensagem não é testar a fronteira.** O mapeamento tem que ter teste no módulo que o usa, senão o teste passa enquanto o produto mostra outra coisa.
- **A superfície onde a pessoa está é a que precisa mostrar o erro.** O painel de diagnóstico atrás do gate não conta: enquanto o setup bloqueia o workspace, o gate é o produto. Foi o único achado que só a prova na tela revelou.
- **Prova longa primeiro no host, app depois.** A ponte HTTP deu o número (146 s) sem build de Tauri; o slot do app foi gasto só no que a tela precisava provar (progresso, frase, retry).

## Follow-ups

- **#162**: o download do netcoredbg tem a mesma forma com prazo total de 300 s; deve reusar esta política, não copiá-la.
- **Cancelar o download pela UI**: o host tem `tools.cancel`, a UI não o expõe. Sem teto total, um servidor que pinga um byte a cada 59 s mantém o download vivo até o limite de 32 MiB.
- **Retomar download interrompido** (Range/partial): hoje o pacote é baixado do zero a cada tentativa.
- **Resolução de nome sem teto**: limite conhecido do host inteiro (não só deste download); resolver de verdade exigiria resolver fora do `ureq` com prazo próprio.
