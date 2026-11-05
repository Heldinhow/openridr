# docs/ — índice

Estado do produto: **OpenRidr 0.3.0-alpha.4**, macOS Apple Silicon. Onde fica cada coisa no código: [MAPA.md](MAPA.md). Termos: [../GLOSSARY.md](../GLOSSARY.md).

## Leia primeiro (vivo)

| Doc | Para quê | Como ler |
|---|---|---|
| [SPEC.md](SPEC.md) | contrato do produto (P0) | normativo |
| [SPEC-MVP-RIDER.md](SPEC-MVP-RIDER.md) + [tickets-mvp/](tickets-mvp/README.md) | fatias F1–F9 e critérios de aceite | normativo |
| [decisions.md](decisions.md) | por quê de cada decisão, em ordem cronológica (append) | `grep -n '^## ' docs/decisions.md \| grep -i <tema>`; entradas "Substituída…" já estão marcadas |
| [implementation-notes.md](implementation-notes.md) | como cada ticket foi feito, contratos, **testids estáveis**, o que não foi verificado | busque por heading `## `; ~760 linhas, não leia inteiro |
| [RELEASE-ALPHA.md](RELEASE-ALPHA.md) | notas da alpha atual (novidades, instalar, limitações, sha256) | reescrito a cada release |
| [install-app.md](install-app.md) | `bun run app:install` e "abrir com" (Finder/Dock/terminal) | |
| [VERIFICACAO-FORMAL.md](VERIFICACAO-FORMAL.md) | 3 regras críticas candidatas a modelo formal + PoC de trust | proposta; evidência em [evidence/verify-openridr.md](evidence/verify-openridr.md) |
| [RETRO-*.md](.) | lições por onda de trabalho | só se o tema for o mesmo |

## Feature → docs

