# AGENTS.md — OpenRidr

IDE C# inspirada no Rider: Tauri 2 + Rust (host) + React/TS (UI). Alvo verificado hoje: macOS ARM64. Windows e macOS Intel não foram executados — não os trate como suportados sem evidência.

## Leia antes de mexer

- `docs/MAPA.md` — **onde fica o quê** no código (inclui como achar a dona de um `data-testid` ou método do host). `WorkspaceView.tsx` (~5.200 linhas) e `crates/host/src/lib.rs` (~7.700) nunca se leem inteiros: `grep -n`, depois `sed -n`.
- `docs/README.md` — índice dos docs: feature → spec/tickets/retro/evidência, e o que é histórico. Não use `docs/PROPOSTA-UI-*`, `release-decision.md`, `index.html`, `wireframes.md` como estado atual.
- `docs/SPEC.md` — contrato do produto (P0).
- `docs/SPEC-MVP-RIDER.md` e `docs/tickets-mvp/` — fatias F1–F9 e critérios de aceite.
- `docs/decisions.md` — decisões incrementais; siga-as e registre uma nova em vez de reabrir uma antiga.
- `docs/implementation-notes.md` — como cada ticket foi feito e o que não foi verificado.
- `docs/REVIEW-MVP-RIDER.md` e `docs/RETRO-MVP-RIDER.md` — dívida e lições conhecidas.
- Trabalho novo segue o main flow das skills: grill → spec na issue-mãe → tickets → implement → `code-review`, **um PR por frente**, sem retro. Detalhes em `docs/README.md` e na decisão de 2026-11-07.

## Agent skills

### Issue tracker

Issues e tickets vivem como GitHub issues em `Heldinhow/openridr`, via `gh`. Ver `docs/agents/issue-tracker.md`.

### Triage labels

