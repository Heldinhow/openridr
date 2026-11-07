# Evidência — proxy corporativo HTTP(S) (#51)

Issue-mãe: [#51](https://github.com/Heldinhow/openridr/issues/51). Tickets: [#133](https://github.com/Heldinhow/openridr/issues/133) (motor e guarda), [#134](https://github.com/Heldinhow/openridr/issues/134) (aplicar nos três fluxos). Spec: [SPEC-CORPORATE-PROXY](../SPEC-CORPORATE-PROXY.md). Decisão: `docs/decisions.md`, 2026-11-06.

Ambiente: macOS 26.1 (Darwin 25), Apple Silicon, Rust 1.99.0 (toolchain pinada), SDK .NET 8.0.408 das fixtures. Nenhum teste toca a rede externa.

## O que foi provado, e como

### 1. Decisão de rota (unidade, sem rede)

`cargo test -p openridr-host --lib http::proxy_tests` — 12 testes:

| Comportamento | Teste |
|---|---|
| `https_proxy` vence `HTTPS_PROXY`; `ALL_PROXY` serve de fallback; `HTTP_PROXY` atende destino https; valor em branco é "não definido" | `proxy_variables_are_read_in_order_with_lowercase_first` |
| `NO_PROXY` em 11 formas (`*`, host exato, sufixo com e sem ponto, `*.x`, `host:porta`, lista com vírgula/espaço, case, `evilcorp.example` **não** casa `corp.example`) e a variável vencendo o proxy | `no_proxy_forms_send_the_destination_direct` |
| `no_proxy` vazio não esconde `NO_PROXY` | `an_empty_no_proxy_does_not_shadow_the_uppercase_value` |
| `host:porta` sem esquema é lido como `http://` | `a_bare_host_and_port_is_read_as_an_http_proxy` |
| `socks5://`, `socks4://`, `https://` falham fechado, nomeando a variável e o esquema, sem ecoar `user:pass` | `an_unusable_proxy_value_fails_closed_without_leaking_credentials` |
| Valor malformado falha fechado | `a_malformed_proxy_value_fails_closed` |
| Porta padrão, IPv6 entre colchetes, URL com credencial recusada | `destinations_are_parsed_with_default_ports_and_bracketed_ipv6` |
| Redirect para `http://` é recusado | `a_redirect_to_plain_http_is_refused` |

### 2. Guarda de endereço (unidade, IP literal)

`cargo test -p openridr-host --lib netguard` — 4 testes: classificação v4/v6 (loopback, RFC1918, link-local, CGNAT, benchmark, multicast, unspecified, IPv4 mapeado/compatível, NAT64, ULA), classificação literal de host sem DNS, resolver recusando destino privado e **liberando** o netloc do proxy configurado, e o pre-flight de destino privado em modo proxy.

### 3. Proxy local de verdade (sem rede externa)

`cargo test -p openridr-host --lib http::proxy_tests::a_proxy_receives_the_connect_instead_of_the_destination` — um proxy HTTP mínimo em `TcpListener` recebe `CONNECT 1.1.1.1:443 HTTP/1.1` (destino público literal, guarda satisfeita, destino nunca contatado) e responde 502; a requisição falha como falha de proxy.

`a_proxied_private_destination_never_reaches_the_proxy` — com `HTTPS_PROXY` configurado e destino `https://127.0.0.1:9/`, a requisição é recusada **antes** de qualquer conexão: o proxy registra zero requisições.

`proxy_credentials_reach_the_client_but_never_the_label` — o `CONNECT` leva `Proxy-Authorization: basic …` (o proxy autenticado funciona) enquanto o rótulo de log/diagnóstico é só `host:porta`.

`a_proxy_answer_of_407_is_explained_as_credentials` — um proxy que responde 407 produz a mensagem que manda ajustar as credenciais em `HTTPS_PROXY` e nomeia o proxy.

### 4. Wiring do Discover (integração, processo isolado)

`cargo test -p openridr-host --test corporate_proxy` — `nuget.search` real do host, com `HTTPS_PROXY` apontando para o proxy local e uma fonte `https://1.1.1.1/v3/index.json` configurada no workspace: o proxy registra **um** `CONNECT 1.1.1.1:443` e a mensagem devolvida nomeia o proxy. Prova que o ponto de chamada usa a porta de entrada única, não só que ela funciona.

### 5. Não-regressão dos fluxos existentes

- `cargo test -p openridr-host --test debugger` — 15 passed, 0 failed, 14 ignored. O instalador controlado do netcoredbg continua baixando do servidor local em loopback sob `OPENRIDR_ENABLE_PROBE=1`: a guarda nova não quebrou o seam.
- `cargo test -p openridr-host --test host -- tools` — 4 passed, incluindo `tools_install_is_real_verified_and_reused` (download real do pacote do csharp-ls a partir do servidor local, SHA-512 conferido), agora pelo `agent_for`.
- `cargo test -p openridr-host --lib` — 356 passed, 0 failed, 11 ignored.
- `cargo test -p openridr-host --test bundled_analysis` — 11 passed, 0 failed, 11 ignored.
- `cargo clippy -p openridr-host --all-targets` sem aviso; `cargo fmt --all` aplicado.

### 6. App Tauri real (macOS arm64, `agent-desktop`)

Bundle de debug do HEAD (`0.3.0-alpha.11`), `OPENRIDR_HOME` isolado em `/tmp/verify-openridr-51-desktop/home`, cópia da fixture em `.../work/OrderDesk`, uma instância só, fechada ao final (`pgrep -x openridr` vazio). O app foi iniciado com `HTTPS_PROXY=http://127.0.0.1:18080` apontando para um proxy `CONNECT` local escrito para esta prova (encaminha bytes, não intercepta TLS).

| Ação | Estado resultante |
|---|---|
| Aba NuGet → Discover → Search com o **proxy fora do ar** | `Could not read the NuGet source index: https://api.nuget.org/v3/index.json: … Connection refused … (via proxy 127.0.0.1:18080)` — [captura](corporate-proxy/proxy-down-error.png) |
| Mesma busca com o **proxy no ar** | a busca completa e responde a partir da fonte (`0 packages … for "Xunitxunit"`, query digitada duas vezes pela ferramenta) — [captura](corporate-proxy/proxy-up-search.png) |
| Log do proxy na busca bem-sucedida | `CONNECT api.nuget.org:443` (índice) **e** `CONNECT azuresearch-usnc.nuget.org:443` (endpoint de busca anunciado) — [log](corporate-proxy/proxy-requests.log) |

O par prova as duas metades: a mensagem nomeia o proxy quando ele falha, e as duas requisições do Discover passam por ele quando ele funciona. Nenhuma das capturas mostra credencial.

## O que não foi verificado

- **Proxy corporativo real** (com autenticação de domínio, PAC/WPAD ou inspeção TLS da empresa): não existe nesta máquina. O proxy da prova é local e sem autenticação; o caminho autenticado está provado só por teste (`Proxy-Authorization` no `CONNECT`).
- **Proxy do macOS System Settings**: fora de escopo por decisão, acompanhado na issue #132. Quem abre o app pelo Finder/Dock não herda `HTTPS_PROXY` do shell — é exatamente o caso que a #132 cobre.
- **SOCKS e proxy `https://`**: recusados de propósito com mensagem; não são suportados pelo `ureq` 2.12 sem dependência nova.
- **Janela de DNS duplo em modo proxy**: declarada na spec e na decisão, não fechada.
- **Windows e macOS Intel**: não executados, como no resto do repo.
- **E2E Playwright**: não usado para esta feature (decisão da #67); a prova de UI é a do app real acima.

## Nota sobre a bateria completa

`cargo test -p openridr-host` (suíte inteira, uma vez) teve **uma** falha em `build_start_needs_the_current_evaluation_and_accepts_an_explicit_library` (`restore_required` em vez de `invalid_state`), um teste que não toca HTTP e que passa 3 de 3 vezes isolado. A segunda execução completa fechou verde. Registrado como instabilidade conhecida da família "restore/evaluation por tempo" — não é regressão desta frente, e nenhum teste desta frente falhou em nenhuma execução.
