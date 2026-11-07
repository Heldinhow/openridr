# Evidência — o smoke noturno reporta numa issue de rastreio (issue #215, tickets #240–#242)

Data: 2026-11-07. Máquina: macOS Apple Silicon (macOS 26), `gh` 2.83.1, `actionlint` local,
`zizmor` 1.32.0 via `uvx`. As runs de prova rodaram em `macos-15` do GitHub Actions. Windows e macOS
Intel não foram executados.

Spec: [SPEC-NIGHTLY-SMOKE](../SPEC-NIGHTLY-SMOKE.md). Tickets: #240 (script), #241 (workflow), #242
(esta prova).

## Critérios de aceite da issue #215

| Critério | Como foi provado | Resultado |
|---|---|---|
| Uma run vermelha deixa **exatamente uma** issue aberta, com os specs e a URL da run | run [42406406643](https://github.com/Heldinhow/openridr/actions/runs/42406406643) numa branch descartável → criou a issue #244 com `tmp-smoke-red.spec.ts` e o link da run | **provado** |
| Uma run verde **fecha** a issue | run [42406925919](https://github.com/Heldinhow/openridr/actions/runs/42406925919) → comentou o link e fechou a #244 (`state=CLOSED`, `stateReason=COMPLETED`) | **provado** |
| O workflow mantém `concurrency` sem cancel-in-progress e **nunca** roda em PR | asserções de contrato do YAML em `tests/shell/e2e-smoke-issue.test.sh` (falham se `pull_request` voltar, se `cancel-in-progress` virar `true` ou se `issues: write` sair do job) + `actionlint` + `zizmor` | **provado** |

## As quatro runs

| Run | Quando (UTC) | Ref | Resultado | O que provou |
|---|---|---|---|---|
| [42364384217](https://github.com/Heldinhow/openridr/actions/runs/42364384217) | 10:32 → 10:38 | `main`, `schedule` | failure | o problema: `solution-git-status.spec.ts:111` vermelho na `main`, 1 failed / 27 passed, **sem ninguém ver** |
| [42406406643](https://github.com/Heldinhow/openridr/actions/runs/42406406643) | 19:28 → 19:30 | `ci/issue-215-proof`, `workflow_dispatch` | failure | run vermelha **cria** a issue #244, com o spec e o link |
| [42406578543](https://github.com/Heldinhow/openridr/actions/runs/42406578543) | 19:30 → 19:34 | idem | failure | run vermelha com a issue aberta **reescreve o corpo** dela e não cria uma segunda (o corpo passou a listar `build.spec.ts`) |
| [42406925919](https://github.com/Heldinhow/openridr/actions/runs/42406925919) | 19:35 → 19:37 | idem | success | run verde **comenta e fecha** a #244 |

A segunda run vermelha não estava planejada: a lista da prova apontava para `build.spec.ts`, que falhou
no runner (ver "Achados"). Ela acabou provando de graça o caminho "reescreve, não duplica" — a #244
continuou sendo a única issue de rastreio aberta, com o corpo trocado.

Corpo que a run vermelha escreveu (issue #244, antes de a nota de prova ser acrescentada à mão):

```text
The nightly E2E smoke is **red**.

- Run: https://github.com/Heldinhow/openridr/actions/runs/42406406643
- Ref: `refs/heads/ci/issue-215-proof`
- Commit: `d0761461deceb1268df2917a5e7970980f40b78a`
- Reported at: 2026-11-07T19:30:29Z

Failed specs (relative to `tests/e2e/`):

- `tmp-smoke-red.spec.ts`

Failed tests:

- the smoke tracking issue is opened by a red run

The workflow closes this issue on the next green run.
```

## Como a prova foi montada

Branch descartável `ci/issue-215-proof` (nunca mesclada, apagada no fim, local e remota):

1. `tests/e2e/tmp-smoke-red.spec.ts` — `expect(1).toBe(2)` — e `scripts/e2e-smoke.txt` com só ele.
2. `gh workflow run e2e-smoke.yml --ref ci/issue-215-proof` → run vermelha → issue #244.
3. A issue recebeu à mão, no topo do corpo, a nota "Prova da #215" (pedido do orquestrador).
4. `scripts/e2e-smoke.txt` trocado por `tests/e2e/build.spec.ts` → run vermelha (o spec falhou) →
   corpo da #244 reescrito.
5. `tests/e2e/tmp-smoke-green.spec.ts` — `expect(1).toBe(1)` — e a lista com só ele → run verde →
   #244 comentada e fechada.
6. Branch apagada dos dois lados; `gh issue list --search '[nightly] e2e-smoke is red in:title'` volta
   vazio.

As duas runs de prova usam uma lista de **um spec** porque o smoke real está vermelho hoje por um
motivo alheio a esta frente (`solution-git-status.spec.ts:111`, issue #245). A prova é do reporte, não
da lista.

## Superfície de verificação

A skill `verify-openridr` oferece três superfícies: A (Playwright), B (navegador contra o host isolado) e
C (app Tauri real com `agent-desktop`). **Nenhuma das três se aplica**: esta frente não toca UI, host nem
produto — o diff é `.github/workflows/e2e-smoke.yml`, `scripts/e2e-smoke-issue.sh`, `tests/shell/`,
`AGENTS.md` e `docs/`. A superfície empírica da mudança é a **run do workflow**, e é ela que prova o
aceite (as quatro runs acima). O que sobra de verificação é o teste do script e os linters do CI.

## Verificação local (na integration branch, depois dos merges)

| Comando | Resultado |
|---|---|
| `sh tests/shell/e2e-smoke-issue.test.sh` | `e2e-smoke-issue tests: all cases passed` (0,6 s, sem rede, sem GitHub) |
| `dash tests/shell/e2e-smoke-issue.test.sh` | all cases passed — o `/bin/sh` do runner é dash |
| `shellcheck scripts/e2e-smoke-issue.sh tests/shell/e2e-smoke-issue.test.sh` | clean |
| `actionlint` | exit 0, sem saída |
| `uvx zizmor --offline --min-severity low .` | `No findings to report. Good job! (4 ignored, 3 suppressed)` |

Falha-antes de cada ticket, registrada nos comentários das issues: o teste do #240 falha com
`expected exit N, got 127` quando o script não existe; as asserções de contrato do #241 falham contra o
YAML anterior (14 falhas) e contra cópias do YAML novo com `pull_request`, `cancel-in-progress: true` ou
`issues: write` removido.

## Achados

- **`build.spec.ts` falhou sozinho no runner** (run 42406578543): `a real build reports its raw log,
  fails on a compile error and navigates to it` recebeu `Build failed · exit 1` onde esperava
  `Build succeeded`. O mesmo spec passou na noturna de 10:32, onde roda **depois** de
  `workspace.spec.ts`. Não investigado: é a ordem da lista (o spec depende de a solução já estar
  aberta?) ou flake de restore a frio. Fora do escopo desta frente; issue **#251** (bug).
- **`solution-git-status.spec.ts:111` vermelho na `main`** desde que entrou na lista (revisão r1 da
  #49, marcada como "ainda NÃO medido"): o teste de symlink espera 1 ícone e recebe 2. Fora do escopo
  desta frente; issue **#245**.

## Code review (subagente único, `gpt-6.1-terra`, fixed point `d458e2c`)

Três achados, todos corrigidos:

| Severidade | Achado | Correção |
|---|---|---|
| médio | `gh issue list` usava a página default (30) e o casamento exato do título acontece depois dela: uma página pequena esconderia a issue e o script abriria uma segunda | `--limit 100` com o motivo no código, e o teste pina `--limit`/`100` no caso em que a busca é a única chamada |
| médio | falha do **checkout** deixava o passo vermelho morrer em `No such file`, sem reportar nada | o passo detecta o script ausente e falha com `::error::` dizendo que a issue não foi aberta; o caso entrou em "Out of Scope" da spec com o motivo (a alternativa seria uma segunda cópia do corpo inline) |
| baixo | a asserção de `issues: write` procurava em tudo depois de `jobs:`, então passaria com a permissão em outro job ou num comentário | a asserção lê o bloco `permissions:` do job `smoke`; provado com um YAML onde a permissão vira comentário (`FAIL - workflow lacks 'issues: write'`) |

Depois das correções: `sh` e `dash` do teste verdes, `shellcheck` clean, `actionlint` ok, `zizmor` sem
achados.

## Não verificado

- **A noturna de verdade** (`schedule` às 03:30 UTC) com este workflow: as runs de prova são
  `workflow_dispatch`. A primeira noite depois do merge é a primeira execução agendada, e ela deve
  abrir a issue de rastreio real por causa de #245.
- **Timeout de 60 min e cancelamento do job**: nenhum dos dois reporta (limite aceito, `shortcut:` no
  script e na spec).
- **`shellcheck` no job `governance`**: o `actionlint` do CI liga o shellcheck por padrão; o workflow
  não mudou o que ele lê, mas a execução no runner não foi observada.
- **Windows e macOS Intel.**
