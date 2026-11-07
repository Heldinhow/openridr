# RETRO — Glifo único na linha de symlink (issue #245, tickets #250 e #254)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-SYMLINK-ROW-GLYPH](SPEC-SYMLINK-ROW-GLYPH.md).
Evidência: [evidence/symlink-row-glyph.md](evidence/symlink-row-glyph.md). Retro sobre **o ambiente de
trabalho do agente**, não sobre o produto.

## O que se sustentou

- **Reproduzir local antes de perguntar transformou "é o runner?" em três linhas de conserto.** A
  issue dizia "descubra por quê"; uma rodada do spec nas portas da sessão deu o mesmo
  `Expected: 1, Received: 2` do run noturno em 1,7 min, e o grill já foi com fato, não com hipótese.
- **O assert do #49 estava certo; o produto tinha derivado.** Contar `.icon-link` era o desenho
  aprovado na spec da frente, então o conserto foi no `TreeRow` e a asserção não foi enfraquecida.
  Quando o teste e o código discordam, vale perguntar qual dos dois a spec escrita escolheu.
- **O grill com recomendação por pergunta fechou em uma rodada** (Q1–Q5) e a segunda rodada foi uma
  pergunta só (Q6), com números: taxa medida, mecanismo provado, custo de cada opção.
- **Um fato medido evitou um teste errado.** `git status` **degrada em silêncio** quando não consegue
  o lock (exit 0 com `.git/index.lock` presente, índice e `.git` read-only), então um teste de "status
  falha com o lock preso" seria impossível. O teste do host foi para o env do filho, com um `git` de
  mentira no `PATH`, e o subagente mediu o vermelho (`["", "", "", "", "0"]`) antes do verde.
- **"Não verificado" foi usado de verdade:** sem app real (UI pura), sem Windows/Intel, e
  `spawn_bounded`/`run_with_deadline` com o env por inspeção, não por teste.

## O que quebrou / custou

- **O primeiro `workflow_dispatch` foi cancelado na fila por outra sessão.** O grupo `e2e-smoke` tem
  `cancel-in-progress: false`, o que substitui o **pendente** mais antigo quando outro disparo chega:
  o meu run sumiu sem job nenhum e eu só descobri olhando a lista. Custo: um ciclo de fila perdido.
- **Dois dos três disparos ficaram vermelhos por specs de outro terreno.** `workspace.spec.ts:326`
  corre contra o poll de 1,5 s do `checkDisk` (se o poll atualiza o conflito antes do clique, não nasce
  o segundo banner) e `build.spec.ts:62` roda `dotnet build --no-restore` e sai 1 no runner logo
  depois de um restore que o próprio teste afirma ter dado certo. **Nenhum dos dois é diagnosticável
  de fora**: os artefatos do Playwright (`test-results/`) não são publicados e o log do MSBuild só
  existe no DOM da aba Build. Sobrou reproduzir local (28/28 verde) e relatar.
- **`gh run view --log` num run de 6 min é caro** e eu repeti isso várias vezes; `--log-failed` existe
  e teria cortado quase tudo.
- **A taxa de flake só apareceu por acidente.** O `--repeat-each` não está em doc nenhum, e a regra de
  entrada do smoke ("duas rodadas verdes seguidas") não pega corrida: as duas rodadas podem passar e a
  terceira falhar. Foi assim que `git.ignored` e `workspace.symlinks` apareceram (1 em 42 execuções).
- **O primeiro turno do implementer subagent morreu com "Provider turn failed"**, sem detalhe, depois
  de duas chamadas. O retry no mesmo alvo funcionou; custou um worktree a mais para limpar.

## Candidatos de melhoria do ambiente, por severidade

1. **O run vermelho do smoke precisa deixar o artefato do Playwright** (alto, acesso à informação).
   Um `actions/upload-artifact` com `test-results/` quando o job falha transforma "vermelho
   indiagnosticável" em trace e screenshot baixáveis — é o que faltou para dizer o que o MSBuild
   reclamou no runner. Caminho de `risk:high` (`.github/workflows/**`): issue filha, não este PR.
2. **A regra de entrada do smoke deve ser `--repeat-each=8`, não "duas rodadas verdes"** (alto, check
   automatizado). Duas rodadas sequenciais não pegam corrida; oito repetições pegaram as duas que
   apareceram nesta sessão. Uma linha em `docs/evidence/e2e-quarantine.md`.
3. **O `AGENTS.md` deveria dizer que um dispatch pendente é cancelado por outro dispatch** (médio,
   navegação). Fila do grupo `e2e-smoke`, `cancel-in-progress: false`, e o efeito real: conferir a fila
   antes e depois de disparar. Hoje isso só se descobre perdendo um run.
4. **`gh run view <id> --log-failed` no lugar de `--log`** (médio, economia de ferramenta). O log
   inteiro de um smoke de 6 min são dezenas de milhares de linhas de `POST /call`.
5. **A tabela de quarentena deveria carregar a receita de medição** (baixo, navegação): portas, um
   host por execução, `--repeat-each=N`, e o comando exato. Sem isso cada relato de flake mede uma
   coisa diferente.
