# Evidência — risk tier lê o caminho cru que o git cita (issue #276)

Data: 2026-11-07. Alvo: macOS ARM64. Nenhuma interface, host ou app tocado: é o classificador de risco.

## O que mudou

`scripts/ci-risk-tier.sh` pede o diff com `git -c core.quotePath=false diff --name-only --no-renames
"$base...$head"` — a mesma flag do hook de pre-commit (#265) — e ganhou um ramo `'"'*` no `case`: o
caminho que o git cita de qualquer jeito (tab, aspas ou quebra de linha no nome) vira `high` com o motivo
`git quoted the path`. O loop de leitura, o `--no-renames`, a tabela de caminhos, o gate, o agregador e o
workflow ficaram intocados.

`scripts/ci-risk-tier-selfcheck.sh` ganhou seis casos (45 → 51): `unicode-docs` (`low`),
`unicode-product` (`medium`), `unicode-product-reason` (casou com `crates/*`, não com o default),
`space-in-name` (`low`), `quoted-tab` (`high`) e `quoted-tab-reason` (o motivo no stderr).

## Comandos e resultados

| Comando | Resultado |
|---|---|
| `sh scripts/ci-risk-tier-selfcheck.sh` | `51 cases passed`, exit 0 |
| `dash scripts/ci-risk-tier-selfcheck.sh` | `51 cases passed`, exit 0 |
| `sh tests/shell/ci-risk-gate.test.sh` | `all cases passed`, exit 0 |
| `GATE_SHELL=dash sh tests/shell/ci-risk-gate.test.sh` | `all cases passed`, exit 0 |
| `shellcheck --shell=sh scripts/ci-risk-tier.sh scripts/ci-risk-tier-selfcheck.sh` | limpo, exit 0 |
| `sh scripts/ci-risk-tier.sh origin/main HEAD` neste branch | `low` para os dois `docs/**` e `high` para os dois `scripts/ci-*`; tier final `high` (esperado: esta frente toca `scripts/ci-*`) |

### Falha-antes (o selfcheck novo contra o script de antes)

`git show f38236f:scripts/ci-risk-tier.sh` (a `main` de antes) num diretório temporário, ao lado do
selfcheck novo:

```text
FAIL unicode-docs: expected low, got medium (exit 0)
    medium: "docs/evid\303\252ncia.md" (unmatched path)
FAIL unicode-product-reason: the path fell to the default instead of the crates/* rule
FAIL quoted-tab: expected high, got medium (exit 0)
    medium: "docs/tab\there.md" (unmatched path)
FAIL quoted-tab-reason: the reason does not say the path was quoted
ci-risk-tier self-check: 4 of 51 cases failed
```

Idêntico sob `dash`. `unicode-product` (o tier) e `space-in-name` passam também no script antigo, como a
spec previu: no caminho de produto o default (`medium`) coincide com o certo — por isso o caso do motivo
—, e o caso do espaço é guarda de regressão da leitura por linha, não prova do defeito.

### O defeito, medido antes da correção

Repositório sintético, script desta `main` (2026-11-07):

```text
$ git diff --name-only --no-renames main...feat
"docs/evid\303\252ncia.md"

$ sh scripts/ci-risk-tier.sh main feat
medium: "docs/evid\303\252ncia.md" (unmatched path)
medium
```

Com `core.quotePath=false` a mesma linha sai `docs/evidência.md` e o tier é `low`. Um nome com tab
continua citado mesmo com a flag (`"docs/tab\there.md"`), e é o que o ramo novo pega.

## Code review

`/code-review` com outro modelo sobre `git diff f38236f..HEAD`: veredito **aprovar**, um achado baixo — o
caso `unicode-product` não provava que o caminho casou com `crates/*`, porque `medium` também é o default
de caminho sem regra. Corrigido com o caso `unicode-product-reason` (o motivo no stderr), que falha no
script antigo.

## Não verificado

- **Playwright, app Tauri e `verify-openridr`**: nenhuma interface, host ou app mudou — a mudança é um
  script de CI. Nenhuma superfície A/B/C foi exercitada nesta rodada.
- **O job `risk` num run real do GitHub**: o classificador roda a partir de `origin/<base>`, e a base só
  tem a versão nova depois do merge. O primeiro run que exercita a correção é o de um PR posterior que
  toque um caminho não-ASCII; até lá a prova é o autoteste.
- **`risk:low` de ponta a ponta (auto-merge armado)**: segue sem prova real, como já registrava
  `evidence/risk-tiers.md`. Esta frente corrige a entrada do cálculo, não o caminho do auto-merge.
- **`dash` do Ubuntu 24.04**: rodei com o `dash` do Homebrew (0.5.13.5), não com o do runner.
- **Windows e macOS Intel**: não verificados, como o resto do repo.
