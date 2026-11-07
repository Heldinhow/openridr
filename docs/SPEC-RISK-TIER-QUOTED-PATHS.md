# SPEC — Risk tier lê o caminho cru que o git cita (issue #276)

Status: ready-for-agent
Fonte: issue GitHub #276 (Heldinhow/openridr) + grill de 2026-11-07 com o orquestrador (Q1–Q5).
Tickets: #276 (a própria issue; esta frente tem um ticket só).
Alvo: macOS Apple Silicon. Windows e macOS Intel continuam fora do que foi executado.

## Problem Statement

O classificador de risco pede ao git a lista de caminhos do PR e o git **cita** o caminho com caractere
não-ASCII (`core.quotePath` é `true` por default). `docs/evidência.md` chega como
`"docs/evid\303\252ncia.md"`, com aspas e escapes octais; nenhum padrão da tabela casa, a linha cai no
default e o tier sobe. Medido em 2026-11-07 neste repositório, com o script desta `main`, num repositório
sintético:

```text
$ git diff --name-only --no-renames main...feat
"docs/evid\303\252ncia.md"

$ sh scripts/ci-risk-tier.sh main feat
medium: "docs/evid\303\252ncia.md" (unmatched path)
medium
```

O esperado é `low`: é documentação. A consequência é operacional, não cosmética — um PR só de
documentação (ou de `tests/**`, ou de `*.css`) com nome não-ASCII não recebe `risk:low`, o job `risk` não
arma o auto-merge e o mantenedor é chamado à toa. Onde o tier correto já é `medium` (`src/**`,
`crates/**`) o defeito é invisível, porque o default coincide com a resposta certa.

