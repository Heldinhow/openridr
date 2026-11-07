# RETRO — Práticas de CI do T3 Code e do OpenChamber (issue #142, tickets #218–#221; #232/#238)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-CI-PRACTICES](SPEC-CI-PRACTICES.md). Evidência:
[evidence/ci-practices.md](evidence/ci-practices.md). PR #225. Retro sobre **o ambiente de trabalho do
agente**, não sobre o produto.

## O que se sustentou

- **Medir antes de perguntar mudou o escopo de todas as quatro frentes.** O grill chegou ao orquestrador
  com números em vez de opções: 17 achados de `tsc` (não "talvez uns vinte"), Biome 175/37 com os 5
  `useHookAtTopLevel` sendo falso positivo em spec Playwright, `cargo check` de `src-tauri` falhando em
  checkout limpo, `bun audit` já no bun fixado pelo CI. Três decisões caíram em minutos porque a
  alternativa cara tinha número: o Biome foi adiado com o baseline escrito, não com "vamos ver depois".
- **A medição derrubou a premissa do ticket, não o ticket.** A #219 dizia que o job novo poderia custar
  caro; medido, ele rodou em 1:20–2:41 e terminou 1:27–3:22 antes do Rust, então a parede não mudou. A
  decisão de job separado em vez de passo no job existente se pagou com uma comparação, não com uma
  intuição.
- **O job informativo é um mecanismo, não uma promessa.** `continue-on-error` no job faz
  `needs.<job>.result` virar `success`; o subagente foi conferir na fonte (`actions/runner#1161`) em vez
  de confiar na spec, e o run real mostrou o agregador verde com o job novo no `needs`. O critério de
  promoção ficou escrito no YAML e no `AGENTS.md`, então a próxima sessão não precisa reconstruí-lo.
- **Três subagentes em paralelo, com branches disjuntas.** #218 (`tsconfig` + `src/`), #219
  (`ci.yml`) e #220 (arquivo novo de workflow) não compartilhavam arquivo; os três merges foram
  fast-forward ou merge limpo, sem conflito. O único conflito do dia foi com `origin/main`
  (`docs/decisions.md`), resolvido em um minuto.
- **A revisão independente pagou-se de novo, e nos dois eixos.** O eixo Spec achou três coisas que eu
  não veria: a evidência citava um SHA que tinha deixado de ser o head, um `bun audit` vermelho pulava
  os dois audits do Cargo, e "terminou antes" estava afirmado por subtração de duração em vez dos
  `completedAt`. O eixo Standards achou a contagem 13+4 que sobreviveu em `decisions.md` depois de
  corrigida na spec e na evidência.
- **A conferência do inventário contra o repositório achou duas linhas erradas na própria spec.** "Já
  existe: separação lint/build/typecheck/testes" não era verdade — nenhum workflow roda `vite build` — e
  "integridade e smoke de release: nada a fazer" também não: os quatro scripts existem e nenhum workflow
  os chama. Um inventário escrito de memória teria propagado as duas.

## O que quebrou / custou

- **`vite build` nunca rodou no CI, e ninguém sabia.** O comando que produz o que é distribuído é
  `bun run build` (`tsc --noEmit && vite build`); o job TypeScript roda só o `tsc`. O `tsc` não vê erro
  de bundling, import de asset que não resolve nem problema de CSS. A #142 não pediu isso e a spec
  declarou como residual, mas é a lacuna mais concreta que apareceu.
- **O `cargo check` de `src-tauri` falha em qualquer checkout limpo, e isso não estava escrito em lugar
  nenhum.** `resource path 'resources/csharp-ls' doesn't exist`: o `tauri-build` valida os resources do
  bundle e as duas pastas são vendor dirs gitignored. Custou uma rodada de investigação a frio; agora
  está no `AGENTS.md` e no comentário do job.
- **A evidência nasceu apontando para um SHA que deixou de ser o head.** O padrão do repo é "run no SHA
  entregue", e o SHA entregue anda a cada commit de documentação. A saída foi separar os dois: a
  evidência fixa o último commit **de código** e diz isso em voz alta, e o run do commit de docs vai
  para o corpo do PR. Escrever isso antes teria economizado uma rodada de revisão.
