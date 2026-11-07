# RETRO — Proxy do macOS System Settings (issue #132)

Rodada de 2026-11-07, branch `feat/issue-132-macos-system-proxy` (`9bdfd61`, `3d2f8fe`, `76b3812`), macOS arm64. Spec: [SPEC-SYSTEM-PROXY](SPEC-SYSTEM-PROXY.md). Evidência: [evidence/system-proxy.md](evidence/system-proxy.md). Tickets: #181, #182. Frente anterior: [#51](https://github.com/Heldinhow/openridr/issues/51) ([RETRO-CORPORATE-PROXY](RETRO-CORPORATE-PROXY.md)).

## O que se sustentou

- **A porta de entrada única do #51 pagou de novo.** A feature inteira no `http.rs` foi encaixar uma fonte a mais na lista ordenada: guarda, falha fechada, rótulo e mensagens vieram de graça. Nenhum cliente novo, nenhuma lista nova de endereço privado, nenhum `ureq::AgentBuilder` fora do módulo.
- **O leitor ficou fora do `http.rs`.** `system_proxy.rs` (parse puro + spawn fino) manteve o diff do arquivo compartilhado pequeno — importava porque a #162 estava mexendo nele na mesma janela.
- **Amostra real antes de escrever o parser.** Rodar `scutil --proxy` na máquina sem proxy deu o fixture do caso "nada configurado" e o custo (~10 ms) que decidiu a leitura por requisição. Nenhuma suposição sobre o formato sobreviveu sem teste.
- **Seam de teste com gate de release.** A variável de debug resolveu dois problemas de uma vez: a suíte parou de depender da configuração de rede da máquina e o caminho de produto passou a ser provável com um `scutil` falso. O gate (`off` em release, programa só em debug) é testado na decisão pura.
- **A prova no app real valeu o slot.** O proxy local registrou `CONNECT api.nuget.org:443` e a tela mostrou `The proxy at 127.0.0.1:54499 (macOS system proxy) refused the connection…`: é a diferença entre "o host funciona" e "o app funciona", e só apareceu porque o binário de dev (sem bundle) foi usado depois de o bundle recusar o download por design.

## O que quebrou / custou

- **O bundle não serve para provar o cliente HTTP.** A primeira tentativa (bundle de debug com `--config` sem resources) parou em "nothing is downloaded on first use": o download de ferramenta só existe fora do bundle. Perdi um ciclo de build+launch até ler a mensagem da tela.
- **Meu próprio spec estava errado na precedência.** Escrevi que `HTTP_PROXY` do ambiente perderia para o `HTTPSProxy` do sistema; o teste reprovou na hora e mostrou que, com o sistema como **último** item (a decisão do grill), qualquer variável do ambiente vence. O código estava certo; a spec foi corrigida.
- **O parser errou o `ExceptionsList` na primeira versão.** `key.trim_end_matches(':')` não remove o `:` quando há espaço depois dele; a lista saía vazia e três testes reprovaram. Foi o único bug de parsing, e o fixture real o pegou.
- **`Route::label` virou código morto no #182.** Ao trocar o rótulo pelo descrito, o `label` ficou só com chamadores de teste; o `cargo check` acusou. Resolvido removendo o método e fazendo os testes asserirem o que o usuário vê.
- **A revisão terra achou quatro coisas reais e uma discussão.** Faltava o harness dos testes **unitários** (só o de integração neutralizava a fonte), o `stalled` não levava a frase de SOCKS/PAC, a frase aparecia mesmo quando um proxy tinha sido usado, e um `HTTPSProxy : ::1` chegava sem colchetes e virava "URL inválida". O quinto (reler o ajuste a cada salto de redirect) foi respondido por escrito e a spec foi ajustada: um salto não é requisição independente.
- **Edição mecânica com contagem de parênteses deixou `, None)` no lugar errado** em sete chamadas multi-linha; `cargo check` pegou na hora, mas custou uma rodada de conserto.

## Lições

- **Quando a feature é "mais uma fonte", o custo está no leitor, não na rota.** O desenho do #51 fez o encaixe ser trivial; o trabalho real foi interpretar o formato do sistema e provar que a suíte não passou a depender da máquina.
- **Teste de unidade também lê o mundo.** O harness de integração já neutralizava `OPENRIDR_*`, mas o binário de testes de unidade tem o seu (`src/test_harness.rs`) e ficou de fora. Dois harnesses, duas listas: ao acrescentar uma variável de ambiente lida pelo host, procure **todos** os lugares que purgam.
- **Diagnóstico que culpa o ajuste errado é pior que diagnóstico nenhum.** A frase de SOCKS/PAC só faz sentido quando não houve proxy; anexá-la sempre teria mandado o usuário mexer no System Settings por causa de um proxy do ambiente.
- **O bundle distribuído e o binário de dev têm caminhos de rede diferentes.** Provar download no app exige o binário de dev; provar o app empacotado exige outra feature. Saber qual superfície tem o código antes de montar a prova economiza um build.
- **Fixture real > formato imaginado.** A amostra do `scutil` desta máquina (sem proxy) e o dicionário documentado cobriram o parser; o caso que quebrou foi o `ExceptionsList` com espaço antes dos dois-pontos, que só apareceu porque havia fixture.

## Follow-ups

- **Proxy realmente configurado no System Settings**: não verificado nesta rodada (não altero a configuração de rede da máquina). Quem tiver um Mac corporativo deve rodar a prova de fumaça: abrir o app pelo Finder com o proxy ligado e ver o Discover sair pelo proxy.
- **PAC/WPAD**: declarado fora; hoje o ajuste é nomeado no diagnóstico e a rota vai direta.
- **SOCKS**: fora (exigiria `socks-proxy` no `ureq`); nomeado no diagnóstico.
- **Windows (WinINET/registro) e Linux (`gsettings`)**: fora de escopo por decisão, sem issue aberta.
- **Campo "Proxy" nas Settings**: fora de escopo por decisão (o usuário corporativo já configurou no macOS).
- **Credenciais do proxy do sistema via Keychain**: fora; a saída é `HTTPS_PROXY` com usuário/senha.
- Não verificado: Windows, macOS Intel, captura de tela da prova (privacidade do caminho home).
