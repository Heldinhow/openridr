# SPEC — CI em pull requests e sessão que abre o próprio PR

Status: ready-for-agent, issue-mãe #56. Insumo: [research/ci-quality.md](research/ci-quality.md).
Medições de 2026-11-06 no `main` (`4df5c1a`).

## Problem Statement

Hoje nenhum check roda em ambiente limpo: a qualidade de um PR é o que a sessão de agente executou
no próprio Mac. Três consequências, todas observadas:

- Um PR pode ser mergeado com `tsc`, testes unitários ou testes do host quebrados — ninguém rodou
  fora da máquina de quem escreveu.
- `cargo fmt --check` e `cargo clippy -D warnings` **não podem** ser checks hoje: `main` tem 596
  hunks de `fmt` (32 arquivos + 2 em `src-tauri`) e ~40 avisos reais de clippy. Qualquer workflow que
  os exija nasce vermelho em todo PR.
- O agente terminou a tarefa e a sessão para: push e PR são proibidos pelo `AGENTS.md`, então o
  orquestrador empurra e abre cada PR à mão, e o corpo do PR não tem contrato (nem issue ligada, nem
  prova).

## Solution

Duas entregas, em dois PRs, porque uma delas é mecânica e a outra é julgamento:

1. **PR mecânico** (branch `ci/issue-56-fmt`): `cargo fmt` num big bang + `.git-blame-ignore-revs`, e
   os avisos de clippy zerados para que `-D warnings` passe. Nenhuma mudança de comportamento.
2. **PR de CI** (branch `ci/issue-56-pr-checks`): workflow de PR com jobs TS, Rust e governança,
   gitleaks como check `security`, smoke E2E noturno, Dependabot, pin de toolchain, template de PR,
   ruleset da `main` **preparado mas não aplicado**, e `AGENTS.md` dizendo que a sessão que termina
   faz push da branch e abre o próprio PR.

Critério de pronto: um PR mostra os checks; um lint/teste falhando impede o merge pelo ruleset; o
`AGENTS.md` descreve o passo de push + PR. "Check verde" passa a significar "verde num runner do
GitHub", não "verde no Mac de alguém".

## User Stories

1. Como mantenedor, quero que todo PR para `main` rode type check e testes unitários de TS num
   runner Linux limpo, para que o PR não dependa da máquina de quem o escreveu.
2. Como mantenedor, quero que todo PR rode `cargo fmt --check`, `cargo clippy -D warnings` e os testes
   do pacote host num runner macOS arm64, para pegar regressão de Rust antes do merge.
3. Como mantenedor, quero que o runner tenha o SDK .NET `8.0.408` que as fixtures fixam, para que os
   testes do host não falhem por SDK ausente.
4. Como mantenedor, quero um único check obrigatório de nome estável, para não reescrever o ruleset
   quando um job interno mudar de nome.
5. Como mantenedor, quero que um PR sem referência a issue ou sem seção de Evidência falhe, para que
   "terminado" nunca signifique "sem prova".
6. Como mantenedor, quero lint do título do PR (conventional), para que o squash gere um histórico
   legível sem depender de o agente escrever a mensagem certa no commit.
7. Como mantenedor, quero os próprios workflows passados por `actionlint` e `zizmor`, porque são
   YAML gerado por agente rodando com token.
8. Como mantenedor, quero segredo detectado em PR e em `main` (gitleaks), para não depender só do
   push protection do GitHub.
9. Como mantenedor, quero os workflows pinados por SHA e Dependabot cuidando desses pins, bun e
   cargo, para que supply chain não dependa de alguém lembrar.
10. Como mantenedor, quero um smoke E2E noturno de um subconjunto de specs, para pegar regressão de
    UI sem pagar 61 specs em cada PR.
11. Como sessão de agente, quero abrir meu próprio PR com Summary/Evidence/Merge Danger, para não
    depender do orquestrador para empurrar a branch.
12. Como sessão de agente, quero saber o que o CI **não** cobre (E2E, app real), para escrever a
    evidência honesta em vez de tratar check verde como prova de UI.
13. Como revisor, quero o ruleset descrito em documento e em JSON aplicável, para que a proteção da
    `main` seja revisável antes de mudar settings do GitHub.
