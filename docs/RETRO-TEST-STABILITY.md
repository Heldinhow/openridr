# RETRO — suite de testes confiável (issue #43)

## O que foi entregue

- **Harness hermético** (`crates/host/tests/support`): uma purga única por binário das variáveis
  `OPENRIDR_*` que o host lê, `EnvVar` que purga antes de gravar e restaura no `Drop`, um guard de
  ambiente único e tolerante a envenenamento, e `support::host()` como o único jeito de abrir host
  em teste. Os seis binários de teste do host passaram a usá-lo.
- **Probe é decisão do binário** (`support::enable_probe()`): acabou o liga/desliga por escopo que
  deixava o `Drop` de um teste restaurar "unset" por baixo de outro — o sintoma exato de
  `processes.probe is a development-only probe` que a issue relatava.
- **Nada de caminho global nem nome fixo em `/tmp`** no harness: alvos de escape passaram a viver em
  diretório temporário único (`openridr-created-<pid>.txt`), e a home de app já era por processo.
- **Teste de launch profile resolve o SDK como a produção** (`global.json` → `sdk::resolve` → muxer
  da raiz resolvida com `DOTNET_ROOT`), em vez do `dotnet` do PATH; sem raiz que satisfaça o pin,
  skip nomeando a versão e as raízes vistas.
- **Medição e fonte única**: `docs/evidence/test-stability.md` com tabela por item da issue,
  3 rodadas finais de 431 testes e falha-antes/passa-depois por ticket; `AGENTS.md`, `docs/README.md`
  e `docs/decisions.md` alinhados.
- **Tickets no GitHub** (#57, #59, #62, #64 resolvidos; #67 parado por decisão do Helder), conforme
  a regra nova.

## Evidência real

- 3 rodadas de `cargo test -p openridr-host` com paralelismo default: **431/431 em cada**, 115–146 s.
- Falha-antes/passa-depois de cada ticket na tabela de `docs/evidence/test-stability.md`.
- Nada de E2E foi executado (quarentena #67), e nenhuma janela do app foi aberta: a mudança é de
  harness de teste e de um teste, não de produto.

## Não verificado

- **E2E/Playwright**: decisão do Helder pendente; specs nomeados na tabela como quarentena com
  motivo. Consertar sem rodar seria inventar prova.
- **A falha original dos testes de instalação do debugger**: não reproduziu no HEAD nem no base
  `196e3b3`, com e sem `~/.dotnet` no PATH. Ficou registrada como não reproduzida, com a hipótese de
  interferência entre processos (outra sessão com app aberto, `CARGO_TARGET_DIR` compartilhado,
  alpha.11 em `/Applications`). O que se provou foi a classe "ambiente herdado decide o teste".
- Windows e macOS Intel.
- `verify-openridr`: não aplicável (nenhuma mudança de UI/host-http/Tauri).

## Lições

- **Ambiente de processo é estado global: ou o harness o decide, ou o shell decide.** Duas linhas de
  `std::env::set_var` e um `Drop` mal posicionado transformam "passa aqui" em "falha no CI". A
  pergunta certa não é "quem limpou?", é "quem é o dono?" — e a resposta tem de ser o binário de
  teste, não o teste isolado.
- **Flake relatado sem ambiente preservado não se conserta, se cerca.** Sem as linhas do panic e sem
  o estado da máquina, o honesto é fechar a classe (ambiente, home, `/tmp`) e registrar "não
  reproduzido". O que não se pode é declarar conserto.
- **Teste que chama `dotnet` do PATH está testando a máquina, não o produto.** O produto já resolvia
  o SDK do `global.json`; o teste era o único lugar com a regra antiga. Falha-antes de 3 segundos
  (o binário `lib` abortando os outros seis) é o tipo de cascata que esconde regressão real.
- **`cargo test` aborta no primeiro binário que falha.** Um único teste determinístico vermelho
  esconde 338 verdes. Enquanto o pacote não é verde, "rode a suíte inteira" não é uma instrução
  executável.
- **Nome fixo em `/tmp` é um lock global sem dono.** Duas sessões no mesmo Mac bastam para uma
  apagar o alvo de escape da outra; `tempfile` + pid custa uma linha.
- **Ticket que vira issue do GitHub precisa de numeração real para os bloqueios.** Publicar na ordem
  de dependência (bloqueadores primeiro) não é burocracia: é o que permite escrever
  `Blocked by #57` em vez de "depende do outro ticket".