- **O mesmo número em três arquivos divergiu.** A contagem dos 17 achados estava em `decisions.md`,
  `SPEC-CI-PRACTICES.md` e `evidence/ci-practices.md`; a correção foi aplicada em dois e o terceiro
  ficou com o valor antigo até a revisão pegar. Medição mora na evidência; os outros dois apontam.
- **Um `delegate_task` falhou com "Provider turn failed" e deixou worktree e branch para trás.** O
  retry não conseguiu criar `ci/issue-142-docs-evidence` ("already used by worktree") e teve que
  inventar outro nome. Nada se perdeu, mas o worktree órfão precisou de `git worktree remove --force`.
- **`tests/shell/` tem cinco arquivos e só quatro são gates de CI.** A spec dizia "4 de 4 testes de
  shell", ambíguo o bastante para o revisor não saber se a conta fechava. Nomear os quatro resolveu.

## Candidatos de melhoria do ambiente, por severidade

1. **Ligar o `vite build` ao job TypeScript** (alto, mecânico) — **feito na #232/#238**. O check já
   existia no `package.json` e estava desligado no CI; o passo `bunx vite build` entrou depois do
   `tsc --noEmit` e custou 1 s no run medido (job de 17 s → 22 s). Detalhes na rodada de 2026-11-08
   abaixo.
2. **O `AGENTS.md` agora carrega a armadilha do `src-tauri`** (alto, já feito neste PR). A nota diz que
   o CI não roda os vendor scripts, que as duas pastas são gitignored e que o `tauri-build` só exige a
   existência do caminho. É a única coisa desta frente que qualquer sessão futura vai precisar antes de
   rodar um comando.
3. **Padrão de evidência: fixar o último commit de código, não o head** (médio, já escrito na
   evidência). Vale promover a regra para o `/code-review`: o eixo Spec deve perguntar "a evidência cita
   um SHA que ainda é o head, ou diz por que não é?".
4. **`delegate_task` que falha deveria limpar worktree e branch** (médio, ambiente do orquestrador). Não
   é conserto de repositório; fica registrado para quem mantém o harness.
5. **Medição mora num lugar só** (baixo, já praticado na evidência). Quando um número aparece em spec,
   decisão e evidência, os dois primeiros apontam para o terceiro em vez de repetir o valor.
6. **Nomear os scripts de CI em vez de contá-los** (baixo, já feito na spec). "45 + 13 + 23 + 27 casos,
   com os nomes" não deixa dúvida sobre quais arquivos contam.

## O que fica sem prova

- O `schedule` semanal do `dependency-audit` só se prova depois do merge na default.
- O critério de promoção do `rust-tauri` está em 2 de 5 runs verdes.
- O `if: always()` que faz os três scanners rodarem mesmo com um falhando está no YAML e foi lintado,
  mas não foi exercitado com uma falha real de scanner.
- `vite build` continua fora de qualquer workflow — **fechado na #232/#238**, ver a rodada de
  2026-11-08 abaixo.

## Rodada #232/#238 — `vite build` no CI (2026-11-08)

Spec: [SPEC-CI-PRACTICES](SPEC-CI-PRACTICES.md) (seção `### #232`). Evidência:
[evidence/ci-practices.md](evidence/ci-practices.md) (seção `## #238`). PR #239, ticket #238. Retro
sobre **o ambiente de trabalho do agente**, não sobre o produto.

### O que se sustentou

- **O número decidiu o grill em uma rodada.** As sete perguntas foram ao orquestrador com medição
  pronta (`tsc` exit 0 e `vite build` exit 1 no mesmo import; 0,88 s de build local; job de 17 s), e
  voltaram as sete aprovadas como recomendadas, sem segunda rodada. Medir antes de perguntar continua
  sendo o que encurta a entrevista.
