# SPEC — Merge em andamento: concluir ou abortar pela UI (issue #114)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#114](https://github.com/Heldinhow/openridr/issues/114) (frente #50, aceitação de Git com remoto).

## Problema

Quando um merge está em andamento na árvore de trabalho, o app explica o estado e recusa as escritas que perderiam trabalho — mas não oferece como terminá-lo nem como desfazê-lo. O menu de branches lê `A merge is in progress; finish or abort it before checking out or updating.`, e as entradas de branch, o `Update…` e os checkboxes do Commit ficam desabilitados. Resolver o conflito editando o arquivo e commitando também está bloqueado: o arquivo conflitado não pode ser selecionado, e mesmo um arquivo não-conflitado do merge falha com o erro cru do Git (`fatal: cannot do a partial commit during a merge`), porque o Git proíbe commit parcial durante um merge. O resultado é um beco sem saída que exige `git merge --abort` num terminal — foi assim que a aceitação da #50 se recuperou.

O próprio app não consegue mais produzir esse estado: o **Update** conflitante aborta o merge que ele mesmo começou (#112). Esse abort foi registrado como **provisório até a #114** justamente porque um merge aberto, sem como concluir, seria o mesmo beco um passo depois.

## Solução

O menu de branches ganha, quando há merge em andamento, duas ações: **Complete merge** e **Abort merge…**.

- **Complete merge** termina o merge que a pessoa resolveu no editor: o host marca como resolvidos os caminhos que estavam conflitados (`git add -u -- <caminhos>`), recusa se algum ainda contém marcador de conflito (`git diff --check HEAD` → `leftover conflict marker`), e cria o merge commit com a mensagem que o Git já preparou (`git commit --no-edit`). Nada é empurrado.
- **Abort merge…** desfaz o merge (`git merge --abort`), atrás de uma confirmação que diz o que volta e o que é descartado: a branch, o index e a árvore voltam ao estado anterior ao merge; resoluções em andamento e o que foi staged durante o merge são descartados; os commits ficam.

Com as duas ações existindo, o **Update** conflitante deixa de abortar o merge: ele fica em andamento, e a mensagem diz exatamente o que fazer (`resolve the conflicted files in the editor, then finish or abort it from the branch menu`). É o ponteiro para a recuperação exata que a issue pede — e o caminho do próprio app passa a chegar nas ações novas.

O Commit tool deixa de ser um segundo beco: durante qualquer operação em andamento ele desabilita **Commit** com o motivo em palavras do app (commit parcial é impossível durante um merge), e a nota do arquivo conflitado aponta o editor e o menu de branches. Para rebase, cherry-pick e revert, que continuam fora do escopo, a nota do menu de branches passa a dizer que o app só conclui/aborta merge e que esses casos pedem terminal.

## User stories

1. Como pessoa que usa o painel Git, quero **terminar um merge** pela UI depois de resolver os conflitos no editor, para não precisar abrir terminal.
2. Como pessoa que usa o painel Git, quero que **concluir o merge** use a mensagem que o Git já preparou, para não inventar uma mensagem de merge a cada vez.
3. Como pessoa que usa o painel Git, quero que **concluir** marque os arquivos resolvidos como parte do merge, para não precisar descobrir que o app não tem stage.
4. Como pessoa que usa o painel Git, quero que o app **recuse concluir** enquanto um arquivo ainda tem marcador de conflito, para não commitar `<<<<<<<` por um clique.
5. Como pessoa que usa o painel Git, quero ver **quais arquivos** ainda têm marcador quando a conclusão é recusada, para saber o que falta resolver.
6. Como pessoa que usa o painel Git, quero **abortar o merge** pela UI, para voltar ao estado anterior sem terminal.
7. Como pessoa que usa o painel Git, quero que abortar **peça confirmação** e diga o que é descartado, para não perder uma resolução por engano.
8. Como pessoa que usa o painel Git, quero que o abort **preserve meus commits**, para desfazer só o merge.
9. Como pessoa que usa o painel Git, quero que um **Update conflitante deixe o merge aberto** em vez de abortá-lo, para poder resolver no editor o conflito que o próprio app criou.
10. Como pessoa que usa o painel Git, quero que a mensagem do Update conflitante diga **onde** concluir ou abortar, para não voltar ao beco.
11. Como pessoa que usa o painel Git, quero que o diálogo de Update **declare** que o merge fica em andamento quando conflita, para a política não surpreender.
12. Como pessoa que usa o painel Git, quero que o **Commit** fique desabilitado com motivo durante um merge, para não receber `cannot do a partial commit during a merge`.
13. Como pessoa que usa o painel Git, quero que a nota do arquivo conflitado **aponte o editor e o menu de branches**, para saber o caminho de saída.
14. Como pessoa que usa o painel Git, quero que **rebase/cherry-pick/revert** digam que o app não os conclui, para não procurar um botão que não existe.
15. Como pessoa que usa o painel Git, quero que **checkout, Update e criação de branch** continuem recusados durante um merge, para o app não atropelar o merge pela metade.
16. Como pessoa que usa o painel Git, quero que **Fetch** continue funcionando durante um merge, porque ele move só refs.
17. Como pessoa que valida a release, quero **prova no app Tauri real** de um merge conflitado concluído e de outro abortado pela UI, para saber que a correção vale no binário.
18. Como pessoa que valida a release, quero a prova de que o **Update conflitante deixa o merge aberto** e que as ações o terminam, para confiar no caminho completo.
19. Como pessoa que mantém o OpenRidr, quero testes de host com merge real e conflito real, para a correção ser provada por efeito (`MERGE_HEAD`, `rev-list`, bytes do arquivo, `status --porcelain`) e não por mensagem.
20. Como pessoa que mantém o OpenRidr, quero que o que não foi verificado (rebase, cherry-pick, Windows, Intel) apareça como não verificado, para não vender suporte que não existe.

## Implementation decisions

- **Escopo: só merge.** `git.mergeContinue` e `git.mergeAbort` recusam (`invalid_state`) quando a operação em andamento não é merge; rebase, cherry-pick e revert continuam sem ação na UI e a nota do menu de branches passa a dizer que esses casos pedem terminal. Motivo: a #112 deixou rebase explicitamente para uma frente própria, e o app nunca cria os outros três.
- **Duas ações no menu de branches, um ponteiro no Commit tool.** O bloco do merge aparece logo abaixo da nota de upstream, com `Complete merge` e `Abort merge…`; o Commit tool não duplica as ações (ele substitui a Solution Explorer no painel esquerdo, e duplicar exigiria sincronizar dois estados). A nota de cada arquivo conflitado e o motivo do Commit desabilitado apontam o editor e o menu de branches.
- **Concluir stageia só os caminhos conflitados.** O host lê o status, roda `git --literal-pathspecs add -u -- <caminhos que estavam conflitados>` (nada de `git add -u` global, que arrastaria mudanças alheias ao merge; o flag literal impede que um nome como `a*.txt` case com `a-other.txt`) e depois `git commit --no-edit`, que usa a `MERGE_MSG` preparada pelo Git. A mensagem do merge não é editável nesta frente. Quando não há caminho conflitado (tudo já staged, por exemplo), o `add` é pulado.
- **Marcador de conflito recusa a conclusão.** Antes de stagear, o host roda `git diff --check HEAD` e `git diff --check --cached HEAD` (com teto de 1 MiB cada) e, se aparecer alguma linha `leftover conflict marker`, recusa listando até 20 caminhos em palavras do app. Os dois checks são necessários porque o commit usa o index: marcador staged e depois removido do worktree só aparece no segundo, e caminho unmerged só aparece no primeiro. É o próprio critério do Git para marcador (`<<<<<<<`, `=======`, `>>>>>>>`), sem heurística de leitura de arquivo; erros de whitespace do mesmo comando são ignorados. Saída truncada **recusa** a conclusão (não prova ausência de marcador) e manda commitar pelo terminal; com 1 MiB isso é rede de segurança, não caminho comum.
- **Abortar pede confirmação; concluir não.** O abort descarta o merge, as resoluções em andamento e o que foi staged durante o merge (comportamento do `git reset --merge` que o `merge --abort` usa), então tem modal com esse texto; concluir é o caminho feliz e o resultado aparece na mensagem da operação.
- **Commit bloqueado durante qualquer operação em andamento.** `git.commit` recusa com `git_operation_in_progress` e mensagem do app (merge aponta o menu de branches; as outras operações, o terminal), porque o Git proíbe commit parcial durante um merge. A UI desabilita o botão **Commit** com o mesmo motivo e mostra a nota. Vale para rebase/cherry-pick/revert também, que já falhavam com erro cru.
- **Update conflitante deixa o merge aberto.** O `pull` deixa de provar a posse do abort e de rodar `git merge --abort`: depois de um merge que falhou, ele relê o status e, se há merge em andamento, devolve `state: "failed"` com a contagem e os caminhos conflitados mais `The merge is still in progress: resolve the conflicted files in the editor, then finish or abort it from the branch menu.` O diálogo de Update passa a declarar isso no lugar do abort. `merge_head` e `abort_conflicting_merge` saem do host. Merge de outro processo é recusado antes de qualquer comando pelo `require_clean` (como hoje), então não há mais o que "provar".
- **Identidade ausente continua em palavras do app.** A checagem de `user.name`/`user.email` que o Update já tinha é extraída e reusada pela conclusão do merge: sem identidade o commit do merge não tem autor, e a mensagem diz onde configurar.
- **Mensagens prometem só o que é certo.** Concluir: `Merge completed.` Abortar: `Merge aborted; the merge is undone and your commits stay.` — o modal avisa que o estado volta ao de antes do merge, que resoluções e o que foi staged durante o merge são descartados e que mudanças não commitadas anteriores ao merge podem não voltar (o `merge --abort` não garante reconstruí-las). O stderr cru continua em `Operation.output`, com o mesmo cap e mascaramento de URL.
- **Sem dependência nova, sem comando novo de Git.** Tudo é Git de linha de comando pelo host (`add -u`, `diff --check`, `ls-files -u`, `commit --no-edit`, `merge --abort`); nada de libgit2.
- **Sem atalho, sem Find Action, sem push automático.** O menu de branches é a superfície; o merge não empurra nada.

## Testing decisions

- **Um teste bom aqui prova o efeito no repositório, não o texto na tela.** Cada caso confere `MERGE_HEAD`, `rev-parse`/`rev-list --parents`, `status --porcelain`, bytes do arquivo e `ls-files -u`, além de `state`/`message` da operação.
- **Seam: as funções de operação do host, com merge real.** Os testes montam um conflito de verdade (dois lados alterando o mesmo arquivo) e chamam `merge_continue`/`merge_abort`; prior art: `update_aborts_a_conflicting_merge_and_keeps_both_commits` e os helpers `remote_repo`/`peer_clone`/`publish`.
- **Casos mínimos**: (1) concluir um merge com a resolução escrita no arquivo cria commit de dois parents, apaga `MERGE_HEAD`, deixa a árvore limpa e o conteúdo resolvido; (2) concluir com marcador remanescente recusa, nomeia o caminho, não cria commit e o merge continua; (3) concluir sem merge em andamento recusa; (4) abortar restaura `HEAD`, árvore limpa e o conteúdo local, apaga `MERGE_HEAD`; (5) abortar sem merge recusa; (6) `git.commit` durante o merge recusa e não cria commit; (7) Update conflitante deixa `MERGE_HEAD` e os dois commits, com a mensagem apontando o menu de branches, e a conclusão depois disso funciona; (8) merge de outro processo continua recusado pelo Update antes de qualquer comando.
- **UI: prova é o app Tauri real com `agent-desktop`** (`desktop.sh up` com `VERIFY_OPENRIDR_DESKTOP_STATE` próprio da issue e `VERIFY_OPENRIDR_DESKTOP_SOLUTION` apontando um clone descartável de `scripts/git-remote-fixture.sh`), como na aceitação da #50. Playwright não é prova de sessão (decisão da #67): nenhum spec E2E ganha caso novo aqui.

## Out of scope

- **Rebase, cherry-pick e revert** pela UI: a nota do menu de branches diz que esses casos pedem terminal; rebase tem frente própria depois da #114 (decisão da #112).
- **UI de resolução de conflito** (três vias, "aceitar meu/lado deles"): a resolução é no editor, com os marcadores do Git.
- **Editar a mensagem do merge commit** (`--no-edit`), escolher política de merge/rebase no diálogo, ou empurrar depois do merge.
- **Atalho de teclado e Find Action** para concluir/abortar.
- Windows e macOS Intel.

## Further notes

- Evidência em `docs/evidence/git-merge-in-progress.md`, capturas em `docs/evidence/git-merge-in-progress/`. Formato de referência: `docs/evidence/git-update-reconcile.md`.
- Decisão registrada em `docs/decisions.md`; `GLOSSARY.md` ganha **Merge in progress** e a entrada **Update** deixa de dizer que o conflito aborta.
- `docs/SPEC-GIT-UPDATE-RECONCILE.md` ganha um ponteiro: o abort do conflito registrado lá é substituído por esta frente.
- `docs/README.md` ganha a linha da frente; `docs/implementation-notes.md` ganha o contrato das operações e os testids novos.
