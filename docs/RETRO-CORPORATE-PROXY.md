# RETRO — Proxy corporativo HTTP(S) sem enfraquecer a guarda SSRF (issue #51)

Rodada de 2026-11-06, PR [#140](https://github.com/Heldinhow/openridr/pull/140) (`fe407fa`, `8564a0f`, `fab225b`, `24c8ee1` + merge de `origin/main`), macOS arm64. Spec: [SPEC-CORPORATE-PROXY](SPEC-CORPORATE-PROXY.md). Evidência: [evidence/corporate-proxy.md](evidence/corporate-proxy.md). Tickets: #133, #134. Follow-up: #132.

## O que se sustentou

- **A guarda foi para onde ainda tem informação, em vez de ser afrouxada.** O motivo do #38 era estrutural: o `ureq` entrega ao resolver o endereço do **proxy**. Separar "destino" de "proxy" (destino validado antes de conectar; proxy isento) resolveu as duas metades de uma vez, e o modo sem proxy ficou byte a byte igual.
- **Uma porta de entrada, garantida pelo compilador.** `agent_builder` virou privado ao módulo de HTTP: um cliente novo não nasce sem rota nem sem guarda. Foi o que fez o #134 ser barato (três chamadas) em vez de três implementações.
- **Teste com proxy local de verdade.** Um `TcpListener` de 60 linhas provou `CONNECT`, credencial no cabeçalho, 407 e "destino privado recusado **sem** o proxy ver conexão" — tudo offline, sem dependência nova. O mesmo truque no app real (proxy `CONNECT` local) provou a mensagem na tela.
- **Fail closed foi a decisão certa e barata.** Recusar `socks5://`/`https://` com mensagem nomeando a variável custou dez linhas; cair para conexão direta teria vazado tráfego em silêncio.
- **A prova desktop pagou.** Duas capturas (proxy fora e proxy no ar) mais o log do proxy mostraram as duas requisições do Discover (`api.nuget.org` e `azuresearch-usnc.nuget.org`) passando pelo proxy — algo que nenhum teste de unidade teria mostrado, porque o endpoint de busca é outro destino descoberto em runtime.

## O que quebrou / custou

- **Duas rodadas de revisão, cinco + quatro achados, todos reais.** A primeira deixou passar credencial sumindo da URL e redirect seguido sem guarda; a segunda achou a variante sem esquema (`user:pass@host:port`), o `Location` relativo quebrado, a porta impossível em `NO_PROXY` e o 407 como status. Nenhum era teórico.
- **`resolve_location` era um atalho herdado do debugger.** Servia ao GitHub (absoluto ou `/path`), mas ao ser reusado por `fetch` teria quebrado `Location: file.nupkg`. Reusar código existente sem revisar o contrato dele é o mesmo erro do "guarda que não guarda".
- **Edição com script Python perdeu trabalho.** Um `assert` no meio do script abortou antes do `write`, e as duas primeiras substituições sumiram — os testes falharam exatamente como se o código não tivesse mudado. Perdi uma rodada de teste por não conferir o arquivo depois do script.
- **Comparar cabeçalho com `to_lowercase()` no valor base64.** O assert falhou com o cabeçalho correto na frente dos olhos; o base64 tem maiúsculas. Duas vezes o mesmo erro (no teste de credencial e na variante sem esquema).
- **Abrir o app provou ser o gargalo.** O primeiro launch pelo `desktop.sh` mostrou "Setting up…" e voltou para Welcome, sem log e sem erro na tela; um segundo launch limpo (com trust prompt) funcionou. Não investiguei a causa — registrei o sintoma, gastei ~20 minutos e segui por outro caminho.
- **Uma falha de teste que não era minha.** `build_start_needs_the_current_evaluation_…` falhou no primeiro run completo (`restore_required` em vez de `invalid_state`), passou 3/3 isolado e no run completo seguinte. É a mesma família de flake já registrada na retro do #112.

## Lições

- **Quando o cliente HTTP resolve uma coisa e você precisa validar outra, a validação tem que sair do resolver.** Nenhum ajuste de flag resolve isso; o desenho tem que separar os dois endereços.
- **Se um teste de segurança passa sem proxy local de verdade, ele não provou nada.** O `TcpListener` provou o `CONNECT`, a credencial, o 407 e a ausência de conexão no caso recusado — quatro coisas que um mock do `ureq` não daria.
- **Reusar função com contrato implícito é dívida.** `resolve_location` "funcionava" porque o GitHub manda `Location` absoluto ou com `/`; o segundo consumidor expôs o contrato. Ao mover código para uso compartilhado, escreva o teste do formato que o primeiro consumidor nunca mandou.
- **Variante de formato é onde o bug mora.** `http://user:pass@host` e `user:pass@host` são o mesmo proxy para o usuário e dois caminhos no código; o mesmo vale para `NO_PROXY` com e sem porta. Cada variante que a doc de uma CLI aceita merece um caso.
- **`assert` dentro de script de edição precisa vir antes de qualquer escrita, ou a escrita precisa ser por arquivo temporário.** Perder substituições silenciosamente custa mais que o tempo de conferir.
- **App que não abre workspace não é bug da sua feature — mas também não é prova.** Registrar o sintoma, seguir pelo caminho que funciona e declarar o que não rodou é melhor que investigar infraestrutura alheia no meio da entrega.

## Follow-ups

- **#132** — proxy do *System Settings* do macOS (e o campo na UI): é o caso real de quem abre o app pelo Finder/Dock, onde `HTTPS_PROXY` nunca chega.
- **Janela de DNS duplo em modo proxy**: fechar exige conectar por IP com `SNI`/`Host` fixados; declarada na spec e na decisão, sem ticket porque muda o cliente HTTP.
- **SOCKS**: exigiria compilar o `ureq` com `socks-proxy`; hoje é recusado com mensagem.
- **`build_start_needs_the_current_evaluation_…`**: terceira aparição da mesma instabilidade (retro do #112, aqui duas vezes). Candidato a quarentena com motivo, como o #67 fez com o E2E.
- **Abrir workspace pelo `desktop.sh` falhou uma vez** sem log nem erro visível; se repetir, investigar `launch_path`/`take_pending_open`.
- Não verificado: proxy corporativo real (autenticação de domínio, PAC/WPAD, inspeção TLS), Windows, macOS Intel.
