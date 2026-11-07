# SPEC — Download do netcoredbg limitado por inatividade (issue #162)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe: [#162](https://github.com/Heldinhow/openridr/issues/162), filha da [#161](https://github.com/Heldinhow/openridr/issues/161). A política é **uma só**: a que a #161 introduziu para o csharp-ls, em [SPEC-TOOLS-DOWNLOAD-STALL](SPEC-TOOLS-DOWNLOAD-STALL.md) — este spec não a rediscute, só diz como o segundo download passa a usá-la.

## Problema

O download do netcoredbg (`crates/host/src/debugger.rs`, `Debugger::download`) tem prazo **total** de 300 s (`ureq` `.timeout()`), que cobre DNS, conexão, headers e corpo — a mesma forma que reprovou o download do csharp-ls em rede lenta na #161. Uma transferência de ~3,4 MB que está recebendo bytes morre no prazo fixo, e a pessoa lê "The download failed" com o progresso subindo na tela.

O alcance é o mesmo da #161: a build distribuída embute o netcoredbg, e `begin_install` recusa instalar quando há adaptador embutido. Quem encontra o prazo é `tauri dev` e a ponte HTTP de dev.

## Solução

O download do netcoredbg passa a ser limitado por **inatividade**, com a política da #161: sem prazo total, `timeout_connect` de 15 s, `timeout_read` e `timeout_write` de 60 s. Uma conexão que para de mandar bytes falha com a frase do app (`http::stalled`), que diz que a conexão parou, por quantos segundos, que nada foi instalado e que dá para tentar de novo; uma conexão que nunca abre continua sendo erro de conexão, não stall.

A política fica num lugar só: `http::download_agent(url, stall)` monta o cliente com os três timeouts e é usado pelo `http::fetch_with` (caminho do csharp-ls) e pelo download do netcoredbg. O laço de redirect do netcoredbg fica onde está, porque ele valida o host oficial a cada hop — o que a política não faz.

## User stories

1. Como pessoa em rede lenta usando `tauri dev`, quero que o download do netcoredbg **termine** em vez de falhar num prazo fixo, para conseguir depurar sem repetir a tentativa até cair numa janela boa.
2. Como pessoa cuja conexão **caiu** no meio do download, quero que ele falhe depois de um silêncio de 60 s, para o painel não ficar em "Downloading…" para sempre.
3. Como pessoa cuja conexão caiu, quero uma mensagem que diga que **a conexão parou** e por quanto tempo ficou muda, para diferenciar de hash errado ou de servidor que recusou.
4. Como pessoa cuja conexão caiu, quero que a mensagem diga que **nada foi instalado nem substituído**, para não duvidar do estado do adaptador.
5. Como pessoa cuja conexão nunca abre, quero que a tentativa desista em segundos e **não** me diga que os dados pararam de chegar, porque nenhum dado era esperado.
6. Como pessoa que mantém o OpenRidr, quero a política de inatividade **num lugar só**, para o próximo download não copiar o defeito nem a correção.
7. Como pessoa que mantém o OpenRidr, quero um teste que **falhe antes e passe depois** numa transferência lenta que progride, para a guarda não ser decorativa.
8. Como pessoa que mantém o OpenRidr, quero que a mensagem de erro não carregue a URL assinada do asset, para o log copiado num relato não vazar o token.
9. Como pessoa que valida a release, quero saber **onde** a correção foi provada e onde não foi, para não vender suporte que não existe.

## Implementation decisions

- **A política mora no cliente HTTP do host.** `http::download_agent(url, stall)` aplica `redirects(0)` + `timeout_connect(DOWNLOAD_CONNECT)` + `timeout_read(stall)` + `timeout_write(stall)`; `http::fetch_with` passa a chamá-lo e o `debugger::download` também. `DOWNLOAD_CONNECT` (15 s) passa a ser `pub(crate)`; `DOWNLOAD_STALL` (60 s), `stalled()` e `is_stall()` já eram.
- **O laço de redirect do netcoredbg fica.** Ele revalida o host oficial (`allowed_url`) a cada hop, coisa que `http::fetch` não faz; trocar por `fetch` exigiria um callback novo no caminho recém-entregue do csharp-ls. A política compartilhada é o cliente, não o laço.
- **A frase é a da #161, palavra por palavra.** `stalled(url, DOWNLOAD_STALL)` — "the connection stalled — no data arrived for 60 seconds. Nothing was installed or replaced; check the network and retry the setup." O botão do painel de Debug se chama "Retry adapter install", não "Retry setup"; o descompasso é aceito em troca de uma frase só, e fica escrito aqui em vez de virar duas.
- **Stall antes dos headers e no meio do corpo são o mesmo defeito.** Antes dos headers: `timed_out(&ureq::Error)` (o `ureq` reporta falha de conexão como `ConnectionFailed`, então um `timeout_connect` não é stall). No corpo: `is_stall(&io::Error)` na leitura. As duas viram a frase do app, nunca o texto do transporte.
- **A URL assinada sai das mensagens.** O asset do netcoredbg vem de um redirect assinado do GitHub; `loggable_url` (já usado na linha de log) passa a alimentar também as duas chamadas de `describe_error` desta função, não só a mensagem nova. Correção de segurança barata, aprovada no grill.
- **Sem prazo total, e isso é decisão herdada.** O `ureq` não combina prazo total com prazo por leitura. O custo aceito é o mesmo da #161: um servidor que pinga um byte a cada 59 s mantém o download vivo, e o teto que existe é `MAX_DOWNLOAD_BYTES` (20 MiB). Resolução de nome continua não interrompível — limite do host, não desta frente.
- **Escopo: só o download.** `MAX_DOWNLOAD_BYTES`, hash, extração, verificação, progresso e a UI não mudam; nenhum `data-testid` novo ou renomeado; nenhuma dependência nova.

