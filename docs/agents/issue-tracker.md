# Issue tracker: GitHub

Issues deste repo vivem como **GitHub issues** em `Heldinhow/openridr`. Use o `gh` CLI para tudo.

O repositório é inferido de `git remote -v`; o `gh` já resolve isso dentro do clone.

## Convenções

- **Criar issue**: `gh issue create --title "..." --body-file <arquivo>`. Corpo multi-linha vai num
  arquivo (heredoc **em arquivo**, depois `--body-file`), nunca em `--body "$(cat <<'EOF' … EOF)"`: uma
  crase no texto dentro dessa forma vira substituição de comando e o shell aborta com `bad substitution`.
- **Ler issue**: `gh issue view <n> --json number,title,body,labels,comments`.
- **Listar**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'`, com `--label`/`--state` conforme o caso.
- **Comentar**: `gh issue comment <n> --body "..."`.
- **Label**: `gh issue edit <n> --add-label "..."` / `--remove-label "..."`.
- **Fechar**: `gh issue close <n> --comment "..."`.

## Tickets de uma frente de trabalho

Toda **frente** (uma spec) tem uma **issue-mãe**; cada fatia/ticket é uma issue filha dela. Nada de `docs/tickets-<tema>/` para trabalho novo.

- **Issue-mãe**: `gh issue create --title "<tema>: ..." --body ... --label enhancement`.
- **Filha ligada à mãe** (sub-issue nativa, é o que a UI mostra):
  - `gh` 2.94+ tem atalho: `gh issue create --parent <mãe> ...` ou `gh issue edit <mãe> --add-sub-issue <filha>`.
  - Com o `gh` 2.83.1 desta máquina, use a API com o **database id** da filha (não o `#number`, não o `node_id`):
    ```sh
    child_id=$(gh api repos/Heldinhow/openridr/issues/<filha> --jq .id)
    gh api --method POST "repos/Heldinhow/openridr/issues/<mãe>/sub_issues" -F sub_issue_id="$child_id"
    ```
  - Independente do caminho, ponha `Part of #<mãe>` na primeira linha do corpo da filha: é o que um humano lê no texto.
- **Bloqueio**: dependência nativa, que é a que aparece na UI:
  ```sh
  blocker_id=$(gh api repos/Heldinhow/openridr/issues/<bloqueadora> --jq .id)
  gh api --method POST "repos/Heldinhow/openridr/issues/<filha>/dependencies/blocked_by" -F issue_id="$blocker_id"
  ```
  `gh api repos/Heldinhow/openridr/issues/<n> --jq .issue_dependencies_summary.blocked_by` conta os bloqueadores **abertos** (o gate ao vivo). Sem a dependência, caia para uma linha `Blocked by: #<n>, #<n>` no topo do corpo.
- **Filha desbloqueada**: nenhum bloqueador aberto (`issue_dependencies_summary.blocked_by == 0`, ou todas as issues do `Blocked by` fechadas) e sem assignee.
- **Assumir**: `gh issue edit <n> --add-assignee @me`, a primeira escrita da sessão. `/wayfinder` também usa `wayfinder:map` / `wayfinder:<tipo>`.
- **Resolver**: `gh issue comment <n> --body "<resultado>"`, depois `gh issue close <n>`, e acrescente o ponteiro (gist + link) na mãe.

## O que continua em `docs/`

- `docs/evidence/<tema>.md` — evidência de verificação.
- `docs/decisions.md` — decisões incrementais (append).
- `docs/SPEC-*.md`, `docs/RETRO-*.md`, `docs/REVIEW-*.md` — **histórico**: a spec de uma frente nova é o corpo da issue-mãe e o fluxo não tem retro.

Tickets antigos em `docs/tickets*/` (e os SPEC/RETRO/evidence deles) **ficam como estão**: histórico, não migrar. Para leitura, é o ticket que manda no trabalho novo; o doc histórico só entra como contexto.

## `data-testid` e critérios

O corpo da issue repete o que o AGENTS.md exige: `data-testid` são contrato, zero dependência nova sem decisão, e "não verificado" explícito no `docs/implementation-notes.md` e na issue.

## PRs como surface de triagem

**PRs as a request surface: no.** _(Mude para `yes` se este repo passar a tratar PR externo como pedido de feature; o `/triage` lê esta flag.)_

Quando virar `yes`, PRs passam pelos mesmos labels/estados, com os equivalentes `gh pr`:

- **Ler PR**: `gh pr view <n> --comments` e `gh pr diff <n>`.
- **Listar PRs externos**: `gh api --paginate 'repos/{owner}/{repo}/pulls?state=open' --jq '.[] | select(.author_association | IN("OWNER","MEMBER","COLLABORATOR") | not) | {number, title, author: .user.login, author_association, labels: [.labels[].name]}'`.
- **Comentar/rotular/fechar**: `gh pr comment`, `gh pr edit --add-label`/`--remove-label`, `gh pr close`.

O GitHub compartilha uma numeração entre issues e PRs: um `#42` solto pode ser qualquer um. Resolva com `gh pr view 42` e caia para `gh issue view 42`.

## Quando uma skill disser "publique no issue tracker"

Crie uma GitHub issue.

## Quando uma skill disser "pegue o ticket relevante"

Leia como em **Ler issue** acima.

## Operações de wayfinding

Usadas por `/wayfinder`. O **mapa** é uma issue única, com as filhas como tickets.

- **Mapa**: issue com label `wayfinder:map`, corpo com Notas / Decisões até agora / Neblina: `gh issue create --label wayfinder:map`.
- **Filha**: issue ligada ao mapa como sub-issue (acima). Sendo impossível, ponha a filha numa task list do corpo do mapa e `Part of #<mapa>` no topo do corpo dela. Labels `wayfinder:<tipo>` (`research`/`prototype`/`grilling`/`task`).
- **Bloqueio**: dependências nativas (acima), com fallback `Blocked by: #<n>`.
- **Fronteira**: filhas abertas do mapa, descartando quem tem bloqueador aberto ou assignee; a primeira na ordem do mapa ganha.
- **Assumir**: `gh issue edit <n> --add-assignee @me`.
- **Resolver**: comentar a resposta, fechar, apontar na seção Decisões do mapa.
