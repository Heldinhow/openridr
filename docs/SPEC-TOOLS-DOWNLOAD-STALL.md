# SPEC — Download de ferramenta limitado por inatividade, não por prazo total (issue #161)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#161](https://github.com/Heldinhow/openridr/issues/161); a mesma forma no download do netcoredbg fica na filha [#162](https://github.com/Heldinhow/openridr/issues/162).

## Problema

O download do pacote do csharp-ls no setup tem prazo **total** de 120 s (`ureq` `.timeout()`), que cobre DNS, conexão, headers e corpo. Em rede lenta o pacote de 21,6 MB não termina nesse prazo — medido 151,19 s na #93, reproduzido em `origin/main` limpo — e o setup falha com "Could not download csharp-ls" **com o download progredindo**. O progresso na tela é medido (bytes lidos do socket), então a pessoa vê os bytes subindo e, de repente, um erro que não corresponde ao que aconteceu.

O alcance é menor do que parece: o download só existe **fora** de um `.app`. A build distribuída embute o csharp-ls e o runtime privado, e o plano de setup recusa baixar quando o par está presente (e falha fechado quando está danificado). Quem encontra o prazo é `tauri dev` e a ponte HTTP de dev, não o DMG.

## Solução

O download passa a ser limitado por **inatividade**, não por tempo total: uma conexão que manda bytes continua até terminar, por mais lenta que seja; uma conexão que **para** de mandar bytes falha depois de 60 s de silêncio, com uma frase do app que diz o que aconteceu, que nada foi instalado e que dá para tentar de novo. A tentativa de conexão tem o seu próprio teto de 15 s, para uma conexão que nunca abre não deixar o setup pendurado.

O retry já existe (`install-retry` → `setup.start`); o que muda é a condição de falha e o texto dela.

## User stories

