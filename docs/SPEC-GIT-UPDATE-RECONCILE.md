# SPEC — Update reconcilia branch divergente (issue #112)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#112](https://github.com/Heldinhow/openridr/issues/112) (frente #50, aceitação de Git com remoto).

## Problema

O **Update…** do painel Git roda `git pull` puro. Quando a branch local e o upstream divergiram — um commit local e outro commit publicado pela contraparte — o Git recusa a reconciliação e o diálogo fica aberto mostrando o hint cru:

```
hint: You have divergent branches and need to specify how to reconcile them.
...
fatal: Need to specify how to reconcile divergent branches.
```

Não há escolha de merge ou rebase, nem como concluir o update: a pessoa precisa sair do app e rodar Git (ou mexer em `pull.rebase`/`pull.ff`) para o painel voltar a atualizar. O trabalho nunca se perde — os dois lados e a árvore sobrevivem — mas o app não termina o que começou, e o texto na tela ensina a editar configuração do Git em vez de explicar o que o app faz.

## Solução

O **Update…** passa a ter uma **política declarada**: quando não é fast-forward, o OpenRidr reconcilia com um **merge commit**. O diálogo diz isso antes de confirmar, mostra os dois contadores (`Ahead` e `Behind`) e, depois do clique, o resultado é um merge de verdade — os dois lados preservados, árvore limpa, `git log` com dois parents.

A política é explícita nos comandos e flags (`git fetch` e depois `git merge --ff --no-edit`), então o resultado não depende de `pull.rebase`/`pull.ff`/`merge.ff` do usuário, e o diálogo declara o override. Se o merge **conflitar**, o OpenRidr **aborta o merge** (`git merge --abort`) e devolve, em palavras do app, quais arquivos mudaram dos dois lados — branch e árvore ficam exatamente como estavam. Erros que o Git explica com conselho de configuração (conflito, non-fast-forward, identidade ausente) viram mensagem do app; o stderr cru continua só no `output` da operação, para auditoria.

## User stories

1. Como pessoa que usa o painel Git, quero que **Update…** termine um update divergente, para não precisar sair do app e rodar `git merge` no terminal.
2. Como pessoa que usa o painel Git, quero que o diálogo de Update diga **qual reconciliação** vai acontecer antes de confirmar, para não aceitar um rebase ou merge sem saber.
3. Como pessoa que usa o painel Git, quero ver **quantos commits locais e quantos do upstream** o update vai juntar, para decidir se atualizo agora ou empurro primeiro.
4. Como pessoa que usa o painel Git, quero que o merge preserve **os dois lados**, para ter certeza de que nada foi reescrito nem descartado.
5. Como pessoa que usa o painel Git, quero que a política do app **não dependa** de configuração global de Git que eu talvez tenha esquecido (`pull.rebase`, `pull.ff`), para o comportamento ser o mesmo em qualquer máquina.
6. Como pessoa que usa o painel Git, quero que o diálogo **declare** que a política do app tem precedência sobre essa configuração, para a diferença não ser silenciosa.
7. Como pessoa que usa o painel Git, quero que um merge **com conflito** não me deixe num beco sem saída, para poder continuar trabalhando sem abrir terminal.
8. Como pessoa que usa o painel Git, quero ver **quais arquivos** conflitaram quando o update aborta, para saber o que a contraparte mudou onde eu também mexi.
9. Como pessoa que usa o painel Git, quero que o update abortado **não altere** branch, commits nem árvore de trabalho, para o abort ser seguro e não uma perda silenciosa.
10. Como pessoa que usa o painel Git, quero a mensagem de erro **em palavras do app**, não o conselho do Git sobre editar configuração, para saber o que aconteceu e o que fazer.
11. Como pessoa que usa o painel Git, quero continuar vendo o **stderr do Git** quando precisar dele, para diagnosticar um caso que o app não previu.
12. Como pessoa que usa o painel Git, quero que o erro de **identidade ausente** (`user.name`/`user.email`) diga que o merge commit precisa de identidade e onde configurá-la, para não decifrar "unable to auto-detect email address".
13. Como pessoa que usa o painel Git, quero que uma recusa de **non-fast-forward** apareça como recusa do app, para não confundir com branch divergente.
14. Como pessoa que usa o painel Git, quero que o update **fast-forward** continue funcionando como hoje, sem merge commit, para a atualização simples não ganhar histórico extra.
15. Como pessoa que usa o painel Git, quero que árvore suja e operação em andamento **continuem recusando** o update com motivo, para o app não auto-stashar nem atropelar um merge pela metade.
16. Como pessoa que valida a release, quero **prova no app Tauri real** de um update divergente terminando em merge commit, para saber que a correção vale no binário e não só no host.
17. Como pessoa que valida a release, quero a prova de que um update abortado por conflito **devolve a árvore limpa** e os dois commits intactos, para confiar no abort.
18. Como pessoa que mantém o OpenRidr, quero testes de host que montem remoto real e divergência real, para a política ser provada por efeito (`rev-list`, `rev-parse`, `MERGE_HEAD`) e não por mensagem.
19. Como pessoa que mantém o OpenRidr, quero que o que não foi verificado (rebase, Windows, Intel, remoto autenticado) apareça como não verificado, para não vender suporte que não existe.
20. Como pessoa que mantém o OpenRidr, quero a decisão registrada em `decisions.md` com a nota de que o abort do conflito é provisório até a #114, para ninguém "consertar" o comportamento achando que é final.

