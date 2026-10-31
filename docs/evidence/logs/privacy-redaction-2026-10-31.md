# Privacy redaction — native-*.png e achados em docs/evidence (2026-10-31)

Este arquivo registra achados de privacidade. **As imagens pré-existentes NÃO foram alteradas nem redigidas neste passo.** Os PNGs listados abaixo continuam como estavam nos commits de origem.

## 1. Redações feitas nesta sessão (commit f78e6af)

Redigidos por caixas pretas sobre palavras detectadas por OCR (helder, MacBook, /Users, /private/var, Air-de): native-00, 07, 08, 09, 17, 18, 19, 20, 47. Verificação por OCR após redação: 0 ocorrências nesses nove arquivos.

## 2. Commits introdutores dos blobs antigos com host/usuário

Os blobs originais (sem redação) estão no histórico local nos commits abaixo. Os commits seguintes só os carregam por herança.

| Arquivo | Commit introdutor do blob original | Achado |
|---|---|---|
| native-19-d13-trusted-after-grant.png | b2f7d78 | zsh prompt com host (`helder@…`) |
| native-20-revoke-restricted.png | 8b223c6 | zsh prompt com host |
| native-00-welcome.png | 6a862b0 | `/Users/helder/Projetos/openridr-…` |
| native-07-trusted.png | 6a862b0 | `/Users/helder/.dotnet` |
| native-08-workspace-tools.png | 6a862b0 | `/Users/helder/.dotnet` |
| native-09-before-run.png | 6a862b0 | `/Users/helder/.dotnet` |
| native-17-terminal-prompt.png | 6a862b0 (alterado em b8bd154) | zsh prompt com host |
| native-18-terminal-echo.png | 6a862b0 (alterado em b8bd154) | zsh prompt com host |
| native-47-run-config-copy-arguments-1440.png | c37c701 | OCR atual sem achado; incluído na redação de f78e6af por precaução (motivo não confirmado por OCR do blob original) |
| 09-rename-*.png e 10-code-actions-*.png (originais) | 5a75082 / 54ceddc (e anteriores) | nenhum achado no OCR atual |
| 10-install-*.png | 6c323ed / 9bf0bec | caminho `/Users/helder/.dotnet` (ver lista 3) |

## 3. Lista completa de PNGs rastreados com achado de OCR (não redigidos)

Categorias: `zsh-prompt-host` (prompt com usuário@host), `/Users/<name>` (caminho com nome de usuário), `/private/var/folders` (tmp por usuário), `author-name` (nome de autor git), `/tmp/ponto(local-copy)`. Gerado por tesseract em 309 PNGs rastreados em `docs/evidence`.

