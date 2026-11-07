# SPEC — Risco de PR por caminho tocado: auto-merge no baixo, aprovação do mantenedor no alto

Status: ready-for-agent, issue-mãe #74. Depende do #56 (já na `main`: `.github/workflows/ci.yml` com
`ci`/`security`, ruleset da `main` aplicado). Decisões do grill em [decisions.md](decisions.md)
(2026-11-06, "Risco de PR por caminho tocado").

## Problem Statement

O mantenedor não tem regra para saber quais PRs precisam dos seus olhos. Tudo o que um agente faz chega
igual: um typo em `docs/` espera a mesma revisão humana que uma mudança em `.github/workflows/**` ou no
modelo de trust. Três consequências:

- PR de baixo risco fica parado esperando uma pessoa que não tem nada a acrescentar — e o
  `docs/ci-main-ruleset.md` foi escrito justamente para dizer que o gate é o check, não a cerimônia.
- PR de alto risco (CI, release, assinatura, trust, updater, dependência) entra do mesmo jeito que um
  PR de docs: nada no GitHub marca esse caminho como perigoso, e a decisão fica na memória de quem
  empurrou a branch.
- Não existe medida de calibragem: se o processo de revisão deixa passar coisa que depois é revertida,
  ninguém sabe — o número não é acompanhado.

## Solution

Um **risk tier** calculado pelos caminhos que o PR toca, não por quem escreveu:

- **Baixo** (`docs/**`, `*.md` exceto `AGENTS.md`, `tests/**`, `crates/*/tests/**`, `*.css`):
  auto-merge em squash assim que `ci` e `security` ficarem verdes.
- **Médio** (código de produto: `src/**`, `crates/**`, `src-tauri/**`, `fixtures/**`,
  `rust-toolchain.toml`, e o default para caminho desconhecido): revisão de código do agente e merge
  pelo orquestrador depois de conferir a evidência.
- **Alto** (`.github/workflows/**`, `.github/rulesets/**`, `AGENTS.md`, `scripts/release*`,
  `scripts/vendor-*`, `scripts/ci-*`, entitlements, `src-tauri/tauri.conf.json`, trust, e mudança de
  dependência): **o mantenedor precisa aprovar a revisão atual do PR**, senão o `ci` não fica verde.

Um PR misto recebe o tier mais alto entre os caminhos que toca. O tier aparece como label no PR
(`risk:low|medium|high`) e é aplicado por um job do próprio `ci.yml`, então o ruleset da `main` não
muda. Junto, entram as regras de processo que o issue pede: teto de três rodadas de revisão, achado
resolvido por escrito e revert rate medido por mês.

## User Stories

1. Como mantenedor, quero que um PR que só toca `docs/` diga isso sozinho no GitHub, para não perder
   tempo revisando o que um check já cobre.
2. Como mantenedor, quero que esse PR de documentação seja mergeado sem mim quando `ci` e `security`
   ficarem verdes, para que documentação não fique na fila do humano.
3. Como mantenedor, quero que um PR que toca `.github/workflows/**` seja marcado como alto risco, para
   que o caminho que roda com token nunca passe sem a minha aprovação.
4. Como mantenedor, quero que `scripts/release*`, `scripts/vendor-*`, entitlements e o updater contem
   como alto risco, para que assinatura e distribuição não sejam tratadas como um ajuste qualquer.
5. Como mantenedor, quero que mudança no modelo de trust (`crates/host/src/trusted.rs`) conte como
   alto risco, para que quem decide o que executa no workspace apareça marcado no PR.
6. Como mantenedor, quero que acrescentar ou trocar dependência conte como alto risco, para que
   supply chain não dependa de eu notar um `Cargo.toml` no meio do diff.
7. Como mantenedor, quero que bumpar a própria `version` num `package.json` continue sendo risco
   médio, para que o tier alto signifique dependência e não ruído.
8. Como mantenedor, quero que um PR misto (docs + workflow) seja tratado pelo caminho mais arriscado,
   para que juntar documentação a uma mudança de CI não dilua o risco.
