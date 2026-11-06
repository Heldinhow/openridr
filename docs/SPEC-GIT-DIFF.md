# SPEC — Git diff: arquivo inteiro lado a lado, modo unificado e hash do commit

Issue-mãe: [#48](https://github.com/Heldinhow/openridr/issues/48) — "Git diff: full-file side-by-side, unified mode and commit hash".

## Problem Statement

O painel de diff do Commit mostra só os hunks que o Git devolveu, mais três linhas de contexto. Para um arquivo de 800 linhas com uma mudança na linha 400, o usuário vê um retângulo de sete linhas: não consegue ler o que está em volta, comparar a forma do método alterado, nem usar o painel como leitura do arquivo — precisa abrir o editor e caçar a mudança de novo. Também não há modo unificado, e o "hash" exibido ao lado de `HEAD`/`Index` é o hash do *blob* extraído de `index abc..def` do patch, não o commit: parece revisão de commit e não é.

O que falta é o arquivo inteiro dos dois lados, ligações visíveis entre as regiões alteradas, uma visão unificada para quem prefere, e o hash do commit de verdade como contexto.

## Solution

Ao abrir o diff de um arquivo, o painel mostra o **arquivo completo** dos dois lados (base e atual), com as regiões alteradas marcadas linha a linha e uma coluna de conectores entre os painéis ligando cada região alterada. Um toggle `Split | Unified` na barra troca para a visão unificada (um editor, prefixos `-`/`+`, mesmas marcações e navegação). A barra passa a mostrar o commit `HEAD` (hash curto + assunto) como contexto real da comparação.

O diff continua sendo o da árvore de trabalho, com a mesma semântica de hoje: sem stage, `Index → Worktree`; com o arquivo todo em stage, `HEAD → Index`. Arquivos adicionados/untracked mostram o arquivo inteiro à direita e "No previous file" à esquerda; arquivos deletados mostram o arquivo inteiro do lado do `HEAD` e "No current file" à direita. Quando o arquivo completo não pode ser exibido (binário, acima dos limites, codificação não suportada), o painel mantém exatamente a visão de hunks atual e explica por quê.

## User Stories

1. Como desenvolvedor revisando uma mudança no meio de um arquivo grande, quero ver o arquivo inteiro dos dois lados, para ler o contexto sem sair do diff.
2. Como desenvolvedor, quero uma coluna de conectores entre as regiões alteradas, para ligar visualmente o que saiu à esquerda ao que entrou à direita.
3. Como desenvolvedor, quero um modo unificado, para revisar em uma coluna quando a janela é estreita ou quando prefiro o formato de patch.
4. Como desenvolvedor, quero que o modo escolhido continue escolhido na próxima vez, para não alternar a cada arquivo.
5. Como desenvolvedor, quero ver o hash do commit `HEAD` (e o assunto) no diff, para saber contra qual commit estou comparando.
6. Como desenvolvedor, quero que o hash mostrado seja o do commit, e não o do blob, para não confundir revisão de arquivo com revisão de commit.
7. Como desenvolvedor revisando um arquivo novo, quero ver o arquivo inteiro à direita e "No previous file" à esquerda, para revisar o arquivo todo.
8. Como desenvolvedor revisando um arquivo deletado, quero ver o arquivo inteiro à esquerda e "No current file" à direita, para conferir o que saiu.
9. Como desenvolvedor com um arquivo binário, grande demais ou em codificação que o editor não mostra, quero a visão de hunks de hoje com uma explicação, para não ficar com a tela vazia nem com um "arquivo inteiro" inventado.
10. Como desenvolvedor, quero que "previous/next difference" continue pulando regiões alteradas nos dois modos, para navegar sem mudar de hábito.
11. Como usuário de leitor de tela, quero que o toggle anuncie o modo escolhido (`aria-pressed`) e que o editor unificado tenha rótulo próprio, para saber o que estou lendo.
12. Como desenvolvedor em repositório sem commits, quero que o diff continue funcionando sem a linha do commit, porque não existe `HEAD` para citar.
13. Como mantenedor, quero a montagem das linhas do arquivo inteiro como função pura testada por unidade, para provar alinhamento, arquivo adicionado/deletado e entradas inválidas.
14. Como mantenedor, quero que o painel caia na visão de hunks quando o alinhamento não puder ser provado, para nunca mostrar linha no lugar errado.
15. Como mantenedor, quero os limites do arquivo completo no host, para o payload e a renderização não dependerem de otimismo do React.
16. Como testador, quero o caminho completo, unificado e o hash provados contra o host real e o E2E, e o resultado conferido no app Tauri real com `agent-desktop`.

## Implementation Decisions

- **Dono dos dados: o host.** Um método novo `git.fileDiff` (`{ path, staged }`) devolve tudo que a tela precisa; o `git.diff` atual **não muda**, porque o gutter do editor o usa a cada save e não deve baixar o arquivo inteiro.
- **Base do diff não muda.** Sem stage: `Index → Worktree` (`git diff -- path`). Com o arquivo todo em stage: `HEAD → Index` (`--cached`). O commit mostrado é o `HEAD` do repositório, como contexto — **não** é o lado esquerdo de um diff unstaged (o lado esquerdo aí é o index, que não tem hash de commit). Isso corrige a promessa do grill: em vez de rotular o lado esquerdo unstaged de `HEAD · <hash>`, o hash do commit vira uma linha própria.
- **Contrato de `git.fileDiff`** (camelCase no JSON; `null` onde não existe):
  - `state`: `ok` | `empty` | `not_a_repository` | `git_missing` | `failed`; `message: string | null` (mesmas regras de dados do `git.diff`).
  - `path`, `staged`.
  - `base: { exists: bool, text: string | null }` e `current: { exists: bool, text: string | null }` — texto completo em LF, BOM removido, codificação decodificada.
  - `hunks: [{ oldStart, oldCount, newStart, newCount }]` — só os cabeçalhos de `git diff -U0`, na ordem.
  - `patch: string` — o patch `-U3` de hoje (limitado a 240 KB), usado como fallback.
  - `truncated: bool` — só do patch de fallback.
  - `full: bool` — `true` quando toda lado existente tem texto completo (é a única condição para a visão de arquivo inteiro).
  - `fullReason: "binary" | "too_large" | "unsupported_encoding" | "unreadable" | "unaligned" | null` — por que `full` é `false`.
  - `commit: { hash, shortHash, subject } | null` — `HEAD` do repositório; `null` quando não há commits.
- **Conteúdo dos lados.** Base: o blob de `HEAD:<path>` (staged) ou `:<path>` (unstaged); atual com stage: o blob de `:<path>`; atual sem stage: o arquivo em disco (`repo.top/path`). Ausência (adicionado, deletado, untracked) vira `exists: false, text: null`; falha de leitura que não seja "não existe" mantém `exists: true, text: null` com motivo. A decodificação usa `fsutil::decode_text` (UTF-8/UTF-16 com BOM, normaliza CRLF) para o host nunca mostrar texto que o editor não mostraria.
- **Ausência é provada, não presumida.** O blob é resolvido por `git rev-parse --verify --quiet <spec>`: exit 1 (sem saída) é ausência comprovada (adicionado/deletado/repo sem commits); qualquer outro erro é leitura real e vira `unreadable`/`failed`, nunca uma adição ou deleção inventada. O tamanho vem de `git cat-file -s` antes de qualquer byte de conteúdo.
- **Confinamento de symlink.** O caminho léxico não basta: um diretório symlink levaria tanto a leitura do host quanto o lado worktree do `git diff` para fora do workspace. Antes de qualquer leitura ou comando que toque a árvore de trabalho (inclusive o patch de fallback e o `--no-index` de untracked), o diretório real do caminho é resolvido e precisa continuar dentro da raiz do repositório; fora dela a leitura é recusada com mensagem e sem conteúdo. Um arquivo symlink é tratado como objeto link (o alvo em texto, que é o que o Git guarda), nunca seguido para ler o alvo. Limitação conhecida e declarada: entre a verificação e o uso existe uma janela TOCTOU que só um sandbox de sistema fecharia.
- **Limites no host, antes de mandar.** Binário = byte `0x00` nos primeiros 8 KB do lado (regra do Git), exceto quando o lado começa com BOM UTF-16: aí é texto que o editor mostra, não binário. Arquivo completo por lado: **1 MiB** ou **30.000 linhas**, o que estourar primeiro; acima disso `full: false` com `too_large`. Codificação inválida → `unsupported_encoding`. Binário → `binary`. Um lado sem texto completo derruba `full` e o painel usa o patch; o texto do outro lado não é usado pela visão de hunks.
- **Captura limitada, não só recorte depois.** Toda leitura de conteúdo é limitada na origem: o lado completo para em 1 MiB + 1 byte (ou é recusado pelo tamanho declarado do blob) e o patch em 4× o limite de caracteres, matando o processo filho ao bater o teto e sinalizando `truncated`; stderr é drenado em thread própria com cópia limitada. `git.diff` legado mantém o mesmo contrato (240.000 caracteres + marcador). Nenhum caminho coleta o arquivo inteiro para depois recusá-lo.
- **Alinhamento: Git decide, host só conta.** O host lê os cabeçalhos do `-U0`; o React monta as linhas do arquivo inteiro a partir dos dois textos e dos hunks. Não entra algoritmo de diff em TypeScript nem dependência nova (`@codemirror/merge` não é dependência do repo). Para arquivo adicionado/deletado os hunks nem são necessários: todas as linhas são `added`/`removed`.
- **Contexto é verificado, não presumido.** As linhas entre hunks e depois do último são contexto por definição; se os dois textos divergirem em qualquer par, a montagem devolve `null` e o painel cai no patch com a razão `unaligned` — nunca mostra uma linha do lado atual escondendo a diferença do outro.
- **UTF-16 alinhado pelo próprio Git sobre o texto decodificado.** Git chama mudança em UTF-16 de binário e não emite hunks; como a spec promete esse texto no editor, o host escreve os dois lados decodificados em arquivos temporários privados (0600, nome único, guarda de `Drop` que remove em todo caminho de retorno) e pede os ranges ao `git diff --no-index -U0` sobre eles. Só inteiros pequenos voltam; o conteúdo não é devolvido por esse caminho. Se o próprio Git não conseguir dar os ranges, `full` cai com `unaligned`.
- **Regra de montagem (`buildCommitRows`)**, função pura em `CommitDiff.tsx`:
  - Cursor `old`/`new` começa em 1. Para cada hunk, as linhas entre o cursor e o bloco são contexto idêntico dos dois textos (`oldStart - old`), com o cuidado do hunk de contagem zero (`oldStart + 1 - old`, que é o caso de inserção no topo/EOF). O bloco do hunk pareia `base[oldStart..oldStart+oldCount]` com `current[newStart..newStart+newCount]` por índice (excesso fica `empty` do outro lado); linha pareada vira `modified` com `mark` intralinha (mesma regra de hoje). Sobra de ambos os lados depois do último hunk vira contexto até o fim.
  - Cursor/gaps negativos, hunk fora do arquivo, contagens que não fecham ou texto ausente onde `exists` é `true` → `null`, e o painel cai no `parseCommitPatch` atual (visão de hunks com o aviso).
  - `differences` = número de hunks; arquivo adicionado/deletado inteiro = 1 região (arquivo vazio = 0).
- **Visão split com conectores.** O painel esquerdo ganha um gutter do CodeMirror no lado `after` (`side: "after"`) desenhando um marcador por linha alterada: `removed` = metade esquerda vermelha, `added` = metade direita verde, `modified` = bloco azul inteiro. Como é gutter, o alinhamento é o da própria linha do editor — sem medir pixel, sem sincronizar rolagem extra. O gutter não aparece no modo unificado.
- **Visão unificada.** Um único CodeMirror read-only; o documento é a projeção das linhas (`- texto`, `+ texto`, ` texto`), uma linha por célula (par `modified` vira duas linhas). O gutter mostra o número da linha **nova**; linha removida não tem número. As marcações de linha (`git-diff-line-*`) e as marcas intralinha continuam. A navegação pulando diferença e a contagem são as mesmas do split.
- **Toggle de modo.** `Split | Unified` na toolbar, dois botões com `role="group"`/`aria-pressed`, `data-testid="commit-diff-mode"`. Default `Split`. A escolha persiste em `localStorage` na chave `openridr.commitDiffMode.v1` (`"split"` | `"unified"`), com o mesmo padrão defensivo de `openridr.layout.v1` (valor inválido ou storage indisponível volta ao default). Sem campo novo no host.
- **Hash do commit.** Linha nova `data-testid="commit-diff-commit"`, texto `HEAD <shortHash> · <subject>`, `title` com o hash completo. Omitida quando `commit` é `null` (repositório sem commits). Nenhum hash de blob é exibido como se fosse revisão.
- **Rótulos dos lados** (mesmos testids de hoje): esquerda `HEAD · <shortHash>` quando staged, `Index` quando unstaged, `No previous file` quando não há base; direita `Index` quando staged, `Worktree` quando unstaged, `No current file` quando o arquivo não existe mais no lado atual.
- **Nota do painel.** Arquivo inteiro: "Showing the full file from Git." Fallback: o texto de hoje ("Showing changed hunks and context from Git") mais a razão: "full file not available: binary file" / ":: too large to show in full" / ":: not valid UTF-8 text". A nota ganha `data-testid="commit-diff-note"`. Patch truncado no fallback continua dizendo "patch truncated by host".
- **Acessibilidade.** O editor unificado tem `aria-label="Unified diff editor"` (os dois do split mantêm os rótulos atuais); o toggle anuncia o modo; a contagem e a linha do commit são texto normal dentro do painel.
- **Contrato de testids.** Nenhum `data-testid` existente é renomeado ou removido. Novos: `commit-diff-mode`, `commit-diff-commit`, `commit-diff-note`, `commit-diff-unified`.

## Testing Decisions

- **O que faz um bom teste aqui:** ele observa o que o usuário vê — linhas do arquivo inteiro, conectores, modo, hash e rótulos — ou a montagem pura das linhas. Nada de afirmar campos internos que derivam a decisão.
- **Unidade (`tests/unit/commitDiff.test.ts`, `bun test`):** `buildCommitRows` com (a) modificação no meio de arquivo grande → todo o arquivo, gaps corretos antes/depois, `differences` = 1; (b) dois hunks → contexto entre eles; (c) arquivo adicionado → todas as linhas `added`, base ausente, 1 região; (d) arquivo deletado → todas `removed`, 1 região; (e) inserção no topo e inserção no EOF (contagem zero); (f) arquivo vazio; (g) hunk fora do arquivo/contagem que não fecha → `null`; (h) projeção unificada (par `modified` vira `-`/`+`, numeração nova, diferenças → linha do documento).
- **Host (`crates/host/src/git.rs`, `mod tests`):** `file_diff` devolve texto completo, hunks e commit `HEAD` de um repo real criado no teste; staged compara `HEAD → index` (base = HEAD, atual = index); unstaged com stage parcial compara `index → worktree` (base = index, não HEAD — o caso que separa o hash do commit do lado esquerdo); untracked → base ausente; deletado → atual ausente; binário (com `\0`) → `full: false`, `fullReason: "binary"`; arquivo acima de 1 MiB → `too_large`; caminho fora do repo → `invalid_state` (reaproveita `validate_path`); repositório sem commits → `commit: null`.
- **E2E (`tests/e2e/git.spec.ts`), host real + repo descartável do teste:** modificar uma linha no meio de um arquivo da fixture e abrir o diff → o arquivo inteiro aparece (linha do topo e do fim visíveis), conector presente, linha do commit igual a `git rev-parse --short HEAD`; toggle para unificado → um editor com `+`/`-`, mesma contagem, e a escolha persiste ao trocar de arquivo; arquivo novo → "No previous file" e conteúdo inteiro; arquivo deletado → "No current file" e o conteúdo do `HEAD`; os textos de fallback (binário) com um arquivo `\0` criado no repo.
- **App real:** `agent-desktop` no app Tauri sobre uma cópia em `/tmp` (do projeto de referência do `AGENTS.md`, sem capturar código privado), uma instância, aberta só para validar split, conectores, unificado e o hash, e fechada ao final. O que o ambiente não permitir entra como "não verificado".

## Out of Scope

- Abrir diff dos arquivos de um commit pelo painel Git log — issue própria, "Related to #48".
- Mudar a base do diff para `HEAD` no caso unstaged, ou exibir staged+unstaged juntos.
- Diff de imagem, prévia de binário, blame, histórico de uma linha.
- Redimensionar os dois lados pelo divisor, colapsar regiões não alteradas, filtros de whitespace.
- Reaproveitar o modo escolhido por arquivo (a preferência é global do painel).

## Further Notes

- "Connectors" é uma coluna entre os painéis: metade esquerda vermelha para linha só removida, metade direita verde para linha só adicionada, bloco azul para linha modificada. É o suficiente para ligar as regiões sem SVG nem posicionamento em pixel.
- O caso `Index → Worktree` é o que separa este spec da leitura literal do grill: o hash exibido é o do `HEAD` (contexto), e o rótulo do lado esquerdo continua honesto (`Index`).
- O fallback não é degradação escondida: a nota do painel diz por que o arquivo inteiro não está ali.
- Seams: o host (`git.fileDiff`) para os dados e o DOM do painel (`commit-diff*`) para o efeito; a função pura `buildCommitRows` para o alinhamento. Nenhum seam novo de UI.
