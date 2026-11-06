# RETRO — aceitação Git com remoto (issue #50)

Rodada de 2026-11-06, `5faaff7`, macOS arm64. Spec: [SPEC-GIT-REMOTE-ACCEPTANCE](SPEC-GIT-REMOTE-ACCEPTANCE.md). Evidência: [evidence/git-remote-acceptance.md](evidence/git-remote-acceptance.md).

## O que se sustentou

- **Commit → push fecha.** Um commit feito no painel chegou ao bare depois do Push, e o status voltou a zero à frente.
- **Push é conservador de verdade.** Sem upstream, o diálogo diz que vai usar `--set-upstream`; com upstream, mostra remoto/branch/commits à frente; nunca força e a rejeição non-fast-forward não toca o commit local.
- **Update traz o trabalho da contraparte.** O pull fast-forward incorporou o commit publicado e o arquivo mudou em disco.
- **Refusals seguras.** Árvore suja desabilita Update e checkout com motivo e nenhum `stash` aparece; o arquivo continua lá. Merge em andamento bloqueia commit de arquivo em conflito.
- **Branch é branch.** Criar, trocar e fazer checkout de branch remota com tracking bateram com `rev-parse` e com os arquivos no disco.
- **Histórico não inventa.** Lista, selos de `root`/`2 parents`, arquivos do commit e filtro bateram com `git log`/`git show`.

## O que quebrou

- **Atalho `⌥⌘W` encerra o app** (#111): o atalho anunciado de Update Project é o mesmo do item nativo Close All. Foi o único evento que custou um relançamento, e por isso a Trust decision teve de ser refeita.
- **Pull divergente não reconcilia** (#112): `git pull` cru falha com o hint do git e o diálogo mostra esse texto ao usuário. Sem escolha de merge/rebase, o trabalho só anda com git na mão.
- **Sem fetch** (#113): o `↓` e a lista de branches remotas são tão frescos quanto o último push/pull. O pull funciona com o contador velho, mas o usuário não tem como atualizar o painel.
- **Merge em andamento é um beco sem saída** (#114): a UI explica e bloqueia, mas não conclui nem aborta.

## Lições

- **Estado de merge criado por CLI precisa ser declarado como precondição.** A tentação é vender "conflito tratado pela UI" quando a UI só leu o estado. O quadro por cenário separa as duas coisas.
- **Fetch manual é precondição, não prova de refresh.** Sem essa distinção, o `↓` parece funcionar sozinho.
- **Atalho de menu nativo vence o keymap do app.** Verificar atalhos anunciados pela Find Action no app real, não só no `keymap.ts`.
- **Componente `<summary>` pode não ser acionável na árvore de acessibilidade.** O menu de branches foi aberto pelas ações de Git; isso é limite da rodada, não passe.
- **Fixture de Git com remoto vale mais que host-http aqui**: o assunto é o efeito no remoto, e só um remoto real (mesmo local) responde.
- **Capture o estado inicial antes do primeiro clique.** No S1 a leitura da árvore existiu, mas nenhuma captura foi preservada; a imagem posterior não substitui a prova do estado inicial. Primeira captura da sessão = estado inicial.
- **Ferramenta de verificação não pode apagar o que recebe.** O fixture nasceu aceitando um caminho e recriando-o com `rm -rf`; a revisão mostrou que `/`, home ou uma pasta de trabalho válida cairiam nisso. O padrão passou a ser `mktemp` + recusa de caminho existente, com teste de shell para as recusas; validação de entrada do launcher também passou a acontecer antes de qualquer rotação de state.

## Follow-ups

- #111, #112, #113, #114 (bugs, `needs-triage`).
- Decidir se a observação de acessibilidade do summary de branch vira ticket.
- Próxima rodada: remoto autenticado (HTTPS/SSH) quando houver decisão sobre credenciais; `force`/tags continuam fora.
