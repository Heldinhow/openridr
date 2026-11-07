# SPEC — Launch profiles: browser, expansão de variáveis e escape de argumentos (issue #53)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente:
[#53](https://github.com/Heldinhow/openridr/issues/53) (continuação do ticket #40, que importou
`launchSettings.json`).

## Problema

Um perfil de `Properties/launchSettings.json` importado hoje só entrega `commandName`, `applicationUrl`,
`environmentVariables`, `commandLineArgs` e `workingDirectory` — e entrega três desses pela metade:

- **`launchBrowser` / `launchUrl` são ignorados.** O Rider e o Visual Studio abrem o navegador na URL do
  perfil depois de iniciar; o OpenRidr não abre nada, e o perfil que funciona no Rider não funciona aqui.
- **Nenhuma expansão de variável.** O `dotnet run` expande `%VAR%` nos valores de `environmentVariables`;
  o host entrega o texto literal, então um perfil que depende disso roda com `%ASPNETCORE_ENVIRONMENT%`
  no lugar do valor. O Visual Studio e o Rider também expandem a forma `$(VAR)`.
- **O escape de `commandLineArgs` não é o do .NET.** O host só alterna um booleano de aspas: `""` dentro
  de aspas não vira `"` literal, sequências de `\` antes de `"` não são colapsadas e uma aspa não fechada
  **falha o lançamento** — enquanto o `dotnet run` aceita a mesma string e entrega argumentos diferentes
  dos que o app entrega. Um perfil com `--filter "A \"B\""` roda com argv diferente no Rider e aqui.

O efeito para quem usa o app: o mesmo `launchSettings.json` se comporta de três formas diferentes
conforme a ferramenta, e o app não tem como explicar por quê.

## Solução

O perfil importado passa a se comportar como o `dotnet run` naquilo que o `dotnet run` faz, e como o
Rider naquilo que ele não faz:

- **Argumentos**: `commandLineArgs` é dividido com as regras do `CommandLineToArgvW` — as mesmas que o
  `dotnet run` usa, medidas contra o CLI real. Aspas não fechadas deixam de ser erro.
- **Variáveis**: `%VAR%` e `$(VAR)` são expandidas nos valores de `environmentVariables` e em
  `applicationUrl`, contra o ambiente do host. Nome indefinido fica literal, como no `dotnet run`.
- **Navegador**: com `launchBrowser: true`, o host abre a URL do perfil (base `ASPNETCORE_URLS` efetivo +
  `launchUrl`) no navegador padrão, no Run e no Debug, quando o endereço começa a aceitar conexão. Só
  abre `http`/`https` com host não vazio, chamando o abridor do sistema com a URL como argumento único —
  nunca por shell. Se não der para abrir, o lançamento continua e o log do Run/Debug diz o motivo.

Nada muda na UI: o perfil continua importado e read-only, e o log do Run/Debug é onde a abertura aparece.

## User stories

