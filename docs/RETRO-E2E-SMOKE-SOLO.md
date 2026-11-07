# RETRO — o spec do smoke passa sozinho e a falha carrega o log cru (issue #251, tickets #255–#257)

Rodada de 2026-11-07, macOS arm64 (macOS 26) + runs em `macos-15` do GitHub Actions. Spec:
[SPEC-E2E-SMOKE-SOLO](SPEC-E2E-SMOKE-SOLO.md). Evidência: [evidence/e2e-smoke-solo.md](evidence/e2e-smoke-solo.md).
Origem: o achado da #215 ("`build.spec.ts` falhou sozinho no runner").

## O que se sustentou

- **O instrumento antes do palpite.** O #255 (log cru + comando + cwd na mensagem de falha) foi feito
  antes de qualquer tentativa de conserto, e a **primeira run vermelha depois dele** entregou a causa
  inteira: `MSB4018 GenerateDepsFile` + `IOException` no `OrderDesk.Core.deps.json` "being used by
  another process". Sem isso, o "flake" teria virado retry cego.
- **O fato que matou a hipótese de ordem custou uma linha de leitura.** Na run vermelha o **segundo**
  teste do mesmo arquivo (mesmo fluxo: abrir, confiar, restaurar, construir) passou em 17,6 s. Se fosse
  estado de workspace deixado por `workspace.spec.ts`, o segundo também falharia. A hipótese virou
  "primeiro build da máquina" antes de qualquer run nova.
- **Medir local antes de gastar runner separou o frio do irrelevante.** Cache NuGet frio
  (`NUGET_PACKAGES`/`DOTNET_CLI_HOME` em diretório novo) deu 2/2 verde: o frio que importa não é o do
  NuGet, é a automação do app ainda rodando.
- **A amostragem de processos provou o mecanismo mesmo sem reproduzir a falha.** `ps` + `lsof` a cada
  100 ms durante o primeiro build mostrou `dotnet test --list-tests` (descoberta automática, que
  constrói), `dotnet build` e o `BuildHost` do csharp-ls vivos ao mesmo tempo. Três runs verdes com essa
  foto já bastaram para nomear o mecanismo e separar o bug do produto (#275).
- **O contrato escrito onde a lista mora.** "Cada spec do smoke passa sozinho, numa máquina fria" ficou
  no cabeçalho de `scripts/e2e-smoke.txt` e na tabela de `docs/evidence/test-stability.md`, não só na
  spec: é o que o próximo spec adicionado vai ler.

## O que quebrou / custou

- **A primeira hipótese (ordem de specs) estava errada e custou duas rodadas de raciocínio.** O que a
  derrubou foi o log da própria run, não uma run nova — eu podia ter olhado o `1 passed` antes.
- **O spec de diagnóstico mudou o timing e nunca reproduziu a falha** (3 runs de amostragem verdes,
  contra 2 de 5 vermelhas do `build.spec.ts` sozinho). Ele clicava Build mais cedo e rodava um laço de
  `ps`/`lsof` a cada 100 ms. Lição: instrumento que muda o timing não serve para reproduzir; o
  instrumento certo é o que **não** muda o caminho (a mensagem de falha do próprio spec).
- **O `lsof` não pegou o dono do arquivo** em nenhuma run vermelha: a atribuição entre descoberta e
  análise ficou por log + lista de processos. O bug do produto cobre os dois, mas a atribuição exata
  continua em aberto — está escrito como limite na evidência.
- **Commitei na branch errada.** O commit do fix (espera da descoberta) saiu na branch de diagnóstico
  `ci/issue-251-diagnosis` em vez da integration branch, e precisou de `cherry-pick`. Custo: um commit
  duplicado no histórico da branch descartável. `git branch --show-current` antes de commitar.
- **A issue de rastreio do smoke virou estado compartilhado entre duas sessões.** A #266 nasceu da minha
  run vermelha, teve o corpo reescrito pela run vermelha da branch da #245, e a minha run verde a fechou
  pelo desenho da #215 — enquanto a #245 continuava vermelha. Não é bug do script (o corpo é "último
  resultado"), é a consequência de duas sessões disparando o mesmo workflow.
- **Oito runs de `e2e-smoke` nesta sessão** (~40 min de runner): 5 do `build.spec.ts` sozinho, 3 do
  spec de diagnóstico, 1 da prova final. As três de diagnóstico foram as menos produtivas.

## O que fica

- `build.spec.ts` espera a descoberta automática antes do primeiro build e, se falhar, a mensagem traz o
  log cru do `dotnet build`.
- #275 (bug do produto): o app roda duas construções do mesmo projeto ao mesmo tempo (descoberta
  automática e/ou análise contra o build explícito). O spec espera; o produto continua exposto.
- A linha da falha em `docs/evidence/test-stability.md` saiu de "não investigado" para causa medida.
