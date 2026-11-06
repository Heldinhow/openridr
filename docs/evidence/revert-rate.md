# Revert rate — quantos PRs mergeados precisaram ser desfeitos

Medida de calibragem do processo de revisão ([SPEC-RISK-TIERS](../SPEC-RISK-TIERS.md)): se o gate de
risco e a revisão estão deixando passar coisa que depois é revertida, este número sobe. Uma linha por
mês, produzida por comando (nunca à mão):

```sh
sh scripts/revert-rate.sh --month 2026-10
```

O script consulta o GitHub (PRs mergeados na janela + commits da `main` na janela), imprime a linha da
tabela e escreve a contagem de revertes não atribuídos no stderr. Rodar é do mantenedor: precisa de
token e não roda no CI.

## Como o número é contado

- **Mergeados**: PRs com `mergedAt` dentro do mês.
- **Revertidos**: commits da `main` no mês cuja mensagem é um revert (`Revert "…"`, tipo convencional
  `revert:`/`revert(escopo):`, ou `This reverts commit …`) **e** que referenciam `(#N)` de um PR
  mergeado na mesma janela. Duas reversões do mesmo PR contam **uma**.
- **Revert de revert** conta contra o PR da reversão — é o PR que de fato mudou a `main` daquela vez.
- **Reverte sem referência** (nem `(#N)`, nem PR dentro da janela) aparece como
  `unattributed=N` no stderr e **não** entra na taxa: melhor um número honesto com aviso do que uma
  atribuição inventada.

## Tabela

| Mês | Mergeados | Revertidos | Taxa |
|---|---|---|---|
| 2026-10 | 1 | 0 | 0.0% |
| 2026-11 (em curso, medido em 2026-11-06) | 55 | 0 | 0.0% |

Comandos e saída medidos em 2026-11-06 (`gh` 2.83.1, autenticado):

```
$ sh scripts/revert-rate.sh --month 2026-10
| 2026-10 | 1 | 0 | 0.0% |
revert-rate: merged=1 reverted=0 unattributed=0 (2026-10)

$ sh scripts/revert-rate.sh --month 2026-11
| 2026-11 | 55 | 0 | 0.0% |
revert-rate: merged=55 reverted=0 unattributed=0 (2026-11)
```

Leitura: o repositório não tem **nenhum** commit de revert na `main` (493 commits em 2026-11-06,
`git log --grep='Revert'` vazio), então 0% é o valor real, não um falso negativo do classificador. A
linha de novembro é de um mês em curso e será substituída pela medição do mês fechado.

## O que este número não prova

- Revert é um sinal grosseiro: PR que quebrou e foi corrigido por outro commit (sem revert) não aparece.
- Um revert feito por force-push da própria branch não deixa commit na `main`.
- A taxa só fica informativa com alguns meses; a primeira linha completa é 2026-10 (um único PR).
