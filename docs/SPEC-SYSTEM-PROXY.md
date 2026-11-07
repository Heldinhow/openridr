# SPEC — Proxy do macOS System Settings para o cliente HTTP do host

Issue-mãe: [#132](https://github.com/Heldinhow/openridr/issues/132) — "Corporate proxy: honour the macOS system proxy settings". Follow-up do [#51](https://github.com/Heldinhow/openridr/issues/51), cuja spec é [SPEC-CORPORATE-PROXY](SPEC-CORPORATE-PROXY.md).

## Problem Statement

O [#51](https://github.com/Heldinhow/openridr/issues/51) fez o cliente HTTP do host seguir `HTTPS_PROXY`/`HTTP_PROXY`/`ALL_PROXY`/`NO_PROXY` do ambiente. Só que um Mac corporativo normalmente **não** define essas variáveis: o proxy mora em Ajustes do Sistema → Rede → Proxies. Um app aberto pelo Finder ou pelo Dock nunca vê esse ajuste no ambiente, então para esse usuário as três requisições que o host faz por conta própria — Discover do NuGet, download de fallback do netcoredbg e download de fallback do csharp-ls — continuam saindo diretas e morrendo em timeout ou `UnknownIssuer`. O usuário não tem como instalar pacote, ver versões nem subir o adaptador de debug, mesmo com o proxy configurado na máquina.

## Solution

O host passa a ler o proxy do **System Settings do macOS** (somente leitura, sem elevação) e a usá-lo como **última fonte** da mesma decisão de rota do #51: variável de ambiente explícita sempre vence; quando o ambiente não nomeia proxy, vale o ajuste do sistema. A guarda de destino, a falha fechada e o formato de diagnóstico do #51 continuam os mesmos — o proxy do sistema é só mais uma origem de rota, não um caminho paralelo.

A lista de exceções do macOS (`ExceptionsList`) é lida com a mesma semântica de `NO_PROXY`, então o feed interno que o usuário excluiu do proxy continua indo direto. Quando o Mac está configurado com algo que este cliente não sabe usar (SOCKS, arquivo PAC), a rota segue direta como hoje — sem regressão — e a mensagem de falha nomeia o ajuste, para o usuário não caçar defeito no NuGet.

## User Stories

1. Como desenvolvedor atrás de proxy corporativo configurado só no System Settings, quero que a busca de pacotes NuGet (Discover) saia por esse proxy, para ver versões disponíveis sem sair da IDE.
2. Como desenvolvedor atrás de proxy corporativo configurado só no System Settings, quero que o download de fallback do netcoredbg saia por esse proxy, para conseguir depurar quando o adaptador ainda não está embutido (`tauri dev`).
3. Como desenvolvedor atrás de proxy corporativo configurado só no System Settings, quero que o download de fallback do csharp-ls saia por esse proxy, para ter análise de código quando não há bundle.
4. Como desenvolvedor que abre o app pelo Finder ou pelo Dock, quero que o proxy funcione sem exportar variável nenhuma, porque o app não herda o ambiente do shell.
5. Como desenvolvedor, quero que uma variável de ambiente explícita continue vencendo o ajuste do sistema, para poder apontar o app para outro proxy (ou dar credenciais) sem mexer na configuração da máquina.
6. Como desenvolvedor, quero que a lista de exceções do macOS seja respeitada, para não mandar tráfego interno ao proxy.
7. Como desenvolvedor, quero que mudar o proxy no System Settings passe a valer sem relançar o app, para não descobrir no meio do trabalho que o app ficou com a configuração velha.
8. Como desenvolvedor, quero uma mensagem que diga que o proxy veio do ajuste do sistema (e não de uma variável), para saber onde corrigir credencial.
9. Como desenvolvedor cujo Mac usa SOCKS ou um arquivo PAC, quero uma mensagem que nomeie esse ajuste quando a requisição falhar, para saber que o problema é um proxy que o app não usa — e não o NuGet.
10. Como responsável pela segurança, quero que a guarda de destino do #51 continue valendo para o proxy do sistema, para que um `nuget.config` malicioso não use o proxy da máquina como ponte para a rede interna.
11. Como responsável pela segurança, quero que o proxy do sistema possa estar em `localhost`/rede privada, para que a configuração corporativa legítima não seja confundida com SSRF.
12. Como responsável pela segurança, quero que as credenciais nunca apareçam em mensagem, log ou evidência, para não vazar segredo em captura de tela.
13. Como mantenedor, quero que a leitura do ajuste do sistema não introduza dependência nova nem UI nova, para o diff continuar revisável e o app continuar sem superfície de configuração duplicada.
14. Como mantenedor, quero que a suíte de testes não dependa do proxy configurado na máquina de quem roda `cargo test`, para o CI e o desenvolvimento local darem o mesmo resultado.
15. Como mantenedor, quero que o caminho de teste que substitui o leitor do sistema não exista em build de release, para uma variável de ambiente não virar porta de entrada em produção.
16. Como usuário, quero que nada mude quando não há proxy nenhum configurado, para a configuração padrão continuar sem surpresa.

## Implementation Decisions

- **Fonte nova, mesma decisão de rota.** O ajuste do sistema entra como **último item** da lista ordenada que o #51 já define (para destino `https`: `https_proxy`, `HTTPS_PROXY`, `all_proxy`, `ALL_PROXY`, `http_proxy`, `HTTP_PROXY`, depois o ajuste do sistema; para `http`: `http_proxy`, `HTTP_PROXY`, `all_proxy`, `ALL_PROXY`, depois o ajuste do sistema). Consequência aceita e testada: o ajuste do sistema é o **último** item, então qualquer variável de proxy do ambiente — inclusive `HTTP_PROXY`, que é o último recurso entre as variáveis — vence o sistema; o ajuste do macOS só decide quando o ambiente não nomeia proxy nenhum.
- **`NO_PROXY` do ambiente vence a lista de exceções do sistema.** Sem `NO_PROXY`/`no_proxy` no ambiente, a `ExceptionsList` do macOS vale — inclusive para um proxy vindo do ambiente (exceção só torna a rota direta, e a guarda continua recusando destino privado).
- **Leitura do ajuste: `scutil --proxy`, em módulo próprio.** `/usr/sbin/scutil --proxy` é leitura pura, sem elevação, ~10 ms, e o app não é sandbox (só hardened runtime). O texto é interpretado por uma função pura, com o formato real do dicionário (`HTTPEnable`/`HTTPProxy`/`HTTPPort`, `HTTPSEnable`/`HTTPSProxy`/`HTTPSPort`, `SOCKSEnable`, `ProxyAutoConfigEnable`, `ExceptionsList`, `ExcludeSimpleHostnames`). A leitura vive fora do módulo de HTTP, que segue sendo a única porta de entrada do cliente; o diff no módulo de HTTP é só o encaixe da fonte na lista.
- **`HTTPSProxy` ausente cai para `HTTPProxy`.** É o que o CFNetwork faz para URL https e o que a configuração corporativa costuma usar (só "Web Proxy (HTTP)" preenchido). Porta ausente usa 80/443.
- **PAC ligado ⇒ nenhum proxy desta fonte.** Um arquivo PAC decide por URL, e este cliente não interpreta PAC: quando `ProxyAutoConfigEnable` está ligado, a fonte do sistema não devolve proxy (rota direta, como hoje), mesmo que um proxy manual esteja preenchido — usar o manual seria adivinhar o que o PAC manda. A mensagem de falha nomeia o PAC.
- **SOCKS ligado e nenhum proxy HTTP/HTTPS ⇒ rota direta, como hoje.** O ajuste do sistema é ambiente da máquina, não instrução para este app: falhar fechado quebraria quem tem SOCKS ligado para o navegador e cuja rede aceita conexão direta. O diagnóstico nomeia o SOCKS para o timeout não virar mistério. (Diferença deliberada em relação ao `socks5://` no ambiente, que falha fechado porque ali o operador nomeou um proxy para o app.)
- **Leitura a cada requisição do host, sem cache.** ~10 ms num pedido de rede é irrelevante, e mudar o proxy no System Settings passa a valer na próxima requisição, sem relançar o app. Um download com redirecionamentos lê uma vez e reusa a decisão base: os saltos de um mesmo download não são requisições independentes, o destino é revalidado a cada um, e reler o ajuste por salto só custaria um processo por salto. `scutil` ausente, com exit não-zero ou texto ilegível ⇒ sem proxy do sistema (rota direta, como hoje); não há timeout próprio para uma chamada local.
- **Guarda inalterada.** A rota vinda do sistema passa pelo mesmo `require_public_target` e pelo mesmo `GuardedResolver` do #51: destino privado/loopback/link-local é recusado antes de qualquer conexão, e o endereço do proxy é isento. Nada de segunda lista de "endereço privado".
- **Diagnóstico nomeia a origem.** O rótulo do proxy passa a ser `host:port` mais a origem quando ela é o sistema (`host:port (macOS system proxy)`); o log do download e as mensagens de 407/`CONNECT` recusado usam esse rótulo. A recomendação de credencial continua sendo `HTTPS_PROXY`, que é a única forma de dar usuário/senha ao cliente (o macOS não entrega credencial do proxy ao app). Mensagem de valor inutilizável (ex.: proxy IPv6, que chega entre colchetes para ser reconhecido como IPv6) nomeia a origem em vez da variável. A frase sobre SOCKS/PAC aparece em toda falha que **não usou proxy** — inclusive um download que parou de receber dados — e não aparece quando um proxy estava em uso, porque nesse caso o ajuste do Mac não é a causa. Credenciais continuam fora de qualquer texto.
- **Seam de teste com gate de release.** Uma variável só de desenvolvimento, `OPENRIDR_SYSTEM_PROXY`, decide a origem do texto: `off` desliga a fonte do sistema; um caminho de programa faz o host rodar esse programa em vez do `scutil`. O modo "programa" vale **só** em build de debug ou com `OPENRIDR_ENABLE_PROBE=1` (o mesmo gate dos outros seams de desenvolvimento); em release só `off` é aceito e qualquer outro valor é ignorado. O harness de teste passa a purgar essa variável e a setar `off`, então a suíte deixa de depender da configuração de rede da máquina.
- **Sem dependência nova, sem UI nova.** `std` + `ureq` 2.12 continuam cobrindo tudo; nenhum campo de proxy na tela, nenhum `data-testid` novo, nenhuma persistência nova.
- **Escopo de plataforma declarado.** Só macOS. Windows (WinINET/registro) e Linux (`gsettings` do GNOME) não leem nada nesta rodada e ficam com o comportamento de hoje — sem regressão, e sem issue de follow-up aberta agora.

## Testing Decisions

Bom teste aqui prova comportamento observável: para um texto de `scutil` e um ambiente dados, qual rota sai; se a requisição chegou ao proxy; se um destino privado foi recusado **sem** conexão; e se o seam respeita o gate de release. Nada de rede externa, nada de Playwright, nada de escrever nos ajustes de rede da máquina.

Seams, do mais alto para o mais baixo:

1. **Interpretação do texto do `scutil` como função pura**, com amostras: a amostra real desta máquina sem proxy (dicionário só com `ExceptionsList`/`FTPPassive`), proxy HTTP+HTTPS ligado, só HTTP ligado (fallback para https), SOCKS ligado sem HTTP/HTTPS, PAC ligado, `ExceptionsList` com várias entradas e `ExcludeSimpleHostnames`.
2. **Decisão de rota com as duas fontes injetadas** (ambiente por closure, ajuste do sistema por valor), em teste de unidade: precedência do ambiente sobre o sistema, `HTTP_PROXY` do ambiente perdendo para o `HTTPSProxy` do sistema, exceção do sistema tornando a rota direta, `NO_PROXY` do ambiente vencendo a lista do sistema, valor vazio contando como não definido, e o rótulo decorado com a origem.
3. **Proxy local de teste** (o `TcpListener` que o #51 já usa) provando o caminho de verdade com a fonte do sistema: destino `https://1.1.1.1/...` com o ajuste do sistema apontando para o proxy local; o proxy recebe `CONNECT 1.1.1.1:443` e responde 502; a requisição falha nomeando o proxy. E o caso de guarda: destino privado com proxy do sistema não abre conexão nenhuma.
4. **Caminho de produto** num binário de teste de integração próprio (`support::run_isolated_leaf`, o padrão do #51): o leaf aponta `OPENRIDR_SYSTEM_PROXY` para um programa que imprime um dicionário com o proxy local e faz um Discover do NuGet — a falha nomeia o proxy e o proxy registra o `CONNECT`. Prova que o ponto de chamada usa a porta de entrada única com a fonte do sistema.
5. **Gate do seam** em teste de unidade sobre a decisão pura: `off` desliga nas duas builds; caminho de programa vale em debug; em release só `off` é aceito e qualquer outro valor é ignorado.
6. **Leitura real** (macOS): o comando real roda e o texto real é interpretado sem erro — a máquina pode não ter proxy, então o teste só exige que a leitura não falhe nem panic.

Prior art: `crates/host/src/http_proxy_test.rs` (proxy local + rota injetada), `crates/host/src/netguard_test.rs` (guarda com IP literal), `crates/host/tests/corporate_proxy.rs` (`run_isolated_leaf` + proxy local para Discover).

## Out of Scope

- **Campo "Proxy" nas Settings do app** (e arquivo de configuração próprio): decidido fora nesta rodada — o usuário corporativo já configurou no macOS, e a UI nova exigiria persistência, precedência extra e `data-testid` novo.
- **Windows (WinINET/registro) e Linux (`gsettings`)**: declarados fora; comportamento de hoje, sem regressão.
- **PAC/WPAD**: não interpretados. PAC ligado ⇒ rota direta + diagnóstico nomeando o PAC.
- **SOCKS**: não suportado (exigiria compilar o `ureq` com `socks-proxy`); SOCKS ligado sem proxy HTTP/HTTPS ⇒ rota direta + diagnóstico.
- **Credenciais do proxy do sistema via Keychain**: o macOS não as entrega ao app; a saída é `HTTPS_PROXY` com usuário/senha.
- **CIDR na lista de exceções** (`169.254/16`, `10.0.0.0/8`): entradas assim não casam. Não fazem falta: destino nessas faixas é recusado pela guarda de qualquer forma.
- **`ExcludeSimpleHostnames`**: ignorado; todo destino do host é nome completo.
- **Cache da leitura** e invalidação por notificação do SystemConfiguration: a leitura é por requisição.
- **Fechar a janela de DNS duplo do modo proxy**: continua como o #51 declarou.

## Further Notes

- A decisão do #51 que deixou o System Settings de fora é substituída por esta frente; o registro vai em `docs/decisions.md` com a data.
- O caso real é o app aberto pelo Finder/Dock: `tauri dev` herda o ambiente do shell, o bundle não. A prova no app real usa o seam de debug (programa no lugar do `scutil`) porque a alternativa seria alterar a configuração de rede da máquina; fica registrado como "não verificado: proxy realmente configurado no System Settings".
- Evidência em `docs/evidence/system-proxy.md`; o que não rodar entra como "não verificado" com o motivo.
