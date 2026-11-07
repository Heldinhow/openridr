# Evidência — pre-commit proporcional ao diff (#265, ticket #271)

Máquina: macOS ARM64, `bun` 1.x, TypeScript 7.0.2, `dash` e `shellcheck` 0.11.0 do Homebrew. Branch
`chore/issue-265-pre-commit-proportional`, commits `410d0a7` (hook + teste + `AGENTS.md`) e `fe88b88`
(mensagem neutra quando o diff não tem caminho). Spec: [SPEC-PRECOMMIT-PROPORTIONAL](../SPEC-PRECOMMIT-PROPORTIONAL.md).

## Teste do hook (`tests/shell/pre-commit.test.sh`)

O hook é o seam: cada caso faz um `git commit` de verdade num repositório temporário, com `bun` e
`cargo` falsos primeiro no `PATH` que só registram argv. Nada é compilado.

| Comando | Resultado |
|---|---|
| `sh tests/shell/pre-commit.test.sh` | `all cases passed` (12 casos), exit 0 |
| `dash tests/shell/pre-commit.test.sh` | `all cases passed`, exit 0 |
| `PRE_COMMIT_HOOK=<hook antigo> sh tests/shell/pre-commit.test.sh` | **falha-antes**: 18 `FAIL`, exit 1 |
| `shellcheck .githooks/pre-commit tests/shell/pre-commit.test.sh` | limpo, exit 0 |
| `dash -n .githooks/pre-commit` | exit 0 |

Casos cobertos: só documentação (nada roda); TypeScript só (`tsc`, sem `cargo`); Rust só (`cargo`, sem
`tsc`, sem `node_modules`); misto (os dois, nesta ordem); amend só de mensagem (nada roda); TypeScript
sem `node_modules` (falha antes do bun, diz `bun install`, nunca `command not found`, commit não é
criado); só documentação sem `node_modules` (passa); `scripts/commit-only.sh` com Rust sujo na árvore
(o commit leva só o `.md` e nada roda — prova que a árvore de trabalho não entra na classificação);
check que falha (o exit code do check vira exit do hook e o commit não é criado); um caminho Rust com
nome não-ASCII; uma renomeação entre baldes; o hook rodado sob `dash`.

## Achados do code review (corrigidos)

O review (subagente, `gpt-6.1-terra`) achou dois caminhos em que o hook ficava **mais fraco** que o
antigo, e os dois foram corrigidos no hook e pinados por teste:

1. **Alto — nome não-ASCII em `crates/` pulava o `cargo check`.** `git diff --name-only` cita o caminho
   (`"crates/a\303\247..."`), a citação não casa com `crates/*` e o commit caía no balde do TypeScript:
   o Rust não era checado. Correção: `core.quotePath=false` no diff, mais um ramo que, para um caminho
   que ainda sai entre aspas (tab, aspas ou quebra de linha no nome), roda os dois checks em vez de
   classificar. Caso 11 do teste.
2. **Médio — renomeação entre baldes escondia o lado Rust.** Com detecção de renomeação, mover
   `crates/host/src/lib.rs` para `docs/lib.rs` listava só o destino e o commit passava como documentação,
   sem `cargo check`. Correção: `--no-renames`, que lista os dois lados. Caso 12 do teste.

Prova falha-antes do conserto: `PRE_COMMIT_HOOK=<hook de 410d0a7> sh tests/shell/pre-commit.test.sh`
→ 6 `FAIL`, incluindo `non-ASCII rust: expected exit 0, got 1` (a mensagem de `bun install` no lugar do
`cargo check`) e `rename: expected 'cargo check --workspace --all-targets', got ''`. Depois do conserto:
`all cases passed` sob `sh` e sob `dash`; contra o hook original (`exec bun run check`), 21 `FAIL`.

## Medições reais (commit de verdade, repositório descartável com `CARGO_TARGET_DIR` do worktree)

| Commit | Hook antigo | Hook novo | O que rodou |
|---|---|---|---|
| só Markdown (`docs/README.md`) | **11.620 ms** | **108 ms** | nada (`pre-commit: nada a checar`) |
| só TypeScript (`src/main.tsx`) | — | 841 ms | `pre-commit: tsc --noEmit` |
| só Rust (`crates/host/src/lib.rs`), sem `node_modules` | — | 4.036 ms | `pre-commit: cargo check --workspace --all-targets` |
| misto (TS + Rust) | — | 1.375 ms | as duas linhas, `tsc` e depois `cargo check` |
| TypeScript sem `node_modules` | — | 96 ms, exit 1 | `pre-commit: node_modules/.bin/tsc não existe, rode bun install` (0 ocorrências de `command not found`) |
| amend que só muda a mensagem | — | 437 ms | nada (`pre-commit: nada a checar`) |

No worktree real, o commit `0486ab4` (hook + teste + spec, caminho TypeScript) imprimiu só
`pre-commit: tsc --noEmit` em 2 s de parede — nenhuma linha de `cargo`.

## Bateria completa (uma vez, no fim)

| Comando | Resultado |
|---|---|
| `bun run check` | exit 0 (7,9 s com `target/` quente) |
| `bun test tests/unit` | 123 pass, 0 fail |

## Não verificado

- **`tests/shell/` não roda no CI**: nenhum job executa esses testes hoje (o `governance` roda
  `actionlint`/`zizmor`). Ligar é a [#269](https://github.com/Heldinhow/openridr/issues/269); aqui o
  teste foi rodado à mão, sob `sh` e sob `dash`.
- **`src-tauri` continua fora do `cargo check` do hook**: `cargo check --workspace --all-targets` não
  alcança o workspace separado. É o mesmo limite de antes, declarado em comentário no hook; quem cobre
  `src-tauri` é o job `rust-tauri` do CI (informativo).
- **Windows e macOS Intel**: não executados.
- **Sem E2E e sem app real**: a mudança não toca UI, host nem o app — é o hook de git. Nenhuma superfície
  do `verify-openridr` (Playwright, navegador, app Tauri) se aplica; a prova é o teste de shell e as
  medições acima.
