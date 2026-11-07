# SPEC — Diff de um arquivo de um commit pelo Git log (issue #101)

Status: ready-for-agent. Alvo: macOS Apple Silicon. Issue-mãe desta frente: [#101](https://github.com/Heldinhow/openridr/issues/101) (*Related to* [#48](https://github.com/Heldinhow/openridr/issues/48)).

## Problema

O Git log lista os arquivos do commit selecionado — status, caminho e origem do rename — e **não há como abrir o diff de um deles**. O painel de diff que existe é o do **working-tree diff**: base = index (sem stage) ou `HEAD` (arquivo todo em stage), lado atual = worktree. Para um arquivo de um commit a comparação é outra: os dois lados são blobs, a base é o **pai** do commit e o lado atual é o próprio commit. Essa superfície não existe.

Consequência prática: revisar um commit no app exige abrir o arquivo no editor e comparar à mão, ou sair para o terminal. O histórico sabe quais arquivos mudaram, mas não mostra o que mudou.

## Solução

Clicar numa linha de arquivo do Git log abre uma **aba de diff read-only daquele arquivo naquele commit**, no mesmo painel do working-tree diff (arquivo inteiro em split com conectores, modo unificado, navegação entre diferenças), com a base `parent:<caminho>` e o lado atual `<commit>:<caminho>`. Arquivo renomeado usa `parent:<caminho original>` como base. Commit raiz não tem pai: a base não existe e o painel diz `No previous file`, como num arquivo adicionado. Arquivo deletado no commit mostra `No current file` e o conteúdo do commit.

O diff de commit **não toca a árvore de trabalho**: os dois lados vêm do banco de objetos do Git, então o resultado não depende do que está no editor, no index ou em disco. A aba é própria (não reusa a aba do working-tree diff), é rotulada com o hash curto e o nome do arquivo, e o painel mostra o hash completo e o assunto do commit como contexto.

## User stories

1. Como desenvolvedor revisando um commit, quero clicar num arquivo da lista do Git log e ver o diff daquele arquivo naquele commit, para revisar sem sair do app.
2. Como desenvolvedor, quero ver o arquivo inteiro dos dois lados, para ler o contexto da mudança e não só o retângulo de hunks.
3. Como desenvolvedor, quero a coluna de conectores entre as regiões alteradas, para ligar visualmente o que saiu ao que entrou.
4. Como desenvolvedor, quero o modo unificado, para revisar em uma coluna quando a janela é estreita.
5. Como desenvolvedor, quero que o modo escolhido continue escolhido, para não alternar a cada arquivo.
6. Como desenvolvedor, quero ver o hash do commit e o assunto no painel, para saber exatamente qual revisão estou lendo.
7. Como desenvolvedor, quero ver o hash do pai no rótulo da esquerda, para saber contra qual commit a comparação foi feita.
8. Como desenvolvedor, quero que o diff de um commit não mude quando eu edito o arquivo no editor, para revisar o commit e não a árvore de trabalho.
9. Como desenvolvedor revisando um arquivo adicionado no commit, quero ver o arquivo inteiro à direita e `No previous file` à esquerda.
10. Como desenvolvedor revisando um arquivo deletado no commit, quero ver o arquivo inteiro à esquerda e `No current file` à direita.
11. Como desenvolvedor revisando um rename com edição, quero que a base seja o arquivo antigo, para ver o que mudou de fato em vez de um arquivo inteiro novo.
12. Como desenvolvedor revisando o commit raiz de um repositório, quero que todos os arquivos apareçam como adicionados, porque não existe pai.
13. Como desenvolvedor, quero andar pelos arquivos do commit dentro do painel, para revisar o commit inteiro sem voltar ao log a cada arquivo.
14. Como desenvolvedor, quero pular de diferença em diferença nos dois modos, para navegar sem mudar de hábito.
15. Como desenvolvedor, quero que o diff de um commit e o diff da árvore de trabalho possam ficar abertos ao mesmo tempo, para comparar as duas leituras.
16. Como desenvolvedor, quero que abrir o diff de outro arquivo do mesmo commit substitua a aba, para o tab strip não crescer com a lista de arquivos do commit.
17. Como desenvolvedor com um arquivo binário ou grande demais, quero a visão de hunks com a explicação, para não ficar com a tela vazia nem com um arquivo inteiro inventado.
18. Como desenvolvedor com um commit que não existe mais (amend, rebase), quero uma mensagem clara em vez de um diff vazio ou de uma adivinhação.
19. Como usuário de leitor de tela, quero que cada linha de arquivo do log seja um botão com nome acessível, para abrir o diff pelo teclado.
20. Como mantenedor, quero o diff de commit como leitura própria do host, para a UI não montar caminho de blob nem rodar Git.
21. Como mantenedor, quero que o diff de commit reusasse a montagem de linhas já testada, para não existir um segundo algoritmo de alinhamento.
22. Como mantenedor, quero nenhum `data-testid` renomeado, para os testes e a evidência antigos continuarem válidos.
23. Como testador, quero os casos de modificado, adicionado, deletado, renomeado e raiz provados contra um repositório real criado no teste.
24. Como testador, quero o caminho completo conferido no app Tauri real com `agent-desktop`, porque nenhum teste de host prova que a tela abre.

## Decisões de implementação

- **Dono dos dados: o host.** Método novo `git.commitDiff` (`{ hash, path, originalPath }`) devolve tudo o que o painel precisa. `git.fileDiff` e `git.diff` **não mudam**: o primeiro continua sendo a leitura do working-tree diff, o segundo o gutter do editor.
- **Contrato de `git.commitDiff`** (camelCase no JSON; `null` onde não existe):
  - `state`: `ok` | `empty` | `not_a_repository` | `git_missing` | `failed`; `message: string | null`.
  - `path`, `originalPath: string | null`.
  - `commit: { hash, shortHash, subject }` — o commit cujo arquivo está sendo mostrado.
  - `parent: { hash, shortHash, subject } | null` — o **primeiro** pai; `null` num commit raiz. Os dois hashes curtos vêm do Git (`%h`), não de recorte de string na UI.
  - `base: { exists, text }` e `current: { exists, text }` — os dois lados como blob, LF, BOM removido, decodificado por `fsutil::decode_text`.
  - `hunks: [{ oldStart, oldCount, newStart, newCount }]` — cabeçalhos do `-U0`, na ordem.
  - `patch: string` — o patch `-U3` limitado (fallback), `truncated: bool`.
  - `full: bool` e `fullReason: "binary" | "too_large" | "unsupported_encoding" | "unreadable" | "unaligned" | null` — mesmas regras do `git.fileDiff`.
- **O commit é resolvido, não aceito como texto.** `git rev-parse --verify --quiet "<hash>^{commit}"` devolve o hash completo; um hash que não resolve (commit amendado/rebaseado, log velho) vira `state: "failed"` com mensagem — nunca um diff vazio nem uma base adivinhada. O hash completo resolvido é o que entra nas specs de blob e de diff.
- **Pai = primeiro pai.** `git rev-parse --verify --quiet "<hash>^"`; exit 1 é "commit raiz" (não é erro). Merge commit não lista arquivos no log de hoje (o log não diffa merge), então não há linha para clicar; se um dia houver, o primeiro pai é a base declarada.
- **Base e atual são blobs.** Base = `parent:<originalPath ?? path>`, atual = `<hash>:<path>`, lidos por `read_blob` — a mesma prova de ausência (`rev-parse --verify --quiet` exit 1), de tamanho (`cat-file -s`) e de erro que o #48 estabeleceu. Ausência vira `exists: false`; erro de leitura vira `unreadable`, nunca uma adição ou deleção inventada.
- **Hunks e patch vêm do Git, com os dois caminhos quando há rename.** `git diff <pai> <hash> -- <old> <new>` (medido: com só o caminho novo o Git mostra add+delete, e o diff de um rename com edição vira dois arquivos inteiros). Commit raiz usa a empty tree resolvida por `git hash-object -t tree /dev/null` — nada de hash mágico no código. Captura limitada por `spawn_bounded`, como no #48.
- **Nenhuma leitura da worktree.** O caminho não é resolvido em disco, então não há confinamento de symlink novo a fazer: um symlink no commit é o blob do link (o alvo em texto), que é o que o Git guarda. Os dois caminhos passam por `validate_path` (o `originalPath` é entrada da UI, ainda que venha do `git.log`). Limite declarado: um `originalPath` errado produziria uma base errada; a UI só passa o que o log devolveu para aquele commit, e o caso de rename tem prova de host.
- **Um tail só para os dois diffs.** A montagem do payload (lados, `full`/`fullReason`, fallback de UTF-16 pelos ranges do Git sobre o texto decodificado, `unaligned`, `cap` do patch, o "No diff" que não mente quando um lado existe e não pôde ser lido) é extraída do `file_diff` para uma função usada pelos dois métodos. `file_diff` mantém comportamento e testes idênticos.
- **Trust.** `git.commitDiff` entra em `EXECUTABLE_METHODS`: sem trust o método é negado como os outros `git.*`.
- **A UI reusa o painel.** `CommitDiff` ganha um alvo opcional (commit) em vez de uma segunda cópia da toolbar, dos rótulos, da nota e dos editores. Com alvo, ele chama `git.commitDiff`; sem alvo, continua chamando `git.fileDiff` exatamente como hoje.
- **Aba própria, slot único.** `data-testid="commit-file-diff-tab"`, independente da aba do working-tree diff (as duas podem ficar abertas ao mesmo tempo). Clicar outro arquivo do mesmo commit — ou de outro — substitui o conteúdo da mesma aba. Rótulo `<shortHash> · <arquivo>`; `title` = `<hash> · <subject> — <path>`.
- **Painel.** `aria-label` "Commit diff for `<path>` at `<shortHash>`"; `id`/`aria-controls` próprios (`commit-file-diff-content`), para não duplicar o id do painel do working-tree diff. Rótulos: esquerda `parent · <parentShort>` (ou `No previous file`), direita `<commitShort>` (ou `No current file`). A linha do commit mostra `<shortHash> · <subject>` com o hash completo no `title`, no mesmo `data-testid="commit-diff-commit"` — o prefixo `HEAD` fica só no working-tree diff, porque o commit mostrado pode não ser o `HEAD`.
- **Navegação.** Os mesmos botões `Previous/Next changed file` e a contagem `n/m files` andam pela lista de arquivos do commit; `Previous/Next difference` continua pulando regiões alteradas nos dois modos.
- **Linhas do log viram botões.** Cada arquivo do commit selecionado é um `<button>` (`data-testid="git-log-file-<path>"`) com o status, o caminho e o `from <original>` de hoje. Sem estado de seleção na linha: a aba aberta é o estado. O log não recarrega ao abrir um diff.
- **Contrato de testids.** Nenhum existente é renomeado ou removido. Novos: `commit-file-diff-tab`, `git-log-file-<path>`. Reusados pelo painel: `commit-diff`, `commit-diff-mode`, `commit-diff-commit`, `commit-diff-note`, `commit-diff-left-label`, `commit-diff-right-label`, `commit-diff-count`, `commit-diff-file-count`, `commit-diff-unified`.
- **Sem dependência nova e sem CSS novo além do necessário** para a linha virar botão (largura, alinhamento à esquerda, foco visível herdado).

## Decisões de teste

- **O que faz um bom teste aqui:** ele observa o que o usuário vê — o diff daquele commit, os rótulos com os dois hashes, a linha do commit, o modo — ou o payload do host contra um repositório real criado no próprio teste. Nada de afirmar campo interno que só deriva a decisão.
- **Host (`crates/host/src/git.rs`, `mod tests`), repo real criado no teste:** modificado no meio de um arquivo (textos completos, hunks e `commit`/`parent` certos); arquivo adicionado no commit (base ausente); arquivo deletado (atual ausente, texto do commit na base); rename com edição (base = conteúdo do caminho antigo no pai, atual = caminho novo, hunk da linha alterada); commit raiz (`parent: null`, base ausente, todas as linhas adicionadas); binário com `\0` → `full: false` + `fullReason: "binary"` e patch de fallback; hash que não existe → `failed` com mensagem; caminho e `originalPath` fora do repositório → `invalid_state`; o diff de commit **não** é afetado por uma alteração não commitada no mesmo arquivo (o caso que separa esta leitura do `git.fileDiff`).
- **Trust:** `git.commitDiff` negado sem trust, pelo mesmo caminho de fronteira que os outros `git.*`.
- **Unidade (`tests/unit/commitDiff.test.ts`):** um caso de rename — base vinda do caminho antigo, atual do novo, um hunk de linha alterada — provando que a montagem de linhas do commit é a mesma de um arquivo modificado. Nenhuma função pura nova é criada; `buildCommitRows` é reusado.
- **Sem E2E novo.** A issue pedia "provado por um E2E contra repositório descartável"; a decisão do Helder (issue #67, repetida no grill desta frente) é que Playwright é só smoke noturno e nunca verificação de UI de sessão. A prova passa a ser **host + unidade + app Tauri real com `agent-desktop`**, e a troca fica registrada aqui e na evidência.
- **App real:** `agent-desktop` sobre uma cópia em `/tmp`, uma instância por vez, `OPENRIDR_HOME` isolado do state do `verify-openridr` com o nome desta issue. Repositório de prova com commit raiz, um commit de modificado, um de adicionado, um de deletado e um de rename com edição; conferir split com conectores, modo unificado, os dois hashes, os rótulos de ausência e que o working-tree diff continua funcionando. O que o ambiente não permitir entra como "não verificado".

## Out of scope

- Diff entre dois commits arbitrários, blame, histórico de uma linha, diff de imagem ou prévia de binário.
- Diff de merge commit (o log não lista arquivos de merge; o primeiro pai fica documentado para quando isso mudar).
- Recarregar o Git log sozinho, no foco ou depois de uma operação (observação já registrada na retro do #112).
- Abrir o arquivo no editor, comentar, stage/unstage ou qualquer escrita a partir do diff de commit.
- Filtro de whitespace, colapsar regiões não alteradas, redimensionar os painéis.
- E2E novo (decisão #67).
- Windows e macOS Intel.

## Further notes

- Evidência em `docs/evidence/git-commit-diff.md`; decisão em `docs/decisions.md` (2026-11-07); `GLOSSARY.md` ganha **Commit diff**; `docs/README.md` ganha a linha da frente; `docs/implementation-notes.md` ganha o contrato do método e os testids novos.
- O painel e a montagem de linhas são os do #48: este spec acrescenta **de onde vêm os dois lados**, não como o diff é desenhado.
- O caso que separa esta leitura do working-tree diff é o arquivo com alteração não commitada: o diff do commit mostra o blob do commit mesmo que o arquivo em disco já tenha mudado.
