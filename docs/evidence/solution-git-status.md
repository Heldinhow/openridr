# Evidência — status Git e marcadores de symlink na Solution Explorer (#49)

Spec: [`docs/SPEC-SOLUTION-GIT-STATUS.md`](../SPEC-SOLUTION-GIT-STATUS.md). Tickets: #107 (symlink),
#108 (status Git), #109 (ignorados), #110 (esta evidência). Ambiente: macOS ARM64, host Rust +
UI React, worktree isolada. Data: 2026-11-06.

## O que foi provado, e como

| Camada | Prova | Resultado |
|---|---|---|
| Mapeamento e consultas puras | `bun test tests/unit/solutionGitStatus.test.ts` | 17 pass, 0 fail |
| Refresh serial | `bun test tests/unit/gitRefresh.test.ts` | 6 pass, 0 fail |
| Cache de listagem | `bun test tests/unit/solutionListing.test.ts` | 4 pass, 0 fail |
| Unidades da UI (todas) | `bun test tests/unit` | 78 pass, 0 fail |
| Host — symlink (`workspace.symlinks`) | `cargo test -p openridr-host -- workspace_symlinks` | 2 pass |
| Host — symlink (`workspace.tree`) | `cargo test -p openridr-host -- workspace_tree_marks` | 1 pass |
| Host — ignorado | `cargo test -p openridr-host -- ignored_` | 9 pass |
| Host — suíte completa (medida no commit `a1a9f48`, **não** repetida depois das correções da revisão) | `cargo test -p openridr-host` | 330 lib + 11 + 15 + 100 + 10 + 7 + 1 pass; 0 fail |
| Tipos + workspace | `bun run check` (`tsc --noEmit` + `cargo check --workspace --all-targets`) | ok |
| E2E | `tests/e2e/solution-git-status.spec.ts` | **escrito, não executado**; entrou em `scripts/e2e-smoke.txt` (decisão #67) |
| App desktop | `agent-desktop` sobre cópia em `/tmp` (bundle debug do HEAD, `OPENRIDR_HOME` isolado) | **verificado** — ver seção abaixo |

Comandos exatos (um filtro por invocação; `cargo test` aceita um filtro antes do `--`):

```sh
export PATH="$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT=$HOME/.dotnet
bun test tests/unit/solutionGitStatus.test.ts
bun test tests/unit/gitRefresh.test.ts
bun test tests/unit/solutionListing.test.ts
bun test tests/unit
cargo test -p openridr-host -- workspace_symlinks
cargo test -p openridr-host -- workspace_tree_marks
cargo test -p openridr-host -- ignored_
bun run check
```

A suíte completa do host foi medida uma vez no commit `a1a9f48` (linha acima); depois das
correções da revisão só os testes focados foram rodados, como pedido.

## Casos cobertos nos testes

**Mapeamento (`src/solutionGitStatus.ts`, 17 casos):** raízes iguais; workspace em subpasta do
repositório; caminho fora do workspace descartado; `conflicted` > `untracked` > `added` >
`modified`; `A` no index com worktree modificado continua `added`; renomeado/tipo alterado como
`modified`; rótulo granular preservado (`Staged + modified`); separador Windows; workspace na raiz
do filesystem; snapshot nulo/`failed`; `ignored` não sobrescreve estado já reportado e não muta o
mapa recebido. `treeQueryId` prova que a mesma lista de caminhos sob uma revisão nova (ou outro
workspace) é uma consulta nova, e que um newline no nome é dado — não separador.

**Refresh serial (`src/gitRefresh.ts`, 6 casos):** pedido feito com uma chamada em voo roda de novo
sob o workspace **atual** (não pinta a resposta de A em B); revogar trust limpa **antes** de a
leitura pendente resolver, não aplica o valor antigo e não dispara leitura nova enquanto
Restricted; reautorizar consulta de novo; falha de contexto antigo é descartada e o workspace atual
carrega; rajada de chamadas coalesce em um rerun; uma chamada em voo por vez.

**Cache de listagem (`src/solutionListing.ts`, 4 casos):** um reload reconsulta o nível aberto e o
tipo novo (link → arquivo) substitui o antigo; resposta iniciada antes do reload nunca entra no
cache novo; leituras concorrentes do mesmo nível dividem um pedido; nível com erro fica vazio
naquele epoch e é reconsultado depois do reload.

**Host, symlink:** link para arquivo, link para pasta, link pendente, link em ciclo (para um
ancestral dentro do workspace), link para fora do workspace e caminho comum; `workspace.symlinks`
sem trust, com lote, omitindo caminho inexistente e caminho através de link de pasta para fora;
link trocado por arquivo no mesmo caminho é relido como arquivo (sem cache por lista de paths);
`workspace.tree` lista um link como objeto (`symlink: true`, `dir: false`) sem consultar o alvo —
inclusive link para pasta externa, link para arquivo externo e link pendente em Restricted —,
recusa expandir link interno/ciclo (`invalid_state`), link para fora (`outside_workspace`) e
caminho abaixo de link (`not_found`).

**Host, ignorado:** arquivo coberto por `*.log` e arquivo dentro de pasta ignorada (`build/`)
respondem; arquivo rastreado que casa o padrão **não** responde; workspace em subpasta do repo;
caminho com espaço e com newline (prova do `-z`); nomes literais com `[`, `]`, `*`, `?` e prefixo
`:` (prova do `--literal-pathspecs`); regras mudam para a mesma lista de paths e a resposta muda;
registro truncado no meio de um nome não vira caminho; repositório ausente vira dado
(`not_a_repository`); índice corrompido vira `failed`; lote para em 256 caminhos.

## Contraste dos tokens de status

Tokens em `src/styles.css` (`:root`). Medição sobre as superfícies reais do tema escuro
distribuído: `--panel #2b2d30`, hover `#313336` (accent 4% sobre o painel), linha selecionada
`#43454a`, linha focada `#2e436e` e a linha `no index` (`--warning` 9% sobre o painel, `#3e3a2f`).
Alvo: **4.5:1** para texto (WCAG 1.4.3).

| Estado | Token | panel | hover | selecionada | focada | no index |
|---|---|---|---|---|---|---|
| `modified` | `#82c0f0` | 7.07 | 6.48 | 4.91 | 5.01 | 5.81 |
| `added` | `#8ad07d` | 7.50 | 6.88 | 5.21 | 5.32 | 6.16 |
| `untracked` | `#eda773` | 6.82 | 6.26 | 4.74 | 4.84 | 5.61 |
| `ignored` | `#b3b3b3` | 6.59 | 6.04 | 4.58 | 4.67 | 5.41 |
| `conflicted` | `#f9a1a1` | 7.00 | 6.42 | 4.86 | 4.96 | 5.75 |

O tema claro tem valores `light-dark()` medidos sobre `#fff` (6.1–8.2:1), mas **não é
distribuído**: os valores claros são forward-compatible e não foram verificados na tela, como já
registrado para os ícones.

A cor não é o único canal: cada linha tem a tag com a palavra do estado (`· modified`, `· added`,
`· untracked`, `· ignored`, `· conflicted`), o atributo `data-git-status` e a descrição AX
(`Git status: Modified`), sem mudar o nome acessível da linha. O `title` da tag carrega o rótulo
granular do painel Commit (`Staged + modified`, `Renamed`…). O fundo âmbar de `no index` e a
seleção continuam como estavam; a tinta é só no nome/tag.

Symlink: um link é **objeto link**. Na listagem ("Show all files") ele não tem tipo conhecido — o
alvo não é consultado — então a linha é folha, com glifo de corrente, `data-symlink="true"` e
descrição AX "Symbolic link — not followed"; um link que aparece como fonte avaliada (o MSBuild o
nomeia) mantém o glifo do arquivo e ganha o badge de corrente no canto. Nenhum link tem chevron ou
expande.

## Prova no app desktop real

Bundle `OpenRidr.app` construído do working tree já com as correções da revisão (`bun run tauri build --debug --bundles app`, 0.3.0-alpha.11),
lançado com `OPENRIDR_HOME` isolado e uma cópia da fixture pública em `/tmp` que o teste transformou
em repositório git descartável (commit inicial, `Order.cs` modificado, `Untracked.cs` novo,
`notes.log` coberto por `.gitignore`, `Linked.cs` → `Order.cs` e `LinkDir` → `src/OrderDesk.Core`).
Uma instância só (`pgrep -x openridr` = PID registrado); fechada por PID com `desktop.sh down`
(`stopped`, `pgrep` vazio).

| Ação | Estado observado |
|---|---|
| Abrir a solução e clicar em **Trust project** | `Trusted mode`; árvore com `Order.cs · modified`, `Untracked.cs · untracked`, `Linked.cs · untracked` |
| Ler a linha do symlink na árvore de acessibilidade | descrição `Symbolic link — not followed. Git status: Untracked`; badge de corrente no glifo |
| Ligar **Show All Files** | `notes.log · no index · ignored` (ignorado real coexistindo com `no index`); `global.json · no index` sem status (rastreado e limpo) |
| Clicar em `LinkDir` (link) | sem chevron, nenhum filho listado; o clique não expande |
| `git add -A && git commit` no terminal, depois ativar o Finder e voltar ao OpenRidr | tags `modified`/`untracked` somem (refresh no foco), `ignored` permanece |
| `git checkout -b feature` + modificar `Order.cs`, mesmo ciclo de foco | `Order.cs · modified` volta; barra mostra a branch `feature` |

Capturas (públicas; fixture do repositório, sem caminho pessoal):

- [`01-status-na-arvore.png`](solution-git-status/01-status-na-arvore.png) — modified, untracked e o symlink marcado.
- [`02-ignorado-e-symlink.png`](solution-git-status/02-ignorado-e-symlink.png) — `ignored` no `notes.log`; `LinkDir` ainda com glifo de pasta (antes da correção do ALTO).
- [`03-apos-checkout.png`](solution-git-status/03-apos-checkout.png) — status volta depois do checkout, na branch `feature`.
- [`04-link-objeto.png`](solution-git-status/04-link-objeto.png) — depois da correção: `LinkDir` com glifo de link (antes: glifo de pasta), sem chevron; `notes.log · ignored` e `Order.cs · modified` na mesma tela.

**Limite da prova:** as capturas são estados estáticos. As transições commit→foco→tags somem e
checkout→foco→`modified` volta foram **observadas ao vivo**, mas não ficaram gravadas como artefato
contínuo; valem como relato, não como prova preservada da sequência. O par antes/depois do link
(`02` × `04`) preserva a mudança visível da correção.

## O que não foi verificado

- **Integração React com respostas atrasadas.** O comportamento de descarte é provado nos helpers
  usados pela UI (`gitRefresh`, `solutionListing`), não por um teste de render (o repo não tem
  renderer de teste). O que o React faz com uma resposta que chega depois do unmount/troca de props
  não foi medido; fica como limite de prova, não como passe.
- **E2E.** O spec `tests/e2e/solution-git-status.spec.ts` foi escrito e entra no repositório, mas
  não rodou nesta sessão (Playwright só no smoke noturno, #67). O caminho real do Git (repo
  descartável, commit, checkout, `.gitignore`, symlink) foi provado no host por testes Rust e no
  app desktop; a integração ponta a ponta do spec só vale quando ele rodar.
- **Windows e macOS Intel.** Não executados.
- **Tema claro.** Não distribuído; valores não verificados.

Uma rodada da suíte completa, com a máquina carregada por outra sessão, teve 2 falhas nos testes
LSP (`lsp_loads…`, `lsp_completion…`); ambas passaram isoladas e a rodada seguinte, com a máquina
livre, foi 100/100. É a classe sensível a carga já registrada em
[`test-stability.md`](test-stability.md) (timeout de 5 s do produto para o csharp-ls), não uma
regressão deste trabalho (nenhum arquivo de LSP foi tocado).

## Limites conhecidos

- Só linhas de arquivo recebem status; pasta/projeto não agregam (fora de escopo, como aprovado).
- Mudança feita fora do app (terminal) aparece no foco da janela ou no Refresh do painel Git; não
  há watcher nem polling.
- Pasta symlink não é navegável pela árvore: os arquivos avaliados abaixo dela não aparecem na
  árvore (o alvo nunca é seguido).
- Com uma solução em subpasta do repositório, o mapeamento usa a raiz do `git.status`; o gutter do
  editor e `git.diff` continuam com o comportamento anterior (fora de escopo).
- `ignored` só é consultado em modo Trusted e para os caminhos visíveis; durante um filtro da
  árvore a tag `ignored` é omitida até o filtro ser limpo.