| Arquivo | Categoria | Commit introdutor |
|---|---|---|
| `docs/evidence/nav-retest/1100-01-welcome.png` | /Users/<name> | 0130761 |
| `docs/evidence/nav-retest/1100-03-trusted.png` | /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1100-09-restore.png` | /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1100-10-build.png` | /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1100-11-run.png` | /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1100-12-tests.png` | /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1100-13-debug.png` | /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1100-15-reopened.png` | /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1440-01-welcome.png` | /Users/<name> | 0130761 |
| `docs/evidence/nav-retest/1440-03-trusted.png` | /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1440-09-restore.png` | /Users/<name>; /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1440-10-build.png` | /Users/<name>; /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1440-11-run.png` | /Users/<name> | 0130761 |
| `docs/evidence/nav-retest/1440-12-tests.png` | /Users/<name>; /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1440-13-debug.png` | /Users/<name>; /private/var/folders | 0130761 |
| `docs/evidence/nav-retest/1440-15-reopened.png` | /private/var/folders | 0130761 |
| `docs/evidence/rider-compare/08-terminal.png` | zsh-prompt-host | 5b28931 |
| `docs/evidence/rider-compare/12-run-config-dropdown.png` | /Users/<name> | 5b28931 |
| `docs/evidence/rider-compare/15-editor-context-menu.png` | /Users/<name> | 4c93615 |
| `docs/evidence/rider-compare/18-build-tool-window.png` | /Users/<name> | 5b28931 |
| `docs/evidence/rider-reference/08-terminal.png` | zsh-prompt-host | 84ad485 |
| `docs/evidence/rider-reference/18-build-tool-window.png` | /Users/<name> | 84ad485 |
| `docs/evidence/screenshots/02-toolchain-missing-sdk.png` | /Users/<name> | 93c493a |
| `docs/evidence/screenshots/02-toolchain-restricted.png` | /Users/<name> | 93c493a |
| `docs/evidence/screenshots/02-toolchain-trusted.png` | /Users/<name> | 93c493a |
| `docs/evidence/screenshots/03-evaluated-tree.png` | /Users/<name> | f7d92a1 |
| `docs/evidence/screenshots/04-restore-failure.png` | /Users/<name>; /private/var/folders | 50389fc |
| `docs/evidence/screenshots/04-restore.png` | /Users/<name>; /private/var/folders | 50389fc |
| `docs/evidence/screenshots/05-csharp-ls-install.png` | /Users/<name> | 8929fa3 |
| `docs/evidence/screenshots/08-completion-unavailable.png` | /Users/<name>; /private/var/folders | d43f248 |
| `docs/evidence/screenshots/11-build-dirty.png` | /Users/<name>; /private/var/folders | 5e8e637 |
| `docs/evidence/screenshots/11-build-failed.png` | /Users/<name>; /private/var/folders | 5e8e637 |
| `docs/evidence/screenshots/11-build-fixed.png` | /Users/<name>; /private/var/folders | 5e8e637 |
| `docs/evidence/screenshots/11-build-ok.png` | /Users/<name>; /private/var/folders | 5e8e637 |
| `docs/evidence/screenshots/11-problems-build.png` | /Users/<name>; /private/var/folders | 5e8e637 |
| `docs/evidence/screenshots/12-run-cancelled.png` | /Users/<name>; /private/var/folders | fd4e78f |
| `docs/evidence/screenshots/12-run-dirty.png` | /Users/<name>; /private/var/folders | fd4e78f |
| `docs/evidence/screenshots/12-run-output.png` | /Users/<name>; /private/var/folders | fd4e78f |
| `docs/evidence/screenshots/12-run-rebuilt.png` | /Users/<name>; /private/var/folders | fd4e78f |
| `docs/evidence/screenshots/12-run-saved-stale.png` | /Users/<name>; /private/var/folders | fd4e78f |
| `docs/evidence/screenshots/13-debugger-plan.png` | /Users/<name> | a067bae |
| `docs/evidence/screenshots/13-debugger-ready.png` | /Users/<name> | a925693 |
| `docs/evidence/screenshots/14-bundle-welcome.png` | /Users/<name>; author-name (HelderSantos/hroger) | 96e1e41 |
| `docs/evidence/screenshots/14-dap-paused.png` | /Users/<name>; /private/var/folders | 7afc56e |
| `docs/evidence/screenshots/14-dap-stepped.png` | /Users/<name>; /private/var/folders | 7afc56e |
| `docs/evidence/screenshots/14-dap-stopped.png` | /Users/<name>; /private/var/folders | 7afc56e |
| `docs/evidence/screenshots/15-run-config-output.png` | /Users/<name>; /private/var/folders | 645ad45 |
| `docs/evidence/screenshots/15-terminal-close-dialog.png` | /private/var/folders | 5864bae |
| `docs/evidence/screenshots/15-terminal-closed.png` | zsh-prompt-host; /private/var/folders | 5864bae |
| `docs/evidence/screenshots/15-terminal-ctrl-c.png` | /private/var/folders | 5864bae |
| `docs/evidence/screenshots/15-terminal-running.png` | zsh-prompt-host; /private/var/folders | 5864bae |
| `docs/evidence/screenshots/16-tests-adapter-missing.png` | /Users/<name>; /private/var/folders | 0257d82 |
| `docs/evidence/screenshots/16-tests-discovery.png` | /Users/<name>; /private/var/folders | 0257d82 |
| `docs/evidence/screenshots/16-tests-mtp.png` | /Users/<name>; /private/var/folders | 0257d82 |
| `docs/evidence/screenshots/16-tests-stale.png` | /Users/<name>; /private/var/folders | 0257d82 |
| `docs/evidence/screenshots/17-tests-run-all.png` | /Users/<name>; /private/var/folders | 4b8885c |
| `docs/evidence/screenshots/17-tests-run-cancel.png` | /Users/<name>; /private/var/folders | 4b8885c |
| `docs/evidence/screenshots/17-tests-run-infrastructure.png` | /Users/<name>; /private/var/folders | 4b8885c |
| `docs/evidence/screenshots/17-tests-run-revoke.png` | /Users/<name>; /private/var/folders | 4b8885c |
| `docs/evidence/screenshots/18-screen-01-trust.png` | /Users/<name>; /private/var/folders | 4168a23 |
| `docs/evidence/screenshots/18-screen-03-solution-explorer.png` | /Users/<name>; /private/var/folders | 4168a23 |
| `docs/evidence/screenshots/18-screen-06-terminal.png` | zsh-prompt-host; /private/var/folders | 4168a23 |
| `docs/evidence/screenshots/18-terminal-started.png` | /private/var/folders | 4168a23 |
| `docs/evidence/screenshots/18-toolchain-trusted.png` | /Users/<name>; /private/var/folders | 4168a23 |
| `docs/evidence/screenshots/f5-tests-quick-list.png` | /Users/<name>; /private/var/folders | dec2743 |
| `docs/evidence/screenshots/native-49-revoke-live-shell-stopped.png` | /tmp/ponto(local-copy) | 3092d16 |
| `docs/evidence/screenshots/ui-trust-screen.png` | /Users/<name> | 89adba0 |
| `docs/evidence/ui-audit/01-setup-empty.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/02-setup-invalid-path.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/03-toolchain-restricted.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/03b-toolchain-restricted-full.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/04-toolchain-expanded.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/19-project-details.png` | /Users/<name> | de99fb1 |
| `docs/evidence/ui-audit/21-restore-success.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/22-tools-before-install.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/24-build-success.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/25-run-output.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/43-settings-build.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/53-git-log.png` | author-name (HelderSantos/hroger) | de99fb1 |
| `docs/evidence/ui-audit/54-git-log-files.png` | author-name (HelderSantos/hroger) | de99fb1 |
| `docs/evidence/ui-audit/55-branch-menu.png` | author-name (HelderSantos/hroger) | de99fb1 |
| `docs/evidence/ui-audit/56-branch-created.png` | author-name (HelderSantos/hroger) | de99fb1 |
| `docs/evidence/ui-audit/58-git-update.png` | author-name (HelderSantos/hroger) | de99fb1 |
| `docs/evidence/ui-audit/60-nuget-no-packages.png` | /Users/<name> | de99fb1 |
| `docs/evidence/ui-audit/61-tests-discovered.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/62-tests-results.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/64-tests-rerun.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/65-tests-quick-list.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/77-debug-adapter.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/80-build-for-debug.png` | /Users/<name>; /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/81-debug-paused.png` | /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/82-debug-stepped.png` | /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/84-terminal.png` | zsh-prompt-host | de99fb1 |
| `docs/evidence/ui-audit/85-terminal-close.png` | /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/86-close-dirty.png` | /private/var/folders | de99fb1 |
| `docs/evidence/ui-audit/95-build-failed.png` | /private/var/folders | de99fb1 |

## 4. Política recomendada antes de qualquer push (decisão do mantenedor)

O branch `fase-1-fluxo-sem-atrito` contém blobs com host/usuário e caminhos pessoais em commits anteriores. **NÃO publicar o branch nem o histórico sem tratar isso antes.** Duas opções, a escolher pelo Helder:

- **(a) Filtrar o histórico** (ex.: `git filter-repo` removendo ou substituindo os blobs listados nas seções 2 e 3), e só então publicar. Custa reescrever SHAs já citados nas evidências.
- **(b) Publicar uma cópia limpa** (branch novo a partir de árvore sem os PNGs com achado, ou com redação feita antes), mantendo o histórico local fora do remoto. Preserva os SHAs locais, mas o repositório publicado não terá o histórico de evidência.

Nenhuma das duas opções foi executada aqui. Não há push, reescrita nem remoção neste passo.