9. Como mantenedor, quero que um caminho que nenhuma regra conhece caia em risco médio, para que o
   esquecimento de uma pasta nova não vire auto-merge.
10. Como mantenedor, quero que erro ao calcular o tier vire risco alto, para que um erro de script
    nunca libere merge.
11. Como mantenedor, quero que a aprovação exigida no risco alto seja uma **review** aprovada, para
    que exista registro de quem aprovou e quando.
12. Como mantenedor, quero que só a aprovação posterior ao último commit conte, para que empurrar
    código novo depois do APPROVED não herde a aprovação antiga.
13. Como mantenedor, quero que a aprovação valha para quem é owner/member/collaborator, para que um
    terceiro não destrave um PR de risco alto.
14. Como sessão de agente, quero um comando único para re-disparar o gate depois que o mantenedor
    aprovar, para não re-rodar a suíte de Rust inteira por causa de um job de segundos.
15. Como sessão de agente, quero que o PR traga o tier calculado, para escrever a evidência com o
    vocabulário certo em vez de adivinhar o que o humano vai olhar.
16. Como sessão de agente, quero que o rótulo `risk:*` não seja entrada do gate, para que ninguém possa
    "baixar" o risco de um PR editando o rótulo.
17. Como sessão de agente, quero saber que PR de risco baixo arma auto-merge sozinho, para não pedir
    merge ao orquestrador quando o check já decide.
18. Como mantenedor, quero que o auto-merge só seja armado no risco baixo, para que um PR de produto
    nunca entre sem alguém olhar a evidência.
19. Como mantenedor, quero que o job de risco rode rápido (segundos, não minutos), para que ele caiba
    em todo PR sem encarecer o CI.
20. Como mantenedor, quero que o job de risco nunca apareça como "pulado = verde" num PR, para que uma
    condição errada no YAML não vire um gate inexistente.
21. Como mantenedor, quero que o agregador `ci` continue com nome e semântica estáveis, para que
    adicionar o gate de risco não mexa no ruleset.
22. Como mantenedor, quero a regra de teto de três rodadas de revisão escrita, para que uma sessão não
    emende uma quarta correção no mesmo beco.
23. Como mantenedor, quero a regra de "todo achado é corrigido ou respondido com motivo", para que
    review não vire sugestão opcional.
24. Como mantenedor, quero ver a revert rate por mês num lugar só, para saber se o processo de
    revisão está funcionando antes de endurecer ou afrouxar o gate.
25. Como mantenedor, quero que o número de revert rate seja produzido por um comando, não por
    arqueologia manual, para que ele exista nos meses em que eu estiver com pressa.
26. Como sessão de agente, quero que o cálculo do tier tenha autoteste com caminhos sintéticos, para
    que a regra de risco tenha prova em vez de confiança.
27. Como mantenedor, quero que a lógica do agregador `ci` tenha teste, para que a lista de jobs
    "PR-only" (agora dois) não quebre em silêncio.
28. Como mantenedor, quero os comandos de setting que faltam (auto-merge) documentados e não
    aplicados, para decidir eu mesmo quando mexer no repositório.
29. Como mantenedor, quero que os labels `risk:*` existam sem eu criar à mão, para que o primeiro PR
    depois desta frente já saia rotulado.
30. Como mantenedor, quero que falhar ao rotular não reprove o PR, para que um problema de permissão
    de token não bloqueie merge de documento.

## Implementation Decisions

### Cálculo do tier

- Um script de shell do repo (`scripts/ci-risk-tier.sh`) recebe a ref de base (e opcionalmente a de
  head) e imprime `low`, `medium` ou `high` em stdout; os motivos vão em stderr, um por caminho.
  Exit 0 quando o tier foi calculado; exit 2 em uso inválido, ref inexistente ou diff vazio (diff
  vazio não é "baixo risco", é bug de quem chamou).
