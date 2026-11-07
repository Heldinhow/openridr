# RETRO — Diretório neutro da ferramenta em um lugar só (issue #93)

Rodada de 2026-11-07, `ffb7de0` (+ `d566487` da spec/docs e o commit da revisão), macOS arm64. Spec: [SPEC-STAGED-TOOLS-NEUTRAL-DIR](SPEC-STAGED-TOOLS-NEUTRAL-DIR.md). Evidência: [evidence/staged-tools-neutral-dir.md](evidence/staged-tools-neutral-dir.md).

## O que se sustentou

- **A reprodução da issue bastou como prova.** `TMPDIR=$(mktemp -d) cargo test -p openridr-host --lib tools::tests::staged_verification` reprova com `Could not run .../csharp-ls --version.` e passa com a correção. Nada de harness novo, nada de cenário inventado.
- **O grill pegou que o bug já estava corrigido na `main`.** O `#90` — o mesmo PR que descobriu o bug — já tinha adicionado o `create_dir_all` em `verify_installed`. Sem checar a `main` antes de implementar, a frente teria reescrito o que existia e deixado o critério 2 (passo compartilhado) em aberto, que era o único de fato pendente.
- **A dedupe é pequena e o diff diz o que faz:** 22 linhas somadas, 12 removidas, três `create_dir_all` apagados. O passo existe em um lugar só, e o teste novo prova a criação sem depender do estado da máquina.
- **O teste determinístico resolveu o que o teste da issue não resolvia.** `staged_verification_...` só reprova em runner limpo — foi exatamente o que escondeu o bug até o CI. `the_neutral_cwd_is_created_before_the_launcher_runs` parte de um `tempdir` onde o diretório não existe e reprova em qualquer máquina se a criação sair da função.
- **A falha-antes foi medida dos dois lados:** o teste novo e a reprodução da issue, ambos com o `create_dir_all` removido, reprovando com o texto exato da issue.

## O que quebrou / custou

- **A issue descrevia o sintoma, não o que faltava.** O texto de "Fix" ("criar o diretório antes do spawn") já estava feito no `#90`; o critério de aceite 2 ("sem segundo lugar para esquecer") era o trabalho real. Um grill que só lesse o "Fix" teria produzido um no-op.
- **A bateria completa não fecha verde nesta máquina, e não é o diff.** Os três testes de install real baixam o `csharp-ls.0.28.0.nupkg` (21,6 MB) com prazo de 120 s em `prepare_local_package`; nesta rede o mesmo download levou **151,19 s** medidos com `curl`. O teste falha igual com o `tools.rs` revertido para o `origin/main` (120,34 s) e passa quando o download termina em **108,79 s**. Registrado em [`test-stability.md`](evidence/test-stability.md) como ambiente/rede, não como passe.
- **A primeira versão criava o diretório no plano.** Colocar a criação dentro de `neutral_cwd()` foi a solução mais curta, mas `tools.plan` chama essa função: uma chamada de leitura passou a criar `$TMPDIR/openridr-neutral` antes de qualquer confirmação, contra o que a própria #153 declarava. A revisão independente pegou (achado médio); a correção separa `neutral_cwd()` (só o caminho, para o plano) de `neutral_cwd_ready()` (cria, para os três spawns).
- **Worktree nova morre no hook de pre-commit.** Sem `node_modules`, `bun run check` falha em `tsc: command not found` e o commit não passa; `bun install` resolve, mas é um passo que se esquece.

## Revisão independente (rodada 1) — um achado médio e dois baixos

- **Médio, corrigido: o plano criava o diretório.** Ver acima. O passo de criação continua em um lugar só; o que mudou é que ele não roda numa chamada de leitura.
- **Baixo, corrigido: a evidência citava um `grep` estreito** (`create_dir_all(&cwd)`), que não pegaria uma duplicata escrita com outro nome. A linha passou a citar a busca ampla por `create_dir_all` e a inspeção das ocorrências.
- **Baixo, resolvido: o link do retro no índice** apontava para um arquivo que ainda não existia; este retro é criado antes do PR.
- **A revisão também registrou o que a evidência já dizia:** suíte completa verde, inclusive em runner limpo, continua sem comprovação nesta máquina.

## Lições

- **Antes de implementar uma issue de bug, medir a reprodução contra a `main`.** Se ela já passa, o trabalho é o critério que falta — e a issue pode estar desatualizada em relação ao PR que a originou.
- **Função que nomeia e cria muda o significado dos chamadores de leitura.** `neutral_cwd()` era "o caminho"; passou a ser "o caminho, criado" e isso vazou para o plano. Quando o efeito colateral importa, ele mora numa função com nome próprio (`neutral_cwd_ready()`), não na que só calcula.
- **Teste que depende de rede precisa de um limite declarado.** O prazo de 120 s do download é menor que o tempo real de uma conexão lenta; a falha resultante parece defeito de código. Reverter o arquivo e medir de novo é o que separa as duas coisas.
- **Um teste que só reprova em runner limpo não é guarda local.** O teste da issue continua sendo a reprodução fiel, mas a guarda que roda em qualquer máquina é a que injeta a raiz.

## Follow-ups

- **O prazo de 120 s do download da ferramenta é apertado para rede lenta** (21,6 MB; medido 151 s aqui). Hoje o usuário nessa rede vê o install falhar com "Could not download csharp-ls". Fora do escopo da #93; candidato a issue própria (aumentar o prazo, tornar configurável, ou reportar progresso com retry).
- Não verificado: app Tauri real (a mudança é de host, sem superfície de UI), Playwright, Windows e macOS Intel; e o caso `TMPDIR` não gravável, onde a mensagem específica do worker de install deixou de existir.