Convenção para trabalho novo: `SPEC-<tema>.md`, tickets em `tickets-<tema>/NN-slug.md` (+ `README.md` com o índice), `RETRO-<tema>.md`, evidência em `evidence/<tema>.md`, decisão nova em `decisions.md`.

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
| Release alfa (DMG) | [RELEASE-ALPHA](RELEASE-ALPHA.md) | — | [RETRO-ALPHA-RELEASE](RETRO-ALPHA-RELEASE.md) | [evidence/release-alpha7.md](evidence/release-alpha7.md) |
| Correções do E2E alpha.5 + camada SDK | [SPEC-FIX-ALPHA5](SPEC-FIX-ALPHA5.md) | [tickets-fix-alpha5/](tickets-fix-alpha5/README.md) | [RETRO-FIX-ALPHA5](RETRO-FIX-ALPHA5.md) | [evidence/fix-alpha5/](evidence/fix-alpha5/VERIFICATION.md) |
| netcoredbg embutido | [SPEC-EMBED-NETCOREDBG](SPEC-EMBED-NETCOREDBG.md) | [tickets-embed-netcoredbg/](tickets-embed-netcoredbg/) | [RETRO-EMBED-NETCOREDBG](RETRO-EMBED-NETCOREDBG.md) | [evidence/embed-netcoredbg.md](evidence/embed-netcoredbg.md) |
| Polimento visual (ícones Lucide, fontes embutidas, escala) | [SPEC-UI-POLISH](SPEC-UI-POLISH.md) | [tickets-ui-polish/](tickets-ui-polish/README.md) (01–06) | [RETRO-UI-POLISH](RETRO-UI-POLISH.md) | [evidence/ui-polish.md](evidence/ui-polish.md), `evidence/ui-polish/depois-*.png` |
| Paridade com o Rider (gaps G01–G23) | [SPEC-RIDER-PARITY](SPEC-RIDER-PARITY.md) | [tickets-rider-parity/](tickets-rider-parity/README.md) (01–16) | `RETRO-RIDER-PARITY` (a fazer) | `evidence/rider-parity.md` (a fazer) |
| Confinamento de arquivos (as duas falhas "conhecidas" do `cargo test --workspace`) | — (`decisions.md`, 2026-11-04) | — | — | [evidence/file-access-confinement.md](evidence/file-access-confinement.md) |
| Menu de contexto em toda linha (sem o menu do WebKit) | [SPEC-NATIVE-CONTEXT-MENU](SPEC-NATIVE-CONTEXT-MENU.md) | [tickets-native-context-menu/](tickets-native-context-menu/README.md) (01) | [RETRO-NATIVE-CONTEXT-MENU](RETRO-NATIVE-CONTEXT-MENU.md) | [evidence/native-context-menu.md](evidence/native-context-menu.md) |
| Solution Explorer visual (ícones/cores, sem chrome de avaliação) | [SPEC-SOLUTION-EXPLORER-VISUAL](SPEC-SOLUTION-EXPLORER-VISUAL.md) | [tickets-solution-explorer-visual/](tickets-solution-explorer-visual/README.md) (01–04) | [RETRO-SOLUTION-EXPLORER-VISUAL](RETRO-SOLUTION-EXPLORER-VISUAL.md) | [evidence/solution-explorer-visual](evidence/solution-explorer-visual.md) |
| Ícones da Solution Explorer (glifos T3 Code/@pierre/trees, pasta só com chevron, guia e seleção) | [SPEC-SOLUTION-FILE-ICONS](SPEC-SOLUTION-FILE-ICONS.md) | [tickets-solution-file-icons/](tickets-solution-file-icons/README.md) (01–03) | [RETRO-SOLUTION-FILE-ICONS](RETRO-SOLUTION-FILE-ICONS.md) | [evidence/solution-file-icons.md](evidence/solution-file-icons.md), `evidence/solution-file-icons/depois-*.png` |
| Correções da aceitação do `main` (export de tests, progresso do setup, TFM, menu de folha, NuGet, AX) | — (`decisions.md`, 2026-11-05) | — | [RETRO-ACCEPTANCE-FIXES](RETRO-ACCEPTANCE-FIXES.md) | [evidence/acceptance-fixes.md](evidence/acceptance-fixes.md), `evidence/acceptance-fixes/*.png` |
| Setup de ferramentas + Tests/NuGet (paridade Rider) | [SPEC-TESTS-NUGET-TOOLS-SETUP](SPEC-TESTS-NUGET-TOOLS-SETUP.md) | [tickets-tests-nuget-tools-setup/](tickets-tests-nuget-tools-setup/README.md) (01–05) | [RETRO-TESTS-NUGET-TOOLS-SETUP](RETRO-TESTS-NUGET-TOOLS-SETUP.md) | [evidence/tests-nuget-tools-setup/desktop.md](evidence/tests-nuget-tools-setup/desktop.md) |
Decisão: netcoredbg 3.2.0-1092 é embutido no bundle (ticket 01); SDK .NET e csharp-ls ficam fora. A prova no app/DMG real continua pendente.

Numeração de tickets **reinicia por pasta** (`tickets/01` ≠ `tickets-mvp/01`): cite sempre a pasta.

## Histórico (não use como estado atual)

| Doc | Por que é histórico |
|---|---|
| [PROPOSTA-UI-2026-10-28.md](PROPOSTA-UI-2026-10-28.md), [index.html](index.html), [wireframes.md](wireframes.md) | proposta de UI anterior a qualquer código ("aguardando aprovar as telas") |
| [release-decision.md](release-decision.md) | decisão de 28/10: "release bloqueado" — superada pelas alphas 1–4 |
| [nav-retest.md](nav-retest.md), [RIDR-VS-RIDER.md](RIDR-VS-RIDER.md), [UI-AUDIT.md](UI-AUDIT.md) | retestes e auditorias datados de 28–29/10 |
| `superpowers/plans/` | plano de aproximação visual (29/10) |
| `evidence/` (screenshots, logs) | parte da evidência bruta foi removida do repositório público (capturas com caminhos pessoais, referências de software proprietário); os `evidence/*.md` marcam o que não está mais aqui |

Specs/tickets antigos mantêm `Status: ready-for-agent` no cabeçalho por serem anteriores à implementação: o estado real está nas retros e no `git log`.
