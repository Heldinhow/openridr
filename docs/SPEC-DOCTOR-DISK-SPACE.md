# SPEC — doctor.sh reporta espaço livre antes do E2E

Issue-mãe: [#259](https://github.com/Heldinhow/openridr/issues/259). Origem: a sessão da #217 perdeu a
rodada com `ENOSPC` no `globalSetup` do E2E (127 Mi livres) e só entendeu depois de rodar `df`
([RETRO-E2E-EVIDENCE-OUTPUT](RETRO-E2E-EVIDENCE-OUTPUT.md)). Em 2026-11-07 o disco interno chegou a
201 MB livres e travou outro E2E; a causa foram ~4.600 pastas `.tmp*` (~18 GB) deixadas por testes do
host no `$TMPDIR`.

## Problem Statement

O `doctor.sh` do skill `verify-openridr` é o passo que existe para descobrir um ambiente quebrado
**antes** de perder uma rodada: ele checa bun, cargo, `node_modules`, SDK 8.0.408, Chrome, portas
ocupadas e as permissões do `agent-desktop`. Ele não olha espaço em disco.

O E2E escreve no diretório temporário do sistema: um app home por porta da UI e o diretório de
capturas. Um disco quase cheio não aparece como "disco cheio" — aparece como `ENOSPC` no meio de uma
cópia de arquivo, minutos depois do começo do run, e a sessão perde a rodada inteira. Sem um número na
saída do doctor, a sessão também não sabe distinguir "o disco está apertado" de "o E2E quebrou".

## Solution

O `doctor.sh` passa a imprimir, no mesmo formato dos outros checks, uma linha com o espaço livre em MiB
no volume do diretório temporário, o piso usado e o motivo do piso. Acima do piso a linha é `ok`; entre
os dois pisos é `note`; abaixo do piso duro é `FAIL` (e o doctor sai 1, como em qualquer requisito duro
faltando). O número medido aparece na linha nos três casos, para a sessão decidir com informação em vez
de adivinhar.

O script continua **somente leitura**: ele mede, não apaga.

## User Stories

1. Como sessão prestes a rodar E2E, quero que o doctor imprima o espaço livre no volume do diretório
   temporário com o número medido, para notar um disco apertado antes de o run morrer com `ENOSPC`.
2. Como sessão, quero que a linha diga o piso e o motivo dele, para não precisar redescobrir por que
   2 GB é "pouco" nesta máquina.
3. Como sessão com espaço abaixo do piso duro, quero `FAIL` e exit 1, para que o doctor bloqueie o run
   em vez de deixá-lo morrer no meio de um download.
4. Como sessão com espaço entre os dois pisos, quero `note` e não `FAIL`, para poder rodar um spec
   único sabendo do risco.
5. Como sessão, quero o diretório medido impresso na linha, para saber qual volume está apertado (o
   interno, onde fica o `$TMPDIR`, e não o externo onde fica o worktree).
6. Como sessão com disco folgado, quero a linha `ok` com o número, para o check não virar ruído.
7. Como sessão, quero que o resto do doctor continue igual, para um aviso de disco não esconder um SDK
   ou um `node_modules` faltando.
8. Como sessão, quero que a medição seja somente leitura, para rodar o doctor a qualquer momento sem
   risco de perder trabalho.
9. Como mantenedor, quero os pisos e o motivo escritos no script e no `SKILL.md`, para a próxima sessão
   não redescobrir os números.
10. Como mantenedor, quero um teste que force cada severidade sem encher um disco, para provar as três
    linhas e o exit code.
11. Como mantenedor, quero que uma medição que falha (`df` ausente ou saída estranha) vire `note` com o
    motivo, nunca um `ok` silencioso.

## Implementation Decisions

- **Volume medido:** o do diretório temporário (`${TMPDIR:-/tmp}`), que é onde vivem o app home do E2E
  e o diretório de capturas. O volume do repositório (`target/`, `node_modules`) fica fora: é outra
  pergunta e outra issue.
- **Medição:** `df -Pk` sobre o diretório temporário; a coluna `Available` (1024-blocos) dividida por
  1024 dá MiB inteiro. `-P` porque o `df` do BSD muda o layout das colunas sem ele.
- **Pisos** (decisão do orquestrador, 2026-11-07): `ok` ≥ 3072 MiB; `note` entre 1024 e 3071 MiB;
  `FAIL` < 1024 MiB. Motivo: até 4 sessões em paralelo, cada uma com um app home de E2E (~200 MiB) mais
  caches e builds; o orquestrador já manda sessões pararem abaixo de 3 GB.
- **Exit code:** `FAIL` soma ao contador de falhas do script (exit 1), como os outros requisitos duros.
  `note` não muda o exit code.
- **Formato da linha:** uma linha, MiB inteiro, o diretório medido, o piso e o motivo — no mesmo
  formato `ok`/`note`/`FAIL` dos outros checks, em inglês como o resto do script.
- **Falha de medição:** se `df` não existir ou a saída não tiver número, a linha sai como `note` com o
  motivo; nunca `ok`.
- **Posição:** junto dos outros checks de ambiente, antes da validação do bundle.
- **Documentação do piso:** comentário no próprio script, ao lado do check, e o parágrafo do Doctor no
  `SKILL.md` do `verify-openridr` (que é o que a sessão lê antes de rodar).
- **Sem dependência nova, sem variável de ambiente nova, sem limpeza:** POSIX sh e `df`, que já existem.
- **Índice:** linha nova em `docs/README.md` (Feature → docs), como manda a convenção.

## Testing Decisions

- **Seam:** o `df` resolvido pelo `PATH`. É um seam que já existe (fronteira de processo), então o
  script não ganha caminho só-de-teste e o teste não precisa de disco cheio de verdade.
- **Teste:** um teste de shell em `tests/shell/`, no molde dos testes de script que já existem lá
  (`sh tests/shell/<nome>.test.sh`, também sob `dash`). Ele põe um `df` falso primeiro no `PATH`, monta
  uma árvore de repositório mínima (para o check de `node_modules` passar) e stubs para as outras
  ferramentas do ambiente, de modo que o exit code dependa só do caso sob teste.
- **Casos:** acima do piso (linha `ok`, exit 0); entre os pisos (linha `note`, exit 0); abaixo do piso
  duro (linha `FAIL`, exit 1). Em todos, o número medido aparece na linha.
- **O que um bom teste prova aqui:** o comportamento externo do script (stdout e exit code), não como o
  MiB é calculado. O teste não roda o doctor contra o disco da máquina.
- **Prior art:** `tests/shell/e2e-smoke-issue.test.sh` (stub de `gh` no PATH, casos numerados, `set -eu`,
  contador de falhas).

## Out of Scope

- Apagar ou limpar o que está no `$TMPDIR` (o doctor é somente leitura; o episódio das ~4.600 pastas
  `.tmp*` é vazamento dos testes do host, não deste check).
- Contar as pastas `.tmp*` órfãs ou medir o tamanho delas (decisão do orquestrador: issue própria).
- Medir o volume do repositório ou o volume externo do worktree.
- Fazer o harness do Playwright checar disco por conta própria.
- Windows e macOS Intel (não verificados; o alvo é macOS ARM64).

## Further Notes

- O piso não é sobre "caber um run": é sobre caber **quatro** runs em paralelo mais a margem que o
  orquestrador usa para mandar as sessões pararem. Se o número de sessões paralelas mudar, o piso muda
  com ele — por isso o motivo fica escrito ao lado do número.
- `df -Pk` no macOS reporta o volume de dados (`/System/Volumes/Data`), que é o que importa para
  `/var/folders`; o worktree desta sessão está em outro volume (externo), e é por isso que a linha
  imprime o diretório medido.
