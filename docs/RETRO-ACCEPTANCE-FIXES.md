# Retro — correções da aceitação do `main` (D1…D6)

Branch `fix/main-acceptance-findings`, de `origin/main` `60e784f`, mesclada com `731c3d8` (PR #28) antes da
verificação final. Evidência: [evidence/acceptance-fixes.md](evidence/acceptance-fixes.md). Data: 2026-11-05.

## O que funcionou

- **Reproduzir no app instalado antes de tocar no código.** D1, D3 e D5 foram reproduzidos no `60e784f`
  (arquivo exportado com `run.revision 0`, `TFM Not selected`, duas barras) e isso virou o "antes" da prova. O
  mesmo vale para o teste de aceitação: um relatório com capturas poupa horas.
- **Seguir o dado até o dono.** D1 e D3 tinham a mesma forma: dois leitores do mesmo estado (export × snapshot;
  descoberta × contexto avaliado). Um ponto de composição e um resolvedor de TFM fecharam os dois sem duplicar
  regra na UI.
- **Desconfiar do diagnóstico antes de "consertar".** D4 e D6 não reproduziram como escritos: o menu da folha já
  era do projeto dono e os textos já estavam na árvore AX. Ler a captura 25 e rodar `agent-desktop find` no app
  instalado evitou duas correções inventadas; sobraram dois defeitos reais menores (seleção não seguia a linha;
  landmarks sem nome).
- **Teste que falha antes.** Para D1 e D3 revertí o código de propósito e vi o teste novo falhar; para D2, o
  teste unitário cobre a soma dupla e o roteiro no app mostra 19→100 % de 21.643.814 bytes.

## O que custou tempo

- **Um `npm`-style E2E que morria sem dizer por quê.** A primeira versão do spec usava `settleRestore`, que
  depende do drawer de ferramentas aberto (`restore-state`), e depois caiu num `evaluation_stale` porque a
  descoberta automática dispara antes da reavaliação pós-restore terminar. Ler o trace (`resources/*.json` do
  `trace.zip`) achou a resposta `invalid_state → evaluation_stale` do host. O helper do spec passou a esperar
  restore **e** avaliação atuais pelo host.
- **Falta de um teste de fronteira para o progresso.** O download real usa uma URL fixa, então não há seam para
  exercitar "progresso medido" no `cargo test`. Ficou coberto por unit (`observed_bytes`,
  `observes_disk_bytes`) e pelo roteiro no app; um seam de download seria o próximo passo.
- **Disco.** O `tauri build --debug` da branch comeu ~2 GB (7,4 GB livres ao fim). O build de release não foi
  tentado: a máquina não tem folga e a aceitação original já falhou por `ENOSPC`.

## Dívida / próximo

- `testsuite.spec.ts:51` espera `tests-filter-count`, `data-testid` que não existe em `src/` desde antes desta
  branch. Ou o painel volta a mostrar a contagem do filtro (o design atual só filtra a árvore) ou o spec é
  corrigido. Hoje é uma falha silenciosa da suíte.
- Descoberta automática × reavaliação pós-restore: o painel dispara a descoberta num modelo que ainda vai ser
  marcado stale e recebe `evaluation_stale`. Não quebra o fluxo (a próxima tentativa funciona), mas gera um
  erro visível em corrida. Candidato a esperar `evaluation.state === "ok" && !stale` antes de disparar.
- `aria-live` no painel de detalhe não foi observado com leitor de tela real; a região nomeada aparece na árvore
  AX do WKWebView, o anúncio não foi medido.
