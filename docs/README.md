# docs/ — índice

Estado do produto: **OpenRidr 0.3.0-alpha.4**, macOS Apple Silicon. Onde fica cada coisa no código: [MAPA.md](MAPA.md). Termos: [../GLOSSARY.md](../GLOSSARY.md).

## Leia primeiro (vivo)

| Doc | Para quê | Como ler |
|---|---|---|
| [SPEC.md](SPEC.md) | contrato do produto (P0) | normativo |
| [SPEC-MVP-RIDER.md](SPEC-MVP-RIDER.md) + [tickets-mvp/](tickets-mvp/README.md) | fatias F1–F9 e critérios de aceite | normativo |
| [agents/](agents/issue-tracker.md) | onde o trabalho vira ticket: GitHub issues (`gh`), labels de triagem, docs de domínio | leia antes de criar/ler/fechar ticket |
| [decisions.md](decisions.md) | por quê de cada decisão, em ordem cronológica (append) | `grep -n '^## ' docs/decisions.md \| grep -i <tema>`; entradas "Substituída…" já estão marcadas |
| [implementation-notes.md](implementation-notes.md) | como cada ticket foi feito, contratos, **testids estáveis**, o que não foi verificado | busque por heading `## `; ~760 linhas, não leia inteiro |
| [RELEASE-ALPHA.md](RELEASE-ALPHA.md) | notas da alpha atual (novidades, instalar, limitações, sha256) | reescrito a cada release |
| [install-app.md](install-app.md) | `bun run app:install` e "abrir com" (Finder/Dock/terminal) | |
| [VERIFICACAO-FORMAL.md](VERIFICACAO-FORMAL.md) | 3 regras críticas candidatas a modelo formal + PoC de trust | proposta; evidência em [evidence/verify-openridr.md](evidence/verify-openridr.md) |
| [RETRO-*.md](.) | lições por onda de trabalho | só se o tema for o mesmo |

## Feature → docs

Convenção para trabalho novo: `SPEC-<tema>.md` em `docs/`, **tickets como GitHub issues** ligadas à issue-mãe da frente (sub-issue, ou `Part of #N` na primeira linha do corpo; bloqueio em `Blocked by: #M`), `RETRO-<tema>.md`, evidência em `evidence/<tema>.md`, decisão nova em `decisions.md`. O tracker e as operações de `gh` estão em [agents/issue-tracker.md](agents/issue-tracker.md); as pastas `tickets-*/` da tabela abaixo são histórico e não recebem tickets novos.