- A tabela de caminhos é a do `decisions.md`: baixo = `docs/**`, `*.md` (exceto `AGENTS.md`),
  `tests/**`, `crates/*/tests/**`, `*.css`; alto = `.github/workflows/**`, `.github/rulesets/**`,
  `AGENTS.md`, `scripts/release*`, `scripts/vendor-*`, `scripts/ci-*`,
  `src-tauri/resources/*.entitlements`, `src-tauri/tauri.conf.json`,
  `crates/host/src/trusted.rs`; médio = `src/**`, `crates/**`, `src-tauri/**`, `fixtures/**`,
  `rust-toolchain.toml` e **qualquer caminho não casado**. Misto → o mais alto.
- **Mudança de dependência** é diff de tabela, não arquivo tocado: `Cargo.lock` e `bun.lock` tocados
  são alto; em `package.json` e nos `Cargo.toml` (raiz e `src-tauri`) o script extrai as tabelas de
  dependência de cada lado (base via `git show`, head via árvore) e as compara ordenadas, para que
  reordenar não conte e para que `[package]`/`scripts` continuem médio. Arquivo de manifesto
  adicionado ou removido conta como mudança de dependência.
- O script não conhece GitHub: recebe refs de git e imprime. É o que torna o autoteste possível.

### Gate no CI

- O job `risk` entra em `.github/workflows/ci.yml`, roda somente em `pull_request`, em `ubuntu-latest`,
  com `contents: read` + `pull-requests: write` + `issues: write`, e ganha tempo próprio curto.
- Passos: checkout com histórico; extrair classificador e gate da revisão base; calcular o tier (erro →
  `high`, com anotação `::error::`); garantir os três labels `risk:*` (`gh label create --force`, cores
  fixas; falha = `::warning::`); aplicar o label calculado e remover os outros dois (falha =
  `::warning::`); no tier alto, exigir uma review `APPROVED` de OWNER/MEMBER/COLLABORATOR cujo
  `commit_id` **é** o head do PR e cujo estado **mais recente** daquele revisor é `APPROVED` (um
  `CHANGES_REQUESTED` posterior retira a aprovação), senão `::error::` e exit 1; no tier baixo,
  `gh pr merge --auto --squash` (falha = `::warning::` com a saída real do `gh` e uma linha no resumo,
  porque `allow_auto_merge` é decisão do Helder e ainda está desligado); escrever tier, motivos e
  resultado no `$GITHUB_STEP_SUMMARY`.
- O job é PR-only e o agregador `ci` passa a receber a lista de jobs PR-only como **dado**
  (`governance,risk`): `skipped` nesses dois só é aceitável fora de `pull_request`, e nunca dentro de
  um PR. A lógica do agregador sai do YAML para `scripts/ci-aggregate.sh`, com teste, para que a lista
  nova não quebre em silêncio.
- O nome `ci` e o ruleset ficam intocados: o gate de risco viaja dentro do agregador.

### Revert rate

- `scripts/revert-rate.sh` consulta o GitHub (`gh api`, PRs mergeados na janela e commits de revert na
  `main`) e imprime uma linha no formato da tabela de `docs/evidence/revert-rate.md`; a classificação
  (o que conta como revert, como o mês é fechado, PRs sem revert) é função pura sobre JSON, testada
  com fixtures locais sem rede.
- O doc guarda a tabela, uma linha por mês, e o comando usado; rodar é do mantenedor, não do CI.

### Processo

- `AGENTS.md` ganha: o fluxo do tier (auto-merge no baixo, aprovação do mantenedor no alto,
  `gh run rerun --failed` depois do APPROVED), o teto de três rodadas de revisão, a regra "achado
  corrigido ou resolvido por escrito" e o ponteiro para os scripts novos.
- `docs/ci-main-ruleset.md` ganha a seção das pendências de setting do Helder (auto-merge) e a
  atualização do bullet que anunciava esta frente.

## Testing Decisions

Teste bom aqui prova comportamento externo: um tier está certo quando o script imprime o esperado para
um conjunto de caminhos, e o gate está certo quando reprova/aceita o que deve — nunca quando uma função
interna foi chamada.