## Testing decisions

- **Seam: a janela de stall é parâmetro.** `Debugger::download` delega para `download_with(asset, path, stall)`; o produto passa `http::DOWNLOAD_STALL` e o teste passa 1 s. É o mesmo recurso que a #161 usou em `fetch_with`, e é o que faz o caso "lento que progride" falhar com a política antiga (prazo total de 1 s) e passar com a nova.
- **Onde: folhas isoladas em `debugger.rs` (`mod tests`).** O teste precisa de `OPENRIDR_ENABLE_PROBE=1` (allowlist de host e guarda de endereço liberam loopback só com o probe) e a suíte de unidade proíbe escrever no ambiente do processo: `#[ignore]` + `run_isolated_leaf`, como as folhas que já existem no módulo. O servidor é um `TcpListener` em `127.0.0.1:0` no próprio teste.
- **Casos mínimos**: (1) 3 pedaços de 1 KiB a cada 1 s (~3 s) com janela de 1 s terminam com o arquivo inteiro e o SHA-256 certo; (2) servidor que manda 3 bytes de 1.000 prometidos e cala volta com a frase do app, com "stalled", "60 seconds" (a constante do produto, não a janela do teste) e "Nothing was installed or replaced"; (3) um erro de leitura que não é timeout continua com a frase antiga ("The download was interrupted"), provado na fronteira que mapeia a mensagem.
- **A fiação já tem prova.** Os testes de integração existentes em `crates/host/tests/debugger.rs` dirigem `debug.install` de ponta a ponta contra um servidor local até `ready`; eles passam a exercitar o cliente novo sem mudança.
- **Falha-antes medida** antes de escrever a correção, com a política antiga (prazo total) e a nova (inatividade) na mesma transferência — os números vão para a evidência.
- **Nada de Playwright**: smoke noturno, não prova de sessão (decisão da #67).

## Evidence

A prova é no **host**, não na tela: em `tauri dev` o gate de setup só libera o workspace com `debug.adapterState == "ready"`, e o adaptador é justamente o que o download instala — o painel de Debug fica atrás do gate e o `debug.install` não é alcançável pela UI. Isso é limite do shell de dev (na release o adaptador é embutido e não há download), não defeito desta frente, e a evidência registra "não verificado no app" com esse motivo.

A prova de host usa o **binário de teste do host** (`Host::new` + `dispatch` reais, seam de artefato controlado, `OPENRIDR_HOME` isolado) com `debug.plan` + `debug.install` contra um servidor local: uma transferência lenta que progride até `ready` e um stall que volta com a frase do app. A ponte HTTP de dev não serve para isso — ela também não alcança `debug.install`, porque `require_trust` exige setup pronto e o setup exige o adaptador que o download instala (medido: `trust.grant` responde `setup_required`).

## Out of scope

- **A UI do painel de Debug e o gate de setup**: nenhuma linha, nenhum testid. A circularidade gate × instalação em `tauri dev` fica registrada na evidência como limite de dev.
- **Cancelar o download pela UI**, retomar com Range, mudar `MAX_DOWNLOAD_BYTES`, o hash ou a extração.
- **O download do csharp-ls**: já feito na #161; aqui só se reusa o cliente.
- Windows e macOS Intel.

## Further notes

- Evidência em `docs/evidence/debugger-download-stall.md`; retro em `docs/RETRO-DEBUGGER-DOWNLOAD-STALL.md`.
- Decisão registrada em `docs/decisions.md` (2026-11-07); `docs/README.md` ganha a linha da frente.
- A política e o raciocínio do `ureq` estão em [SPEC-TOOLS-DOWNLOAD-STALL](SPEC-TOOLS-DOWNLOAD-STALL.md); este spec não os repete.
