# RETRO — Merge em andamento: concluir ou abortar pela UI (issue #114)

Rodada de 2026-11-07, `8a6ad87`–`1d6935c` (+ merge de `origin/main` em `af6fc82`), macOS arm64. Spec: [SPEC-GIT-MERGE-IN-PROGRESS](SPEC-GIT-MERGE-IN-PROGRESS.md). Evidência: [evidence/git-merge-in-progress.md](evidence/git-merge-in-progress.md).

## O que se sustentou

- **Medir o Git antes de desenhar pagou duas vezes.** O grill levou fatos medidos (`git commit --only` recusado durante merge, `git add -u` limpando unmerged, `git diff --check HEAD` achando marcador, `merge --abort` descartando staged) e o desenho saiu pronto: nenhuma decisão precisou ser refeita na implementação, e o review não achou nada de desenho — só os dois buracos de execução abaixo.
- **Virar a política do Update foi o que tornou a feature alcançável.** O caminho do próprio app (Update → conflito → merge aberto → Complete merge) foi o da prova no app real; sem a virada, a única forma de chegar no estado seria um terminal, e a issue continuaria valendo para o caminho principal.
- **Deletar a posse do abort simplificou o host.** `merge_head` e `abort_conflicting_merge` saíram; o `update_failure` ficou menor e a mensagem passou a apontar o menu. O código do #112 existia para uma situação (abortar o próprio merge) que a #114 eliminou.
- **O fixture de remoto local + `agent-desktop` provou o fluxo inteiro, não só o host.** Update conflitante, recusa por marcador, resolução no editor do app, merge commit de dois parents, Commit bloqueado e abort com modal — cada passo conferido no disco (`MERGE_HEAD`, `rev-parse`, `status --porcelain`, bytes do arquivo).
- **A falha-antes dos três casos da revisão foi medida em um minuto.** Reverter as três correções e rodar `merge_continue` reprovou os três testes novos; restaurar e rodar passou. É o padrão que o repo já usa (`pull_with`/`fetch_with`) aplicado a um limite de saída (`merge_continue_with`).
- **O re-check foi barato e pegou o que a correção deixou.** 3 dos 4 achados fechados; o médio do abort voltou `NOT CLOSED` porque o modal ainda abria com a promessa antiga antes do aviso, e um `LOW` de doc. Corrigir e recapturar custou ~10 min.

## O que quebrou / custou

- **O widget de branches não é alcançável pela árvore de acessibilidade.** O `<summary>` do menu sai como `disclosure` `interactive: false`, sem `ref_id`; abrir o menu exigiu clique físico em coordenadas (bounds de `agent-desktop find` → `mouse-click --xy 446,117 --headed`). Pior: depois de `focus-window`, o primeiro clique físico às vezes só ativa a janela, então o padrão virou "clicar, conferir `Search branches` no snapshot, clicar de novo" — e um clique a mais fechou o menu que eu queria fotografar. Custo: ~10 min de tentativa e erro e uma captura refeita.
- **A primeira captura do Commit tool saiu com o toast do abort anterior por cima.** Toast não expira sozinho na janela; foi preciso `Dismiss notification` e refazer. Captura de estado precisa de estado limpo: fechar toast, fechar menu, e só então fotografar.
- **Erro de concordância só apareceu no app.** "1 file still contain conflict markers" passou nos testes (que checavam a substring "conflict markers") e apareceu na primeira rodada real. Corrigir exigiu reconstruir o bundle e refazer a rodada inteira para a evidência não misturar redações. Lição: mensagem com contagem (1 vs N) se revisa antes da primeira rodada, não depois.
- **A bateria completa ficou vermelha uma vez por carga, não por código.** `cargo test -p openridr-host` teve 4 falhas de LSP/tools (deadlines de 120 s para install de ferramenta, que sozinho leva 85 s) na primeira rodada; os 4 passaram sozinhos, passaram juntos (396 s) e a segunda bateria completa passou (108 no binário de integração). Custo: ~25 min de re-runs para provar que não era meu. Não entrou na tabela de falhas conhecidas porque não reproduziu; fica como observação de estabilidade.
- **Merge de `origin/main` conflitou nos quatro docs append-only.** `docs/README.md`, `decisions.md`, `implementation-notes.md` e `GLOSSARY.md` conflitaram porque os dois lados acrescentaram no fim. Resolver é mecânico (manter os dois lados; a entrada antiga do `Update` no glossário é a única que sai), mas o AGENTS não diz o padrão de resolução — quem nunca viu perde tempo decidindo.
- **O slot do app foi de outra sessão no meio.** A #53 estava com a janela quando fui recapturar; esperei o orquestrador liberar e refiz o build. Nada foi perdido, mas a recaptura ficou em duas etapas.

