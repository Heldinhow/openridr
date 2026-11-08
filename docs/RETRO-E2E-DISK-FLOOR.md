# RETRO — o E2E recusa a rodada abaixo do piso duro de disco (issue #270, ticket #287)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-E2E-DISK-FLOOR](SPEC-E2E-DISK-FLOOR.md).
Evidência: [evidence/e2e-disk-floor.md](evidence/e2e-disk-floor.md). Origem: candidato 1 do retro
[RETRO-DOCTOR-DISK-SPACE](RETRO-DOCTOR-DISK-SPACE.md) — o doctor avisa, mas nada obriga a sessão a ler.

## O que se sustentou

- **A sonda antes da spec mudou o desenho.** A issue pedia a medição no `globalSetup`; medir o Playwright
  1.65 mostrou que o `webServer` é um plugin e sobe **antes** do `globalSetup` (probe de 10 linhas com
  `webServer` + `globalSetup`: `server already up = true`). O guarda foi para o load do
  `playwright.config.ts`, que é o único ponto anterior a qualquer servidor, e o AC "aborta antes de subir
  qualquer servidor" passou a ser verdade. Sem a sonda, o critério teria nascido falso e ninguém notaria.
- **O precedente da #258 evitou reabrir a decisão.** "Falha no load, de propósito, não no `globalSetup`"
  já estava escrito em `SPEC-E2E-RESTRICTED-PORTS`; o grill só precisou confirmar que a classe era a
  mesma. Custo: uma pergunta, não uma rodada.
- **O seam já existia.** O `df` resolvido pelo `PATH` deixou a recusa ser provada sem encher disco e sem
  variável só-de-teste. O módulo novo tem 63 linhas e nenhum caminho de configuração novo.
- **A prova de "antes de qualquer servidor" ficou mais forte que o `--list`.** Uma rodada real (não
  `--list`) com `cargo` e `bun` armadilha no `PATH` — stubs que criariam um sentinela se fossem
  chamados — saiu 1 sem nenhum sentinela. Com o guarda no `globalSetup`, os dois sentinelas existiriam.
- **O code review pegou o bug que os testes do implementer não pegavam.** O parser aceitava qualquer
  segunda linha com uma quarta palavra numérica: `garbage\nfoo bar baz 0` virava `0` e **recusava** a
  rodada (contrariando o fail-open), e 400 dígitos viravam `Infinity` e davam `ok` silencioso. Dois
  testes novos; medido no código velho (`0`, `Infinity`, `refuse`).
- **A falha-antes foi medida duas vezes**, pelo subagente (`0 pass / 1 fail / 1 error` com o módulo
  ausente; `7 pass / 2 fail` sem a chamada no config) e pela sessão no parser antigo.

## O que quebrou / custou

- **A issue afirmava um fato falso sobre uma dependência.** "Antes de subir host e Vite, o `globalSetup`
  mede" era falso em 1.65. Custo: uma rodada de grill, um comentário de correção na issue e o AC #1
  reescrito. A issue não citava a versão do Playwright nem trazia a sonda.
- **O molde do `df` falso foi copiado com o defeito que a #259 já tinha corrigido.** O stub do
  `doctor.test.sh` recusa invocação fora de `df -Pk <diretório>`, mas aceita argumento extra; o stub novo
  herdou isso e o review pegou (`$#` agora é 2). A lição "um dublê de processo deve afirmar a forma da
  chamada" estava escrita e não foi aplicada por inteiro.
- **O par de portas foi repassado ao subagente antes de ser conferido.** A 5417 estava ocupada por outra
  sessão (`fix-issue-264-tree-query-retry`) quando o par 2525/5417 foi atribuído; o subagente rodou
  `environment.spec.ts` nesse par e passou por sorte (o host era o dele). A conferência com `lsof` custou
  200 ms e foi feita **depois**.
- **O `doctor.sh` continua com a mesma fragilidade do parser.** Ele lê `awk 'NR==2 {print $4}'` e um
  `df` que respondesse lixo com número na quarta coluna imprimiria `FAIL disk 0 MiB` — o caso que o
  harness agora recusa. Fora de escopo por decisão (a issue diz para não tocar no doctor), mas é a mesma
  classe.

## Candidatos para o ambiente (por severidade)

1. **`tests/shell/*.test.sh` num job de CI (já é a #269).** Oito testes de shell, incluindo o do doctor,
   rodam à mão. O achado do stub com argumento extra não teria sido pego por eles — foi o review —, mas
   um job `dash` sobre `tests/shell/` transformaria "rodei à mão" em guardrail e teria pego a regressão do
   `df` falso no mesmo PR.
2. **Sonda obrigatória antes de um AC sobre comportamento de dependência.** Uma linha em
   `docs/agents/` (não em `AGENTS.md`, que é `risk:high`): "critério de aceite que afirma ordem de
   execução, formato de saída ou timing de Playwright/Vite/Tauri precisa de sonda medida e da versão
   citada". O custo desta rodada foi uma pergunta de grill; o custo de não ter medido seria um AC falso
   que passa no review.
3. **O par de portas conferido no momento da atribuição, não depois.** Um `scripts/e2e-ports-free.sh
   <ui> <host>` (ou uma linha no prompt do orquestrador) que roda `lsof` nas 9 portas e recusa a
   atribuição de um par ocupado. Barato e elimina a classe "duas sessões no mesmo host".
4. **O `doctor.sh` validar a forma da linha do `df`** como o harness passou a fazer (seis campos,
   `Available` só dígitos, `Capacity` com `%`). Issue própria, pequena, no molde do que o review pediu
   aqui.

## Lições

- **Critério de aceite sobre ordem de execução de uma dependência só vale depois de medido.** A sonda
  custou 10 minutos; o AC errado custaria a credibilidade do check.
- **Fail-open precisa de teste do lado do lixo, não só do lado do "sem número".** `0` e `Infinity` são
  os dois jeitos de um parser inventar um número; o caso "sem número" já estava coberto e os dois não.
- **Copiar um molde de teste copia os buracos dele.** O stub do doctor tinha a mesma falha que a #259
  registrou como corrigida; o review pegou de novo. O molde precisa ser lido como código, não como
  receita.
- **Conferir o recurso compartilhado antes de repassá-lo.** Porta, worktree e home são estado
  compartilhado: o `lsof` antes do handoff, não depois.

## Não verificado

- Disco realmente cheio: a recusa é provada com `df` falso; encher o volume interno não é aceitável
  nesta máquina.
- `df` ausente de verdade no `PATH` de uma sessão real (coberto pela função pura e pelo stub que recusa a
  invocação).
- `OPENRIDR_E2E_HOME`/`OPENRIDR_SCREENSHOT_DIR` noutro volume: por decisão, mede-se sempre o `$TMPDIR`.
- Suíte E2E completa e smoke noturno; app Tauri real; Windows e macOS Intel.
