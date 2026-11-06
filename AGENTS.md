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
- Trabalho novo segue a convenção de `docs/README.md`: `SPEC-<tema>.md`, **tickets como GitHub issues**, `RETRO-<tema>.md`, `evidence/<tema>.md`.

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
bun run build          # tsc --noEmit + vite build
bun run check          # tsc --noEmit + cargo check --workspace --all-targets
bun run hooks          # uma vez por clone: aponta core.hooksPath para .githooks
bun run test:e2e       # Playwright; não rode junto com dev (portas 1420/4317)
bun run host:http      # ponte HTTP de dev na 4317, com OPENRIDR_HOME em .scratch/host-home
```

Rust: os pacotes se chamam `openridr-host` e `openridr-host-http` (não `host`). `cargo test` aceita **um** filtro antes do `--`; para vários: `cargo test -p openridr-host -- filtro_a filtro_b`.

**Worktree nova não tem `node_modules`** (nem `target/` quente): rode `bun install` antes de `check`, `build` ou E2E. Sem isso, `tsc`/`playwright` falham e greps em `node_modules/` voltam vazios.

`check` não inclui `cargo fmt --check` nem `clippy -D warnings`: o código atual não passa em nenhum dos dois. Não reformate o repo inteiro dentro de um ticket; formate apenas o que tocar até essa dívida ser paga.

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

- **Playwright é só smoke noturno** (decisão do Helder, issue #67): nunca em PR, nunca como verificação de UI por sessão de agente — sessão verifica UI com `agent-desktop`. A lista do smoke é [`scripts/e2e-smoke.txt`](scripts/e2e-smoke.txt) (`bunx playwright test $(grep -v '^#' scripts/e2e-smoke.txt)`); a quarentena com motivos está em [`docs/evidence/e2e-quarantine.md`](docs/evidence/e2e-quarantine.md). O workflow noturno é a issue #56.
- `bun run test:e2e` roda a suíte inteira e sobe o host e o Vite sozinho, na 1420 (UI) e 4317 (host). Portas alternativas: `OPENRIDR_E2E_UI_PORT` e `OPENRIDR_E2E_HOST_PORT`. Não use `OPENRIDR_SCREENSHOT_DIR` default se não quiser reescrever `docs/evidence/screenshots/`.
- No Mac com o SDK em `~/.dotnet`, rode com `PATH=$HOME/.dotnet:$HOME/.dotnet/tools:$PATH DOTNET_ROOT=$HOME/.dotnet DOTNET_CLI_UI_LANGUAGE=en bun run test:e2e` (ou `bunx playwright test tests/e2e/<spec>.ts` para um spec).
- `playwright.config.ts` usa `channel: "chrome"` por padrão. Sem Chrome (ex.: VPS Linux), rode `PLAYWRIGHT_CHANNEL=chromium bun run test:e2e`.
- Os specs usam `fixtures/OrderDesk` de verdade e restauram o arquivo; workers = 1 porque host e arquivos são estado compartilhado.
- Specs que dependem de dotnet/csharp-ls/netcoredbg real fazem `skip` quando o ambiente não tem a ferramenta. Skip não é passe: leia o motivo e registre "não verificado".
- Não rode a suíte inteira por reflexo durante outra execução; as portas e a fixture são compartilhadas.

### Projeto de referência para validar no app real (repo privado)

`/Users/helder/Projetos/gabagool-dotnet` (`Gabagool.sln`, 5 projetos `net8.0`: Api, Application, Domain, Infrastructure, Tests com xUnit; `global.json` no SDK `8.0.408`, `rollForward: latestPatch`). Além das fixtures, é o alvo para validar no app real: abrir a solução, Problems, Build, Run da Api, Tests, Debug e Git.

- É repositório **privado** do Helder: nunca edite a pasta original — copie para `/tmp` antes de abrir, compilar ou mexer.
- Nunca copie código, nomes de domínio, capturas ou trechos dele para este repo público: a evidência descreve só o resultado, e print só se não mostrar código dele.
- Nunca faça commit nem push nele.

## Verificação antes de entregar

O passo `/code-review` é, por padrão:

1. **Checagem objetiva e curta** do diff contra a spec/ticket e os padrões deste arquivo (`## Regras`, `data-testid`, dependências). Lista só desvios concretos com `arquivo:linha`.
2. **Rodar o skill `verify-openridr`** (`.agents/skills/verify-openridr/SKILL.md`) nas features tocadas: UI via Playwright/host isolado e, quando a tela do Mac estiver desbloqueada e com permissões, o app Tauri real via agent-desktop. O que não rodou entra como "não verificado".
3. **Code review em subagente único** (decisão de 2026-11-06) depois da implementação, antes do retro/PR: `delegate_task` do T3 com target Pi, model `openai-codex/gpt-6.1-terra` e option `thinking=medium`; fixed point = `git merge-base origin/main HEAD`; spec = a issue (e as filhas). Corrija achado alto/médio na hora; baixo, só se for simples. Sem `delegate_task` disponível: avise o orquestrador com o SHA do commit e siga. Revisão adversarial aberta/multi-agente continua só se o usuário pedir.

Resto do fluxo (grill → spec → tickets → implement → retro) segue igual. Propostas de verificação formal: [docs/VERIFICACAO-FORMAL.md](docs/VERIFICACAO-FORMAL.md); a PoC de trust roda com `cargo test -p openridr-host --test trust_model`.

## Regras

- **`data-testid` são contrato**: não renomeie nem remova os existentes; cada fatia adiciona os seus e mantém `role="tab"` no cabeçalho inferior.
- **Zero dependência nova** de UI/runtime sem decisão registrada e aprovação explícita.
- Um componente por arquivo; comentários em inglês; uma função de erro por fronteira (use `messageOf`/`requireAction` em vez de repetir mapeamento).
- **Prova real, não inventada**: o que o ambiente não permitiu executar vai como "não verificado" no `implementation-notes.md` e no ticket.
- **Ticket**: trabalho novo nasce como GitHub issue ligada à issue-mãe da frente (sub-issue, ou `Part of #N` na primeira linha do corpo), com bloqueio em `Blocked by: #M`. `docs/SPEC-<tema>.md`, `docs/RETRO-<tema>.md` e `docs/evidence/<tema>.md` continuam em `docs/`. Tickets antigos em `docs/tickets-*/` são histórico: não migre, não crie pasta nova no molde deles.
- **Commit**: um por ticket/fatia, mensagem convencional (`feat(git): ...`, `fix(search): ...`, `docs: ...`, `chore: ...`), com código, testes e checklist atualizado.
- **Stage explícito**: use `scripts/commit-only.sh "msg" <paths>` (faz o `git add -N` dos novos e `git commit --only -m "msg" -- <paths>`); à mão, é `git commit --only -m "msg" -- <paths>`; nunca use `git add -A` nem altere o index do usuário. Duas armadilhas que falharam 38 vezes em 10 sessões: (1) `-m` vai **antes** do `--` (depois dele vira pathspec); (2) arquivo/pasta **novo** precisa de `git add -N <paths>` antes (só marca intenção), senão dá `pathspec ... did not match`. Deixe `dist/`, `target/`, `test-results/` e afins fora (ver `.gitignore`).
- **Sem push, remote ou PR** a partir de sessão de agente. O hook de pre-commit roda `bun run check`; não use `--no-verify`.
