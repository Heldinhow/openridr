# Ruleset da `main`

Este documento e `.github/rulesets/main.json` descrevem a proteção da `main`. **Aplicado em
2026-11-06** (ruleset `main`, id 26731350, `enforcement: active`): o GitHub devolve exatamente estas
regras, acrescentando só campos que ele mesmo preenche (`require_extra_approval_for_unattributed_changes`,
`required_reviewers`, `do_not_enforce_on_create`). Mudar settings do GitHub continua sendo decisão do
Helder: o JSON do repo é a fonte, e a API é quem aplica.

## O que a regra faz

| Regra do ruleset | Efeito prático |
|---|---|
| `pull_request` | `main` só recebe commit por PR. Zero aprovação obrigatória no ruleset (um mantenedor); revisão obsoleta é descartada no push. A aprovação do tier alto é do job `risk`, não daqui — e o PR do dono passa sem ela (ver abaixo). |
| `allowed_merge_methods: ["squash"]` | Só squash: o título do PR vira a mensagem do commit, e é ele que o `governance` valida no padrão conventional. |
| `required_status_checks` com `ci` e `security` | Merge bloqueado enquanto os checks `ci` e `security` não passarem. `strict`: o PR tem de estar em cima da `main` atual. O `ci` é agregador: o gate de risco viaja dentro dele. |
| `required_linear_history` | Sem merge commit na `main`. |
| `non_fast_forward` | Sem force-push. |
| `deletion` | `main` não pode ser apagada. |

`ci` e `security` são os nomes dos jobs em `.github/workflows/ci.yml`. Renomear qualquer um deles sem
atualizar este JSON remove o check obrigatório **em silêncio** (o nome antigo nunca mais aparece, e o
ruleset passa a exigir um check que não existe).

## Como aplicar (depois do primeiro run verde)

1. O workflow precisa ter rodado pelo menos uma vez e passado: os nomes `ci` e `security` só existem
   como check depois disso. Rode `workflow_dispatch` no branch do PR (o workflow tem esse gatilho
   justamente para isso) e confira o resultado.
2. Aplique:

   ```sh
   gh api --method POST repos/Heldinhow/openridr/rulesets \
     --input .github/rulesets/main.json
   ```

3. Confira que entrou e que o JSON é o que está no repo:

   ```sh
   gh api repos/Heldinhow/openridr/rulesets
   gh api "repos/Heldinhow/openridr/rulesets/$(gh api repos/Heldinhow/openridr/rulesets --jq '.[0].id')"
   ```

4. Faça um PR de teste (uma linha em qualquer doc) e confirme que o botão de merge fica bloqueado até
   `ci` e `security` ficarem verdes.

Para remover: `gh api --method DELETE "repos/Heldinhow/openridr/rulesets/<id>"`.

## Settings do repo

O que o repo precisa para o gate funcionar, e o estado de cada um. Mudar setting é decisão do Helder; a
sessão documenta e não aplica.

| Setting | Estado | Por quê |
|---|---|---|
| `allow_auto_merge` | **ligado** (2026-11-06, autorizado pelo Helder) | o job `risk` arma auto-merge nos PR `risk:low`; com o setting desligado ele só emitiria `::warning::` e o PR de docs esperaria alguém |
| `allow_squash_merge` | ligado | é o único método que o ruleset aceita (`allowed_merge_methods: ["squash"]`) |

Conferir: `gh api repos/Heldinhow/openridr --jq '{allow_auto_merge, allow_squash_merge}'`.
Se algum dia o auto-merge for desligado, religar é
`gh api --method PATCH repos/Heldinhow/openridr -F allow_auto_merge=true`, e o job `risk` passa a avisar
(`::warning::` + resumo) em vez de armar o merge — ele nunca reprova por causa do setting.
Os labels `risk:low|medium|high` **não** estão nesta lista: o job os cria de forma idempotente
(`gh label create --force`) na primeira vez que roda.

## O que ficou de fora, e por quê

- **Merge queue**: indisponível para repositório de conta de usuário (só organização pública ou
  Enterprise).
- **CODEOWNERS e duas aprovações**: um mantenedor, zero forks; sem a regra "require review from code
  owners" o CODEOWNERS não obriga ninguém, e duas aprovações com uma pessoa é cerimônia. A segunda
  opinião real vem do `/code-review` com outro modelo, fora do GitHub.
- **Conventional commit por regra nativa**: as regras de metadados de commit são Enterprise. O caminho
  público é squash-only + lint do título no job `governance`, que é o que está aqui.
- **Gate de tamanho de PR e de cobertura**: regra que falha em cima de julgamento. Fica como linha no
  template de PR, não como check.
- **Rótulo de risco por caminho tocado (#74)**: implementado, e **sem mexer neste ruleset**. O job `risk`
  do `ci.yml` calcula o tier (`scripts/ci-risk-tier.sh`), rotula o PR `risk:low|medium|high`, arma
  auto-merge no baixo e, no alto, exige aprovação do mantenedor (`scripts/ci-risk-gate.sh`). Ele entra no
  `needs` do agregador `ci`, então continua sendo **dois** checks obrigatórios: `ci` e `security`.
  Justificativa e tabela de caminhos em [SPEC-RISK-TIERS](SPEC-RISK-TIERS.md).
- **Aprovação no tier alto, num repo de um mantenedor só (#149)**: o gate aceita o PR cujo **autor** é
  `OWNER` sem review nenhuma, porque o GitHub recusa aprovação no próprio PR (HTTP 422) e este ruleset
  não tem `bypass_actors`. Decisão do Helder de 2026-11-07, custo aceito e registrado em
  [decisions.md](decisions.md): no alto, o gate é registro, não revisão. Isso **não** muda o ruleset e
  **não** arma auto-merge no alto — o merge continua manual. Se um dia entrar uma segunda conta com
  write access (opção (a) da #149), o curto-circuito do dono sai do gate e a aprovação volta a valer.
