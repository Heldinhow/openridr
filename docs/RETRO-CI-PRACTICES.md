# RETRO — Práticas de CI do T3 Code e do OpenChamber (issue #142, tickets #218–#221)

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

1. **Ligar o `vite build` ao job TypeScript** (alto, mecânico). O check já existe no `package.json` e
   está desligado no CI — é exatamente o caso "guardrail que existe e ninguém roda". Um passo
   `bunx vite build` depois do `tsc --noEmit` custa ~10–20 s e pega erro que o `tsc` não vê. Não entra
   neste PR (que é `risk:high` e já passou por revisão): vira issue filha de #142.
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
- `vite build` continua fora de qualquer workflow — residual declarado na spec, candidato 1 acima.