1. Como pessoa que usa um perfil `http` do Rider, quero que o OpenRidr abra o navegador na mesma URL, para não copiar a URL do log toda vez.
2. Como pessoa que usa `launchUrl: "/swagger"`, quero cair direto na página do perfil, para não navegar até ela depois de abrir.
3. Como pessoa que usa `launchUrl` absoluta (outro host/porta), quero que ela vença a URL do app, para o perfil poder apontar para outro serviço.
4. Como pessoa que usa `launchBrowser: false`, quero que nada abra, para o perfil declarar isso e o app respeitar.
5. Como pessoa que depura, quero o mesmo comportamento no Debug, para o fluxo de depuração não ser diferente do Run.
6. Como pessoa que usa `applicationUrl` com duas entradas (`https;http`), quero que o app abra a primeira, para a escolha ser previsível.
7. Como pessoa que usa `environmentVariables` com `%VAR%`, quero o valor expandido como no `dotnet run`, para o perfil não rodar com o texto literal.
8. Como pessoa que usa a forma `$(VAR)` do Visual Studio/Rider, quero que ela também expanda, para o perfil importado do IDE continuar valendo.
9. Como pessoa que usa `%VAR%` de uma variável que não existe, quero o texto preservado, para o app não inventar valor nem apagar o token.
10. Como pessoa que usa `%` literal num valor (porcentagem), quero que ele sobreviva, para a expansão não estragar o valor.
11. Como pessoa que usa `%VAR%` dentro de `applicationUrl`, quero a URL expandida, para a porta vir do ambiente.
12. Como pessoa que usa `commandLineArgs` com aspas escapadas, quero o mesmo argv que o `dotnet run`, para o programa receber o que o perfil pediu.
13. Como pessoa que usa `commandLineArgs` com aspas não fechadas, quero que o app rode como o `dotnet run` em vez de recusar o perfil.
14. Como pessoa que usa `commandLineArgs` com `""` ou sequências de `\`, quero o mesmo resultado do CLI, para o perfil não precisar de sintaxe própria do OpenRidr.
15. Como pessoa que abre um projeto de outra pessoa, quero que um `launchSettings.json` malicioso não consiga abrir `file:`/`javascript:`/esquema customizado, para o perfil não virar execução arbitrária.
16. Como pessoa que abre um projeto em modo restrito, quero que nada abra, para o modo restrito continuar sem executar nada.
17. Como pessoa que usa o app, quero que uma falha ao abrir (abridor ausente, URL inválida, app que não sobe) não derrube o Run/Debug, para o lançamento continuar sendo o que importa.
18. Como pessoa que usa o app, quero ler no log do Run/Debug a URL aberta ou o motivo de não ter aberto, para não adivinhar.
19. Como pessoa que valida a release, quero prova no app Tauri real de que o navegador abre e de que a URL inválida não abre, para confiar no comportamento do binário.
20. Como pessoa que mantém o OpenRidr, quero testes de host para a gramática do argv e para a validação de URL, para a paridade ser provada por tabela medida, não por opinião.
21. Como pessoa que mantém o OpenRidr, quero que o que não foi verificado (Windows, Intel, abridor Linux) apareça como não verificado, para não vender suporte que não existe.

## Implementation decisions

- **Gramática do argv = `CommandLineToArgvW`, medida contra o CLI.** O divisor do host passa a seguir as
  regras que o `dotnet run` usa (o SDK entrega `commandLineArgs` como `ProcessStartInfo.Arguments`, e o
  processo filho divide com essas regras): espaço/tab separam fora de aspas; `\` só é especial antes de
  `"` (2n `\` + `"` → n `\` e a aspa fecha/abre; 2n+1 `\` + `"` → n `\` + `"` literal); dentro de aspas,
  `""` → `"` literal; aspa não fechada é tolerada (o resto da string é o argumento); `#` e `'` são
  literais. A matriz de referência foi capturada do `dotnet run` real (SDK 8.0.408, macOS ARM64) com 30
  casos e vira teste. O erro `Unclosed quote in launch profile commandLineArgs.` deixa de existir.
- **Expansão: `%VAR%` e `$(VAR)`, uma passada, sem reexpandir o valor substituído.** A expansão usa o
  ambiente do processo do host, como `Environment.ExpandEnvironmentVariables` no `dotnet run`: um valor do
  próprio perfil **não** vê outro valor do perfil (medido: `%EV_A%-two` expandiu para o valor do ambiente
  do pai, não para o do perfil). Nome vazio ou indefinido mantém o texto original (`%NOPE%`, `%%`,
  `$(NOPE)` ficam como estão); `%` de porcentagem sobrevive porque só um par `%nome%` definido muda.
- **Onde expande: `environmentVariables` e `applicationUrl`.** `commandLineArgs` fica literal para o argv
  não divergir do `dotnet run` (que não expande ali); `workingDirectory` fica literal pelo mesmo motivo.
  `$(VAR)` é extensão de Visual Studio/Rider — o `dotnet run` não a expande (o próprio SDK comenta
  "MSBuild variables are not expanded like they are in VS") — e entra porque a issue pede e o perfil
  importado do IDE usa.
- **URL do navegador: base efetiva + `launchUrl`.** A base é a primeira entrada não vazia do
  `ASPNETCORE_URLS` efetivo — isto é, `environmentVariables.ASPNETCORE_URLS` quando existe (vence
  `applicationUrl`, como hoje), senão `applicationUrl`, dividido por `;`. `launchUrl` absoluta `http(s)`
  vence a base; `launchUrl` relativa é concatenada (começa com `/` → `esquema://autoridade` + caminho;
  senão base sem `/` final + `/` + `launchUrl`). Sem base, não abre. A concatenação é declarada: não é
  resolução RFC 3986 completa (o caso teórico de `applicationUrl` com caminho resolve por concatenação).
- **Validação antes do spawn, e é a única porta.** Só abre `http`/`https` com host não vazio; qualquer
  outro esquema (`file:`, `javascript:`, custom), host vazio ou porta `0` (porta dinâmica, o app escolhe
  outra) resulta em **não abrir** com o motivo no log. A URL validada é a que vai no argv — nunca há
  segunda montagem depois da validação.
- **Abrir = um processo direto, sem shell.** macOS `open <url>`, Linux `xdg-open <url>`: o programa é
  fixo e a URL vai como **um** argumento de `Command`, nunca uma string montada para `sh -c`/`cmd /c`.
  Windows fica fora desta frente e é declarado não suportado/não verificado (o projeto não suporta
  Windows hoje): o log diz que abrir navegador não é suportado nesta plataforma.
