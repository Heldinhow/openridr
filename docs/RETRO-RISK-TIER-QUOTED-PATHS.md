# RETRO — risk tier lê o caminho cru que o git cita (issue #276)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-RISK-TIER-QUOTED-PATHS](SPEC-RISK-TIER-QUOTED-PATHS.md).
Evidência: [evidence/risk-tier-quoted-paths.md](evidence/risk-tier-quoted-paths.md). Origem: o candidato
nº 1 do [RETRO-PRECOMMIT-PROPORTIONAL](RETRO-PRECOMMIT-PROPORTIONAL.md), que já nomeava o arquivo, a linha
e a correção.

## O que se sustentou

- **A retro anterior fez o trabalho de investigação.** O candidato dizia "`scripts/ci-risk-tier.sh:40` tem
  o mesmo bug de citação; uma flag fecha a classe". A sessão começou com o mecanismo conhecido: medi o
  defeito em 30 s (repositório sintético, script da `main`) e as cinco perguntas do grill foram todas de
  política — `high` ou despir as aspas, `-z` ou a flag, mexer ou não no loop. Uma rodada, respostas na
  primeira tentativa.
- **O seam já existia e não custou nada.** `scripts/ci-risk-tier-selfcheck.sh` roda o script de verdade
  contra repositórios git sintéticos: seis casos novos em ~30 linhas, nenhum seam novo, nenhuma variável
  só-de-teste, nenhum refactor para testar.
- **O implementer subagent entregou no primeiro round**, com a falha-antes medida por ele (script antigo →
  3 de 50 falhando) e sem tocar em docs, `AGENTS.md` ou no workflow. O merge na integration branch foi sem
  conflito.
- **O code review pagou o preço, uma vez.** Achou que `unicode-product` (esperado `medium`) não provava
  nada: `medium` também é o default de caminho sem regra. É exatamente a coincidência que escondeu o bug
  original — onde o tier certo já era `medium` o defeito era invisível —, agora escondida dentro do teste
  que deveria prová-lo. Virou o caso `unicode-product-reason`, e a falha-antes passou de 3 para 4 de 51.
- **A bateria completa saiu barata** porque só shell mudou: `bun run check` 10,9 s, `cargo clippy` 3,5 s,
  `cargo fmt --check` limpo. Nenhum arquivo Rust ou TS tocado.

## O que quebrou / custou

- **Um ticket, e ele é a própria issue.** A convenção manda ticket como issue filha com `Part of #276`;
  com uma fatia só, criar uma filha que repete a mãe seria ruído. O ticket é a #276 (já com os critérios de
  aceite e `ready-for-agent`), e o orquestrador aprovou o número antes. É a primeira frente em que a
  "issue-mãe" e o "ticket" coincidem — vale saber que a convenção não cobre esse caso explicitamente.
- **`grep -c` enganou a mim mesmo.** Para conferir que a worktree do subagente sumiu, contei linhas
  casando `risk-tier` no `git worktree list` e li `2` como falha; o segundo casamento era
  `ci-issue-74-risk-tiers`, outra worktree, com outro nome. Contar linhas que casam um padrão amplo não é
  conferir o alvo — a mesma lição de "medir é conferir o que foi medido" que a retro da #265 registrou.
- **Worktree nova + mudança só de shell paga `bun install`.** O hook de pre-commit classifica `scripts/**`
  como TypeScript (decisão da #265), então o commit dos dois scripts exigiu `node_modules/.bin/tsc` numa
  worktree que nunca rodaria `tsc` por outro motivo. Custo medido: 2 s de `bun install` e ~1 s de `tsc`.
  Não vale mudar a tabela do hook por isso; fica registrado.
- **`git worktree remove` precisou de `--force`** porque o subagente deixou `node_modules` (untracked) na
  worktree. Não é problema, mas o passo de limpeza não é o comando óbvio.

## Candidatos para o ambiente (por severidade)

1. **Caso cujo esperado coincide com o default precisa asserção de motivo.** `medium` é o tier de caminho
   sem regra, então um caso que espera `medium` passa com a regra quebrada. O conserto já está no
   selfcheck (`unicode-product-reason`), mas a regra vale para os outros casos da mesma família
   (`unknown-tsconfig`, `unknown-html`, `fixtures`, `toolchain` esperam `medium` sem conferir o motivo).
   Uma rodada futura pode acrescentar as asserções de motivo que faltam — é o tipo de buraco que só o
   review pega, porque o teste fica verde.
2. **`tests/shell/*.test.sh` continua fora do CI (issue #269).** Esta frente acrescentou casos a um
   autoteste que roda só à mão; o `AGENTS.md` manda rodá-lo antes de empurrar, mas nada obriga. A #269 já
   existe; esta rodada é mais um dado de que a pasta sem job é dívida ativa.
3. **O grill fica barato quando a retro nomeia o arquivo e a linha.** Não é candidato de ferramenta, é de
   hábito: o candidato nº 1 da retro da #265 economizou uma sessão inteira de investigação. Vale continuar
   escrevendo candidatos com `arquivo:linha` e a correção provável.

## Lições

- **Um teste cujo valor esperado coincide com o default não prova nada.** O motivo (stderr) é a asserção;
  o tier sozinho é ambíguo. Mesma forma do bug: o defeito era invisível onde o default coincidia com a
  resposta certa.
- **Reusar a lição escrita transforma investigação em execução.** Citação do git, `--no-renames` e leitura
  por linha já estavam decididos e testados no hook; aqui foi aplicar, não redescobrir.
- **Falhar fechado é uma escolha de política, não de código.** `high` para o caminho que o git cita de
  qualquer jeito custa um mantenedor chamado num PR de nome exótico; `low` custaria um auto-merge em cima
  de um caminho que o classificador não leu. A decisão foi do orquestrador e está na spec.
- **Conferir limpeza é conferir o caminho exato**, não contar linhas que casam um padrão.

## Não verificado

- **O job `risk` num run real do GitHub**: o classificador roda a partir de `origin/<base>`, então a base
  só tem a versão nova depois do merge. A prova desta rodada é o autoteste e o `ci-risk-tier.sh` rodado
  contra `origin/main..HEAD` (que classificou `docs/**` como `low` e `scripts/ci-*` como `high`).
- **`risk:low` de ponta a ponta (auto-merge armado)**: segue sem prova real, como já registrava
  `evidence/risk-tiers.md`; esta frente corrige a entrada do cálculo, não o caminho do auto-merge.
- **`dash` do Ubuntu 24.04**: rodei com o `dash` do Homebrew (0.5.13.5), não com o do runner.
- **Playwright, app Tauri e `verify-openridr`**: nenhuma interface, host ou app mudou — a mudança é um
  script de CI. Nenhuma superfície A/B/C foi exercitada.
- **Windows e macOS Intel**: não verificados, como o resto do repo.