14. Como mantenedor, quero o toolchain Rust pinado no repo, para que CI e Mac compilem com a mesma
    versão e o MSRV deixe de ser desconhecido.
15. Como mantenedor, quero que `git blame` ignore o commit de formatação, para não perder a autoria
    do código com o big bang.
16. Como sessão de agente, quero que a verificação de UI seja feita com `agent-desktop` no app real (e
    Playwright só no smoke noturno), para não confundir harness de E2E com prova de sessão.

## Implementation Decisions

### PR 1 — mecânico (`ci/issue-56-fmt`)

- `cargo fmt --all` no workspace (`crates/host`, `crates/host-http`) **e** em `src-tauri`
  (`--manifest-path src-tauri/Cargo.toml`): `src-tauri` é workspace separado e tem 2 hunks próprios.
- Um commit só de formatação, sem nenhuma edição manual; o hash desse commit vai para
  `.git-blame-ignore-revs` num segundo commit (com o comando `git config blame.ignoreRevsFile` no
  próprio arquivo, como comentário).
- Clippy: `cargo clippy --fix` para o que é mecânico e, no resto, ou correção pontual sem mudança de
  comportamento, ou `#[allow(…)]` com comentário de uma linha dizendo por quê (`too_many_arguments` e
  `large_enum_variant` são estruturais e ficam allow; `Default` faltando vira `impl` real). O
  contrato é `cargo clippy --workspace --all-targets -- -D warnings` sair limpo.
- Nenhum teste muda de expectativa; se um teste quebrar por causa de um fix de clippy, o fix está
  errado.

### PR 2 — CI (`ci/issue-56-pr-checks`)

- **Um workflow de PR** (`.github/workflows/ci.yml`): `pull_request` para `main`, `push` em `main` e
  `workflow_dispatch` (para poder exercitar o workflow a partir de um branch antes de ele ser
  obrigatório). `concurrency` com `cancel-in-progress` por ref; `permissions: contents: read`
  no topo e ampliado só no job que precisa; `actions/checkout` com `persist-credentials: false`.
- **Job `typescript`** (ubuntu): bun na versão do repo (`1.3.9`), `bun install --frozen-lockfile`,
  `bunx tsc --noEmit`, `bun test tests/unit`.
- **Job `rust`** (macos-15): toolchain instalada pelo `rust-toolchain.toml` via `rustup` (o runner já
  tem rustup; nenhuma action de toolchain a pinar), cache de cargo, `cargo fmt --all -- --check` e o
  mesmo check para `src-tauri`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `actions/setup-dotnet` com `8.0.408`, `cargo test -p openridr-host`.
  - Clippy cobre só o workspace principal: `src-tauri` compila o Tauri inteiro e não entra em PR.
    Fica escrito no comentário do workflow e no `AGENTS.md`.
- **Job `governance`** (ubuntu): `actionlint` e `zizmor` sobre `.github/workflows/`, lint do título do
  PR (conventional, com a lista de tipos usada no repo) e gate do corpo do PR.
- **Gate do corpo do PR**: um script do repo (não uma chamada inline de 30 linhas no YAML) exige
  referência a issue (`Closes/Fixes/Resolves/Part of/Refs #N`) e um heading de Evidência com conteúdo;
  aceita PT e EN; PRs do Dependabot são isentos. O script tem self-check no molde
  `scripts/*-selfcheck.sh` do repo, e o self-check roda no próprio job.
- **Job `security`**: gitleaks sobre o range de commits do PR (histórico completo no push para
  `main`). O "security review por PR que bloqueia achado alto" **não** entra aqui: depende de revisão
  por LLM com credencial, então vira issue filha bloqueada por decisão do Helder. Até lá, o check
  `security` é determinístico (segredo) e o corpo do PR carrega o bloco de revisão de segurança.
- **Job `ci`**: agregador `needs:` dos quatro, `if: always()`, falha se qualquer um falhou. É o nome
  estável do check obrigatório — renomear job interno não mexe no ruleset.
- **Smoke E2E noturno** (workflow separado, `schedule` + `workflow_dispatch`, macos-15): lê a lista de
  specs de `scripts/e2e-smoke.txt`, que é da quarentena da #67 e já chegou pronta (`workspace`,
  `build`, `run`, `qa2-diagnostics`, 25 testes, ~2,8 min medidos lá). Uma linha por spec, caminho
  relativo à raiz, `#` comentário; lista vazia ou caminho que não existe falha com mensagem. Nunca
  roda em PR. Sem Chrome no runner, usa o Chromium do Playwright (`PLAYWRIGHT_CHANNEL=chromium`).
