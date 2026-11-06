# Evidência — status Git e marcadores de symlink na Solution Explorer (#49)

Spec: [`docs/SPEC-SOLUTION-GIT-STATUS.md`](../SPEC-SOLUTION-GIT-STATUS.md). Tickets: #107 (symlink),
#108 (status Git), #109 (ignorados), #110 (esta evidência). Ambiente: macOS ARM64, host Rust +
UI React, worktree isolada. Data: 2026-11-06.

## O que foi provado, e como

| Camada | Prova | Resultado |
|---|---|---|
| Mapeamento puro | `bun test tests/unit/solutionGitStatus.test.ts` | 14 pass, 0 fail |
| Unidades da UI (todas) | `bun test tests/unit` | 65 pass, 0 fail |
| Host — symlink | `cargo test -p openridr-host -- workspace_symlinks workspace_tree_marks` | 2 pass |
| Host — ignorado | `cargo test -p openridr-host -- ignored_` | 5 pass |
| Host — suíte completa | `cargo test -p openridr-host` | 330 lib + 11 + 15 + 100 + 10 + 7 + 1 pass; 0 fail |
| Tipos + workspace | `bun run check` (`tsc --noEmit` + `cargo check --workspace --all-targets`) | ok |
| E2E | `tests/e2e/solution-git-status.spec.ts` | **escrito, não executado** (decisão #67: Playwright só no smoke noturno) |
| App desktop | `agent-desktop` sobre cópia em `/tmp` (bundle debug do HEAD, `OPENRIDR_HOME` isolado) | **verificado** — ver seção abaixo |

Comandos exatos (o `cargo test` aceita um filtro antes do `--`; vários filtros vão depois dele):

```sh
export PATH="$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH" DOTNET_ROOT=$HOME/.dotnet
bun test tests/unit/solutionGitStatus.test.ts
cargo test -p openridr-host -- workspace_symlinks workspace_tree_marks
cargo test -p openridr-host -- ignored_
cargo test -p openridr-host
bun run check
```

## Casos cobertos nos testes

**Mapeamento (`src/solutionGitStatus.ts`, 14 casos):** raízes iguais; workspace em subpasta do
repositório; caminho fora do workspace descartado; `conflicted` > `untracked` > `added` >
`modified`; `A` no index com worktree modificado continua `added`; renomeado/tipo alterado como
`modified`; rótulo granular preservado (`Staged + modified`); separador Windows; workspace na raiz
do filesystem; snapshot nulo/`failed`; `ignored` não sobrescreve estado já reportado e não muta o
mapa recebido.

**Host, symlink:** link para arquivo, link para pasta, link pendente, link em ciclo (para um
ancestral dentro do workspace), link para fora do workspace e caminho comum; `workspace.symlinks`
sem trust, com lote, omitindo caminho inexistente e caminho através de link de pasta para fora;
`workspace.tree` devolve `symlink: true` e `dir: true` para o link de pasta, recusa expandir link
interno/ciclo (`invalid_state`), link para fora (`outside_workspace`) e caminho abaixo de link
(`not_found`).

**Host, ignorado:** arquivo coberto por `*.log` e arquivo dentro de pasta ignorada (`build/`)
respondem; arquivo rastreado que casa o padrão **não** responde; workspace em subpasta do repo;
caminho com espaço e com newline (prova do `-z`); repositório ausente vira dado
(`not_a_repository`); lote para em 256 caminhos.

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

Symlink: marcador de corrente no canto do glifo (`.symlink-mark`), `data-symlink="true"` e
descrição AX "Symbolic link — not followed"; pasta symlink não tem chevron e não lista o nível.

## Prova no app desktop real

Bundle `OpenRidr.app` construído deste HEAD (`bun run tauri build --debug --bundles app`, 0.3.0-alpha.11),
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
| Clicar em `LinkDir` (pasta symlink) | sem chevron/`aria-expanded`, nenhum filho listado; o clique não expande |
| `git add -A && git commit` no terminal, depois ativar o Finder e voltar ao OpenRidr | tags `modified`/`untracked` somem (refresh no foco), `ignored` permanece |
| `git checkout -b feature` + modificar `Order.cs`, mesmo ciclo de foco | `Order.cs · modified` volta; barra mostra a branch `feature` |

Capturas (públicas; fixture do repositório, sem caminho pessoal):

- [`01-status-na-arvore.png`](solution-git-status/01-status-na-arvore.png) — modified, untracked e o symlink marcado.
- [`02-ignorado-e-symlink.png`](solution-git-status/02-ignorado-e-symlink.png) — `ignored` no `notes.log` e `LinkDir` sem chevron.
- [`03-apos-checkout.png`](solution-git-status/03-apos-checkout.png) — status volta depois do checkout, na branch `feature`.

## O que não foi verificado

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