1. Como pessoa em rede lenta abrindo um projeto em `tauri dev`, quero que o download do csharp-ls **termine** em vez de falhar num prazo fixo, para o setup não me obrigar a tentar de novo até cair numa janela boa.
2. Como pessoa em rede lenta, quero que a barra de progresso **não pare** de contar enquanto os bytes chegam, para saber que o download está vivo.
3. Como pessoa cuja conexão **caiu** no meio do download, quero que ele falhe em algum momento, para o setup não ficar em "Downloading…" para sempre.
4. Como pessoa cuja conexão caiu, quero uma mensagem que diga que **a conexão parou** e por quanto tempo ela ficou muda, para diferenciar de um erro de hash ou de um servidor que recusou.
5. Como pessoa cuja conexão caiu, quero que a mensagem diga que **nada foi instalado nem substituído**, para não ficar na dúvida sobre o estado da ferramenta.
6. Como pessoa cuja conexão caiu, quero um botão **Try again** que roda o plano de novo, para recuperar sem reabrir o workspace.
7. Como pessoa atrás de um proxy lento, quero que um primeiro byte demorado não seja tratado como conexão morta, para o proxy não virar um erro falso.
8. Como pessoa cuja conexão nunca abre, quero que o setup **desista** da conexão em segundos, não em minutos.
9. Como pessoa que mantém o OpenRidr, quero a política de rede do download num lugar só, para o próximo download (netcoredbg, #162) reusá-la em vez de copiar o defeito.
10. Como pessoa que mantém o OpenRidr, quero um teste que **falhe antes e passe depois** numa transferência lenta que progride, para a guarda não ser decorativa.
11. Como pessoa que mantém o OpenRidr, quero um teste do caso de stall com a frase do app, para a metade "erro claro" do critério também ter guarda.
12. Como pessoa que valida a release, quero prova no app Tauri real (progresso subindo e, depois, o stall com retry), para saber que a correção vale no binário e não só no teste.
13. Como pessoa que mantém o OpenRidr, quero que o que não foi verificado apareça como não verificado, para não vender suporte que não existe.

## Implementation decisions

- **Inatividade, não prazo total.** O cliente do download deixa de usar `.timeout()` (total, e que por contrato do `ureq` tem precedência sobre os timeouts por operação) e passa a usar `.timeout_connect(15 s)` + `.timeout_read(60 s)`. `timeout_read` é o tempo máximo de **cada** leitura do socket: a transferência só morre quando passa 60 s sem um byte.
- **Escrita e conexão têm o mesmo teto.** `timeout_write` recebe a mesma janela de stall: uma escrita que trava porque o outro lado parou de ler não pode pendurar o setup. `timeout_connect` fecha a conexão que não abre; um **erro de conexão não é stall** e não recebe a frase de inatividade (o `ureq` o reporta como `ConnectionFailed`, e a detecção exige `Io` antes de olhar a cadeia de `source`).
- **Limite conhecido: resolução de nome.** O `ureq` não interrompe a resolução DNS, e o prazo total que existia antes também não a limitava (ele é conferido entre operações, depois de o resolvedor voltar). O resolvedor do sistema tem os próprios timeouts, e isso vale para toda requisição do host — não é regressão desta frente, e fica escrito em vez de fingir um teto que não existe.
- **Sem teto total, e isso é decisão registrada.** O `ureq` não permite prazo total e por leitura ao mesmo tempo, e um teto manual exigiria watchdog. O custo aceito: um servidor que pinga um byte a cada 59 s mantém o download vivo até o limite de 32 MiB do pacote, e a UI não tem botão de cancelar (só retry). O limite de bytes é o teto que existe.
- **Dois números com motivo.** 15 s de conexão é o mesmo valor que o download do netcoredbg já usa; 60 s de silêncio é folgado para o primeiro byte de um proxy que bufferiza e ainda assim bounded — 30 s foi recusado por ser curto para esse caso.
- **A política mora no cliente HTTP do host**, junto do roteamento de proxy e da guarda de endereço, e não no instalador: o instalador passa a janela de stall e recebe a resposta. O próximo download a reusar isso é o do netcoredbg (#162).
- **O cliente do download aceita o ambiente injetado.** A função de download ganha uma variante que recebe o ambiente (o mesmo par que já existe para o roteamento de proxy), para o teste dirigir um proxy local sem escrever no ambiente do processo — a suíte tem regra de não vazar `OPENRIDR_*`/proxy entre testes.
- **Mensagem do stall.** `Could not download csharp-ls: <url>: the connection stalled — no data arrived for 60 seconds. Nothing was installed or replaced; check the network and retry the setup.` O mesmo texto para o stall antes dos headers e no meio do corpo: são o mesmo defeito para quem usa. Ela não promete nada sobre o estado da instalação anterior além do que é verdade — o destino só é tocado depois do download verificado.
- **Stall no corpo e nos headers são detectados de formas diferentes.** Corpo: `io::ErrorKind::TimedOut` na leitura. Headers: o erro do `ureq` traz um `io::Error` `TimedOut` na cadeia de `source`, e é isso que se procura — nunca o texto do erro.
- **Uma linha de UI, um testid novo.** O painel já mostrava `error` e o botão `install-retry`, mas o **gate** de setup mostrava só `setup.error`: um download que falhava aparecia como `Failed` sem motivo, e o gate é a única superfície enquanto o setup bloqueia o workspace. O gate passa a mostrar o erro da ferramenta em parágrafo próprio, com o testid **novo** `setup-gate-tool-error`; nenhum `data-testid` existente é renomeado ou removido, e nenhuma dependência entra.
- **Escopo: só o csharp-ls.** O netcoredbg tem a mesma forma com prazo total de 300 s e fica na #162, para o diff desta frente ser revisável.

## Testing decisions

- **Teste bom aqui prova transferência, não texto de tela.** O caso de sucesso confere que **todos** os bytes chegaram depois de uma transferência mais longa que a janela de stall; o caso de stall confere que a chamada voltou com a frase do app.
- **Seam: a variante do cliente de download com ambiente injetado**, contra um **proxy local** que serve o corpo. O destino é um IP público literal (`http://1.1.1.1/...`) para não depender de DNS nem de rede, e o proxy em `127.0.0.1` é permitido pela guarda de endereço — o mesmo desenho dos testes de proxy que já existem.
- **Falha-antes medida**: com o prazo total de hoje, a mesma transferência de 1,5 s com janela de 400 ms morre em 16 KiB; com a leitura por inatividade, chega aos 61.440 bytes. Medido antes de escrever a correção.
- **Casos mínimos**: (1) transferência em 12 pedaços ao longo de ~2,4 s com janela de 1 s termina com o corpo inteiro (a folga de 5× entre o intervalo de envio e a janela mantém o teste longe do agendador); (2) servidor que manda headers e para volta com a frase do stall e sem bytes; (3) servidor que não manda nem headers volta com a mesma frase; (4) conexão que nunca abre (TEST-NET-1, teto de 300 ms) **não** recebe a frase de stall; (5) a leitura do corpo que estoura vira a frase do download — testada na fronteira do instalador, não só na função de mensagem.
- **Prova no app Tauri real com `agent-desktop`**, em `tauri dev` com os seams de debug (`OPENRIDR_ENABLE_PROBE=1`, `OPENRIDR_TOOLS_MUXER`, `OPENRIDR_TOOLS_PACKAGE_URL`/`_SHA512`) apontando para um servidor local: progresso subindo numa entrega lenta e, depois, o stall com a frase e o botão de retry. Uma instância por vez, `pgrep -x openridr` antes.
- **Nada de Playwright**: smoke noturno, não prova de sessão (decisão da #67).

## Out of scope

- **O download do netcoredbg** (mesma forma, prazo total de 300 s): #162.
- **Cancelar o download pela UI**: o host tem `tools.cancel`, a UI não o expõe; fica fora desta frente.
- **Retomar download interrompido** (Range/partial): o pacote é baixado do zero a cada tentativa.
- **Mudar o limite de 32 MiB** ou o tamanho do pacote.
- **Proxy corporativo**: o roteamento e as mensagens de proxy não mudam (frente da #51).
- Windows e macOS Intel.

## Further notes

- Evidência em `docs/evidence/tools-download-stall.md`; retro em `docs/RETRO-TOOLS-DOWNLOAD-STALL.md`.
- Decisão registrada em `docs/decisions.md` (2026-11-07); `docs/README.md` ganha a linha da frente; `GLOSSARY.md` ganha **Download stall**.
- O caminho de download continua sendo fallback de `tauri dev`/ponte HTTP: a build distribuída embute o par, e o spec diz isso em voz alta para ninguém procurar o defeito no DMG.