A classe já foi fechada no hook de pre-commit (#265/#271): lá o caminho citado pulava o `cargo check`.
O classificador de risco já usa `--no-renames`; falta a citação. Este script é a última ocorrência de
`git diff --name-only` no repositório fora do hook.

## Solution

O classificador pede o diff com `core.quotePath=false`, então um nome não-ASCII sai cru e casa com a
tabela: `docs/evidência.md` vira `low`. Para o caminho que o git **ainda** cita mesmo assim — tab, aspas
ou quebra de linha no nome, que nenhuma configuração deixa sair cru — o comportamento é declarado:
o tier é `high`, com motivo próprio (`git quoted the path`). Um caminho que o classificador não consegue
ler não é um caminho de documentação: falha fechado, como já acontece em erro de git.

Os caminhos continuam sendo lidos um por linha, então um nome com espaço (`docs/guia de estilo.md`)
permanece um caminho só e é classificado como documentação.

## User Stories

1. Como mantenedor, quero que um PR que só toca `docs/<nome não-ASCII>.md` receba `risk:low`, para o
   auto-merge armar sem mim num PR que um check já cobre.
2. Como mantenedor, quero o mesmo para `tests/**` e `*.css` com nome não-ASCII, para a correção não valer
   só para uma família de caminho.
3. Como mantenedor, quero que um caminho de produto com nome não-ASCII (`crates/host/src/ação.rs`)
   continue `medium`, para a correção não baixar o tier de código de produto.
4. Como mantenedor, quero que um caminho com espaço no nome seja classificado pelo nome inteiro, para
   `docs/guia de estilo.md` não virar `docs/guia` (documentação) e `de`/`estilo.md` (desconhecido).
5. Como mantenedor, quero que um caminho que o git cita de qualquer jeito (tab, aspas ou quebra de linha)
   resulte em `high`, para que um nome que o classificador não lê nunca caia em tier baixo por acidente.
6. Como mantenedor, quero que esse `high` apareça no motivo de stderr (`git quoted the path`), para eu
   distinguir um caminho ilegível de uma mudança de CI de verdade quando o gate me chamar.
7. Como sessão de agente, quero que o autoteste do classificador tenha um caso de documentação com nome
   não-ASCII esperando `low`, para a falha-antes provar o defeito e o passa-depois provar a correção.
8. Como sessão de agente, quero o caso de nome com espaço no autoteste, para a leitura por linha ter prova
   de regressão em vez de confiança.
9. Como sessão de agente, quero o caso de caminho citado (tab) esperando `high`, para o comportamento
   declarado não depender de alguém lembrar da regra.
10. Como sessão de agente, quero rodar o autoteste sob `sh` e sob `dash`, para o `/bin/sh` do runner não
    ser uma aproximação que passa só no Mac.
11. Como mantenedor, quero a regra escrita na spec do risco e uma linha em `decisions.md`, para a próxima
    sessão saber por que existe um `high` que não vem da tabela de caminhos.
12. Como mantenedor, quero `AGENTS.md` intocado, para a correção não mexer no arquivo que todo agente lê
    quando ele não afirma nada falso sobre citação.
13. Como mantenedor, quero zero dependência nova, zero script novo e nenhum `data-testid` tocado, para esta
    frente ser uma flag, um ramo e os casos de teste.

## Implementation Decisions

- **Fonte dos caminhos: o mesmo diff de hoje, com a citação desligada.** O classificador passa a pedir
  `git -c core.quotePath=false diff --name-only --no-renames "$base...$head"` — a mesma flag do hook de
  pre-commit (#265). `--no-renames` continua: um rename lista os dois lados, então apagar `trusted.rs` não
  lê como "caminho desconhecido".
- **Por que não `-z`.** `git diff -z` é imune a citação por construção, mas a leitura NUL-delimitada exige
  `read -d ''`, que o `dash` — o `/bin/sh` do runner — não tem. A flag resolve o caso que existe (nome
  não-ASCII) sem trocar a forma de ler.
- **Caminho que ainda sai citado é `high`, declarado.** Depois de `core.quotePath=false` o git ainda cita
  um nome com tab, aspas ou quebra de linha (medido: `"docs/tab\there.md"`). O ramo que reconhece a linha
  começando por aspas emite `high` com o motivo `git quoted the path` e não tenta despir as aspas: um
  caminho ilegível não é documentação, e tier baixo arma auto-merge. É a mesma política do hook, que nesse
  caso roda **os dois** checks.
- **O loop de leitura fica como está.** `IFS` com quebra de linha, `for path in $paths` e `set -f` já leem
  um caminho por linha, sem divisão por espaço e sem glob. Medido: os dois estilos se comportam igual
  aqui, então a mudança não toca o loop — só acrescenta o ramo das aspas. O caso de nome com espaço entra
  no autoteste como prova de regressão.
- **Um script só.** `scripts/ci-risk-tier.sh` é a única ocorrência de `git diff --name-only` fora do hook;
  `scripts/ci-aggregate.sh`, `scripts/ci-risk-gate.sh` e `scripts/revert-rate.sh` não classificam caminho.
  Nada de refatorar os quatro juntos.
- **`AGENTS.md` não muda.** Ele diz que a tabela de risco tem uma fonte só (o script mais o teste), o que
  continua verdade; a regra nova entra na spec do risco e em `decisions.md`.
- **Sem dependência nova, sem script novo, sem mudança no workflow.** O job `risk` continua extraindo o
  classificador de `origin/<base>` e chamando com as mesmas refs.

## Testing Decisions

Teste bom aqui prova comportamento externo: um tier está certo quando o script imprime o esperado para um
conjunto de caminhos reais num repositório git de verdade. Nada de chamar função interna.

- **Seam existente, nenhum seam novo**: `scripts/ci-risk-tier-selfcheck.sh`, que já cria repositórios
  sintéticos em `mktemp` e roda o script de verdade, um caso por linha de saída. É o mesmo seam do hook de
  pre-commit (`tests/shell/pre-commit.test.sh`), que já cobre nome não-ASCII e nome com espaço.
- **Casos novos** (todos no autoteste):
  - `docs/<nome não-ASCII>.md` → `low` (documentação);
  - `crates/host/src/<nome não-ASCII>.rs` → `medium` (caminho de produto continua no tier da tabela);
  - `docs/<nome com espaço>.md` → `low` (leitura por linha: o nome inteiro, não pedaços);
  - `docs/<nome com tab>.md` → `high` com motivo `git quoted the path` (comportamento declarado).
- **Falha-antes / passa-depois**: o autoteste rodado contra o script de antes da correção tem que ficar
  vermelho no caso não-ASCII (e no do tab, que hoje cai em `medium`); depois da correção, verde. A prova
  vai para a evidência com o comando e a saída.
- **Dois shells**: `sh scripts/ci-risk-tier-selfcheck.sh` e `dash scripts/ci-risk-tier-selfcheck.sh` — o
  runner usa `dash`, o Mac usa `bash` como `/bin/sh`.
- **Regressão do resto**: os 45 casos atuais continuam verdes; `sh tests/shell/ci-risk-gate.test.sh` e
  `GATE_SHELL=dash sh tests/shell/ci-risk-gate.test.sh` também (o gate não muda, mas o classificador é
  entrada dele). `shellcheck --shell=sh` nos dois arquivos tocados.

## Out of Scope

- Trocar a leitura para NUL-delimitado (`-z`): exigiria `bash` no script.
- Despir as aspas e classificar o caminho citado: a decisão é falhar fechado.
- Ligar `tests/shell/*.test.sh` no CI: é a issue #269, não esta frente.
- Mudar a tabela de risco, o gate, o agregador, o workflow ou o auto-merge.
- Caminhos não-ASCII em `Cargo.toml`/`package.json` (a leitura de manifesto passa a receber o caminho cru
  de graça, mas não há caso novo para ela).

## Further Notes

- Custo aceito: um PR que toque um nome com tab/aspas/quebra de linha passa a exigir aprovação do
  mantenedor. É raro e preferível a um tier baixo por acidente.
- Evidência em `docs/evidence/risk-tier-quoted-paths.md`; retrospectiva em
  `docs/RETRO-RISK-TIER-QUOTED-PATHS.md`.
- Lição herdada da #265 que esta frente reusa: um guardrail que decide por caminho precisa desconfiar do
  formato em que o git entrega os caminhos — citação e renomeação são transformações silenciosas, e o
  default tem que ser "checar mais", nunca "checar menos".
