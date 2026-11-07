# SPEC — pre-commit proporcional ao diff

Issue-mãe: [#265](https://github.com/Heldinhow/openridr/issues/265). Origem: o hook de pre-commit roda
`bun run check` inteiro em todo commit, o que custa ~16 s de `cargo check` por uma linha de Markdown e
já empurrou uma sessão para `--no-verify` num amend de mensagem — atalho que o `AGENTS.md` proíbe. O
candidato nº 1 de melhoria do ambiente em [RETRO-CI-PRACTICES](RETRO-CI-PRACTICES.md). Worktree nova sem
`node_modules` morre no hook com `tsc: command not found` (exit 127), que fala de um binário ausente em
vez do que fazer.

## Problem Statement

Quem commita neste repositório paga o check inteiro, qualquer que seja o commit. Um commit só de
Markdown, um ajuste de um arquivo `.ts` e um amend que muda apenas a mensagem custam o mesmo
`tsc --noEmit && cargo check --workspace --all-targets`. O custo é o `cargo check` (~16 s), e é ele que
faz o `--no-verify` parecer razoável.

Numa worktree nova, antes do primeiro `bun install`, o mesmo hook falha com `tsc: command not found`:
a mensagem aponta um binário que não existe, não a ação que resolve (`bun install`).

## Solution

O hook de pre-commit passa a decidir pelo que **aquele commit** toca:

- só documentação → não roda nada;
- TypeScript/React (ou caminho que não é nem documentação nem Rust) → `tsc --noEmit`;
- Rust → `cargo check --workspace --all-targets`;
- os dois → os dois, como hoje.

Sem `node_modules` e com `tsc` a rodar, o hook falha antes de invocar o bun, com uma linha dizendo para
rodar `bun install`. Um commit só de documentação numa worktree sem `node_modules` passa.

`bun run check` continua existindo, com o mesmo conteúdo, para uso manual e para o CI: o hook filtra, não
substitui.

## User Stories

1. Como sessão que commita uma linha de Markdown, quero que o commit não rode `cargo check`, para não
   pagar ~16 s por uma mudança que o compilador não tem o que checar.
2. Como sessão que commita só TypeScript, quero que rode `tsc --noEmit` e não `cargo check`, para pegar
   o erro de tipo sem pagar o compilador Rust.
3. Como sessão que commita só Rust, quero que rode `cargo check` e não `tsc`, para não depender de
   `node_modules` numa mudança que não toca a UI.
4. Como sessão que commita UI e Rust juntos, quero os dois checks, para o commit misto não passar com
   metade das linguagens verificada.
5. Como sessão que faz um amend só de mensagem, quero que o hook não rode nada, para corrigir a
   mensagem não custar o check inteiro.
6. Como sessão que usa `scripts/commit-only.sh`, quero que o filtro olhe os caminhos daquele commit (não
   o index inteiro nem a árvore de trabalho), para o commit parcial ser classificado pelo que ele
   realmente leva.
7. Como sessão numa worktree nova sem `node_modules`, quero uma mensagem que diga para rodar `bun
   install`, para não perder a rodada decifrando `tsc: command not found`.
8. Como sessão numa worktree nova sem `node_modules` que commita só documentação, quero que o commit
   passe, para não ser obrigada a instalar a UI para versionar um `.md`.
9. Como sessão, quero ver na saída do hook qual check está rodando (ou que nada roda), para saber o que
   o commit está pagando.
10. Como mantenedor, quero o hook em POSIX `sh`, limpo em `shellcheck` e sob `dash`, para ele rodar
    igual no Mac e no `/bin/sh` do runner.
11. Como mantenedor, quero que `bun run check` continue sendo o check completo, para o comando manual e
    o CI não mudarem de contrato.
12. Como mantenedor, quero um teste de shell que rode o hook de verdade num repositório temporário, para
    os critérios acima serem provados por comportamento externo e não por inspeção do script.
13. Como mantenedor, quero o teste cobrindo o caminho do `git commit --only` do `commit-only.sh`, para o
    fluxo real de commit deste repositório não ficar sem prova.
14. Como mantenedor, quero a frase do `AGENTS.md` sobre o hook corrigida, para o arquivo que todo agente
    lê não afirmar algo que deixou de ser verdade.

## Implementation Decisions

- **Fonte dos caminhos: o diff do commit em andamento.** `git diff --cached --name-only` dentro do hook
  é exatamente o que está sendo commitado em todos os caminhos que este repositório usa: com `git commit
  --only` (o do `commit-only.sh`) o git aponta `GIT_INDEX_FILE` para um index temporário e o diff lista
  só os caminhos dados; com `-a`, idem; num amend só de mensagem o diff sai vazio. `git diff --name-only
  HEAD` (a alternativa óbvia) over-reporta: num commit `--only` de documentação com Rust sujo na árvore
  ele lista o Rust — é o custo de hoje. Verificado à mão num repositório descartável antes de escrever
  esta spec.
- **Nomes que o git cita e renomeações** (achados do code review). O diff é pedido com
  `core.quotePath=false` — um nome não-ASCII como `crates/host/src/ação.rs` sai cru, não como
  `"crates/a\303\247..."`, que não casaria com os padrões de Rust e pularia o `cargo check` — e com
  `--no-renames`, para uma movimentação listar os dois lados: tirar um `.rs` de `crates/` continua
  rodando `cargo check`. Caminho que ainda sai entre aspas (tab, aspas ou quebra de linha no nome) não é
  classificado: roda os dois checks.
- **Classificação por caminho, três baldes.** Documentação = `*.md` em qualquer diretório mais
  `docs/**`. Rust = `crates/**`, `src-tauri/**`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain*`. Qualquer
  outro caminho cai no balde do TypeScript. Não existe lista explícita de TS: "não é documentação nem
  Rust" já cobre `src/`, `tests/`, os configs da raiz e `package.json`, e é uma lista a menos para
  manter em dia.
- **Caminho ambíguo roda `tsc`, não o check completo** (decisão do orquestrador, 2026-11-07). O que
  sobra de ambíguo é `scripts/**`, `fixtures/**` (C#), `.githooks/**`, `.github/**`: `tsc` custa 0,8 s e
  pega um `package.json`/`tsconfig` quebrado; `cargo check` nesses caminhos não tem o que checar.
- **Sem limiar de "diff grande"** (decisão do orquestrador, 2026-11-07). O check é por linguagem, não por
  tamanho: um diff grande só de TS não tem Rust para checar, e um misto já roda os dois pela tabela. Um
  limiar em número de arquivos ou linhas só acrescentaria custo e um número inventado.
- **Um arquivo só: `.githooks/pre-commit`.** Nada de `scripts/pre-commit.sh` com o hook virando `exec`:
  o teste roda o arquivo que o git roda de verdade, então o hook real não fica sem cobertura.
- **Comandos: os mesmos do `bun run check`.** O hook invoca `bun run tsc --noEmit` e `cargo check
  --workspace --all-targets` conforme a tabela; `bun run check` fica intocado. Paridade de flags, sem
  caminho de código que só o hook conhece.
- **`node_modules` só no ramo do TypeScript.** O gate é a existência de `node_modules/.bin/tsc` (um
  `node_modules` pela metade dá o mesmo `command not found`), e a mensagem diz `rode bun install` e sai
  1. Commit só de documentação não passa por esse gate.
- **Saída do hook:** uma linha por check que roda (`pre-commit: tsc --noEmit`, `pre-commit: cargo check
  --workspace --all-targets`) e uma linha quando nada roda (`pre-commit: nada a checar`, sem dizer
  "só documentação": um amend de mensagem tem diff vazio e não mudou documento nenhum). É o que torna
  "as duas saídas ausentes" medível.
- **Se `git diff --cached` falhar**, o hook cai no `bun run check` completo — falha para o lado caro, que
  é o comportamento de hoje.
- **Limite conhecido, declarado no hook:** `cargo check --workspace --all-targets` não alcança
  `src-tauri` (workspace separado). É o mesmo limite de hoje; cobrir `src-tauri` no hook é outra issue.
- **`AGENTS.md` numa linha** (decisão do orquestrador, 2026-11-07): a regra de Commit diz que o hook roda
  `bun run check`; passa a dizer que ele roda o check proporcional ao diff e que `bun run check` é o
  completo. Isso torna o PR `risk:high`, e o merge é do mantenedor/orquestrador — custo aceito para o
  arquivo não afirmar algo falso.
- **Sem dependência nova, sem variável de ambiente nova, sem `data-testid`.** POSIX `sh`, `git`, `bun` e
  `cargo`, que o hook já usa.

## Testing Decisions

- **Seam:** o próprio `.githooks/pre-commit`, executado por um `git commit` de verdade num repositório
  git temporário, com `bun` e `cargo` falsos primeiro no `PATH` que só registram argv e exit code. É a
  fronteira de processo que já existe (mesmo molde de `tests/shell/doctor.test.sh`), então o hook não
  ganha variável só-de-teste e o teste prova o comportamento externo — quais comandos rodaram, com que
  argumentos, o exit code e a mensagem — não como os caminhos foram classificados.
- **Casos:** documentação só (nada roda); TypeScript só (`tsc`, sem `cargo`); Rust só (`cargo`, sem
  `tsc`); misto (os dois); amend só de mensagem (nada); sem `node_modules` com TS (mensagem com `bun
  install`, exit 1, nenhum `command not found`); documentação só sem `node_modules` (passa); um commit
  feito por `scripts/commit-only.sh` com Rust sujo na árvore (cobre o `git commit --only` e prova que a
  árvore não entra na classificação); um caminho Rust com nome não-ASCII; uma renomeação entre baldes; e
  o hook rodado sob `dash`, já que o `/bin/sh` do runner não é o do Mac.
- **O que um bom teste prova aqui:** o efeito observável do commit (quais comandos o hook invocou, o que
  saiu na tela, se o commit foi criado), nunca a forma do script. Um stub de `cargo` que falha também
  prova que o hook propaga o exit code do check.
- **Prior art:** `tests/shell/doctor.test.sh` (stubs primeiro no `PATH`, casos numerados, `set -eu`,
  contador de falhas, `*_SHELL=dash` para rodar sob o `/bin/sh` do runner).
- **Rodado à mão**, como os outros de `tests/shell/`: nenhum job do CI os executa hoje. Ligar
  `tests/shell/*.test.sh` no CI é a [#269](https://github.com/Heldinhow/openridr/issues/269), fora
  desta frente.

## Out of Scope

- Ligar `tests/shell/` no CI (issue #269).
- Cobrir `src-tauri` no `cargo check` do hook (workspace separado; o job `rust-tauri` do CI é o dono
  disso, e é informativo).
- Limiar de tamanho de diff (decidido contra; ver Implementation Decisions).
- Mudar `bun run check`, o CI ou o `scripts/commit-only.sh`.
- Windows e macOS Intel (não verificados; o alvo é macOS ARM64).

## Further Notes

- Medições desta máquina (macOS ARM64, TS 7.0.2): `bun run tsc --noEmit` = 0,8 s; `cargo check
  --workspace --all-targets` ≈ 16 s (o número da issue). O ganho do filtro é quase todo no lado do
  Rust.
- O hook é `#!/bin/sh`: `shellcheck` e `dash` limpos, como o `AGENTS.md` pede para `scripts/`.
- Nenhum `--no-verify` em nenhum momento desta implementação: o caminho honesto é o hook ficar barato o
  bastante para ninguém querer o atalho.
