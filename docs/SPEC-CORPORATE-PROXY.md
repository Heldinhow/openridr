# SPEC — Proxy corporativo HTTP(S) sem enfraquecer a guarda SSRF do NuGet

Issue-mãe: [#51](https://github.com/Heldinhow/openridr/issues/51) — "Support corporate HTTP(S) proxies without weakening the NuGet SSRF guard".

## Problem Statement

Numa máquina atrás de proxy corporativo, as únicas requisições HTTP que o host faz por conta própria não chegam ao destino: Discover do NuGet (aba NuGet), download de fallback do netcoredbg e download de fallback do csharp-ls. Elas saem diretas e morrem em timeout ou em `UnknownIssuer` de inspeção TLS — o usuário não tem como instalar pacote, ver versões disponíveis nem subir o adaptador de debug, mesmo com `HTTPS_PROXY` configurado no shell.

O motivo de estar assim é deliberado: o #38 desligou `proxy-from-env` do `ureq` porque a guarda SSRF do NuGet (`SafeResolver`) é o *resolver* do cliente, e o `ureq` entrega ao resolver o endereço do **proxy**, não o do destino. Ligar o proxy como estava jogaria fora a guarda (o destino nunca é validado) e, ao mesmo tempo, recusaria um proxy em `localhost`/rede privada — que é justamente onde proxy corporativo costuma viver.

O que falta é: usar o proxy quando o ambiente manda, continuar recusando destino privado/loopback/link-local, e deixar claro no diagnóstico qual proxy foi usado quando ele falha.

## Solution

O host passa a decidir a rota de cada requisição HTTP a partir do ambiente (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` e as versões minúsculas), respeitando `NO_PROXY`/`no_proxy`. Quando há proxy, a requisição sai pelo proxy; quando não há, sai direta como hoje.

A guarda SSRF deixa de depender do resolver para o destino: o host valida o **host de destino** antes de qualquer requisição (classificação literal e, em modo proxy, resolução DNS — qualquer resposta privada/loopback/link-local recusa), e o resolver continua recusando endereço privado quando o que ele resolve é o próprio destino (conexão direta). O endereço do **proxy** é isento dessa regra: escolher um proxy em `localhost` ou na rede privada é decisão explícita do operador, não do conteúdo do projeto.

Quando o valor do ambiente não é utilizável (esquema que o cliente não suporta, URL inválida), a requisição falha com mensagem própria em vez de sair direta: cair para conexão direta fura a política da rede e vaza tráfego, então "falhar fechado" é o comportamento correto. Quando o proxy responde 407 ou recusa o `CONNECT`, a mensagem nomeia o proxy (host:porta, nunca credenciais).

Processos filhos (`dotnet restore/build/test`, csharp-ls, netcoredbg) já herdam o ambiente e continuam assim; o host só garante que não limpa essas variáveis.

## User Stories

1. Como desenvolvedor atrás de proxy corporativo, quero que a busca de pacotes NuGet (Discover) saia pelo proxy configurado, para ver versões disponíveis sem sair da IDE.
2. Como desenvolvedor atrás de proxy corporativo, quero que o download de fallback do netcoredbg saia pelo proxy, para conseguir depurar quando o adaptador ainda não está embutido (`tauri dev`).
3. Como desenvolvedor atrás de proxy corporativo, quero que o download de fallback do csharp-ls saia pelo proxy, para ter análise de código quando não há bundle.
4. Como desenvolvedor atrás de proxy corporativo, quero que `dotnet restore`, build e teste continuem funcionando com o proxy que já está no ambiente, para não precisar de configuração duplicada.
5. Como desenvolvedor, quero que `NO_PROXY` seja respeitado (inclusive para o feed interno da empresa), para não mandar tráfego interno ao proxy.
6. Como desenvolvedor, quero uma mensagem clara quando o proxy exige autenticação, para saber que o problema é credencial e não rede.
7. Como desenvolvedor, quero uma mensagem clara quando o proxy recusa o `CONNECT` ou não responde, para não caçar defeito no NuGet.
8. Como desenvolvedor, quero que um valor de proxy que o cliente não suporta (ex.: `socks5://`) falhe dizendo isso, para não ficar no escuro nem sair direto sem saber.
9. Como responsável pela segurança, quero que uma fonte NuGet que aponta para endereço privado/loopback/link-local continue recusada mesmo com proxy configurado, para que um `nuget.config` malicioso não use o proxy como ponte para a rede interna.
10. Como responsável pela segurança, quero que o proxy em si possa estar em `localhost`/rede privada, para que a configuração corporativa legítima não seja confundida com SSRF.
11. Como responsável pela segurança, quero que um proxy público não seja aceito como desculpa para alcançar destino privado, para manter a guarda no **host de destino**.
12. Como responsável pela segurança, quero que as credenciais do proxy nunca apareçam em mensagem, log ou evidência, para não vazar segredo em captura de tela.
13. Como mantenedor, quero uma única porta de entrada para construir cliente HTTP no host, para que um cliente novo não nasça sem proxy nem sem guarda.
14. Como mantenedor, quero a decisão de rota como função pura testável sem tocar o ambiente do processo, para provar precedência e `NO_PROXY` sem flakiness.
15. Como mantenedor, quero a guarda de endereço em módulo próprio, compartilhado entre NuGet e os downloads, para não existirem duas listas de "endereço privado" divergindo.
16. Como mantenedor, quero testes que provem o uso do proxy contra um proxy local de teste, sem rede externa, para o CI não depender de proxy corporativo real.
17. Como testador, quero que o seam de instalador controlado (`OPENRIDR_ENABLE_PROBE`) continue funcionando com servidor local em loopback, para não quebrar a suíte do adaptador de debug.
18. Como usuário, quero que nada mude quando não há proxy configurado, para a configuração padrão continuar sem surpresa.

## Implementation Decisions

- **Uma porta de entrada.** Todo cliente HTTP do host nasce de uma única função que recebe a URL de destino e devolve o `ureq::AgentBuilder` já com TLS de sistema, resolução guardada e proxy decidido. Construir `ureq::AgentBuilder` direto fica restrito ao módulo de HTTP (o teste de confiança TLS usa um construtor privado, como hoje). Os três pontos — Discover do NuGet, download do netcoredbg, download do csharp-ls — passam por ela; o download do netcoredbg reconstrói o cliente a cada salto de redirecionamento, porque o destino muda.
- **Fonte de configuração: só ambiente.** Para destino `https`: `https_proxy`, `HTTPS_PROXY`, `all_proxy`, `ALL_PROXY`, `http_proxy`, `HTTP_PROXY` — nessa ordem (minúscula primeiro, como `curl`; `HTTP_PROXY` como último recurso porque muitos guias corporativos só o definem, e todo destino do host é https). Para destino `http`: `http_proxy`, `HTTP_PROXY`, `all_proxy`, `ALL_PROXY`. Valor vazio conta como não definido. `NO_PROXY`/`no_proxy`: `no_proxy` primeiro, depois `NO_PROXY`.
- **`NO_PROXY`**: separador vírgula ou espaço; `*` (qualquer destino direto); host exato; sufixo de domínio com ponto inicial (`.corp.example`) e sem (`.corp.example` e `corp.example` casam `a.corp.example` e `corp.example`, não `evilcorp.example`); `*` como primeiro caractere de uma entrada é lido como sufixo (`*.corp.example` ≡ `.corp.example`); `host:porta` casa só aquela porta; comparação case-insensitive, sem ponto final. Sem bypass implícito de loopback: quem manda é `NO_PROXY`.
- **Só proxy HTTP.** O cliente é compilado sem `socks-proxy`; `socks4://`, `socks5://`, `https://` e afins no ambiente são recusados com mensagem que nomeia a variável e o esquema. Esquema ausente (`proxy.corp:8080`) é lido como `http://`. **Falha fechada**: proxy configurado e não utilizável → requisição falha, nunca sai direta.
- **Segredo não vaza.** Toda mensagem, log e evidência usa o proxy como `host:porta`; usuário/senha de `user:pass@host:porta` são removidos antes de qualquer texto. A URL crua das variáveis de ambiente não é registrada.
- **Guarda de endereço em módulo compartilhado.** A classificação de endereço privado (loopback, `10/8`, `172.16/12`, `192.168/16`, link-local, `0.0.0.0/8`, CGNAT `100.64/10`, benchmark `198.18/15`, multicast, unspecified, IPv4 mapeado/compatível e NAT64 `64:ff9b::/96`, ULA `fc00::/7`, link-local `fe80::/10`) e a classificação literal de host (`localhost`, `*.localhost`, `*.local`, octetos no formato IPv4) saem de `nuget.rs` para um módulo próprio, usado tanto por `validate_source_arg` quanto pela guarda das requisições. Nenhuma lista duplicada.
- **Modo direto (sem proxy): guarda no resolver, como hoje.** O resolver recusa a requisição se *qualquer* resposta DNS do destino for privada, e o `ureq` conecta a esses mesmos endereços — a garantia forte de hoje é preservada.
- **Modo proxy: guarda antes de conectar, no host de destino.** O `ureq` entrega ao resolver apenas o endereço do proxy, então a validação do destino acontece na construção do cliente: classificação literal do host e resolução DNS do destino; qualquer resposta privada → recusa, sem abrir conexão com o proxy. O resolver, nesse modo, resolve o proxy sem aplicar a regra de endereço privado quando o netloc pedido é exatamente o do proxy configurado (casamento `host:porta`), e aplica a regra para qualquer outro netloc.
- **Diferença de garantia, declarada.** Com proxy, o DNS do destino é resolvido duas vezes — uma pelo host, para validar, e uma pelo proxy (ou pelo destino, no `CONNECT`) para conectar. A garantia passa a ser "se o nome resolve para privado, é recusado", não "a conexão vai ao IP que validamos". Aceito explicitamente no grill: fechar essa janela exigiria conectar por IP com `SNI`/`Host` fixados, o que muda o cliente HTTP e não é escopo do #51. Sem proxy, nada muda.
- **Proxy é decisão do operador, destino não.** O endereço do proxy é isento da regra de endereço privado. É isso que permite proxy em `localhost` (sidecar, `mitmproxy` local) e em rede privada corporativa.
- **Seam de instalador controlado preservado.** `OPENRIDR_ENABLE_PROBE=1` em build de debug (mesma condição do seam atual do adaptador) permite destino loopback/privado na guarda de endereço, porque a suíte do netcoredbg baixa de um servidor local em `127.0.0.1`. Em release a flag não existe. A política de produto do NuGet (`validate_source_arg`: só HTTPS, host literal privado recusado) **não** é afetada pela flag.
- **Diagnóstico.** Falha de proxy vira mensagem própria: 407 ("o proxy em `host:porta` exige autenticação; ajuste as credenciais em `HTTPS_PROXY`"), `CONNECT` recusado, proxy inalcançável/DNS do proxy. A mensagem de CA corporativa do #38 é mantida. O log do download mostra `GET <url> via proxy <host:porta>` quando há proxy.
- **Filhos herdam o ambiente.** `dotnet restore/build/test`, csharp-ls e netcoredbg continuam sem filtro de ambiente; nenhum código remove `HTTP(S)_PROXY`/`NO_PROXY`. Um teste garante que a limpeza de ambiente do host não toca essas variáveis.
- **Sem superfície de UI nova.** Nada de campo de proxy na tela, nenhum `data-testid` novo. O usuário configura o ambiente (shell/app bundle) como configura qualquer outra CLI.
- **Sem dependência nova.** `std::net`, `ureq` 2.12 e `serde` já presentes cobrem tudo; o proxy de teste é um `TcpListener` do `std`.

## Testing Decisions

Bom teste aqui prova comportamento observável: qual rota foi escolhida para uma URL e um ambiente dados, se a requisição chegou ao proxy, e se um destino privado foi recusado **sem** conexão. Não testa estrutura interna (nomes de campo, forma do struct de rota).

Seams, do mais alto para o mais baixo:

1. **Decisão de rota como função pura** com um leitor de ambiente injetado (closure), em teste de unidade: precedência por esquema, minúscula antes de maiúscula, `ALL_PROXY` como fallback, `HTTP_PROXY` para destino https, `NO_PROXY` (todos os formatos acima), valor vazio, esquema não suportado, credenciais removidas da mensagem. Nada de rede, nada de escrever no ambiente do processo — o motivo de existir a injeção é não repetir o flake de ambiente herdado já registrado em `docs/evidence/test-stability.md`.
2. **Guarda de endereço** em teste de unidade, com IP literal (sem DNS): resolver recusa `10.0.0.1`, `127.0.0.1`, `169.254.1.1`, `100.64.0.1`, `198.18.0.1` em modo direto; aceita o netloc do proxy configurado e recusa um segundo netloc privado em modo proxy; pre-flight recusa destino privado (literal e por DNS de nome `localhost`) antes de qualquer conexão — provado por um listener local que não recebe conexão alguma.
3. **Proxy local de teste** (proxy HTTP mínimo em `TcpListener`, dentro do próprio teste) provando o caminho de verdade sem rede externa: destino `https://1.1.1.1/...` (IP público literal, guarda satisfeita, destino nunca contatado) com `HTTPS_PROXY` apontando para o proxy local; o proxy recebe `CONNECT 1.1.1.1:443` e responde 502; a requisição falha com mensagem que nomeia o proxy. Prova que houve proxy e não conexão direta.
4. **Wiring por fluxo** em binário de teste de integração próprio (processo próprio, então o ambiente pode ser escrito; padrão `support::run_isolated_leaf` do harness): uma busca de Discover do NuGet com `HTTPS_PROXY` para o proxy local devolve erro que nomeia o proxy e o proxy registra o `CONNECT` — prova que o ponto de chamada usa a porta de entrada única. O download do netcoredbg continua provado pelo seam controlado já existente (servidor local + `OPENRIDR_ENABLE_PROBE`), que precisa continuar verde: é o teste de não-regressão da guarda e do seam.
5. **Processos filhos** herdam o ambiente: teste que confirma que as variáveis de proxy configuradas para o host chegam ao `dotnet` invocado (ou, na falta de `dotnet` no ambiente, teste de unidade sobre a montagem do comando que não remove as variáveis).

Prior art: `crates/host/src/http_tls_trust_test.rs` (construtor privado + teste com CA local), `crates/host/tests/debugger.rs` (servidor HTTP determinístico local + seam patcheado), `crates/host/tests/support/mod.rs` (`run_isolated_leaf` para testes que escrevem ambiente).

Nada de rede externa em teste algum; nada de Playwright para provar esta feature.

## Out of Scope

- **Proxy do System Settings do macOS** (e qualquer leitura de `scutil --proxy`): a maioria dos usuários corporativos configura por lá, não por ambiente. Fica em issue de follow-up ligada ao #51.
- **Campo de proxy na UI / arquivo de configuração do OpenRidr**: só ambiente nesta rodada.
- **SOCKS** (`socks5://`, `socks4://`): exigiria compilar o `ureq` com `socks-proxy`; recusado com mensagem clara em vez de suportado.
- **Proxy `https://` (TLS até o proxy)**: o `ureq` 2.12 não suporta; recusado com mensagem.
- **PAC/WPAD**, bypass de proxy por `localhost` automático, e `NO_PROXY` com CIDR.
- **Fechar a janela de DNS duplo do modo proxy** (validar e conectar pelo mesmo IP): declarada como diferença de garantia, ver decisões.
- **Certificado do proxy / inspeção TLS**: já coberto pelo #38 (CA do sistema); nada novo.

## Further Notes

- O `#38` desligou o proxy de propósito e registrou o motivo no corpo do PR; esta spec é o follow-up que ele promete. A decisão que substitui aquele ponto vai em `docs/decisions.md` com a data.
- Se o usuário tiver `HTTPS_PROXY` no shell mas abrir o app pelo Finder/Dock, o app não herda o ambiente do shell: é exatamente o caso que o follow-up do System Settings cobre. A mensagem de falha deve continuar citando CA/proxy para o usuário não achar que o NuGet quebrou.
- Evidência em `docs/evidence/corporate-proxy.md`; o que não rodar (proxy corporativo real, app desktop atrás de proxy) entra como "não verificado" com o motivo.