- **Pin de toolchain**: `rust-toolchain.toml` com `channel = "1.99.0"` (a versão em que o repo
  compila), componentes `rustfmt` e `clippy`, e `rust-version = "1.99"` nos três crates. Decisão
  registrada: MSRV = a toolchain pinada; baixá-la exige medição, não é promessa.
- **Dependabot** (`.github/dependabot.yml`): bun na raiz, cargo na raiz, cargo em `src-tauri`,
  github-actions na raiz; semanal, agrupado por ecossistema, limite de PRs abertos.
- **Template de PR** (`.github/pull_request_template.md`): `Closes #N`, `## Summary`, `## Evidence`,
  `## Security review`, `## Merge danger`. Headings em inglês; o gate aceita PT e EN.
- **Ruleset da `main`**: `.github/rulesets/main.json` (PR obrigatório, checks `ci` e `security`,
  histórico linear, squash-only, sem force-push nem delete) + passo a passo de aplicação em doc.
  **Não aplicado** nesta entrega: settings do GitHub são decisão do Helder.
- **`AGENTS.md`**: a regra "sem push/PR" vira o fluxo de sessão que empurra a branch e abre o próprio
  PR (merge continua com mantenedor/orquestrador, e PR que toca `.github/workflows/**`,
  `scripts/release*`, assinatura ou dependências pede aprovação do Helder). A seção de verificação
  passa a dizer que UI de sessão é `agent-desktop` e que Playwright é só o smoke noturno; e ganha o
  que o CI não cobre.

## Testing Decisions

Um teste bom aqui prova comportamento externo: um workflow é "certo" quando um YAML inválido,
inseguro ou um corpo de PR incompleto faz o check falhar, e um PR correto passa.

- **Gate do corpo**: é o único código novo de verdade. Teste = self-check do script com corpos
  sintéticos (com issue e evidência; sem issue; sem evidência; evidência vazia; PR do Dependabot),
  rodado localmente e no job `governance`. Prior art: `scripts/release-macos.sh --check` e
  `scripts/openridr-selfcheck.sh`.
- **YAML dos workflows**: `actionlint` + `zizmor` são o teste (é para isso que eles entram no job).
  Nenhum teste novo em TS ou Rust: o PR 2 não toca produto.
- **PR 1**: a prova é `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D
  warnings` e a suíte atual (`cargo test -p openridr-host`, `bunx tsc --noEmit`, `bun test
  tests/unit`) continuarem verdes. Falha-antes: os três primeiros estão vermelhos no `main` hoje.

## Out of Scope

- Lint de TypeScript (ESLint/Biome): dependência nova, decisão própria.
- CodeQL, `cargo-deny`, `knip`, `cargo-machete`, `cargo-mutants`, cobertura: camadas seguintes da
  pesquisa.
- E2E em PR e qualquer decisão sobre manter/migrar Playwright (#43, #67).
- Ruleset aplicado, auto-merge e rótulo de risco (#74).
- Revisão de segurança por LLM com credencial: issue filha, bloqueada por decisão do Helder.
- Consertar teste instável (#43, #57, #59, #62): quarentena com motivo, não conserto silencioso.
- `rust-version` de verdade abaixo de 1.99 (medição de MSRV).

## Further Notes

- Custo em dinheiro: R$0 (runner padrão em repo público). Custo em tempo: ~5 min por PR no pacote
  mínimo, rodando TS e Rust em paralelo; o passo caro é o `setup-dotnet` (~1 min, sem cache do SDK).
- O que o CI **não** cobre: E2E, app Tauri real, visual, o que depende de rede (NuGet/csharp-ls).
  Evidência de UI continua sendo `agent-desktop` na sessão.
- Nome dos checks é contrato com o ruleset: `ci` e `security`. Mudar exige atualizar
  `.github/rulesets/main.json` no mesmo commit.
- Evidência desta frente: `docs/evidence/ci-pr.md`; retrospectiva em `docs/RETRO-CI-PR.md`.