| Feature / onda | Spec | Tickets | Retro / review | Evidência |
|---|---|---|---|---|
| P0 original (Trust, SDK, MSBuild, LSP, build, run, DAP, PTY, VSTest) | [SPEC](SPEC.md) | [tickets/](tickets/README.md) (01–18) | [release-decision](release-decision.md) (histórico) | `evidence/NN-*.md`, [acceptance-matrix](acceptance-matrix.md) |
| MVP Rider F1–F9, auditorias, Solution Explorer | [SPEC-MVP-RIDER](SPEC-MVP-RIDER.md), [SPEC-SOLUTION-EXPLORER](SPEC-SOLUTION-EXPLORER.md) | [tickets-mvp/](tickets-mvp/README.md) (01–41) | [RETRO-MVP-RIDER](RETRO-MVP-RIDER.md), [REVIEW-MVP-RIDER](REVIEW-MVP-RIDER.md), [RETRO-SOLUTION-EXPLORER](RETRO-SOLUTION-EXPLORER.md) | `evidence/`, [UI-AUDIT](UI-AUDIT.md) |
| Welcome + Trust uma vez | [SPEC-WELCOME-TRUST](SPEC-WELCOME-TRUST.md) | [tickets-welcome-trust/](tickets-welcome-trust/) | [RETRO-WELCOME-TRUST](RETRO-WELCOME-TRUST.md), [REVIEW-WELCOME-TRUST](REVIEW-WELCOME-TRUST.md) | `evidence/welcome-trust/` |
| Run/Debug com build automático | `decisions.md` (2026-11-01) | — | [RETRO-AUTO-BUILD](RETRO-AUTO-BUILD.md) | `evidence/auto-build-run-debug.md` |
| Alfa 3: debug sem diálogo + confiar | [SPEC-ALPHA3-DEBUG-TRUST](SPEC-ALPHA3-DEBUG-TRUST.md) | `tickets-mvp/35–37` | [RETRO-ALPHA3-DEBUG-TRUST](RETRO-ALPHA3-DEBUG-TRUST.md) | `evidence/alpha3-debug-trust.md` |
| QA bug hunt → 4 altos (QA) | [QA-BUGHUNT-2026-11-02](QA-BUGHUNT-2026-11-02.md), [SPEC-QA-TOP4](SPEC-QA-TOP4.md) | — | [RETRO-QA-TOP4](RETRO-QA-TOP4.md) | — |
| QA devflow → 4 altos | [QA-DEVFLOW-2026-11-02](QA-DEVFLOW-2026-11-02.md), [SPEC-DEVFLOW-TOP4](SPEC-DEVFLOW-TOP4.md) | — | [RETRO-DEVFLOW-2026-11-02](RETRO-DEVFLOW-2026-11-02.md) | — |
| Médios: navegação/atalhos/busca | [SPEC-MEDIUM-NAV-SEARCH](SPEC-MEDIUM-NAV-SEARCH.md) | [tickets-medium-nav/](tickets-medium-nav/README.md) | [RETRO-MEDIUM-NAV-SEARCH](RETRO-MEDIUM-NAV-SEARCH.md) | — |
| Médios: debug/run/build | [SPEC-MEDIUM-DEBUG](SPEC-MEDIUM-DEBUG.md) | [tickets-medium-debug/](tickets-medium-debug/README.md) | [RETRO-MEDIUM-DEBUG](RETRO-MEDIUM-DEBUG.md) | — |
| Médios: editor/estado | [SPEC-MEDIUM-EDITOR-STATE](SPEC-MEDIUM-EDITOR-STATE.md) | [tickets-medium-editor/](tickets-medium-editor/README.md) | [RETRO-MEDIUM-EDITOR-STATE](RETRO-MEDIUM-EDITOR-STATE.md) | — |
| Abrir do Finder/Dock/terminal | [SPEC-OPEN-WITH](SPEC-OPEN-WITH.md) | [tickets-open-with/](tickets-open-with/README.md) | [RETRO-OPEN-WITH](RETRO-OPEN-WITH.md) | [evidence/03-open-with](evidence/03-open-with.md) |
| Release alfa (DMG) | [RELEASE-ALPHA](RELEASE-ALPHA.md) | — | [RETRO-ALPHA-RELEASE](RETRO-ALPHA-RELEASE.md) | [evidence/release-alpha10.md](evidence/release-alpha10.md) |
| Correções do E2E alpha.5 + camada SDK | [SPEC-FIX-ALPHA5](SPEC-FIX-ALPHA5.md) | [tickets-fix-alpha5/](tickets-fix-alpha5/README.md) | [RETRO-FIX-ALPHA5](RETRO-FIX-ALPHA5.md) | [evidence/fix-alpha5/](evidence/fix-alpha5/VERIFICATION.md) |
| netcoredbg embutido | [SPEC-EMBED-NETCOREDBG](SPEC-EMBED-NETCOREDBG.md) | [tickets-embed-netcoredbg/](tickets-embed-netcoredbg/) | [RETRO-EMBED-NETCOREDBG](RETRO-EMBED-NETCOREDBG.md) | [evidence/embed-netcoredbg.md](evidence/embed-netcoredbg.md) |
| Análise embutida (csharp-ls 0.28.0 + runtime .NET 10 privado) | [SPEC-BUNDLED-ANALYSIS](SPEC-BUNDLED-ANALYSIS.md) | [tickets-bundled-analysis/](tickets-bundled-analysis/README.md) (01–03) | [RETRO-BUNDLED-ANALYSIS](RETRO-BUNDLED-ANALYSIS.md) | [evidence/bundled-analysis.md](evidence/bundled-analysis.md) |
| Polimento visual (ícones Lucide, fontes embutidas, escala) | [SPEC-UI-POLISH](SPEC-UI-POLISH.md) | [tickets-ui-polish/](tickets-ui-polish/README.md) (01–06) | [RETRO-UI-POLISH](RETRO-UI-POLISH.md) | [evidence/ui-polish.md](evidence/ui-polish.md), `evidence/ui-polish/depois-*.png` |
| Paridade com o Rider (gaps G01–G23) | [SPEC-RIDER-PARITY](SPEC-RIDER-PARITY.md) | [tickets-rider-parity/](tickets-rider-parity/README.md) (01–16) | `RETRO-RIDER-PARITY` (a fazer) | `evidence/rider-parity.md` (a fazer) |
| Confinamento de arquivos (as duas falhas "conhecidas" do `cargo test --workspace`) | — (`decisions.md`, 2026-11-04) | — | — | [evidence/file-access-confinement.md](evidence/file-access-confinement.md) |
| Menu de contexto em toda linha (sem o menu do WebKit) | [SPEC-NATIVE-CONTEXT-MENU](SPEC-NATIVE-CONTEXT-MENU.md) | [tickets-native-context-menu/](tickets-native-context-menu/README.md) (01) | [RETRO-NATIVE-CONTEXT-MENU](RETRO-NATIVE-CONTEXT-MENU.md) | [evidence/native-context-menu.md](evidence/native-context-menu.md) |
| Solution Explorer visual (ícones/cores, sem chrome de avaliação) | [SPEC-SOLUTION-EXPLORER-VISUAL](SPEC-SOLUTION-EXPLORER-VISUAL.md) | [tickets-solution-explorer-visual/](tickets-solution-explorer-visual/README.md) (01–04) | [RETRO-SOLUTION-EXPLORER-VISUAL](RETRO-SOLUTION-EXPLORER-VISUAL.md) | [evidence/solution-explorer-visual](evidence/solution-explorer-visual.md) |
| Ícones da Solution Explorer (glifos T3 Code/@pierre/trees, pasta só com chevron, guia e seleção) | [SPEC-SOLUTION-FILE-ICONS](SPEC-SOLUTION-FILE-ICONS.md) | [tickets-solution-file-icons/](tickets-solution-file-icons/README.md) (01–03) | [RETRO-SOLUTION-FILE-ICONS](RETRO-SOLUTION-FILE-ICONS.md) | [evidence/solution-file-icons.md](evidence/solution-file-icons.md), `evidence/solution-file-icons/depois-*.png` |
| Correções da aceitação do `main` (export de tests, progresso do setup, TFM, menu de folha, NuGet, AX) | — (`decisions.md`, 2026-11-05) | — | [RETRO-ACCEPTANCE-FIXES](RETRO-ACCEPTANCE-FIXES.md) | [evidence/acceptance-fixes.md](evidence/acceptance-fixes.md), `evidence/acceptance-fixes/*.png` |
| Release assinado e notarizado pelo GitHub Actions (issue #44) | [SPEC-release-ci](SPEC-release-ci.md) | issues [#58](https://github.com/Heldinhow/openridr/issues/58), [#61](https://github.com/Heldinhow/openridr/issues/61), [#63](https://github.com/Heldinhow/openridr/issues/63), [#65](https://github.com/Heldinhow/openridr/issues/65), [#66](https://github.com/Heldinhow/openridr/issues/66), [#68](https://github.com/Heldinhow/openridr/issues/68) | [RETRO-release-ci](RETRO-release-ci.md) | [evidence/release-ci.md](evidence/release-ci.md) |
| Primeira abertura com quarentena (Gatekeeper) | [SPEC-FIRST-OPEN](SPEC-FIRST-OPEN.md) | #71, #72 | [RETRO-FIRST-OPEN](RETRO-FIRST-OPEN.md) | [evidence/first-open.md](evidence/first-open.md) |
| CI em PR (TS, Rust, governança, segredos) + sessão abre o próprio PR | [SPEC-CI-PR](SPEC-CI-PR.md) | issues [#80](https://github.com/Heldinhow/openridr/issues/80) a [#88](https://github.com/Heldinhow/openridr/issues/88) | [RETRO-CI-PR](RETRO-CI-PR.md) | [evidence/ci-pr.md](evidence/ci-pr.md) |
| Debug avançado: hit count, watches persistentes (issues #94–#96) | [SPEC-DEBUG-ADVANCED](SPEC-DEBUG-ADVANCED.md) | #94, #95, #96 | [RETRO-DEBUG-ADVANCED](RETRO-DEBUG-ADVANCED.md) | [evidence/debug-advanced.md](evidence/debug-advanced.md) |
| Setup de ferramentas + Tests/NuGet (paridade Rider) | [SPEC-TESTS-NUGET-TOOLS-SETUP](SPEC-TESTS-NUGET-TOOLS-SETUP.md) | [tickets-tests-nuget-tools-setup/](tickets-tests-nuget-tools-setup/README.md) (01–05) | [RETRO-TESTS-NUGET-TOOLS-SETUP](RETRO-TESTS-NUGET-TOOLS-SETUP.md) | [evidence/tests-nuget-tools-setup/desktop.md](evidence/tests-nuget-tools-setup/desktop.md) |
| Suite de testes confiável (issue #43) | [SPEC-TEST-STABILITY](SPEC-TEST-STABILITY.md) | issues GitHub #57, #59, #62, #64, #67 | [RETRO-TEST-STABILITY](RETRO-TEST-STABILITY.md) | [evidence/test-stability.md](evidence/test-stability.md) |
| Risco de PR por caminho: label, aprovação do mantenedor no alto, auto-merge no baixo (issue #74) | [SPEC-RISK-TIERS](SPEC-RISK-TIERS.md) | #123, #122, #124, #125, #126 | [RETRO-RISK-TIERS](RETRO-RISK-TIERS.md) | [evidence/risk-tiers.md](evidence/risk-tiers.md), [evidence/revert-rate.md](evidence/revert-rate.md) |
| E2E: smoke noturno + quarentena (issue #67) | [SPEC-TEST-STABILITY](SPEC-TEST-STABILITY.md) | issue #67 (#56 é o workflow) | — | [evidence/e2e-quarantine.md](evidence/e2e-quarantine.md) |
| Debug: sessão que termina sem parar (#46) | [SPEC-DEBUG-NO-STOP](SPEC-DEBUG-NO-STOP.md) | #76, #78 (GitHub) | [RETRO-DEBUG-NO-STOP](RETRO-DEBUG-NO-STOP.md) | [evidence/debug-no-stop.md](evidence/debug-no-stop.md) |
| Git diff: arquivo inteiro, modo unificado e hash do commit (#48) | [SPEC-GIT-DIFF](SPEC-GIT-DIFF.md) | #98, #99, #100 (GitHub); follow-up #101 | [RETRO-GIT-DIFF](RETRO-GIT-DIFF.md) | [evidence/git-diff.md](evidence/git-diff.md) |
| Diff de um arquivo de um commit pelo Git log (#101) | [SPEC-GIT-COMMIT-DIFF](SPEC-GIT-COMMIT-DIFF.md) | #169, #170, #171 (GitHub) | [RETRO-GIT-COMMIT-DIFF](RETRO-GIT-COMMIT-DIFF.md) | [evidence/git-commit-diff.md](evidence/git-commit-diff.md) |
| Solution Explorer: status Git e marcador de symlink (#49) | [SPEC-SOLUTION-GIT-STATUS](SPEC-SOLUTION-GIT-STATUS.md) | #107, #108, #109, #110 (GitHub) | [RETRO-SOLUTION-GIT-STATUS](RETRO-SOLUTION-GIT-STATUS.md) | [evidence/solution-git-status.md](evidence/solution-git-status.md) |
| Aceitação Git com remoto (#50): push, pull, branches e histórico | [SPEC-GIT-REMOTE-ACCEPTANCE](SPEC-GIT-REMOTE-ACCEPTANCE.md) | #104, #105, #106 (GitHub); bugs #111–#114 | [RETRO-GIT-REMOTE-ACCEPTANCE](RETRO-GIT-REMOTE-ACCEPTANCE.md) | [evidence/git-remote-acceptance.md](evidence/git-remote-acceptance.md) |
| Launch profiles: navegador, `%VAR%`/`$(VAR)` e argv do .NET (#53) | [SPEC-LAUNCH-PROFILES-EXTRAS](SPEC-LAUNCH-PROFILES-EXTRAS.md) | #157, #158, #159 (GitHub) | [RETRO-LAUNCH-PROFILES-EXTRAS](RETRO-LAUNCH-PROFILES-EXTRAS.md) | [evidence/launch-profiles-extras.md](evidence/launch-profiles-extras.md) |
| Atualização dentro do app pelo updater do Tauri (#55) | [SPEC-IN-APP-UPDATER](SPEC-IN-APP-UPDATER.md) | #195, #196, #197, #198 (GitHub) | [RETRO-IN-APP-UPDATER](RETRO-IN-APP-UPDATER.md) | [evidence/in-app-updater.md](evidence/in-app-updater.md) |
| Proxy corporativo HTTP(S) sem enfraquecer a guarda SSRF (#51) | [SPEC-CORPORATE-PROXY](SPEC-CORPORATE-PROXY.md) | #133, #134 (GitHub); follow-up #132 | [RETRO-CORPORATE-PROXY](RETRO-CORPORATE-PROXY.md) | [evidence/corporate-proxy.md](evidence/corporate-proxy.md) |
| Menu nativo × keymap (issue #111): ⌘W/⌥⌘W voltam a chegar ao app | [SPEC-NATIVE-MENU-KEYMAP](SPEC-NATIVE-MENU-KEYMAP.md) | #120, #129, #121 (GitHub) | [RETRO-NATIVE-MENU-KEYMAP](RETRO-NATIVE-MENU-KEYMAP.md) | [evidence/native-menu-keymap.md](evidence/native-menu-keymap.md) |
| Update reconcilia branch divergente (#112): merge declarado, conflito aborta | [SPEC-GIT-UPDATE-RECONCILE](SPEC-GIT-UPDATE-RECONCILE.md) | #127, #128 (GitHub) | [RETRO-GIT-UPDATE-RECONCILE](RETRO-GIT-UPDATE-RECONCILE.md) | [evidence/git-update-reconcile.md](evidence/git-update-reconcile.md) |
| Fetch explícito no painel Git (#113): contador e branches remotas em dia | [SPEC-GIT-FETCH](SPEC-GIT-FETCH.md) | #137, #138 (GitHub); relacionada #136 | [RETRO-GIT-FETCH](RETRO-GIT-FETCH.md) | [evidence/git-fetch.md](evidence/git-fetch.md) |
| Prazo em push e pull (#136): rede não congela o host | [SPEC-GIT-NETWORK-DEADLINE](SPEC-GIT-NETWORK-DEADLINE.md) | #147, #148 (GitHub); relacionada #113 | [RETRO-GIT-NETWORK-DEADLINE](RETRO-GIT-NETWORK-DEADLINE.md) | [evidence/git-network-deadline.md](evidence/git-network-deadline.md) |
| Projetos fora do diretório da solution (#52): escopo do workspace, confinamento, análise e execução | [SPEC-PROJECTS-OUTSIDE-SOLUTION](SPEC-PROJECTS-OUTSIDE-SOLUTION.md) | #143, #144, #145, #146 (GitHub) | [RETRO-PROJECTS-OUTSIDE-SOLUTION](RETRO-PROJECTS-OUTSIDE-SOLUTION.md) | [evidence/projects-outside-solution.md](evidence/projects-outside-solution.md), `evidence/projects-outside-solution/*.png` |
| Download de ferramenta limitado por inatividade (#161): link lento não falha por prazo | [SPEC-TOOLS-DOWNLOAD-STALL](SPEC-TOOLS-DOWNLOAD-STALL.md) | #163, #164, #165 (GitHub); filha #162 | [RETRO-TOOLS-DOWNLOAD-STALL](RETRO-TOOLS-DOWNLOAD-STALL.md) | [evidence/tools-download-stall.md](evidence/tools-download-stall.md) |
| Diretório neutro da ferramenta em um lugar só (#93): verificação não falha em máquina fria | [SPEC-STAGED-TOOLS-NEUTRAL-DIR](SPEC-STAGED-TOOLS-NEUTRAL-DIR.md) | #153 (GitHub) | [RETRO-STAGED-TOOLS-NEUTRAL-DIR](RETRO-STAGED-TOOLS-NEUTRAL-DIR.md) | [evidence/staged-tools-neutral-dir.md](evidence/staged-tools-neutral-dir.md) |
| Merge em andamento: concluir ou abortar pela UI (#114) | [SPEC-GIT-MERGE-IN-PROGRESS](SPEC-GIT-MERGE-IN-PROGRESS.md) | #154, #155, #156 (GitHub) | [RETRO-GIT-MERGE-IN-PROGRESS](RETRO-GIT-MERGE-IN-PROGRESS.md) | [evidence/git-merge-in-progress.md](evidence/git-merge-in-progress.md), `evidence/git-merge-in-progress/*.png` |
| Gatilho do menu de branches como botão com nome acessível (#168) | [SPEC-BRANCH-MENU-A11Y](SPEC-BRANCH-MENU-A11Y.md) | #176 (GitHub); follow-up #177 | [RETRO-BRANCH-MENU-A11Y](RETRO-BRANCH-MENU-A11Y.md) | [evidence/branch-menu-a11y.md](evidence/branch-menu-a11y.md) |
| App de dev em um comando + abrir workspace por argv (#91) | [SPEC-DEV-APP](SPEC-DEV-APP.md) | #172, #173 (GitHub) | [RETRO-DEV-APP](RETRO-DEV-APP.md) | [evidence/dev-app.md](evidence/dev-app.md) |
| Os outros três menus da toolbar como botões acessíveis (#177) | [SPEC-TOOLBAR-MENUS-A11Y](SPEC-TOOLBAR-MENUS-A11Y.md) | tickets de #177 (GitHub); follow-up de #168 | [RETRO-TOOLBAR-MENUS-A11Y](RETRO-TOOLBAR-MENUS-A11Y.md) | [evidence/toolbar-menus-a11y.md](evidence/toolbar-menus-a11y.md) |
| Proxy do macOS System Settings para o cliente HTTP do host (#132) | [SPEC-SYSTEM-PROXY](SPEC-SYSTEM-PROXY.md) | #181, #182 (GitHub) | [RETRO-SYSTEM-PROXY](RETRO-SYSTEM-PROXY.md) | [evidence/system-proxy.md](evidence/system-proxy.md) |
Decisão: netcoredbg 3.2.0-1092, csharp-ls 0.28.0 e o runtime .NET 10.0.14 privado são embutidos no bundle; o SDK do projeto continua externo (resolvido por `global.json`). Download de ferramentas no primeiro uso deixou de existir no app/DMG; o fluxo de download continua só como fallback de `tauri dev`.

`decisions.md`, `GLOSSARY.md` e a tabela acima são **append-only**: cada frente acrescenta a sua linha e
um merge de `main` conflita por construção. Resolução: ficam **as duas entradas**, a desta branch primeiro
e as antigas na ordem em que entraram — nunca reordenar nem reescrever entrada velha.

Numeração de tickets nas pastas históricas **reinicia por pasta** (`tickets/01` ≠ `tickets-mvp/01`): cite sempre a pasta. Ticket novo é GitHub issue: cite pelo número (`#58`).

## Histórico (não use como estado atual)

| Doc | Por que é histórico |
|---|---|
| [PROPOSTA-UI-2026-10-28.md](PROPOSTA-UI-2026-10-28.md), [index.html](index.html), [wireframes.md](wireframes.md) | proposta de UI anterior a qualquer código ("aguardando aprovar as telas") |
| [release-decision.md](release-decision.md) | decisão de 28/10: "release bloqueado" — superada pelas alphas 1–4 |
| [nav-retest.md](nav-retest.md), [RIDR-VS-RIDER.md](RIDR-VS-RIDER.md), [UI-AUDIT.md](UI-AUDIT.md) | retestes e auditorias datados de 28–29/10 |
| `superpowers/plans/` | plano de aproximação visual (29/10) |
| `evidence/` (screenshots, logs) | parte da evidência bruta foi removida do repositório público (capturas com caminhos pessoais, referências de software proprietário); os `evidence/*.md` marcam o que não está mais aqui |

Specs/tickets antigos mantêm `Status: ready-for-agent` no cabeçalho por serem anteriores à implementação: o estado real está nas retros e no `git log`.