## Revisão independente — 2 altos e 2 médios, todos reais

- **Alto: a guarda de marcador olhava só o worktree.** `git diff --check HEAD` compara `HEAD` com a árvore; o commit usa o index. Marcador staged e removido da árvore passaria e o merge commit levaria `<<<<<<<`. Agora roda também `--cached HEAD` e une os caminhos.
- **Alto: nome conflitado virava pathspec.** `git add -u -- <caminho>` trata `*`/`?`/`[` como glob; um conflitado `a*.txt` arrastaria `a-other.txt`. Agora `--literal-pathspecs` — o mesmo padrão que `git.ignored` já usava desde a #49, e que eu não reusei de primeira.
- **Médio: o abort prometia restauração que o Git não garante.** `merge --abort` pode não reconstruir mudanças não commitadas anteriores ao merge (merge iniciado fora do app). O modal e o toast passaram a prometer só o que é certo.
- **Médio: saída truncada do check virava "sem marcador".** Agora recusa (teto de 1 MiB, rede de segurança) e manda commitar pelo terminal.
- **Re-check:** o médio do abort voltou `NOT CLOSED` porque o modal ainda tinha a frase antiga antes do aviso, e um `LOW` porque a nota de implementação citava a mensagem antiga. Quando o achado é **texto**, corrigir em todas as superfícies de uma vez (modal, toast, nota, spec) — a que ficou de fora foi justamente a que o re-check pegou.

## Lições

- **"O que vai ser commitado" é o index, não a árvore.** Qualquer guarda antes de um commit tem de olhar o index (`--cached`); `git diff HEAD` só vê a árvore.
- **`--` não torna o caminho literal.** Pathspec é glob por padrão; o repo já tinha a resposta (`--literal-pathspecs`) e ela deve ser o default de todo comando que recebe nome de arquivo do usuário.
- **Validação truncada é validação falhada.** Se a saída foi cortada, não há prova: recusar é mais barato que commitar marcador.
- **Mensagem com contagem se lê em voz alta (1 e N) antes de rodar o app.** O teste de substring não pega concordância.
- **O menu de branches não tem `ref_id`; a receita de clique físico deve ficar escrita.** Bounds de `find` já vêm em coordenadas de tela; o primeiro clique depois de `focus-window` pode só ativar a janela; conferir `Search branches` no snapshot antes de agir economiza a rodada perdida.
- **Captura de estado exige estado limpo.** Toast e menu abertos contaminam a imagem; fechar antes de fotografar.

## Follow-ups

- **Acessibilidade do menu de branches**: o `<summary>` do widget não expõe `ref_id`/ação na árvore de acessibilidade (nem é alcançável por teclado). É o único caminho para Complete/Abort merge hoje; vale um ticket de a11y (papel de botão, foco, `aria-expanded`).
- **Estabilidade da bateria completa**: os 4 testes de LSP/tools têm deadline de 120 s e o install real leva ~85 s sozinho; sob carga paralela estoura. Se repetir, entra na tabela de `docs/evidence/test-stability.md` (a fonte única) com a medição.
- **Padrão de resolução dos docs append-only** no merge de `origin/main`: manter os dois lados e, no glossário, trocar a entrada substituída. Se isso repetir, vale uma linha no AGENTS.
- **Rebase/cherry-pick/revert pela UI** continuam fora (decisão da #112/#114); o host recusa e a nota manda ao terminal.
- Não verificado: Windows, macOS Intel, conflito binário, `conflict-marker-size` customizado, remoto autenticado.