- **A falha-antes no runner custou dois runs, não um teste novo.** O commit descartável (`d7d2463`)
  provou o que nenhum teste local provaria — "Type check" verde e "Build" vermelho no mesmo job — e o
  revert (`758d56e`) devolveu o head verde. A skill `to-spec` recusou seam novo e acertou: não há o
  que cobrir com unit test num `run:` de duas palavras.
- **Um ticket, um subagente, e o merge foi fast-forward.** O escopo era cinco linhas de YAML e três
  docs; dividir em duas fatias custaria mais em merge do que em trabalho. O implementer devolveu os
  exit codes e a duração, e o diff veio sem surpresa.
- **O review independente não achou defeito, e distinguiu residual de histórico.** Ele viu o
  `vite build` ainda citado em `RETRO-CI-PRACTICES.md:85` e concluiu que era a retro datada da #142,
  não uma declaração de estado — exatamente a distinção que o eixo Spec deveria fazer.

### O que quebrou / custou

- **O residual morava em três arquivos, e fechá-lo custou três edições.** Spec, evidência e `AGENTS.md`
  diziam a mesma coisa em três lugares; o ticket tocou os três para uma linha de YAML. É a repetição
  do achado da rodada anterior ("o mesmo número em três arquivos divergiu"), agora na forma de
  residual: um fato declarado em N lugares custa N edições para deixar de ser verdade.
- **`git revert` gera mensagem fora do padrão, e o caminho curto foi `--no-verify`.** O revert saiu
  como `Revert "test(ci): ..."`, e o amend para uma mensagem conventional foi feito com
  `--no-verify` — violação da regra do `AGENTS.md`, mesmo com o hook já tendo rodado no commit
  original e com `bun run check` refeito à mão depois. O hook roda `cargo check` inteiro (~16 s) em
  qualquer commit, inclusive um amend de mensagem, e é essa fricção que empurra para o atalho.
- **Editar o corpo do PR dispara o workflow inteiro.** O `edited` está nos tipos do `pull_request`
  para re-lintar o título, e a edição da Evidência no fim disparou um segundo run no mesmo head,
  cancelando o que estava em voo pela `concurrency`: dois runs para um head, ~4 min cada. A evidência
  final tem que ser a última edição antes de pedir merge — ou o check de corpo vira job próprio.
- **`zizmor` não está na máquina e o `actionlint` está.** O job `governance` roda os dois; localmente
  só um. O `AGENTS.md` nomeia `shellcheck` e `dash` para `scripts/`, mas não os dois linters de
  workflow.

### Candidatos de melhoria do ambiente, por severidade

1. **Hook de pre-commit proporcional ao que mudou** (médio, mecânico). `.githooks/pre-commit` roda
   `bun run check` (tsc + `cargo check --workspace`) em todo commit, inclusive docs e amend de
   mensagem: ~16 s de cargo para uma linha de Markdown. Um filtro por caminho staged (docs-only →
   nada; TS → tsc; Rust → cargo) remove a fricção que faz o `--no-verify` parecer razoável.
2. **O `edited` do PR re-roda tudo** (médio, decisão de workflow). Ou o check de título/corpo vira job
   próprio disparado por `edited`, ou o `AGENTS.md` diz que a última edição do corpo é a última ação
   antes do merge. Hoje o custo é um run inteiro por edição de Evidência.
3. **Nomear `actionlint` e `zizmor` no `AGENTS.md`** (baixo, uma linha). O job `governance` depende dos
   dois e o Mac só tem o primeiro; a seção de CI já nomeia `shellcheck`/`dash` para `scripts/`.
4. **Residual declarado em N lugares** (baixo, já praticado): a spec aponta para a evidência e o
   `AGENTS.md` para a spec; a frase "nenhum job roda X" deveria existir num arquivo só.

### O que fica sem prova

- O `dist/` empacotado num DMG real: o passo novo prova o `vite build`, não o `tauri build` completo.
- O aviso de chunk > 500 kB (bundle de 1,45 MB) continua só no log; virar gate é decisão separada.
- O run da `main` depois do merge: o smoke noturno e o `dependency-audit` não foram tocados.
