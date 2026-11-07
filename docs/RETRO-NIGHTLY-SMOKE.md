# RETRO — o smoke noturno reporta numa issue de rastreio (issue #215, tickets #240–#242)

Rodada de 2026-11-07, macOS arm64 (macOS 26) + runs em `macos-15` do GitHub Actions. Spec:
[SPEC-NIGHTLY-SMOKE](SPEC-NIGHTLY-SMOKE.md). Evidência: [evidence/nightly-smoke.md](evidence/nightly-smoke.md).
Origem: o follow-up do retro da #210 ("o smoke noturno precisa de leitor").

## O que se sustentou

- **O grill de fatos antes da spec mudou o escopo e achou o caso real.** Grepar `e2e-smoke.yml` e
  `playwright.config.ts` mostrou que o workflow não tem `issues: write` nem reporter legível por
  máquina; `gh run list` mostrou que **o smoke estava vermelho na `main` naquele minuto** (run
  42364384217, `solution-git-status.spec.ts:111`). A frente deixou de ser hipótese e passou a ter um
  primeiro caso real, que a primeira noite depois do merge vai pegar sozinha.
- **Capturar um relatório de verdade antes de escrever o `jq` pagou duas vezes.** Um relatório real do
  Playwright 1.64.1 (spec descartável, ~2 s, sem host e sem dotnet) mostrou duas coisas que nenhum JSON
  escrito à mão teria mostrado: suites **aninhadas** carregam `file` com valor errado
  (`../../../../private/tmp/...`), e o `file` é relativo ao `testDir` (`workspace.spec.ts`, não
  `tests/e2e/workspace.spec.ts`). O primeiro filtro que escrevi devolvia caminho quebrado.
- **`workflow_dispatch` reportar foi o que tornou a prova barata.** Três runs de ~2 min cada provaram
  criar, reescrever e fechar, sem esperar a noite e sem depender de o smoke real estar verde.
- **O seam do `gh` falso + fixture real dá um teste de 0,6 s.** O mesmo padrão de `ci-risk-gate.sh`
  (`GH=${GH:-gh}`) mais o fixture de verdade: 48 asserções, sem rede, sem GitHub, e roda em `dash`.
- **A prova acidental valeu mais que a planejada.** A segunda run vermelha (não planejada) provou o
  caminho "reescreve o corpo e não cria uma segunda", que o plano só cobria no teste.
- **Os dois implementer subagents entregaram com falha-antes medida.** O #240 veio com o fixture copiado
  byte a byte e o filtro certo; o #241 com as asserções de contrato do YAML falhando contra o YAML
  anterior (14 falhas) e contra três cópias sabotadas.

## O que quebrou / custou

- **`build.spec.ts` falhou sozinho no runner** e transformou a run "verde" em vermelha: precisei de uma
  terceira run. Ganho colateral: o caminho "não duplica" provado com run real, e a issue **#251**.
- **A primeira captura do relatório real falhou por resolução de módulo** (`/tmp` sem `node_modules`):
  `Cannot find module '@playwright/test'`. Custo: 5 min e um symlink para o `node_modules` do worktree.
- **O `--limit` do `gh issue list` estava escrito no relatório do subagente como "não verificado" e eu
  não agi.** Virou achado médio do code review: a página default é 30 e o casamento do título acontece
  depois dela, então uma página pequena esconderia a issue e o script abriria uma segunda.
- **A asserção de `issues: write` procurava em tudo depois de `jobs:`.** Passaria com a permissão em
  outro job ou dentro de um comentário — asserção fraca, achado baixo do review.
- **A nota "Prova da #215" no corpo da issue de teste foi apagada pela run vermelha seguinte.** É o
  design (o corpo é o último resultado), mas mostra que anotação manual no corpo de uma issue de
  rastreio é efêmera; a nota teve de ser reescrita à mão.
- **O `git commit --amend` da spec** foi necessário porque o commit do spec saiu antes de eu medir a
  forma do relatório. Commit local não empurrado, mas é sinal de que a spec foi escrita cedo demais para
  os fatos que ela mesma cita.

## Lições

- **Relatório de ferramenta: capture um de verdade antes de escrever o parser.** A forma documentada não
  conta as armadilhas; a real conta. Um fixture real também é o que faz o teste provar o filtro, não a
  imaginação de quem escreveu.
- **"Não verificado" no relatório do subagente é backlog, não rodapé.** Ler a lista e decidir caso a
  caso antes de aceitar o merge. O `--limit` estava lá, explícito, e só virou correção no review.
- **Asserção sobre YAML por `grep` precisa de escopo.** "Existe em algum lugar do arquivo" passa com a
  linha no job errado ou num comentário; o escopo (bloco do job, bloco de permissões) é o que faz a
  asserção valer. Vale para qualquer contrato de arquivo lido como texto.
- **Um caminho de prova não planejado pode provar mais que o planejado.** Quando uma run "de controle"
  falha por outro motivo, o resultado dela ainda é evidência — basta olhar o que ela exercitou.
- **Timeout não é o único buraco de reporte: o checkout também é.** Sem código não há script, e o limite
  agora está escrito na spec e grita no log com `::error::` em vez de morrer num `No such file`.

## Follow-ups

- **#251 — `build.spec.ts` falha sozinho no runner** e passa depois de `workspace.spec.ts`. Hipótese de
  dependência de ordem; a mensagem do build (`exit 1`) não foi preservada.
- **#245 — `solution-git-status.spec.ts:111`** vermelho na `main` desde que entrou na lista (revisão r1
  da #49, marcada como "ainda NÃO medido").
- **A primeira execução `schedule` de verdade** depois do merge é a primeira prova da noturna agendada;
  ninguém a observou ainda. Ela deve abrir a issue de rastreio real por causa de #245.
- **Plano B do job separado** (`needs: smoke`, `if: always()`, output com a lista) se o timeout de 60 min
  doer na prática: é o que cobriria timeout e cancelamento.
- **`gh issue list --limit 100`** é um teto; se o repositório um dia tiver mais de 100 issues com
  `e2e-smoke` no título, o script precisa paginar de verdade.

## Não verificado

- A run **agendada** (`schedule`) com este workflow: as provas são `workflow_dispatch`.
- O `shellcheck` que o job `governance` liga por padrão no `actionlint`: rodou localmente (clean), mas a
  execução no runner não foi observada.
- Windows e macOS Intel.