- **Quando abre: quando o endereço aceita conexão, com prazo.** Uma thread separada espera (máximo 15 s,
  tentativa a cada 200 ms) o host:porta da URL aceitar uma conexão TCP e então abre; se o lançamento
  terminar antes, não abre. O Run/Debug nunca espera por isso: o processo já está rodando e o log recebe
  a linha depois. Prazo esgotado → não abre e o log diz que o app não aceitou conexão.
- **Falha ao abrir nunca falha o lançamento.** Abridor ausente, `spawn` recusado, prazo esgotado: linha no
  log (`Launch browser skipped: <motivo>.`) e o Run/Debug seguem como hoje. Sucesso: `Opened <url> in the
  default browser.` no log do Run (aba Run) e no log DAP (aba Debug).
- **Trust não ganha gate novo.** Run e Debug já exigem workspace confiado (`require_trust`) antes de
  spawnar; a abertura acontece depois desse gate, então modo restrito continua sem executar nada.
- **Sem dependência nova, sem método novo no contrato do host, sem UI nova.** A decisão entra em
  `LaunchOptions` (o mesmo caminho de Run e Debug), o abridor usa `std::process::Command`, e o log
  existente (`run-log`, `dap-log`) mostra o resultado. Nenhum `data-testid` novo.

## Testing decisions

- **Teste bom aqui prova a tabela, não a implementação.** O divisor é uma função pura: a asserção é
  `entrada → argv`, com os 30 casos medidos no `dotnet run` real. Nada de espiar o laço interno.
- **Seam: as funções puras do host** (`split_launch_args`, expansão, decisão de URL, argv do abridor) mais
  o `launch_options` que já é o seam de Run e Debug. Prior art: os testes de launch profile em
  `crates/host/src/run_configs.rs` (`launch_args_keep_empty_and_escaped_quotes`,
  `launch_profiles_preserve_order_and_apply_project_values`).
- **Falha-antes/passa-depois obrigatório** para a gramática do argv (casos que hoje dão argv diferente ou
  erro: `""` dentro de aspas, `\` antes de `"`, aspa não fechada) e para a validação de URL (esquema não
  http(s), host vazio, porta 0, `launchUrl` absoluta vs relativa).
- **Espera por conexão testada com socket real**: um `TcpListener` em `127.0.0.1:0` prova que a espera
  retorna quando o endereço aceita; uma porta fechada com prazo curto prova que ela desiste.
- **O abridor não é executado em teste.** O teste confere o argv construído (`open` + a URL como único
  argumento, nenhum shell) e a decisão; abrir o navegador de verdade é provado no app real.
- **Prova de UI é o app Tauri real com `agent-desktop`** (decisão da #67; Playwright não é prova de
  sessão): perfil com `launchBrowser: true` abre a janela do navegador na URL, `launchUrl` cai na página,
  `launchBrowser: false` não abre, esquema inválido não abre e o log do Run mostra o motivo.
- **Fixture**: `fixtures/LaunchProfiles` ganha perfis para os casos (browser, `launchUrl`, `%VAR%`,
  `$(VAR)`, argumentos com escape, esquema inválido), mantendo `http`/`https`/`IIS Express` como estão
  porque os testes existentes dependem deles.

## Out of scope

- **Windows e macOS Intel** (não suportados hoje): no Windows o abridor fica fora e é declarado.
- **Abrir navegador em configuração de run manual** (sem perfil): `launchBrowser` é propriedade do
  `launchSettings.json`, não da configuração manual.
- **`DOTNET_LAUNCH_PROFILE`** e outras variáveis que o `dotnet run` injeta: fora desta issue.
- **`workingDirectory` do perfil**: o `dotnet run` o ignora e o OpenRidr já o aplica (divergência existente
  e documentada); não muda aqui.
- **Escolher entre as entradas de `applicationUrl`** (preferir https, perguntar, etc.): abre a primeira.
- **Abrir a URL de novo quando o app reinicia** (hot reload/watch): não há watch nesta frente.
- **Navegador embutido** ou escolha de navegador por perfil.

## Further notes

- Evidência em `docs/evidence/launch-profiles-extras.md`, capturas em
  `docs/evidence/launch-profiles-extras/`. Base anterior: `docs/evidence/launch-settings.md`.
- Decisão registrada em `docs/decisions.md`; `GLOSSARY.md` ganha **Launch profile** (o perfil importado do
  `launchSettings.json`, read-only) e **Launch browser** (a abertura da URL do perfil depois do início).
- `docs/README.md` ganha a linha da frente; `docs/implementation-notes.md` ganha o contrato do perfil
  (gramática, expansão, URL) e o que ficou sem prova.
- A matriz de referência do `dotnet run` (30 casos, SDK 8.0.408) fica no teste do host, não num doc: o
  teste é a fonte.