## Implementation decisions

- **Política fixa: merge, com flags explícitas.** O update faz **fetch e depois merge**, não `git pull`: `git fetch --quiet` e `git merge --ff --no-edit @{upstream}`. Nunca rebase (reescreveria commits locais e cairia no estado de rebase que a UI também não conclui nem aborta — #114), nunca `--ff-only`, nunca `--no-ff` (isso criaria merge commit mesmo em fast-forward). As flags vencem `pull.rebase`/`pull.ff`/`merge.ff` do usuário de propósito: a política é do app e é declarada no diálogo. Não há valor novo na API: `git.pull` continua sem parâmetros.
- **Fetch separado do merge por causa da posse do abort.** O fetch move só refs; antes do merge o host resolve o commit do upstream com `rev-parse @{upstream}` e guarda o hash. Assim, quando o merge falha, o app só aborta um merge cujo `MERGE_HEAD` seja exatamente esse commit — um merge iniciado por outro processo (janela entre a checagem inicial e o comando) é reportado e deixado intacto, com o motivo. Sem essa prova de posse, "existe merge em andamento" não basta para abortar.
- **O diálogo de Update declara a política.** Além de `Branch`/`Upstream`/`Behind`, passa a mostrar `Ahead` e uma linha de política: reconcilia com merge commit quando os dois lados andaram; fast-forward continua fast-forward; as flags do app vencem a configuração de `pull` do repositório. Testids novos (`update-ahead`, `update-policy`); os existentes (`update-dialog`, `update-confirm`, `update-cancel`, `update-error`) não mudam de nome nem de papel.
- **Conflito aborta e devolve a árvore.** Depois de um merge que falhou, o host relê o status: se o merge em andamento é o deste update (posse provada pelo hash, acima), roda `git merge --abort` e devolve `state: "failed"` com mensagem do app listando até 20 caminhos conflitados ("Update stopped: 1 file changed on both sides (…). OpenRidr aborted the merge; your branch and working tree are unchanged."). O abort é não destrutivo: o fetch já ocorreu (refs de tracking atualizadas), a branch local, o índice e a árvore voltam ao estado anterior. Isso é **provisório** até a #114 entregar concluir/abortar merge pela UI; quando ela existir, deixar o merge aberto para resolução no editor passa a ser a alternativa preferível.
- **Três classes de erro deixam de vazar o conselho do Git.** O host mapeia, em palavras do app: conflito (a mensagem acima), non-fast-forward recusado ("Update stopped: git refused a non-fast-forward update…"), e identidade ausente ("Update needs a Git identity for this repository: set user.name and user.email so the merge commit has an author."). Qualquer outra falha mantém a cauda do stderr como mensagem, como hoje. O stderr completo continua em `Operation.output` (cap e mascaramento de URL inalterados) em todos os casos.
- **Detecção é pelo estado, não pelo contador da tela.** A decisão de abortar olha o status depois da falha (marcador de operação em andamento e arquivos conflitados), nunca `ahead`/`behind` da tela, que podem estar defasados sem fetch (#113). O update continua sem `fetch` separado: `pull` já busca.
- **Recusas existentes ficam.** Árvore suja e operação em andamento continuam recusando o update com o motivo atual (sem auto-stash), antes de rodar qualquer comando que escreva.
- **Sem dependência nova, sem comando novo de Git.** Tudo o que a correção usa (`pull`, `merge --abort`, `status`) já é Git de linha de comando pelo host; nada de libgit2.

## Testing decisions

- **Um teste bom aqui prova o efeito no repositório, não o texto na tela.** Cada caso confere `rev-parse`/`rev-list`/`git log --format=%p`, presença de `.git/MERGE_HEAD`, conteúdo dos arquivos e `status --porcelain`, além do `state`/`message` da operação.
- **Seam: a função de pull do host, com um remoto real.** Os testes montam um repositório bare temporário, um clone `peer` que publica e a árvore local que commita; nada de mock de Git. Prior art: `checkout_of_a_remote_branch_creates_a_local_tracking_branch` e `push_requires_confirmation_never_forces_and_sets_upstream` em `crates/host/src/git.rs`.
- **Casos mínimos**: (1) update fast-forward continua fast-forward, sem merge commit e com `ahead`/`behind` zerados; (2) update divergente cria merge commit com dois parents, mantém os dois arquivos e passa a `ahead 2, behind 0` (o merge commit local não foi empurrado); (3) update divergente com `pull.rebase=true`, `pull.ff=only` e `merge.ff=only` no repositório ainda faz merge (a config não vence o app); (4) conflito no mesmo arquivo aborta: `MERGE_HEAD` ausente, `HEAD` igual ao de antes, árvore limpa, arquivo com o conteúdo local, mensagem em palavras do app com o caminho, e o stderr cru em `output`; (5) identidade ausente vira mensagem do app, sem "unable to auto-detect"; (6) nenhuma mensagem de falha contém conselho de `pull.rebase`/`pull.ff`; (7) um merge em andamento de outro commit não é abortado (posse não provada) e o `MERGE_HEAD` estranho continua lá.
- **UI: prova é o app Tauri real com `agent-desktop`** (`desktop.sh up` com `VERIFY_OPENRIDR_DESKTOP_SOLUTION` apontando para um clone descartável de `scripts/git-remote-fixture.sh`), como na aceitação da #50. Playwright não é prova de sessão (decisão da #67): o spec de diálogo existente (`medium-nav-git-dialogs.spec.ts`) não ganha caso novo aqui.
- **O fixture de remoto local não muda**: `scripts/git-remote-fixture.sh` já cria `origin.git`, `user` e `peer`; a divergência é produzida por shell (commit em `user`, commit + push em `peer`).

## Out of scope

- **Escolha de rebase** no diálogo, ou respeitar `pull.rebase` do usuário: fica para um ticket próprio, depois da #114 (concluir/abortar rebase pela UI).
- **Concluir ou abortar merge pela UI** (#114): esta frente aborta o conflito, não resolve.
- **Ação de fetch** e o contador `↓` defasado (#113).
- **Atalho `⌥⌘W`** sombreado por Close All (#111).
- `push --force`, tags, submodules, remoto autenticado/rede.
- Windows e macOS Intel.

## Further notes

- Evidência em `docs/evidence/git-update-reconcile.md`, capturas em `docs/evidence/git-update-reconcile/`. Formato de referência: `docs/evidence/git-remote-acceptance.md`.
- Decisão registrada em `docs/decisions.md` (2026-11-06); `GLOSSARY.md` ganha **Update** e tem a entrada **Divergent branch** corrigida (o app agora tem política).
- `docs/README.md` ganha a linha da frente; `docs/implementation-notes.md` ganha o contrato da operação e os testids novos.
