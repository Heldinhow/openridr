# RETRO — Launch profiles: navegador, expansão e argv (issue #53)

Rodada de 2026-11-07, commits `8b93f06` … `066e89e`, macOS arm64. Spec:
[SPEC-LAUNCH-PROFILES-EXTRAS](SPEC-LAUNCH-PROFILES-EXTRAS.md). Evidência:
[evidence/launch-profiles-extras.md](evidence/launch-profiles-extras.md). Tickets: #157, #158, #159.

## O que se sustentou

- **Medir o CLI antes de escrever a regra pagou três vezes.** A gramática do argv virou uma matriz de
  40 casos capturada do `dotnet run` real (SDK 8.0.408) e o divisor antigo reprovava **16** deles; a
  matriz nova (52 casos, com os de `\` par + `""` e de espaço não-ASCII) é o teste. Sem medir, a
  "gramática do .NET" teria sido implementada de memória e errado nos casos que importam.
- **Medir também derrubou a premissa da issue.** O `dotnet run` expande `%VAR%` **só** em
  `environmentVariables`, **nunca** `$(VAR)` e **ignora** `launchBrowser`/`launchUrl`; quem honra é o
  `dotnet watch`/Rider. A spec passou a declarar o que é paridade de CLI e o que é extensão de IDE, em
  vez de repetir "como o dotnet run" para tudo.
- **A URL validada antes do spawn e o abridor sem shell fecharam como planejado.** `file:`,
  `javascript:`, host vazio, credenciais, porta 0 e wildcard do Kestrel não abrem e o log diz o motivo;
  o abridor recebe a URL como **um** argumento. O caso `browser-bad` no app real mostrou a linha de
  skip e nenhuma janela.
- **A espera por conexão com prazo e sem bloquear o lançamento funcionou no app.** O Run abriu
  `http://localhost:5090/page` (base + `launchUrl`), o Debug abriu `http://localhost:5092/debug-page`,
  `browser-off` não abriu nada e o log do Run/Debug registrou cada caso.
- **A fixture ganhou um modo `--listen` e isso destravou a prova.** Sem um app que escuta de verdade,
  "abrir o navegador" só seria provável por unidade; com o `HttpListener` no fixture, a prova no app é
  real e barata.

## O que quebrou / custou

- **Cinco achados médios e dois baixos na revisão independente, e quatro deles eram reais.**
  - O divisor errava `\` par seguido de `""` dentro de aspas (`a "b\\"" c"`): o .NET entrega `b\" c`,
    o divisor entregava `b\ c`. Só apareceu porque a revisão testou uma combinação que a matriz não
    tinha; a matriz agora tem os oito casos medidos.
  - `wait_ready` checava conexão **antes** de "o lançamento ainda vive": um resto de processo
    escutando na porta faria abrir navegador de um run morto. A ordem inverteu e há teste com listener
    vivo + lançamento morto.
  - Debug com breakpoint de startup pausa antes de escutar; a espera só aceitava `running` e desistia
    para sempre. Passou a aceitar `running | paused`.
  - `open` era logado como "Opened" só por ter sido spawnado; um `open` que sai não-zero (sem
    navegador, sem display) mentia no log. Agora o status do abridor decide a linha.
  - O prazo de 15 s não era teto de verdade: DNS sem prazo e 500 ms por endereço podiam estourar.
    Cada tentativa passou a ser limitada pelo tempo restante, e o DNS ficou declarado como limite.
  - `char::is_whitespace()` separava em `\n`, `\r`, NBSP e VT; o CLI só separa em espaço e tab
    (medido). Virou `' ' | '\t'`.
  - O sétimo (teste do fixture pode passar sem rodar quando não há SDK) é a convenção da casa — skip é
    "não verificado", `OPENRIDR_REQUIRE_DOTNET=1` troca por falha — e ficou como está.
- **A matriz de 40 casos não pegou o caso que a revisão achou.** Ela foi montada a partir de combos
  que eu imaginei; a combinação `\` par + `""` ficou de fora justamente por ser exótica. Medir mais
  casos do que "parece necessário" teria fechado antes.
- **A suíte completa falhou em três testes que não são desta frente, e custou 20 minutos provar.**
  Os três baixam o `csharp-ls` (21,6 MB) em paralelo, cada um com home isolado, e o download do host
  tem prazo de 120 s; rodando sozinhos passam (88,6 s, 110,2 s, 119,3 s) e o mesmo teste passa no
  `main`. Ficou registrado na tabela de [test-stability](evidence/test-stability.md).
- **Peguei o slot do app sem pedir.** O `pgrep` estava vazio quando subi, mas a instância que o
  orquestrador viu era a minha (do ciclo anterior) e outra sessão (#161) estava esperando. O protocolo
  é pedir antes, mesmo quando o `pgrep` está limpo.
- **O seletor de perfis não é alcançável por `ref`.** O `summary` do menu de run config aparece como
  `disclosure` sem `ref_id`; a prova precisou de clique por coordenada (`mouse-click --headed`) para
  abrir o menu e depois os itens (que têm ref). É a limitação já registrada no skill, e ela custou
  duas tentativas de coordenada.
- **O log DAP não vira captura.** O cabeçalho da sessão mostra o caminho do adaptador com o home do
  usuário; a prova do Debug ficou no texto da linha `Opened …` e na segunda aba do Safari.
- **A janela do Safari aberta pela prova não fecha por script.** O AppleEvent expirou (permissão de
  automação) e o `press cmd+w` não teve foco verificado; ficou aberta e declarada.

## Lições

- **Paridade com CLI se prova com tabela medida, não com memória.** O `dotnet run` tem regras
  específicas (aspa não fechada tolerada, `""` literal dentro de aspas, `\` só antes de `"`, só espaço
  e tab separando) e a única forma de não errar é capturar a tabela do binário real e commitar a
  tabela como teste.
- **A ordem das checagens é contrato.** "Está escutando?" antes de "o lançamento vive?" abre navegador
  de processo morto; a checagem de vida vem primeiro, sempre.
- **Sucesso de spawn não é sucesso de efeito.** Para qualquer comando de sistema cujo efeito é fora do
  processo (`open`, `xdg-open`), o exit status é a única evidência que o log pode usar.
- **Prazo sem teto por tentativa não é prazo.** Resolver DNS e conectar são duas esperas; cada uma
  precisa caber no tempo restante, e o que não dá para limitar (DNS no `std`) se declara.
- **Skip não é passe, e teste que baixa 21 MB não roda em paralelo.** A tabela de estabilidade é o
  lugar de registrar, e a medição isolada é o que separa "ambiente" de "regressão".
- **Fixture que escuta é o que torna feature de navegador provável.** Um console que imprime e sai não
  prova abertura de navegador; o modo `--listen` custou 20 linhas de C# e pagou a prova inteira.

## Follow-ups

- **Windows e `xdg-open`:** o abridor fica fora no Windows (declarado) e o caminho Linux não foi
  executado; nenhum dos dois tem prova.
- **DNS sem prazo:** o worker é destacado, então não trava o host, mas um resolver lento atrasa a linha
  do log além dos 15 s. Um resolvedor com prazo exigiria thread própria e canal.
- **`DOTNET_LAUNCH_PROFILE` e outras variáveis que o `dotnet run` injeta** continuam fora; o perfil
  importado não as define.
- **`workingDirectory` do perfil:** o `dotnet run` ignora e o OpenRidr aplica — divergência anterior a
  esta frente, agora declarada na spec.
- **Slot do app real:** pedir antes de subir, mesmo com `pgrep` limpo, e avisar o fechamento por PID.