Cinco papéis canônicos com o mesmo nome do label (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`) mais `bug`/`enhancement`. Ver `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` na raiz + `docs/decisions.md` (o log de decisões faz o papel de `docs/adr/`) e o índice em `docs/README.md`. Ver `docs/agents/domain.md`.

## Estrutura

```text
src/                 React/TS da UI (editor, painéis, git, testes, terminal)
crates/host/         lógica e estado do host, sem dependência de Tauri
crates/host-http/    ponte HTTP só de dev/E2E (127.0.0.1, nunca distribuída)
src-tauri/           janela Tauri; chama o mesmo dispatch do host
tests/e2e/           specs Playwright contra host real + fixture real
fixtures/            soluções C# de teste (OrderDesk, MissingSdk, ...)
scripts/             install-app.sh, openridr (CLI abrir com), selfcheck
.scratch/            rascunho de sessões — não é fonte
docs/                SPECs, decisões, notas, evidência (tickets novos: GitHub issues)
docs/agents/         config das skills: issue tracker, labels, docs de domínio
```

A UI fala com o host por `dispatch`; não duplique lógica de host no React.

## Comandos

```sh
bun install
bun run dev            # Vite (UI em 1420)
bun run tauri dev      # app desktop completo
sh scripts/dev-app.sh  # app de dev com o par embutido (worktree nova); aceita .sln em argv
bun run build          # tsc --noEmit + vite build
bun run check          # tsc --noEmit + cargo check --workspace --all-targets
bun run hooks          # uma vez por clone: aponta core.hooksPath para .githooks
bun run test:e2e       # Playwright; default 1420/4317, outro par se essas portas estiverem em uso
bun run host:http      # ponte HTTP de dev na 4317, com OPENRIDR_HOME em .scratch/host-home
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings  # job rust-tauri
cargo test --manifest-path src-tauri/Cargo.toml                                  # os 6 testes do pacote Tauri
```

Rust: os pacotes se chamam `openridr-host` e `openridr-host-http` (não `host`). `cargo test` aceita **um** filtro antes do `--`; para vários: `cargo test -p openridr-host -- filtro_a filtro_b`.

**Worktree nova não tem `node_modules`** (nem `target/` quente): rode `bun install` antes de `check`, `build` ou E2E. Sem isso, `tsc`/`playwright` falham e greps em `node_modules/` voltam vazios.

`check` continua sem `cargo fmt --check` nem `clippy -D warnings` para ser rápido; o CI roda os dois. Antes de empurrar a branch, rode `cargo fmt --all` (e `cargo fmt --manifest-path src-tauri/Cargo.toml` se tocou `src-tauri`) e `cargo clippy --workspace --all-targets -- -D warnings`: vermelho aí é vermelho no PR.

`src-tauri` é workspace separado: `--workspace` não o alcança. O CI **não roda os vendor scripts** (`vendor-analysis.sh`, `vendor-netcoredbg.sh`): `src-tauri/resources/csharp-ls/` e `src-tauri/resources/dotnet/` são gitignored e o `tauri-build` só exige que o caminho exista, então o job `rust-tauri` cria as duas pastas vazias com `mkdir -p`. Numa worktree nova, faça o mesmo antes do clippy/testes de `src-tauri`; sem elas o build falha com `resource path 'resources/dotnet' doesn't exist`.

### Falhas conhecidas (não são suas)

A fonte única é a tabela de [`docs/evidence/test-stability.md`](docs/evidence/test-stability.md): teste, sintoma, causa, status e ambiente. Resumo de 2026-11-06:

**`debugger.spec.ts` foi medido na issue #43** (12 de 13 falhando, 20,8 min): o setup do spec chama `debug.install`, que o adaptador embutido do host de E2E recusa com `invalid_state` — a causa é a mesma que o `docs/RETRO-DEBUG-NO-STOP.md` (#92) registrou. O spec ficou quarentenado com esse motivo em [`docs/evidence/e2e-quarantine.md`](docs/evidence/e2e-quarantine.md) (skip no arquivo), e os asserts novos do #92 continuam lá para quando ele for reescrito.

- **Rust (`cargo test -p openridr-host`): nenhuma falha conhecida.** Os dois testes que falhavam no `main` (`file_access_is_confined_to_the_workspace` e `evaluation_returns_the_real_msbuild_model_for_every_project`) foram corrigidos em `docs/evidence/file-access-confinement.md`; o do launch profile (usava o `dotnet` do PATH) e a corrida de `OPENRIDR_ENABLE_PROBE` foram corrigidos na issue #43. Qualquer falha em Rust agora é sua: investigue.
- **E2E: quarentenado, não consertado.** `find-usages` (intermitente), 2 em `run-configs.spec.ts`, 2 em `tests.spec.ts` (relatados em `docs/RETRO-DEVFLOW-2026-11-02.md`) e o bloco da issue #43 — `debugger.spec.ts` no fluxo antigo de `debug.install` (o netcoredbg virou bundle), `csharp-ls.spec.ts:150`, `testsuite.spec.ts` (`tests-filter-count`), `tests.spec.ts:317`, `ui-layout` em 1440 px — seguem fora do verde e bloqueados pela decisão do Helder sobre o Playwright (issue #67). Não os conserte por conta própria nem os trate como passe.
- **Ambiente não muda mais o resultado do Rust.** O harness limpa `OPENRIDR_*` herdado, isola a home e não usa nome fixo em `/tmp`. Se um teste do host falhar só na sua máquina, é um vazamento novo: registre na tabela em vez de exportar variável.

## .NET SDK 8.0.408

- Cada fixture fixa o SDK no próprio `global.json` (`8.0.408`, `rollForward: disable`). Não altere o pin nem o roll-forward.
- O host de análise usa o maior SDK estável 10.x detectado; o projeto usa o SDK do `global.json`.
- O app apenas detecta SDKs (PATH, `~/.dotnet`, `~/Library/Application Support/OpenRidr/sdks`). Nunca instala nem edita `global.json`.
- Qualquer raiz detectada pode satisfazer o pin: `~/.dotnet` é tão válido quanto `OpenRidr/sdks/8.0.408`. Specs e código não podem exigir o caminho gerenciado; o contrato é a versão resolvida (e o `sdkRoot` absoluto).
- Ferramentas vão para `~/Library/Application Support/OpenRidr/tools/<nome>/<versão>`; nunca global.

## E2E

- **A superfície de verificação segue o que mudou** (decisão do Helder, 2026-11-07). Mudança de interface pura → **Playwright em modo web**, só os specs da área, escrevendo/atualizando o spec do cenário. Nativo do shell — menu nativo, atalhos do sistema, updater, seletor nativo, árvore de acessibilidade do WebKit, single-instance, DMG — → **app Tauri real com `agent-desktop`**, uma instância por vez. O **host isolado** (`scripts/web.sh`, no skill `verify-openridr`) fica para o que não tem spec ou precisa de captura.
- **Playwright nunca em PR.** O único E2E automatizado é o smoke noturno (workflow da issue #56): a lista é [`scripts/e2e-smoke.txt`](scripts/e2e-smoke.txt) (`bunx playwright test $(grep -v '^#' scripts/e2e-smoke.txt)`) e a quarentena com motivos está em [`docs/evidence/e2e-quarantine.md`](docs/evidence/e2e-quarantine.md). Spec quarentenado não é passe. Uma run vermelha abre (ou reescreve) a issue de rastreio `[nightly] e2e-smoke is red` com os specs que falharam e o link da run; a primeira verde comenta o link e a fecha.
- **Portas: default 1420 (UI) / 4317 (host).** Sessão que roda em paralelo, ou junto com `bun run dev`, escolhe outro par (faixa 15xx/44xx) e confere com `lsof -nP -iTCP:<porta> -sTCP:LISTEN`; porta ocupada falha na hora pelo `strictPort` do Vite de teste. O app home do E2E é derivado da porta da UI, então pares diferentes não compartilham home. **Só o par do harness é escolhido**: o host auxiliar de um spec sobe em porta efêmera (`--port 0`) e `tests/e2e/isolated-host.ts` lê a porta que o kernel concedeu, então não existe faixa de portas reservadas acima do par.
- `bun run test:e2e` roda a suíte inteira e sobe o host e o Vite sozinho, na 1420 (UI) e 4317 (host). Portas alternativas: `OPENRIDR_E2E_UI_PORT` e `OPENRIDR_E2E_HOST_PORT`. As capturas caem em `<tmpdir>/openridr-e2e-screenshots-<porta da UI>`, nunca no repositório; `docs/evidence/screenshots/` só recebe captura com `OPENRIDR_SCREENSHOT_DIR` apontando para lá de propósito.
- No Mac com o SDK em `~/.dotnet`, rode com `PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet DOTNET_CLI_UI_LANGUAGE=en bun run test:e2e` (ou `bunx playwright test tests/e2e/<spec>.ts` para um spec).
- `playwright.config.ts` usa `channel: "chrome"` por padrão. Sem Chrome (ex.: VPS Linux), rode `PLAYWRIGHT_CHANNEL=chromium bun run test:e2e`.
- Os specs usam `fixtures/OrderDesk` de verdade e restauram o arquivo; workers = 1 porque host e arquivos são estado compartilhado.
- Specs que dependem de dotnet/csharp-ls/netcoredbg real fazem `skip` quando o ambiente não tem a ferramenta. Skip não é passe: leia o motivo e registre "não verificado".
- Não rode a suíte inteira por reflexo: rode os specs da área. O par de portas separa worktrees, não dois specs do mesmo worktree — a fixture é a mesma e `test-results/` também (duas sessões no mesmo worktree rodam com `--output` próprio, ou uma apaga o trace da outra).

### Projeto de referência para validar no app real (repo privado)

`/Users/helder/Projetos/gabagool-dotnet` (`Gabagool.sln`, 5 projetos `net8.0`: Api, Application, Domain, Infrastructure, Tests com xUnit; `global.json` no SDK `8.0.408`, `rollForward: latestPatch`). Além das fixtures, é o alvo para validar no app real: abrir a solução, Problems, Build, Run da Api, Tests, Debug e Git.

- É repositório **privado** do Helder: nunca edite a pasta original — copie para `/tmp` antes de abrir, compilar ou mexer.
- Nunca copie código, nomes de domínio, capturas ou trechos dele para este repo público: a evidência descreve só o resultado, e print só se não mostrar código dele.
- Nunca faça commit nem push nele.

## Pull request e CI

**Um PR por frente, não por ticket.** A sessão empurra a branch de integração (`git push -u origin <branch>`) e abre o PR **draft** depois do primeiro merge nela, marcando-o pronto no fim; o corpo é o do `.github/pull_request_template.md` (`Closes` da mãe e de cada ticket, `## Summary`, `## Evidence`, `## Security review`, `## Merge danger`). Sem referência a issue e sem Evidência o job `governance` reprova o PR: é contrato, não sugestão.

- **Merge não é da sessão.** Com o CI verde, quem mergeia é o mantenedor/orquestrador — exceto `risk:low`, que o GitHub mergeia sozinho quando os checks obrigatórios ficam verdes (ver “Risco do PR” abaixo). A sessão não faz merge, não dá `--force` na branch depois do PR aberto e nunca usa `--no-verify`.
- **PR de risco alto pede o Helder**: `.github/workflows/**`, `.github/rulesets/**`, `AGENTS.md`, `scripts/release*`/`vendor-*`/`ci-*`, assinatura/entitlements, `src-tauri/tauri.conf.json`, o modelo de trust e mudança de dependência (`Cargo.lock`/`bun.lock`, ou tabela de dependência em `Cargo.toml`/`package.json`). O tier é calculado pelos caminhos (não por quem escreveu) e o gate não fica verde até a aprovação — ou, quando o autor do PR é o dono do repo, sem review nenhuma: ver “Risco do PR”.
- **Squash de PR misto não é ignorável no blame.** `.git-blame-ignore-revs` só pode listar commits que existem na `main`, e o squash agrega o PR inteiro num commit só: se ele mistura formatação com mudança real (foi o caso do #89, squash `92722bc`: `cargo fmt` + clippy + guarda de teste + `make_writable`), ignorá-lo esconderia a edição. Formatação em massa vai num PR **separado** e sem nenhuma edição manual; sem isso, o blame aponta para o squash.
- **Settings do GitHub não são da sessão**: ruleset, labels e auto-merge ficam em `.github/rulesets/main.json` e [docs/ci-main-ruleset.md](docs/ci-main-ruleset.md), para o Helder aplicar.

O que o CI (`.github/workflows/ci.yml`) roda em todo PR para `main`:

| Job | O que roda |
|---|---|
| `typescript` (ubuntu) | `bun install --frozen-lockfile`, `tsc --noEmit`, `vite build`, `bun test tests/unit` |
| `rust` (macos-15) | `cargo fmt --check` (workspace e `src-tauri`), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p openridr-host` com o SDK 8.0.408 das fixtures |
| `rust-tauri` (macos-15) | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` e `cargo test --manifest-path src-tauri/Cargo.toml`, com as duas pastas de resource vazias; **informativo** |
| `governance` | `actionlint`, `zizmor`, título do PR no padrão conventional, corpo com issue + Evidência |
| `risk` | tier calculado pelos caminhos, label `risk:*`, no alto o PR do dono passa sem review e os demais precisam de aprovação, auto-merge no baixo (`scripts/ci-risk-tier.sh`, `scripts/ci-risk-gate.sh`) |
| `security` | `gitleaks` no histórico do PR |
| `ci` | agregador (`scripts/ci-aggregate.sh`); é o check obrigatório do ruleset |

O `rust-tauri` é **informativo**: `continue-on-error: true` faz o `result` do job virar `success`, então o agregador `ci` fica verde e o ruleset não muda. Promoção a bloqueante = remover a flag num PR próprio, depois de **5 execuções verdes consecutivas** (pelo menos uma com cache frio), duração estável abaixo de 5 min e nenhum flake; o check obrigatório continua sendo o agregador `ci`, então promover não toca em setting do GitHub.

O smoke E2E é noturno e não bloqueia PR: a lista de specs está em `scripts/e2e-smoke.txt` (caminho relativo à raiz, um por linha) e quem manda nela é a quarentena da #67.

Antes de empurrar um PR que mexe em `scripts/` ou em `tests/shell/`, rode o que o CI vai rodar, com as duas ferramentas que o Mac não traz de fábrica: `brew install shellcheck dash` (o `actionlint` do job `governance` liga o shellcheck por padrão; sem ele, o local passa e o run reprova) e **`dash <arquivo>`** em vez de `sh <arquivo>` — o `/bin/sh` do runner é dash, o do Mac é bash, e `10#`, `local` e `[[ ]]` passam num e falham no outro.

### Risco do PR: tier pelos caminhos, auto-merge no baixo, aprovação no alto

O job `risk` calcula o tier pelos caminhos que o PR toca (`scripts/ci-risk-tier.sh`, tabela em
[docs/SPEC-RISK-TIERS.md](docs/SPEC-RISK-TIERS.md)) e rotula o PR:

| Tier | O que é | O que muda para a sessão |
|---|---|---|
| `risk:low` | `docs/**`, `*.md` (exceto `AGENTS.md`), `tests/**`, `crates/*/tests/**`, `*.css` | nada: o job arma auto-merge e o PR entra em squash quando `ci` e `security` ficam verdes |
| `risk:medium` | código de produto (`src/**`, `crates/**`, `src-tauri/**`, `fixtures/**`) e o **default** para caminho que nenhuma regra conhece | entrega como hoje: o orquestrador confere a Evidência antes do merge |
| `risk:high` | CI, rulesets, `AGENTS.md`, `scripts/release*`/`vendor-*`/`ci-*`, entitlements, `tauri.conf.json`, trust, mudança de dependência | abre o PR e para: sem auto-merge, o merge é manual do mantenedor/orquestrador |

- **O label não é entrada do gate.** Quem decide é o cálculo pelos caminhos; editar o rótulo não baixa o tier nem substitui review.
- **No alto, o PR do dono do repo passa sem review** (decisão do Helder de 2026-11-07, opção (b) da #149): o GitHub recusa autoaprovação (422) e o repo tem um mantenedor só, então o gate aceita `author_association == OWNER` do **autor** do PR. Custo aceito e escrito em [docs/decisions.md](docs/decisions.md): o tier alto não significa mais “outra pessoa olhou”.
- **Para qualquer outro autor, aprovação no alto é review `APPROVED` na revisão atual**: de owner/member/collaborator, apontando para o commit que é o head do PR. Trazer `main` para o branch cria um commit e invalida a aprovação anterior: o mantenedor aprova de novo. Um `CHANGES_REQUESTED` posterior do mesmo revisor retira a aprovação.
- **O classificador e o gate rodam da revisão base**, não do PR (um PR que muda `scripts/ci-*` não pode classificar nem aprovar a si mesmo). A brecha que sobra é o YAML do próprio workflow, que num `pull_request` vem do merge commit: mexer nele é `risk:high` e aparece no diff.
- **Depois do APPROVED, o gate precisa de rerun:** `gh run rerun --failed <run-id>` (só os jobs que falharam rodam; o Rust não é refeito). O evento de review não dispara o workflow, de propósito — um workflow separado exigiria um terceiro check obrigatório no ruleset. No PR do dono não há APPROVED nem rerun: o gate fica verde no primeiro run.
- **Auto-merge depende de setting do repo** (`allow_auto_merge`), ligado desde 2026-11-06: o job arma o merge no tier baixo e o GitHub faz o squash quando `ci` e `security` ficam verdes. Se o setting for desligado, o job avisa e o PR espera alguém — nunca reprova por isso. Estado e comando em [docs/ci-main-ruleset.md](docs/ci-main-ruleset.md).
- **Mudar o gate é mudança de risco alto** (`scripts/ci-*`, `.github/workflows/**`): antes de empurrar, rode `sh scripts/ci-risk-tier-selfcheck.sh`, `sh tests/shell/ci-aggregate.test.sh`, `sh tests/shell/ci-risk-gate.test.sh`, `GATE_SHELL=dash sh tests/shell/ci-risk-gate.test.sh` e o mesmo par para `tests/shell/ci-workflow-bootstrap.test.sh` (o `GATE_SHELL` é o que faz o gate rodar sob o `/bin/sh` do runner, não só o teste), `actionlint` e `zizmor --offline --min-severity low .`. A tabela de risco tem uma fonte só: o código de `scripts/ci-risk-tier.sh` mais o teste; mudar um dos dois sem o outro deixa o `self-check` vermelho.
- **PR do Dependabot cai em alto** (lockfile é mudança de dependência) e passa pelo mantenedor: é intencional.

### O que o CI não cobre

- **Nada de UI**: nenhum Playwright em PR, nenhum app Tauri real, nenhum visual. Check verde não é prova de que a tela funciona: a prova de UI é da sessão, pela superfície que a mudança pede (ver `## E2E`).
- E2E completo e tudo o que depende de rede (NuGet, csharp-ls).
- `src-tauri` fica fora do clippy **do job `rust`** (workspace separado, compila o Tauri inteiro): quem o cobre é o `rust-tauri`, informativo por enquanto.
- Windows e macOS Intel continuam não verificados.

Por isso a Evidência do PR é obrigatória e é da sessão: comando e resultado do que rodou, e "não verificado: <motivo>" no que não rodou.

### Teto de três rodadas de revisão

Depois de **três rodadas** de review/correção no mesmo PR sem verde, pare e repense a abordagem em vez de
emendar a quarta: escreva em qual decisão o caminho está errado (issue do PR, ou um comentário no PR com
a mudança de plano proposta) e espere. Cada rodada é uma passada completa de revisão com seus achados,
não cada commit.

**Todo achado é corrigido ou respondido por escrito.** Achado de revisão não é sugestão: ou entra no
commit, ou fica no PR com o motivo concreto de não entrar (contraria a spec, fora de escopo com a issue
filha aberta, risco maior que o problema). "Depois" não é resposta.

## Verificação antes de entregar

O passo `/code-review` é, por padrão:

1. **Checagem objetiva e curta** do diff contra a spec/ticket e os padrões deste arquivo (`## Regras`, `data-testid`, dependências). Lista só desvios concretos com `arquivo:linha`.
2. **Rodar o skill `verify-openridr`** (`.agents/skills/verify-openridr/SKILL.md`) nas features tocadas, pela superfície que a mudança pede: **Playwright em modo web** para interface pura (só os specs da área, escrevendo/atualizando o spec do cenário); **app Tauri real com `agent-desktop`** (receitas em `.agents/skills/test-app-agent-desktop`) só para o que é nativo do shell — menu nativo, atalhos do sistema, updater, seletor nativo, árvore de acessibilidade do WebKit, single-instance, DMG; **host isolado** (`web.sh`) para o que não tem spec ou precisa de captura. O que não rodou entra como "não verificado".
3. **`code-review` de dois eixos** (Standards e Spec, em subagentes paralelos) na branch de integração depois de todos os tickets, antes de o PR ficar pronto para review: `delegate_task` do T3 com target Pi, model `openai/gpt-6.1-terra` e option `thinking=medium`; fixed point = `git merge-base origin/main HEAD`; spec = a issue-mãe (e as filhas). Corrija achado alto/médio na hora; baixo, só se for simples. Sem `delegate_task` disponível: avise o orquestrador com o SHA do commit e siga. Revisão adversarial aberta/multi-agente continua só se o usuário pedir.

O fluxo é grill → spec → tickets → implement → `code-review`, **sem `/retro`**: `docs/RETRO-*.md` e `docs/SPEC-*.md` são histórico e não recebem frente nova. Propostas de verificação formal: [docs/VERIFICACAO-FORMAL.md](docs/VERIFICACAO-FORMAL.md); a PoC de trust roda com `cargo test -p openridr-host --test trust_model`.

## Regras

- **`data-testid` são contrato**: não renomeie nem remova os existentes; cada fatia adiciona os seus e mantém `role="tab"` no cabeçalho inferior.
- **Zero dependência nova** de UI/runtime sem decisão registrada e aprovação explícita.
- Um componente por arquivo; comentários em inglês; uma função de erro por fronteira (use `messageOf`/`requireAction` em vez de repetir mapeamento).
- **Prova real, não inventada**: o que o ambiente não permitiu executar vai como "não verificado" no `implementation-notes.md` e no ticket.
- **Ticket**: a spec de uma frente é o corpo da issue-mãe e cada ticket é uma sub-issue dela (ou `Part of #N` na primeira linha do corpo), com bloqueio nativo ou `Blocked by: #M`. Nada de `docs/SPEC-*` nem `docs/RETRO-*` para trabalho novo — os existentes e os `docs/tickets-*/` são histórico: não migre, não crie pasta nova no molde deles. Evidência continua em `docs/evidence/<tema>.md`.
- **Commit**: um por ticket/fatia, mensagem convencional (`feat(git): ...`, `fix(search): ...`, `docs: ...`, `chore: ...`), com código, testes e checklist atualizado.
- **Stage explícito**: use `scripts/commit-only.sh "msg" <paths>` (faz o `git add -N` dos novos e `git commit --only -m "msg" -- <paths>`); à mão, é `git commit --only -m "msg" -- <paths>`; nunca use `git add -A` nem altere o index do usuário. Duas armadilhas que falharam 38 vezes em 10 sessões: (1) `-m` vai **antes** do `--` (depois dele vira pathspec); (2) arquivo/pasta **novo** precisa de `git add -N <paths>` antes (só marca intenção), senão dá `pathspec ... did not match`. Deixe `dist/`, `target/`, `test-results/` e afins fora (ver `.gitignore`).
- **Push e PR são da sessão; merge não.** A frente termina com a branch de integração empurrada e o PR aberto, pronto para review — o quando e o corpo estão em "Pull request e CI". Nunca merge feito pela sessão, nunca `--force` na `main`, nunca `--no-verify` — o hook de pre-commit roda o check proporcional ao diff (docs: nada; TS: `tsc --noEmit`; Rust: `cargo check`; misto: os dois) e `bun run check` continua sendo o completo.
