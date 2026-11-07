# Evidência — glifo único na linha de symlink (#245)

Spec: [`docs/SPEC-SYMLINK-ROW-GLYPH.md`](../SPEC-SYMLINK-ROW-GLYPH.md). Tickets: #250 (glifo),
#254 (`GIT_OPTIONAL_LOCKS`). Follow-up: #264 (consulta de árvore sem retry). Ambiente: macOS ARM64,
worktree isolada, portas 1820/4717 (home do E2E derivada da porta da UI). Data: 2026-11-07.

## Causa raiz — não é runner-only

- Run noturno [42364384217](https://github.com/Heldinhow/openridr/actions/runs/42364384217) (`main`,
  2026-11-07): 1 failed / 27 passed em 4,8 min, em `solution-git-status.spec.ts:111` —
  `getByTestId('tree-file-LinkDir').locator('.icon-link')` com `Expected: 1`, `Received: 2`.
- Reprodução local idêntica **antes** do conserto: 1 failed / 2 passed, mesmo assert e mesmo
  `Received: 2`, nas portas 1820/4717. Determinístico, não é ambiente do runner.
- Causa: a linha de uma pasta symlink desenhava **dois** glifos de corrente — o ícone passado à mão
  pelo `SolutionTree` e o badge de canto que o `TreeRow` desenha para todo `symlink`. O desenho
  aprovado no #49 (`SPEC-SOLUTION-GIT-STATUS`, "Symlink na UI") diz que a linha da listagem é folha
  **com glifo de corrente** e que o badge de canto é para o link que aparece como fonte avaliada.

## O que foi feito

- **#250** (commit `6dc1c29`): uma regra no `TreeRow` — `symlink && !icon` desenha o glifo de
  corrente como ícone da linha, `symlink && icon` desenha o badge de canto; os dois call sites do
  `SolutionTree` deixaram de passar o ícone à mão. Spec E2E endurecido (`.symlink-mark` = 0 na linha
  do link de pasta). Sem prop nova, sem `data-testid` novo.
- **#254** (commit `81d4020`): os quatro spawns de `git` do host passam `GIT_OPTIONAL_LOCKS=0`, para
  o `git status` disparado no foco da janela não segurar `.git/index.lock` contra um `git commit`
  concorrente. Teste do host com um `git` de mentira no `PATH` (falha-antes:
  `GIT_OPTIONAL_LOCKS reached git as ["", "", "", "", "0"]`).

## Medições (portas 1820/4717)

| Medida | Antes | Depois |
|---|---|---|
| `solution-git-status.spec.ts`, uma rodada | 1 failed / 2 passed, determinístico (`.icon-link` = 2) | 3 passed |
| `--repeat-each=6` (18 execuções) | — | 17 passed; 1 falha em `a .gitignore match…` (`data-git-status` vazio em 10 s) |
| `--repeat-each=8` (24 execuções) | — | 24 passed |
| `git commit` do spec com `index.lock` | 1 falha em ~5 rodadas completas; mecanismo provado fora do Playwright (exit 128) | 0 em 42 execuções |
| glifo `.icon-link` = 1 | falha determinística | 0 falhas em 42 execuções |
| lista inteira do smoke (5 specs) | — | 28 passed (2,9 min) |

O mecanismo do `index.lock` foi provado à parte, num repositório descartável: `git status` em laço
faz `git add`/`git commit` concorrente falhar com `fatal: Unable to create '…/.git/index.lock': File
exists` (exit 128). Também medi que `git status` **degrada em silêncio** quando o lock já está preso
(exit 0 com `.git/index.lock` presente, índice read-only e `.git` read-only) — por isso o teste do
host observa o env do filho, não o exit code.

## Smoke noturno — `workflow_dispatch` na branch

SHA em todos os disparos: `9829c8389eedf8019f49a94a5e1d9981baf4a1fc`. O spec desta issue passou
**3/3 em todos os disparos**.

| Run | Resultado | O que falhou |
|---|---|---|
| [42412077351](https://github.com/Heldinhow/openridr/actions/runs/42412077351) | cancelada na fila, sem jobs | outra sessão disparou o mesmo workflow depois; o grupo `e2e-smoke` substitui o pendente mais antigo |
| [42412536089](https://github.com/Heldinhow/openridr/actions/runs/42412536089) | vermelha, 27 passed / 1 failed | `workspace.spec.ts:326` — corrida com o poll de 1,5 s do `checkDisk` (#273; 3/3 verde local) |
| [42413266179](https://github.com/Heldinhow/openridr/actions/runs/42413266179) | vermelha, 26 passed / 2 failed | `build.spec.ts:62` (`Build failed · exit 1` no runner, **#251**) e `run.spec.ts:352` (caiu junto da build, mesma família da #251) |
| [42414123022](https://github.com/Heldinhow/openridr/actions/runs/42414123022) | vermelha, 27 passed / 1 failed | `build.spec.ts:62` de novo, mesmo sintoma (**#251**) |

A lista inteira do smoke passa local em 28/28 (2,9 min) no mesmo SHA, então as falhas acima são de
outro terreno e têm dono: `workspace.spec.ts:326` corre contra o poll de 1,5 s do `checkDisk` (se o
poll atualizar o conflito antes do clique, não nasce o segundo banner) e está na **#273**;
`build.spec.ts:62` roda `dotnet build --no-restore` e sai 1 no runner logo depois de um restore que o
próprio teste afirma ter dado certo — é a **#251**, que outra sessão está instrumentando no runner
(nem o log do MSBuild nem os artefatos do Playwright saem do job, então o erro não é legível daqui).
`run.spec.ts:352` só apareceu junto da build quebrada, não é família do flake do workspace.

## O que não foi verificado

- **App Tauri real** (`agent-desktop`): mudança de UI pura, nada de shell nativo — a superfície é o
  Playwright em modo web.
- **Windows e macOS Intel.**
- **Flake da consulta de árvore** (`git.ignored` e `workspace.symlinks` sem retry): 1 falha em 42
  execuções; follow-up #264, sem quarentena (a taxa é baixa e o spec continua no smoke).
- **`spawn_bounded` e `run_with_deadline` do host:** receberam o env por inspeção; o teste do shim
  cobre `spawn_bytes` e `info()`.
- **`workspace.spec.ts:326` (#273) e `build.spec.ts:62` (#251) no runner:** fora do escopo desta
  issue e sem run verde para o AC literal — ver a seção acima.