- **`scripts/ci-risk-tier.sh`**: autoteste no molde da casa (`scripts/*-selfcheck.sh`), com repositórios
  git sintéticos em `mktemp` — um caso por linha de saída, exit não-zero no primeiro erro. Casos: cada
  família de caminho (docs, md na raiz, AGENTS.md, css, src, crates, tests, crates/*/tests, workflows,
  rulesets, release, vendor, ci-*, entitlements, tauri.conf.json, trusted.rs, caminho desconhecido),
  mistos (docs+src = medium, src+workflow = high), dependência (`Cargo.lock`, `bun.lock`,
  `package.json` com troca de versão de dependência, `package.json` só com mudança em `scripts`,
  `Cargo.toml` só com `version`, `Cargo.toml` com dependência nova, manifesto adicionado) e os erros
  (ref inexistente, diff vazio). Falha-antes: o script não existe, então o autoteste começa vermelho.
- **`scripts/ci-aggregate.sh`** (agregador extraído): teste em `tests/shell/`, com o JSON de `needs`
  como entrada. Casos: PR tudo verde passa; `push` com os dois PR-only `skipped` passa; PR com
  `governance: skipped` reprova; PR com `risk: skipped` reprova; PR com `rust: failure` reprova;
  `push` com `rust: skipped` reprova; `workflow_dispatch` com os dois `skipped` passa. Prior art:
  `tests/shell/git-remote-fixture.test.sh`.
- **`scripts/revert-rate.sh`**: classificação testada sobre JSON de fixture (mergeados, revertidos,
  revert de revert), sem rede; o caminho `gh api` fica como "não verificado no CI".
- **YAML**: `actionlint` e `zizmor --offline --min-severity low .` continuam sendo o teste do workflow;
  o job entra no `governance` por já estarem lá.
- **Prova real**: esta própria frente é um PR de risco alto (toca `.github/workflows/**`), então o
  primeiro run real do gate é a prova — ele deve reprovar até a aprovação do mantenedor e passar
  depois do rerun.

## Out of Scope

- Mudança no ruleset da `main` (o gate viaja no agregador `ci`): nenhum check obrigatório novo.
- **Fechar o resto da brecha "workflow do PR decide"**: com `pull_request`, o YAML que roda é o do merge
  commit, então um PR que apaga o passo do gate do próprio `ci.yml` ainda pode ficar verde. O que esta
  frente faz é tirar a decisão das mãos do PR (classificador e gate vêm da base) e deixar a brecha
  residual num PR que mexe no YAML — visível e de risco alto. Fechar de vez exige `pull_request_target`
  para o gate e um terceiro check obrigatório no ruleset, ou seja, settings: é decisão do Helder e vira
  issue filha, não trabalho desta frente.
- Aplicar `allow_auto_merge` e criar labels por fora do job: settings são do Helder; o comando fica
  documentado.
- CODEOWNERS e "require review from code owners".
- Security review por LLM com credencial (issue filha do #56).
- Medir tempo de fila, tamanho de PR ou cobertura: continuam linhas de julgamento.
- Reverter o auto-merge de um PR específico: `gh pr merge --disable-auto` já existe.
- Dashboard de revert rate: uma tabela num doc, atualizada quando o mantenedor roda o comando.

## Further Notes

- Custo: o job de risco é de segundos (um `git diff`, uma chamada de reviews, um PATCH de label) e
  roda em paralelo ao Rust; o agregador deixa de ser "3 s" e passa a esperar o job mais lento, como
  hoje.
- O gate não substitui julgamento: um PR de risco médio que toca o coração do editor continua sem
  auto-merge, e o `## Merge danger` do template continua sendo texto que o autor escreve.
- Consequência esperada e aceita: PR do Dependabot cai em risco alto (muda lockfile), logo passa pelo
  mantenedor.
- Arquivos de risco que **não** entram nesta rodada e ficam para quando aparecerem: paths de
  notarização fora de `scripts/release*`, e qualquer coisa sob `vendor/` (que hoje é dependência
  vendorizada e cairia em médio por default).
- Evidência desta frente: `docs/evidence/risk-tiers.md`; retrospectiva em `docs/RETRO-RISK-TIERS.md`.
