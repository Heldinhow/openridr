# CI e qualidade automática de PR — pesquisa priorizada

Escopo: como elevar a qualidade do código **gerado por agentes de IA** neste repo usando CI e
validações automáticas de PR. Pesquisa de fontes primárias (docs oficiais do GitHub, Cargo/Clippy,
Tauri, Bun, OpenSSF/Google eng-practices, README dos próprios tools) + medições locais nesta
worktree. Nenhum código de produto foi tocado e nenhum workflow foi criado — isto é insumo para o
Helder decidir.

Data: 2026-11-06. Base: `46e7bc7` (`main`).

Já planejado e **não** recontado como novidade: #56 (CI em PR), #44 (release assinada por tag),
fluxo grill → spec → tickets → `/code-review` com outro modelo → verificação no app real.
O que segue avalia lacunas nesse plano e lista camadas candidatas fora dele.

---

## 1. Foto do repo (medido, não estimado)

| Fato | Número | Como |
|---|---|---|
| `.github/` | não existe | `ls -a` |
| `main` protegida / rulesets | **não** (HTTP 404, `rulesets: []`) | `gh api .../branches/main/protection` |
| Repositório | público, MIT, **user-owned** (`Heldinhow`), 0 stars / 0 forks | `gh repo view`, API |
| LOC TS/TSX em `src/` | ~23,8k | `wc -l` |
| LOC Rust em `crates/` | ~47,7k | `wc -l` |
| Testes Rust | 428 `#[test]` (285 na lib + integrações) | `grep -c` |
| `cargo test -p openridr-host --lib` | 285 pass / **1 fail** / 3,7 s (17 s com build) | rodado aqui |
| Testes unit TS | 39 testes em 7 arquivos, **195 ms**, 151 asserts | `bun test tests/unit` |
| `tsc --noEmit` | **3,7 s**, 0 erros | `bunx tsc --noEmit` |
| `bun install --frozen-lockfile` | 0,3 s (cache quente), 101 pacotes | rodado aqui |
| `cargo clippy --workspace --all-targets` | **45 warnings** em 27 tipos (28 ocorrências na lib + testes), 11 s cold | rodado aqui |
| `cargo fmt --check` | **596 hunks em 32 arquivos** (+2 em `src-tauri`) | rodado aqui |
| Config de lint TS | **nenhuma** (sem eslint/biome/prettier) | `find`/`package.json` |
| Workspace Rust | só `crates/host` + `crates/host-http`; `src-tauri` é workspace separado | `Cargo.toml` |
| E2E | 61 specs, `workers: 1`, host real + `dotnet` | `playwright.config.ts` |
| Deps vendorizadas | `vendor/` com pin SHA-256 conferido em `scripts/vendor-netcoredbg.sh` | script |

Duas consequências imediatas:

1. **A #56 descreve um lint que não existe.** `bun run check` hoje é `tsc --noEmit && cargo check --workspace --all-targets`. Não há ESLint, Biome nem Prettier no repo. Ou a #56 ganha um ticket de lint antes, ou o texto dela muda.
2. **`cargo fmt --check` como required check hoje quebra tudo**: 596 hunks em 32 arquivos. Exige um PR mecânico dedicado antes (com `.git-blame-ignore-revs`) ou fica fora.

---

## 2. Restrições da plataforma que mudam o plano

Verificadas na documentação oficial; várias derrubam itens que pareciam óbvios:

| Item | Realidade | Fonte |
|---|---|---|
| Minutos de runner | **Grátis** para runners padrão do GitHub em repo público, em qualquer OS. Custo em dinheiro ≈ R$0; o custo é fila e tempo de parede. | [Billing do Actions](https://docs.github.com/en/billing/managing-billing-for-your-products/managing-billing-for-github-actions/about-billing-for-github-actions) |
| Runner macOS | `macos-15`/`macos-26`/`macos-latest` = **3 vCPU, 7 GB, arm64 (M1)**. Alvo certo para este app. Actions da comunidade podem não funcionar em arm64. | [GitHub-hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) |
| Merge queue | Só em **repo público de organização** (ou privado Enterprise). `Heldinhow/openridr` é de usuário → **indisponível**. | [Merge queue](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-a-merge-queue) |
| Rulesets | Disponíveis em repo público já no **GitHub Free** (branch e tag rules). É o caminho moderno no lugar de "branch protection". | [Regras disponíveis](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets) |
| Conventional Commits nativo | As regras de **metadados de commit** (regex em mensagem, e-mail de autor, nome de branch) são **Enterprise**. Não dá para exigir `feat:`/`fix:` por ruleset aqui. | [Regras (Enterprise)](https://docs.github.com/en/enterprise-cloud@latest/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets) |
| CODEOWNERS + review obrigatório | Funciona, mas CODEOWNERS só **obriga** se a regra "require review from Code Owners" estiver ligada; com um único mantenedor não adiciona revisão, só cerimônia. | [About code owners](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners) |
| PR template | Preenche o corpo, **não** é obrigatório nem validável nativamente. Exigir conteúdo exige um job próprio. | [PR template](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/creating-a-pull-request-template-for-your-repository) |
| Link PR↔issue | Palavras-chave (`Closes #12`, `Fixes #12`) fecham a issue no merge; o vínculo é visível na issue. É o mecanismo que o gate de evidência pode checar. | [Linking PR to issue](https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/linking-a-pull-request-to-an-issue) |
| Secret scanning | **Grátis em repo público**, roda sozinho. | [About secret scanning](https://docs.github.com/en/code-security/secret-scanning/introduction/about-secret-scanning) |
| Push protection no repo | Exige **GitHub Secret Protection (pago)**. O que já existe de graça é o "push protection for users", ligado por padrão, que bloqueia push de segredo para repo público (inclui push pela linha de comando). | [About push protection](https://docs.github.com/en/code-security/secret-scanning/introduction/about-push-protection) |
| Code scanning / CodeQL | **Grátis em repo público**. CodeQL cobre **Rust** (editions 2021/2024) **e** TypeScript; para Rust exige `rustup` + `cargo` e não suporta features de nightly. | [About code scanning](https://docs.github.com/en/code-security/code-scanning/introduction-to-code-scanning/about-code-scanning-with-codeql), [linguagens suportadas](https://codeql.github.com/docs/codeql-overview/supported-languages-and-frameworks/) |
| Attestations (proveniência) | Disponíveis em repo público nos planos Free/Pro/Team. | [Artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations) |
| Dependabot | Suporta `bun` (≥1.1.39, `bun.lock` texto — é o formato aqui), `cargo` e `github-actions`. | [Ecossistemas](https://docs.github.com/en/code-security/dependabot/ecosystems-supported-by-dependabot/supported-ecosystems-and-repositories) |
| Pin de actions | SHA completo é a recomendação oficial; existe também "workflow dependency lockfile" (mais forte, mais novo). Dependabot atualiza SHAs pinados. | [Secure use](https://docs.github.com/en/actions/reference/security/secure-use) |
| WebDriver do Tauri no macOS | `tauri-driver` é **só Windows/Linux**. No macOS só via `@wdio/tauri-service` com o **WebDriver embutido** (`tauri-plugin-wdio-webdriver`) — dependência nova, e o exemplo oficial de CI da Tauri só cobre Linux/Windows. | [Tauri WebDriver](https://v2.tauri.app/develop/tests/webdriver/), [CI](https://v2.tauri.app/develop/tests/webdriver/ci/) |

---

## 3. Tabela priorizada

Colunas: **esforço** (horas de trabalho), **custo de CI** (tempo por execução, em runner grátis),
**ruído** (falso positivo / manutenção), **recomendação**.

| # | Camada | Problema que resolve aqui | Esforço | Custo CI | Ruído | Recomendação |
|---|---|---|---|---|---|---|
| 1 | **CI mínimo em PR** (#56, ajustado): `tsc`, `bun test tests/unit`, `cargo clippy -D warnings`, `cargo test -p openridr-host` | hoje um PR pode entrar sem nada rodar em ambiente limpo | 3–5 h | ~4–6 min | baixo | **agora** |
| 2 | **Ruleset na `main`**: PR obrigatório, required check, dismiss stale, linear history, squash | fecha o "verde no meu Mac" como critério de merge | 30 min | 0 | nenhum | **agora** |
| 3 | **`cargo fmt` num PR mecânico + `.git-blame-ignore-revs`**, depois `fmt --check` required | divergência de estilo entre sessões de agente | 1–2 h | ~5 s | baixo (após o big bang) | **agora** (big bang) |
| 4 | **`dotnet 8.0.408` no CI** (`actions/setup-dotnet`; a imagem macos-15 só tem 8.0.424) antes dos testes Rust | sem SDK exato um teste falha, não faz skip (medido aqui) | 1 h | ~1 min | baixo | **agora** |
| 5 | **PR template + gate de evidência** (exige `Closes #N` e seção de evidência; job de ~30 linhas) | agente entrega "feito" sem prova; issue e PR se descolam | 2 h | ~10 s | médio (texto livre) | **agora** |
| 6 | **Conventional Commits no merge**: squash-only + lint do título do PR (`action-semantic-pull-request`) | a regra nativa é Enterprise; hoje o merge é por merge commit, então o título nunca vira mensagem | 1 h | ~5 s | baixo | **agora** |
| 7 | **Dependabot** (`bun`, `cargo`, `github-actions`, semanal, agrupado) | deps envelhecendo sem ninguém olhar; CVE só aparece no dia | 1 h | 0 (Dependabot é grátis) | médio (PRs de bump) | **agora** |
| 8 | **`actionlint` + `zizmor`** nos workflows | workflow escrito por agente com injection/permissão demais | 1–2 h | ~10 s | baixo | **agora** (quando o 1º workflow existir) |
| 9 | **SHA-pin das actions** + `permissions:` mínimo + `persist-credentials: false` | supply chain do próprio CI | 1 h | 0 | baixo (Dependabot mantém) | **agora** |
| 10 | **`knip`** (código/exports/deps mortos no TS) | IA cria abstração especulativa e export sem uso | 2–4 h (triagem) | ~20 s | **alto no começo** | **depois** (medir uma vez antes) |
| 11 | **`cargo-deny`** (advisories + licenses + sources) | CVE e licença em 133 crates; repo MIT distribui binário | 2 h | ~30 s | baixo | **depois** (semanal) |
| 12 | **`cargo-machete`** (deps Rust mortas) | agente adiciona crate e não usa | 30 min | ~5 s | médio (heurística) | **depois** (mensal) |
| 13 | **`cargo nextest`** no lugar de `cargo test` | flakes, timeouts, retry, saída legível | 1–2 h | igual | baixo | **depois** (com o #1) |
| 14 | **`cargo-mutants --in-diff`** nos caminhos críticos (`trust`, `restore`, `dap`, `debugger`) | teste fraco escrito por IA passa e não prova nada | 3–4 h | minutos por PR tocado | médio (mutantes equivalentes) | **depois** (nightly/label) |
| 15 | **CodeQL default setup** (Rust + TS), não-required | injeção/path traversal que clippy não vê | 1–2 h | ~10–15 min (Rust compila) | baixo | **depois** (semanal) |
| 16 | **Cobertura `cargo llvm-cov` report-only** | visibilidade, sem virar meta | 2 h | ~1 min | baixo | **depois** (semanal, sem gate) |
| 17 | **`rust-toolchain.toml` + `rust-version`** no `Cargo.toml` | hoje `cargo check` local usa 1.99; CI pode pegar outra | 30 min | 0 | nenhum | **agora** (barato) |
| 18 | **Smoke E2E noturno** (subconjunto Playwright: `shell`, `workspace`, `trust-prompt`) | 61 specs não cabem por PR, mas zero E2E é pior | 3 h | 10–20 min/noite | médio (flakes conhecidos) | **depois** |
| 19 | **Revisão por LLM em PR** (Copilot code review ou similar) | segunda opinião em PR de agente | 1 h | grátis em repo público (compute) + custo do plano | **alto** | **depois, opcional, nunca required** |
| 20 | **Teste de componente React** | UI hoje só é coberta por E2E caro | 4–8 h | ~1 min | médio | **depois** (exige dep nova) |
| 21 | **E2E via WebDriver do Tauri no macOS** | fecha o gap "UI testada só com host HTTP" | 8 h+ | 5–10 min | alto | **não agora** |
| 22 | **Gate de tamanho de PR** | PR gigante é irrevisável | 30 min | 0 | alto (agente produz diffs grandes legítimos) | **não** (regra no template, sem gate) |
| 23 | **Gate de cobertura (x%)** | — | 2 h | — | alto | **não** |
| 24 | **Merge queue / CODEOWNERS / 2 reviewers** | — | — | — | — | **não** (indisponível ou inútil com 1 mantenedor) |
| 25 | **Stryker (mutation TS)** | — | 4 h | alto | alto | **não** (StrykerJS não tem runner de `bun test`) |
| 26 | **`cargo-udeps`** | — | 1 h | alto (nightly) | médio | **não** (use `machete`) |
| 27 | **SLSA nível 3 / build reproducível** | — | dias | alto | — | **não** (overkill) |

---

## 4. Justificativa por item

### 4.1 O que entra agora

**(1) CI mínimo.** É o único item que muda a natureza do problema: hoje o critério de merge é o Mac
do agente. Os números medidos aqui dizem que o pacote é barato: `tsc` 3,7 s, unit tests 195 ms,
clippy 11 s (cold, no Mac; no runner de 3 vCPU conte 40–90 s), testes do host 3,7 s depois do build.
O gargalo real é o **SDK .NET**, não o Rust. Um job separado para o lado TS em `ubuntu-latest`
(tsc/bun não precisam de macOS) e outro em `macos-15` para Rust+dotnet mantém o PR em ~5 min.
Correções à #56: (a) não existe lint TS — ou virá de um ticket próprio (item 4.2) ou a issue muda de
texto; (b) `cargo clippy -D warnings` falha hoje com **45 warnings** em 27 tipos (≈28 auto-fixáveis por
`cargo clippy --fix`), então o primeiro PR é de limpeza; (c) sem `dotnet` 8.0.408 na `PATH` o teste
`run_configs::tests::launch_profile_runs_fixture_with_effective_environment_and_arguments` **falha**
(medido), não faz skip.

**(2) Ruleset.** Sem proteção, um required check é decoração. Rulesets bastam no Free para repo
público e substituem branch protection. Ligar: require PR, require status checks, dismiss stale
approvals, require linear history, block force-push/delete. Um detalhe operacional: **mantenha um
job com nome estável** como required (ex.: `ci`) mesmo que ele agregue outros — renomear job obriga a
editar a regra.

**(3) Formatação.** `cargo fmt --check` não tem modo "só arquivos alterados", então é big bang ou
nada. 596 hunks em 32 arquivos é um PR mecânico de uma tarde, e `.git-blame-ignore-revs` preserva o
`git blame` ([git-blame](https://git-scm.com/docs/git-blame)). Depois disso o check custa ~5 s e
elimina a maior fonte de diff espúrio entre sessões. **Não rode `cargo fmt` em arquivos que você
está tocando antes do big bang** — mistura formatação com lógica no mesmo commit e o hook de
pré-commit já sofre com isso hoje.

**(4) `.NET 8.0.408`.** As fixtures fixam o SDK em `global.json` com `rollForward: disable`
(`AGENTS.md`). A imagem `macos-15` já traz 8.0.101/204/303/**424** e 9.x/10.x, mas **não** 8.0.408
([readme da imagem](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-Readme.md)),
e o pin não pode mudar — logo `actions/setup-dotnet` vai baixar 8.0.408 (~1 min por execução, sem
cache nativo; o `cache: true` do action cacheia pacotes NuGet, não o SDK). Sem esse passo a suíte
falha (não skippa).

**(5) Template + gate de evidência.** Não existe gate nativo para conteúdo de corpo de PR, e o
template só preenche. Um job de ~30 linhas checando `(Closes|Fixes|Resolves) #\d+` e a presença de
`## Evidência`/`## Evidence` no corpo pega o caso clássico de agente que "terminou" sem prova. Custo
~10 s, sem dependência nova. Alternativa madura: [Danger JS](https://danger.systems/js/) — mais
poderoso, mais uma dependência de CI.

**(6) Conventional Commits.** O rule de metadados de commit é Enterprise; o caminho público é
lint do **título do PR** com squash-only (o título vira a mensagem do commit). O histórico já é
majoritariamente conventional (29 dos últimos 50 commits); o resto são `Merge pull request #N`,
porque hoje o merge usa **merge commit** e o título do PR nunca vira mensagem. Com squash-only o
lint de título passa a ser o gate real:
[`amannn/action-semantic-pull-request`](https://github.com/amannn/action-semantic-pull-request)
(1,4k stars, ativo) leva 20 linhas de YAML.

**(7) Dependabot.** Hoje as deps são atualizadas à mão. `bun` (o `bun.lock` é texto — suportado),
`cargo` e `github-actions` (que também atualiza SHAs pinados). Semana + agrupamento por ecossistema
evita 30 PRs. Renovate é mais configurável; Dependabot já está no GitHub, custo zero.

**(8–9) Workflows e supply chain.** Assim que o primeiro workflow nascer, ele é código gerado por
agente rodando com token de escrita. `actionlint` (sintaxe, tipos de expressão, shellcheck,
injection) e [zizmor](https://github.com/zizmorcore/zizmor) (injection, permissões, persistência de
credencial) são estáticos, rodam em segundos e são exatamente o tipo de erro que agente comete.
Junto: SHA completo em `uses:`, `permissions:` mínimo por job, `persist-credentials: false` no
checkout — recomendação oficial em [Secure use](https://docs.github.com/en/actions/reference/security/secure-use).

**(17) Toolchain pinado.** Não há `rust-toolchain.toml` nem `rust-version`; o repo compila hoje com
`rustc 1.99.0`. Sem pin, "verde no CI" e "verde no Mac" podem divergir por toolchain, e o MSRV é
desconhecido. Duas linhas de config, zero custo.

### 4.2 O que entra depois (e por quê)

**(10) `knip`.** É a ferramenta mais alinhada ao problema "slop": encontra export, arquivo e
dependência sem uso — o que agente produz em série. Mas em código com 23,8k LOC e muitos padrões
dinâmicos (`data-testid` como contrato, imports por glob) o primeiro relatório costuma ser grande e
parcialmente falso. Por isso: rodar **uma vez, medir**, decidir se vira check semanal. Nunca
required no primeiro contato.

**(11) `cargo-deny`.** Um só tool cobre advisories (RustSec), licenças, bans (duplicatas) e
`sources` (registry permitido) — cobre o que `cargo audit` faz e mais. Relevante porque o repo é MIT
e **distribui binário** (netcoredbg, Roslyn, runtime .NET privado): um crate GPL entrando por
transitividade hoje passa invisível. Semanal, não por PR: advisory novo não deve bloquear um PR que
não toca deps.

**(12) `cargo-machete`.** Rápido e em stable; `cargo-udeps` precisa de nightly e é mais caro. A
ferramenta é "imprecisa por design" (README), então: mensal, saída revisada, não gate.

**(13) `cargo-nextest`.** Retries com marcação `flaky`, timeouts por teste, partição/shard e saída
muito melhor — útil quando o alvo é CI com 428 testes, alguns que spawnam `dotnet`.
[Retries](https://nexte.st/docs/features/retries/), [partições](https://nexte.st/docs/ci-features/partitioning/).
Custo: mais um binário instalado no CI (`taiki-e/install-action`).

**(14) `cargo-mutants --in-diff`.** É a resposta automatizada para "teste fraco escrito por IA":
injeta mutações e exige que os testes peguem. `--in-diff` restringe aos arquivos alterados (bem mais
barato), `--shard k/n` paraleliza, e o exit code distingue "mutante não pego" de "baseline quebrado".
Rodar em nightly ou por label `mutation`, nos módulos que são máquina de estado
(`trust`, `restore`, `test_run`, `debugger`, `dap`) — os mesmos de `docs/VERIFICACAO-FORMAL.md`.
Mutantes equivalentes geram ruído; trate como sinal, não como gate de PR.

**(15) CodeQL.** Grátis em repo público e cobre Rust e TS. Para Rust ele **compila** o crate
(portanto ~10–15 min e precisa de `rustup`+`cargo`); TS é rápido. Recomendação: default setup semanal
(Rust + javascript-typescript), **não required** — code scanning é caça a classes específicas
(injection, path traversal), não a estilo.

**(16) Cobertura.** Útil como número visível, ruim como meta. O Google é explícito: cobertura "não é
medida perfeita de qualidade de teste" e "um mandato de x% deve vir com investimento em
infraestrutura, e engenheiros passam a tratar a meta como checkbox"
([Google Testing Blog](https://testing.googleblog.com/2020/08/code-coverage-best-practices.html)).
Aqui a cobertura certa é a **de mutação** nos módulos críticos; `cargo llvm-cov` entra só para
tendência.

**(18) Smoke E2E noturno.** 61 specs, `workers: 1`, host real e `dotnet` — isso não é PR material.
Mas o valor de E2E é justamente o que o host HTTP + Playwright já cobre barato. Um subconjunto de 3–5
specs estáveis por noite (e um `workflow_dispatch`) pega regressão que unit test não pega, sem
bloquear PR.

**(19) Revisão por LLM.** O fluxo já tem `/code-review` com outro modelo após cada implementação —
isso é melhor que um bot em PR, porque tem contexto do ticket. O que a plataforma oferece: Copilot
code review roda em PR, comenta como um humano, e o "approval assessment" **não conta** para
required approvals ([docs](https://docs.github.com/en/copilot/using-github-copilot/code-review/using-copilot-code-review)).
Prós: segunda opinião sem humano. Contras: não determinístico (não serve de gate), ruído em diff
grande, e custa plano de Copilot (o compute do Actions é grátis em repo público). Posição honesta:
opcional, por label, nunca required.

**(20) Teste de componente.** Os 7 arquivos de teste unit não importam React: são funções puras.
Qualquer teste de componente exige dependência nova (`happy-dom`/`jsdom` + testing-library) e decisão
registrada — a regra do repo é zero dep nova sem aprovação. Fica para quando um bug de UI mostrar
que E2E é caro demais.

**(21) WebDriver do Tauri.** `tauri-driver` não existe para macOS; o caminho novo é o WebDriver
**embutido** do `@wdio/tauri-service` + `tauri-plugin-wdio-webdriver` — dois pacotes e um plugin
novos, mais build do app no CI, mais a superfície que hoje é coberta de graça pelo host HTTP. O
exemplo oficial de CI da própria Tauri ainda é Linux/Windows. Não vale antes de o CI básico estar
verde por semanas.

### 4.3 O que não vale aqui (honestidade)

- **Merge queue**: indisponível para repo de usuário. E com um PR por vez não há o que enfileirar.
- **CODEOWNERS + 2 approvals**: 1 mantenedor, 0 forks. Vira teatro; o ganho real de revisão já vem
  do `/code-review` com outro modelo.
- **Gate de tamanho de PR**: o fluxo já é ticket-sized e o Google trata tamanho como julgamento, não
  regra ("100 linhas é razoável, 1000 é demais… fica a critério do revisor",
  [Small CLs](https://google.github.io/eng-practices/review/developer/small-cls.html)). Vale como
  linha no template, não como check que falha.
- **Gate de cobertura**: ver acima.
- **Stryker**: os runners suportados são Cucumber/Jasmine/Jest/Karma/Mocha/Tap/Vitest — não há runner
  de `bun test` ([config](https://stryker-mutator.io/docs/stryker-js/configuration/)). Migrar de
  runner de teste só por mutation testing é caro; o mutation testing que importa aqui é o do Rust.
- **`cargo-udeps`**: nightly, mais lento, mesmo objetivo do `machete`.
- **SLSA L3, build reproducível, TLA+ no CI**: custo de dias por um app alpha de um mantenedor.
  (A PoC de `trust_model` já cobre o que importa em trust.)
- **Rodar a suíte E2E inteira por PR**: 61 specs em `workers: 1` com `dotnet` real. Não é dinheiro
  (runner é grátis), é tempo de parede e flakes conhecidos (`find-usages`, `run-configs`, `tests`).

---

## 5. Pacote mínimo (proposta)

Tudo abaixo custa ~1 semana e ~5 min por PR.

**Workflow `ci.yml`** (`on: pull_request` → `main`, `push: main`, `concurrency: cancel-in-pr`):

*Job A — TS (ubuntu-latest, 4 vCPU)*
1. `actions/checkout@<sha>` com `persist-credentials: false`
2. `oven-sh/setup-bun@<sha>` com cache
3. `bun install --frozen-lockfile`
4. `bunx tsc --noEmit`
5. `bun test tests/unit`
6. (quando existir lint) `bun run lint`

*Job B — Rust (macos-15)*
1. checkout + `dtolnay/rust-toolchain@<sha>` (versão fixa, vinda do `rust-toolchain.toml` novo)
2. `Swatinem/rust-cache@<sha>`
3. `cargo fmt --check` (só depois do PR mecânico)
4. `cargo clippy --workspace --all-targets -- -D warnings` (só depois de zerar os 45)
5. `actions/setup-dotnet` com `8.0.408`
6. `cargo test -p openridr-host` (ou `cargo nextest run -p openridr-host`)

*Job C — governança (ubuntu, segundos)*
7. `actionlint` + `zizmor`
8. gate de evidência (regex de `Closes #` + seção no corpo)
9. lint do título do PR (conventional)

**No GitHub (settings, feito à mão uma vez)**
- Ruleset na `main`: require PR, require check `ci`, dismiss stale approvals, linear history,
  block force-push e delete; squash-only como método de merge.
- Dependabot: `bun`, `cargo`, `github-actions`, semanal, agrupado.

**No repo**
- `.github/pull_request_template.md` com `Closes #`, `## Resumo`, `## Evidência`, `## Risco de merge`.
- `rust-toolchain.toml` + `rust-version` nos crates.
- PR mecânico de `cargo fmt` + `.git-blame-ignore-revs`.

## 6. Pacote completo (o que vem depois)

Semanal/nightly: `cargo-deny` (advisories+licenças+sources), CodeQL (rust+ts), smoke E2E (3–5 specs),
`cargo llvm-cov` report-only, `knip` + `cargo-machete` (mensal), `cargo-mutants --in-diff` por label
nos módulos de estado.

Release (#44): `tauri-action`, notarização, e — barato no mesmo workflow —
`actions/attest-build-provenance` no DMG (grátis em repo público; `gh attestation verify` confere).
O DMG já publica SHA-256 manualmente; a attestation dá o mesmo para quem baixa.

---

## 7. Ajustes sugeridos nas issues existentes

**#56 (CI em PR)** — quatro correções:
1. "`bun run check` (type check + **lint**)" descreve algo inexistente. Ou entra ticket de lint
   (ESLint `typescript-eslint` é dep nova → precisa decisão registrada; endurecer o `tsconfig` com
   `noUnusedLocals`, `noUnusedParameters`, `noUncheckedIndexedAccess` é grátis e pega boa parte do
   slop de TS) ou a issue muda.
2. `cargo fmt --check` precisa de um PR mecânico **antes** (596 hunks, 32 arquivos).
3. `cargo clippy -D warnings` exige zerar 45 warnings (~28 auto-fixáveis); é um PR de limpeza, não
   um passo de YAML.
4. Falta `actions/setup-dotnet` com `8.0.408` — sem ele a suíte Rust falha (não skippa) e a issue
   não menciona dependência de SDK.

**#44 (release)** — acrescentar attestation de proveniência e o hash já publicado; e usar
`squash`/`tag` protegida (ruleset de tag também está disponível no Free para repo público).

**AGENTS.md (depois da #56)** — a regra "sessão não dá push/PR" é hoje o gargalo real: o
orquestrador empurra e abre PR à mão. Com CI required, a sessão pode abrir o próprio PR (o merge
continua humano). Vale também registrar, para os agentes, o que o CI **não** cobre (E2E, app real) —
o OpenSSF é explícito que código de IA "não é atalho para os processos de engenharia"
([OpenSSF](https://best.openssf.org/Security-Focused-Guide-for-AI-Code-Assistant-Instructions)).

---

## 8. Custo e tempo, resumido

- **Dinheiro**: R$0. Runner padrão em repo público é grátis, inclusive macOS arm64.
- **Tempo por PR (pacote mínimo)**: ~4–6 min (TS 1–2 min + Rust 3–4 min, rodando em paralelo).
- **Cache**: `Swatinem/rust-cache` derruba o Rust de ~90 s para ~20–30 s; cache de bun idem.
  `cargo-mutants` e CodeQL são os itens caros e ficam fora do PR.
- **Custo de manutenção** (o que realmente importa com 1 mantenedor): cada camada nova é um lugar
  onde um agente pode gastar meia hora tentando entender por que o check falhou. Prefira 5 checks
  que ninguém discute a 15 que ninguém lê.

---

## 9. Não verificado

- Nenhum workflow foi escrito ou executado; tempos de CI para o pacote proposto são estimativas a
  partir das medições locais (Mac rápido, cache quente) + especificação dos runners (3 vCPU/7 GB).
- `cargo-machete`, `knip`, `cargo-deny`, CodeQL, `cargo-mutants` e `nextest`: capacidades vêm da
  documentação/README; **nenhum foi rodado neste repo**.
- E2E: tempo total dos 61 specs não medido (não se roda E2E junto com `dev`; portas e fixture são
  estado compartilhado).
- `noUncheckedIndexedAccess` no `tsconfig`: quantos erros aparecem é desconhecido.
- `.NET 8.0.408` em `macos-15` via `setup-dotnet`: presumido a partir da versão pinada nas fixtures,
  não exercitado.

## 10. Fontes

GitHub: [billing do Actions](https://docs.github.com/en/billing/managing-billing-for-your-products/managing-billing-for-github-actions/about-billing-for-github-actions) ·
[runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) ·
[rulesets disponíveis](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets) ·
[rulesets (Enterprise, metadados)](https://docs.github.com/en/enterprise-cloud@latest/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets) ·
[branch protection](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches) ·
[merge queue](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-a-merge-queue) ·
[secret scanning](https://docs.github.com/en/code-security/secret-scanning/introduction/about-secret-scanning) ·
[push protection](https://docs.github.com/en/code-security/secret-scanning/introduction/about-push-protection) ·
[code scanning](https://docs.github.com/en/code-security/code-scanning/introduction-to-code-scanning/about-code-scanning-with-codeql) ·
[CodeQL: linguagens](https://codeql.github.com/docs/codeql-overview/supported-languages-and-frameworks/) ·
[secure use / SHA pinning](https://docs.github.com/en/actions/reference/security/secure-use) ·
[attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations) ·
[CODEOWNERS](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners) ·
[PR template](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/creating-a-pull-request-template-for-your-repository) ·
[PR↔issue](https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/linking-a-pull-request-to-an-issue) ·
[Dependabot: ecossistemas](https://docs.github.com/en/code-security/dependabot/ecosystems-supported-by-dependabot/supported-ecosystems-and-repositories) ·
[Copilot code review](https://docs.github.com/en/copilot/using-github-copilot/code-review/using-copilot-code-review) ·
[imagem macos-15 (SDKs .NET)](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-Readme.md)

Rust: [Clippy: grupos de lints](https://doc.rust-lang.org/clippy/lints.html) ·
[cargo-deny](https://embarkstudios.github.io/cargo-deny/) ·
[cargo-nextest: retries](https://nexte.st/docs/features/retries/) ·
[cargo-nextest: partições](https://nexte.st/docs/ci-features/partitioning/) ·
[cargo-mutants](https://mutants.rs/) ·
[cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) ·
[cargo-machete](https://github.com/bnjbvr/cargo-machete) ·
[cargo-udeps (nightly)](https://github.com/est31/cargo-udeps) ·
[resolver e MSRV](https://doc.rust-lang.org/cargo/reference/resolver.html) ·
[git blame ignore-revs](https://git-scm.com/docs/git-blame)

TS/UI: [knip](https://knip.dev/) · [StrykerJS: configuração e runners](https://stryker-mutator.io/docs/stryker-js/configuration/) · [Bun: instalar em CI](https://bun.com/docs/install/ci)

Tauri: [WebDriver](https://v2.tauri.app/develop/tests/webdriver/) · [exemplo de CI](https://v2.tauri.app/develop/tests/webdriver/ci/) · [pipeline GitHub/tauri-action](https://v2.tauri.app/distribute/pipelines/github/)

Engenharia/segurança: [OpenSSF — Security-Focused Guide for AI Code Assistant Instructions](https://best.openssf.org/Security-Focused-Guide-for-AI-Code-Assistant-Instructions) ·
[Google — code coverage best practices](https://testing.googleblog.com/2020/08/code-coverage-best-practices.html) ·
[Google — small CLs](https://google.github.io/eng-practices/review/developer/small-cls.html) ·
[OpenSSF Scorecard (checks)](https://github.com/ossf/scorecard/blob/main/docs/checks.md) ·
[zizmor](https://github.com/zizmorcore/zizmor) · [actionlint](https://github.com/rhysd/actionlint) ·
[danger-js](https://danger.systems/js/) ·
[action-semantic-pull-request](https://github.com/amannn/action-semantic-pull-request)
