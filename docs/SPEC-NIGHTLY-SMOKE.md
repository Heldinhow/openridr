# SPEC — o smoke E2E noturno abre uma issue de rastreio quando falha (issue #215)

Status: ready-for-agent
Fonte: issue GitHub #215 (Heldinhow/openridr) + grill de 2026-11-07 com o orquestrador (Q1–Q6) + retro
da #210 (`docs/RETRO-E2E-PORTS.md`, "Follow-ups": "o smoke noturno (#56) precisa de leitor").
Tickets: issues filhas de #215 (criadas por esta frente).
Alvo: macOS Apple Silicon. Windows e macOS Intel continuam fora do que foi executado.

## Problem Statement

O smoke noturno de E2E roda, falha e ninguém fica sabendo. O workflow `e2e-smoke.yml` é o único E2E
automatizado do repositório (decisão da #67) e não é check obrigatório de nada; quando ele fica
vermelho, o resultado vive numa página de Actions que ninguém abre. A consequência está medida duas
vezes:

- `workspace.spec.ts` ficou vermelho na `main` da #177 até a #210 — quatro locators apontando para um
  elemento que só existe com o menu aberto. O spec está na lista do smoke desde a #67.
- No dia em que esta spec foi escrita, o smoke estava vermelho na `main`: run
  [42364384217](https://github.com/Heldinhow/openridr/actions/runs/42364384217) (2026-11-07,
  `schedule`), `tests/e2e/solution-git-status.spec.ts:111` — `1 failed, 27 passed`, 4,8 min. Ninguém
  viu. Esse é o problema, acontecendo.

O smoke sem leitor não é guardrail: é registro. Quem quebra um spec que está na lista não recebe
nenhum sinal, e o vermelho atravessa frentes inteiras até alguém tropeçar nele por acidente.

## Solution

Quando o smoke fica vermelho, o próprio job abre — ou atualiza — **uma** issue de rastreio com os
specs que falharam e o link da run. Quando ele volta a ficar verde, o job fecha essa issue. Nada mais
muda no CI: nenhum check obrigatório novo, nenhum Playwright em PR, o `concurrency` sem cancelamento
continua igual, e a lista de specs continua sendo `scripts/e2e-smoke.txt`.

A issue é o leitor que faltava: ela aparece na lista de issues abertas do repositório, que é onde o
mantenedor olha, em vez de uma página de Actions que ele não abre.

## User Stories

1. Como mantenedor, quero que uma noite vermelha vire uma issue aberta no repositório, para descobrir
   o problema sem abrir a página de Actions.
2. Como mantenedor, quero que essa issue diga **quais specs** falharam e **qual run** provou isso, para
   ir direto ao log certo em vez de procurar a run.
3. Como mantenedor, quero que a issue traga também os títulos dos testes que falharam, para saber se é
   o spec inteiro ou um caso, sem abrir o log.
4. Como mantenedor, quero no máximo **uma** issue aberta de rastreio, para a lista de issues abertas não
   virar um histórico de noites vermelhas.
5. Como mantenedor, quero que a issue seja atualizada na noite seguinte, para o corpo dela ser sempre o
   último resultado e não o primeiro.
6. Como mantenedor, quero que a data de criação da issue funcione como "vermelho desde", para saber há
   quantas noites o smoke está quebrado sem ler o corpo.
7. Como mantenedor, quero que a primeira noite verde feche a issue, para uma issue aberta significar
   "está vermelho agora".
8. Como mantenedor, quero que a issue traga o `ref` e o SHA da run, para não confundir uma execução
   manual numa branch com a noturna da `main`.
9. Como mantenedor, quero que uma run que falhou **antes** de os specs rodarem (instalação, download do
   navegador, bootstrap) também abra a issue, com o motivo "falhou antes do relatório", para que o
   silêncio não volte pela porta dos fundos.
10. Como mantenedor, quero que a falha em fechar a issue numa run verde deixe o job vermelho, para uma
    run verde nunca conviver com uma issue de rastreio aberta mentindo.
11. Como sessão de agente, quero que o workflow continue sem trigger de PR e sem `pull_request`, para
    nada de Playwright voltar ao caminho do PR por acidente.
12. Como sessão de agente, quero que a lógica de criar/atualizar/fechar more num script com teste
    local, para poder provar o comportamento sem esperar a noite nem gastar uma run de 5 minutos.
13. Como sessão de agente, quero rodar esse teste sem rede e sem GitHub, com um `gh` falso no `PATH`,
    para ele caber no laço normal de desenvolvimento.
14. Como mantenedor, quero que a prova real (issue abre numa run vermelha, fecha numa verde) seja feita
    com `workflow_dispatch` numa branch descartável, para não depender de acordar às 04:30.
15. Como mantenedor, quero que a issue de teste criada por essa prova seja fechada no fim, para não
    ficar uma issue de rastreio falsa na lista.
16. Como mantenedor, quero que o limite conhecido (timeout ou cancelamento do job não reporta) esteja
    escrito na spec e no código, para ninguém supor cobertura que não existe.
17. Como revisor, quero que o contrato do workflow (sem trigger de PR, `concurrency` sem cancelamento,
    `issues: write` no job) tenha asserção, para a próxima edição não desfazer os critérios de aceite
    em silêncio.

## Implementation Decisions

### O que reporta (Q1)

`schedule` na `main` **e** `workflow_dispatch` em qualquer ref. O guard é
`github.ref == 'refs/heads/main' || github.event_name == 'workflow_dispatch'`. O `schedule` só roda na
default branch de qualquer forma; o `workflow_dispatch` fica incluído porque é o único jeito de provar
os dois caminhos sem esperar a noite, e porque disparo manual é ato deliberado. O corpo da issue mostra
`ref` e SHA, então uma execução em branch não se confunde com a noturna.

### Uma issue, achada pelo título (Q2)

- Título fixo: `[nightly] e2e-smoke is red`.
- Busca: `gh issue list --state open --search 'e2e-smoke in:title' --json number,title`, casando o
  título **exato** no `jq`. Nada de label novo: label é setting do repositório, e a sessão não cria
  setting. A busca é por `e2e-smoke` (não pelo marcador com colchetes, que a sintaxe de busca do GitHub
  interpreta) e o casamento exato acontece depois, no shell.
- Se mais de uma issue aberta casar, a de menor número é a que vale; as outras não são tocadas. É um
  estado que só existe por intervenção manual.

### Vermelho com a issue já aberta: reescrever o corpo (Q3)

O corpo é reescrito com o último resultado (run, ref, SHA, specs, testes). Nada de um comentário por
noite: o histórico de runs já está na página do workflow, e a issue fica legível. A data de criação da
issue é o "vermelho desde".

### Um job só (Q4)

Os passos de reportar ficam no mesmo job `smoke`, com `if: failure()` e `if: success()`. Isso cobre
falha de teste, falha de instalação, falha de download do navegador e falha de qualquer passo anterior,
porque um passo com `if: failure()` roda mesmo assim. O que **não** é coberto é timeout de 60 min e
cancelamento: o job morre e nenhum passo posterior roda. Limite aceito e escrito como `shortcut:` no
script e na spec.

### Fechar falhando deixa o job vermelho (Q5)

O passo que fecha a issue numa run verde sai com `::error::` e código diferente de zero. Uma run verde
com a issue de rastreio aberta é o mesmo silêncio que esta frente existe para matar. Criar/atualizar
numa run vermelha já é vermelho de qualquer forma, mas o passo também reporta `::error::` para o motivo
aparecer no log.

### A lógica mora num script, não no YAML

`scripts/e2e-smoke-issue.sh`, chamado por dois passos do workflow (um para vermelho, um para verde).
Contrato:

- `RESULT=red|green` (obrigatório), `RUN_URL`, `REF`, `SHA`, `REPO` (obrigatórios).
- `REPORT`: caminho do relatório JSON do Playwright. Ausente ou ilegível numa run vermelha significa
  "a run falhou antes de os specs rodarem" — o corpo diz isso em vez de listar specs.
- `GH`: caminho do `gh` (default `gh`), e `GH_TOKEN`: credencial. É o mesmo seam de
  `scripts/ci-risk-gate.sh` (`GH=${GH:-gh}`), que os testes apontam para um stub.
- Saída: uma linha por decisão no stdout (o log da run conta o que ele fez), `::error::` no stderr em
  qualquer falha do `gh`, e código diferente de zero.

O YAML fica com a montagem do relatório e a chamada do script; nenhuma decisão de "criar ou atualizar"
mora lá.

### O relatório: reporter JSON, e a forma dele foi medida

O workflow passa `--reporter=list,json` com `PLAYWRIGHT_JSON_OUTPUT_NAME` apontando para
`$RUNNER_TEMP`, então o log continua legível e o script ganha um arquivo. O `playwright.config.ts`
continua com `reporter: [["list"]]`: quem pede o JSON é a linha de comando do CI, não o default da
suíte.

A forma do relatório foi capturada de uma execução real do Playwright (spec descartável que falha, sem
host e sem dotnet, ~2 s), e ela tem uma armadilha que muda o filtro: **suites aninhadas também têm
`file`, com valor errado.** Num relatório real de um spec com `describe`, a suite de topo trouxe
`file: "shape.spec.ts"` e as suites internas trouxeram
`file: "../../private/tmp/pw-shape/shape.spec.ts"`. Filtrar `has("file")` e pegar `.file` devolve o
caminho quebrado; o filtro correto herda o `file` da suite de topo ao descer a recursão e devolve
`shape.spec.ts` para os dois testes que falharam.

- Specs que falharam: o `file` herdado da suite de topo, para cada spec com `ok == false`, único.
- Testes que falharam: o `title` de cada spec com `ok == false`, único.
- **O `file` é relativo ao `testDir`, não à raiz do repositório.** O `playwright.config.ts` usa
  `testDir: "./tests/e2e"`, então o relatório traz `workspace.spec.ts` e não
  `tests/e2e/workspace.spec.ts`; o `config` do relatório não carrega o `testDir`, só o `rootDir`
  absoluto. O script não inventa prefixo (nada de `tests/e2e/` hardcoded) nem ganha variável de
  ambiente nova: o corpo da issue diz "relativo a `tests/e2e/`" e lista o nome como o Playwright deu.
- Relatório presente, vermelho e **sem** spec com `ok == false` (falha de `globalSetup`, por exemplo):
  o corpo diz que a run falhou sem spec identificada e manda olhar o log. O relatório distingue os dois
  casos sem adivinhação: `stats.unexpected` conta os testes que falharam e `errors` é a lista de erros
  fora de teste (é onde a falha de `globalSetup` aparece). Não é o caso comum, mas é o caso em que o
  bootstrap de ferramentas falha — e ele falha por rede.

### Corpo da issue

Vermelho: o link da run, o `ref` e o SHA, a data, a lista de specs, a lista de testes, e a frase de
rodapé dizendo que o workflow fecha a issue na próxima run verde. Verde: um comentário com o link da
run verde antes de fechar, para a issue fechada contar a própria história.

### Permissões e superfície de ataque

`issues: write` entra **no job**, não no workflow (o `permissions` de topo continua `contents: read`),
que é o padrão do job `risk` do `ci.yml`. Nenhuma expressão `${{ }}` é interpolada dentro de um bloco
`run:`: `RUN_URL`, `REF` e `SHA` chegam por `env:`, que é o que o `zizmor` cobra. Nenhuma action nova,
nenhuma dependência nova: `gh` e `jq` já são ferramentas da casa e já estão nos runners.

### O que não muda

`concurrency: e2e-smoke` com `cancel-in-progress: false`, nenhum trigger de `pull_request`, nenhum
check obrigatório, nenhuma mudança em `scripts/e2e-smoke.txt`, nenhuma mudança no `playwright.config.ts`
e nenhum `data-testid`.

## Testing Decisions

- **Bom teste aqui prova comportamento observável do script**: quais chamadas `gh` ele faz, com que
  argumentos, e qual é o exit code. Nada de espiar função interna nem de casar o texto do corpo inteiro.
- **Seam 1 (obrigatório): o script, com `gh` falso no `PATH`.** É o mesmo seam de
  `tests/shell/ci-risk-gate.test.sh` (stub `gh` respondendo `gh api`, `GH` apontável) e de
  `tests/shell/ci-aggregate.test.sh` (entradas por env, sem rede). Um stub `gh` grava o argv de cada
  chamada e responde a busca com uma lista fixa, então o teste prova criar vs reescrever vs fechar sem
  tocar no GitHub.
- **Fixture do relatório: um JSON real do Playwright**, capturado de uma spec descartável que falha (sem
  host, sem dotnet, ~2 s), e guardado ao lado do teste. É o que prova que o filtro casa com a forma de
  verdade — inclusive a armadilha do `file` das suites aninhadas, que um JSON escrito à mão não teria.
- **Seam 2 (mesmo arquivo de teste): asserções sobre o contrato do YAML.** Sem trigger de
  `pull_request`, `cancel-in-progress: false`, `issues: write` no job, e os dois passos de reporte
  presentes. `actionlint` e `zizmor` validam sintaxe e permissões; nenhum dos dois sabe que "nunca roda
  em PR" é um critério de aceite.
- **Casos do seam 1:** vermelho sem relatório cria com "falhou antes dos specs"; vermelho com relatório
  cria com os specs e os títulos; vermelho com issue aberta reescreve o corpo e não cria segunda; verde
  com issue aberta comenta e fecha; verde sem issue sai 0 sem escrever nada; falha do `gh` sai diferente
  de zero com `::error::`.
- **Falha-antes/passa-depois:** hoje não existe script nem passo, então o "antes" é o teste falhando por
  ausência do script; o "depois" é o teste verde. O `sh tests/shell/e2e-smoke-issue.test.sh` roda em
  segundos e entra no laço normal, como os outros quatro testes de `tests/shell/`.
- **A prova de aceite não é o teste:** é a run de verdade, descrita abaixo.

## Out of Scope

- **Consertar o spec vermelho de hoje** (`tests/e2e/solution-git-status.spec.ts:111`, o teste de symlink
  com `2` ícones onde o spec espera `1`). Vira issue avulsa, não filha desta: é spec de UI, não CI.
- Rodar Playwright em PR, transformar o smoke em check obrigatório, ou mexer no ruleset.
- Cobertura de timeout de 60 min e de cancelamento do job (limite aceito, com `shortcut:` no código).
- Label nova de rastreio (setting do repositório).
- Artefato do relatório do Playwright (`upload-artifact`): o relatório é lido no mesmo job.
- Comentar na issue a cada noite vermelha.
- Windows e macOS Intel.

## Further Notes

- **O primeiro dia já tem caso real.** O smoke está vermelho na `main` agora, então a primeira noite
  depois do merge abre a issue de rastreio de verdade — a prova do caminho "criar" vem de graça, e o
  `solution-git-status.spec.ts` é o primeiro conteúdo dela.
- **Prova planejada (aceite da issue).** Numa branch descartável, com o workflow desta frente:
  1. uma spec que falha de propósito entra na lista do smoke, `workflow_dispatch` → a run fica vermelha
     e a issue de rastreio nasce com o nome do spec e o link da run;
  2. a spec sai da lista, `workflow_dispatch` → a run fica verde e a issue é fechada;
  3. a issue de teste é fechada de qualquer forma no fim, e a branch descartável é apagada.
  Custo: duas runs de `macos-15` (a primeira curta, a segunda com a lista real), e uma issue criada e
  fechada no repositório. É a única prova honesta: o workflow não roda em PR, por decisão.
- **Por que não um job separado com `needs: smoke`.** Cobriria timeout e cancelamento, mas custaria
  transportar a lista de specs por output e poria `issues: write` num job que não roda código de teste.
  Se o timeout doer na prática, é o plano B, e o custo já está medido: um output multi-linha e um job.
- **Risco do PR.** A frente toca `.github/workflows/**` e `scripts/`: é `risk:high`. O PR do dono do
  repositório passa sem review (decisão de 2026-11-07), e o merge é do mantenedor/orquestrador.
