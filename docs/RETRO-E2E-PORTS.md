# RETRO — E2E com portas por sessão e Playwright como verificação de UI (issue #210, tickets #211–#213)

Rodada de 2026-11-07, macOS arm64. Spec: [SPEC-E2E-PORTS](SPEC-E2E-PORTS.md). Evidência:
[evidence/e2e-ports.md](evidence/e2e-ports.md). Origem: a decisão do Helder de 2026-11-07 sobre a
superfície de verificação de UI, que deixou `AGENTS.md` e o skill `verify-openridr` dizendo o contrário.

## O que se sustentou

- **O grill de fatos antes da spec evitou meio ticket.** O "Done when" pedia portas por variável de
  ambiente; elas já vinham de `OPENRIDR_E2E_UI_PORT`/`OPENRIDR_E2E_HOST_PORT` desde a `5031a5a`. O que
  faltava era o app home fixo, que o texto da issue não nomeava. Uma leitura de `tests/e2e/environment.ts`
  e um `git log` naquele arquivo mudaram o escopo antes de qualquer linha escrita.
- **Chavear o home pela porta da UI foi a decisão certa, e o custo apareceu como previsto.** Duas
  execuções na mesma porta não podem coexistir (o `strictPort` do Vite de teste garante), então a porta
  é a chave natural; o preço é o cache de csharp-ls por porta, que a spec já declarava.
- **A prova foi barata porque o segundo checkout foi preparado antes.** `bun install` (0,4 s) +
  `cargo build -p openridr-host-http` (39 s) no checkout novo, e a medição em si foram 1,1 min por lado,
  com as duas execuções sobrepostas.
- **O slot do E2E funcionou como o processo manda.** Pedido antes de tocar 1420/4317, usado uma vez
  (build + falha-antes + passa-depois), liberado com `lsof` vazio e aviso "E2E livre".
- **A falha-antes com `-g` no título custou 42 s em vez de 2,6 min.** Rodar só os três testes que
  falham prova o "antes" sem pagar o spec inteiro.
- **O subagente de docs entregou a lista das frases removidas**, o que deixou a revisão do contrato
  (`AGENTS.md`) ser uma leitura de diff em vez de uma caça.

## O que quebrou / custou

- **`workspace.spec.ts` estava vermelho na main desde a #177, e ninguém sabia.** O spec está na lista do
  smoke noturno; a #177 trocou o menu de workspace actions de `<details>` para popover montado
  condicionalmente e atualizou os `.locator("summary")`, mas deixou quatro `reveal(close-workspace)`
  apontando para um elemento que só existe com o menu aberto. Como Playwright não roda em PR e o smoke
  noturno não tem quem leia o resultado, o vermelho atravessou duas frentes. Apareceu aqui só porque a
  prova de duas worktrees usou esse spec primeiro.
- **O default de `OPENRIDR_SCREENSHOT_DIR` escreve dentro do repositório.** A primeira prova (spec com
  3 falhas) regravou `docs/evidence/screenshots/07-conflict-compare.png` na árvore de trabalho; o
  checkout descartável de `/tmp` ficou com a mesma sujeira e recusou `git worktree remove` sem `--force`.
  Nada foi commitado, mas custou um `git status` de atenção e um `git checkout --`.
- **A convenção de portas nasceu errada: par, não faixa.** Os specs que sobem host próprio usam
  `hostPort + 1` até `hostPort + 7`; os dois pares da evidência (4438 e 4439) só não colidiram porque
  `build.spec.ts` não sobe host auxiliar. Quem leu a doc e escolhesse pares vizinhos teria a primeira
  sessão tentando a porta principal da segunda. Achado do code review, corrigido no mesmo PR.
- **`git diff <commit>` inclui a árvore de trabalho.** Um `git diff HEAD~1 --stat` depois do merge fez
  o PNG sujo parecer parte do commit do subagente; o commit de fato tinha seis arquivos de texto.
  `git show <commit>` ou `git diff <commit>..HEAD` não têm essa armadilha.

## Lições

- **Antes de documentar uma convenção, grepe todos os consumidores do valor.** `grep -rn 'hostPort +'`
  teria mostrado os sete offsets antes de a doc afirmar que o par era a unidade de isolamento.
- **Spec na lista do smoke não é spec verde.** A lista da #67 foi medida uma vez, em 2026-11-06; um spec
  que entra nela e quebra depois continua listado. Smoke sem leitor não é guardrail — é registro.
- **`OPENRIDR_SCREENSHOT_DIR` é obrigatório em spec que pode falhar.** Rodar `build.spec.ts` ou
  `workspace.spec.ts` sem ele reescreve evidência versionada na árvore de trabalho.
- **Um spec que sobe host próprio deveria provar que o host é dele.** O polling de `/health` aceita
  qualquer processo naquela porta; com dois pares próximos, a sessão usa o host da outra sem avisar.
- **`git diff <commit>` não é `git diff <commit>..HEAD`.** Para revisar o que um commit mudou, `git show`.

## Follow-ups

- **O smoke noturno (#56) precisa de leitor.** `workspace.spec.ts` ficou vermelho por duas frentes sem
  ninguém notar; o workflow `e2e-smoke.yml` roda e falha em silêncio. Candidato: o job abrir/atualizar
  uma issue de rastreio quando o smoke falhar (mudança de CI, `risk:high`).
- **Host auxiliar sem identidade.** Os sete specs que sobem host próprio em `hostPort + N` aceitam
  qualquer processo que responda `/health` naquela porta. Candidato: subir com `--port 0` e ler a porta
  impressa pelo binário, ou comparar o `pid` do `/health` com o processo filho.
- **`OPENRIDR_SCREENSHOT_DIR` com default dentro do repo.** Candidato: default em diretório temporário,
  e o caminho versionado só quando a variável for setada de propósito.
- **Não verificado:** suíte E2E completa (63 specs), app Tauri real (nada aqui é superfície nativa),
  Windows, macOS Intel, e o disparo do workflow noturno.
