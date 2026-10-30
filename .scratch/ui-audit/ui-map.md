# OpenRidr MVP visual audit map (F1–F9)
Paths relative to repo root. Generated for the headed-Chrome UI audit. Testids are the contract-stable ones used by specs.

## 0. Boot
- App switches SetupScreen → WorkspaceView on `stage`; reopening a workspace resets trust.
- Recent solutions are session-only, not persisted.

## 1. First-run / toolchain
- Header: brand "OpenRidr" + "No workspace open".
- `workspace-path-input` autofocus, placeholder `/Users/you/src/OrderDesk/OrderDesk.sln`; `workspace-open` ("Open workspace"/"Opening…"); helper `workspace-path-help`.
- `workspace-error` role=alert; ambiguous → `workspace-candidates` + `workspace-candidate-{index}`.
- Trust card after open: `workspace-path`, TrustControls, `open-text-only` ("Open text only"/"Open workspace").
- TrustControls: `trust-panel`, `mode-restricted`/`mode-trusted`, `trust-checkbox`, `trust-grant-reason`, `trust-grant`, `trust-revoke`, `trust-error`.
- `toolchain` section: `sdk-host`, `sdk-project`, `sdk-tfms`, `debug-adapter-card`, `sdk-redetect`, roots list, `sdk-error`, `sdk-project-message`, `sdk-open-global-json`, `sdk-host-instructions`.
- Debug adapter card = DebugAdapterPanel: `debug-state`, `debug-flag-checkbox`, `debug-install`, `debug-plan-open`, …
- `processes-panel` only when host tracks processes.
- Outline preview is text-only, "not evaluated".

## 2. Open fixtures/OrderDesk
- Path: `/Users/helder/Projetos/openridr/fixtures/OrderDesk/OrderDesk.sln`.
- fill `workspace-path-input` → `workspace-open` → restricted card → `open-text-only`, or `trust-checkbox` → `trust-grant` → `mode-trusted` → `open-text-only`.
- In workspace: toolbar + rail + `tree`; `mode-badge` = "Restricted mode".
- Notices: `workspace-tools-toggle` → `restore-panel`, `restore-state/-scope/-action/-cancel/-retry/-reload/-command/-log`, DebugAdapterPanel(compact), ToolInstaller, LspPanel.
- Tree: `tree-project-{name}`, `tree-file-{relPath}`; evaluated tags "MSBuild evaluated" vs "Text outline · not evaluated"; `eval-run`, `eval-state`, `eval-disabled-reason`, `eval-stale`, `eval-stale-run`; `eval-details`, `eval-target-path`, `eval-packages`, `eval-restore`, `eval-revision`, `eval-error`, `eval-open-project`, `eval-retry`, `eval-project-log`, `eval-log`.
- Tabs: `tab-{path}` + `tab-label`, `close-buffer`; `dirty-marker`; `breadcrumb`, `editor-revision`; `save-file`, `rename-open`, `code-actions-open`; empty state "Your code, in focus.".
- `mode-badge` → `trust-banner` → TrustControls; status `status-mode`.

## 3. Single 44px top bar
1. `toolbar-project` summary → `toolbar-project-menu`, "Recent solutions", `recent-solution-{index}`.
2. `mode-badge` summary → `trust-banner`.
3. `toolbar-branch`, `branch-current`, ↑/↓; popup `branch-menu`.
4. Startup project select, aria-label "Startup project".
5. `eval-configuration`, `eval-tfm`.
6. `toolbar-run-config`, `run-config-menu`, `run-config-active`, `run-config-option-default`, `run-config-option-{id}`, `run-config-edit`.
7. `toolbar-build` "Build ⌘F9"; `toolbar-run`; `toolbar-debug`; `toolbar-debug-stop`.
8. `toolbar-search` aria-label "Search Everywhere".
9. `toolbar-settings` aria-label "Open Settings".
10. `workspace-actions-menu` "•••": `open-find-action`, `open-keymap`, `sdk-redetect`, `sdk-open-global-json`, `git-commit-action`, `close-workspace`.

## 4. Rail, tool windows, splitters
- `tool-windows`; `toolband-left`, `toolband-bottom`; icons `toolband-{id}`, aria-pressed, aria-label = title.
- Registry: `solution` left "Solution Explorer"; `commit` left "Commit"; `build`, `problems`, `debug`, `terminal`, `tests`, `git`, `nuget` bottom.
- Bottom tabs `bottom-tab-{id}` inside "Bottom panel"; label `Problems (n)`. Body `output-content`.
- One active window per slot; clicking active icon hides; Shift+Esc hides active.
- Splitters: separators "Solution explorer width" and "Output panel height"; arrows ±16; ranges 220–420 / 120–500.
- Persisted `openridr.layout.v1`.

## 5. Status bar
`status-workspace`, `status-breadcrumb`, `status-mode`, `status-sdk`, `status-evaluation`, `status-dirty`, `status-keymap`, `status-encoding`, `status-indent`, `status-cursor`.

## 6. Search Everywhere — ⌘T
- `find-action`; "Search Everywhere"; `find-action-profile`; `search-tab-{all|classes|files|symbols|actions|text}`; `find-action-input`; `find-action-list`.
- `find-action-item-{actionId}`, `find-action-shortcut-{actionId}`, `find-action-conflict-{actionId}`, `find-action-reason-{actionId}`; `search-item-{index}`; `search-state`, `search-loading`, `find-action-empty`/`search-empty`, `search-truncated`, `search-status`.
- `search-preview`, `search-preview-path`, `search-preview-code`, `search-preview-action`, `search-preview-error`.
- Keys: ↑/↓, Enter, Tab/⇧Tab, Esc. Shift Shift ≤400ms. Persisted `openridr.search`.

## 7. Find Action — ⇧⌘A
- Same dialog, initial tab "actions". Disabled rows show reason. `workspace-actions-menu` mirrors registry.

## 8. Run configurations dialog
- `run-config-dialog`; list `run-config-item-default`, `run-config-item-{id}`, `run-config-new`.
- Fields `run-config-name`, `-project`, `-configuration`, `-tfm`, `-args`, `-cwd`, env rows, `.env` rows.
- `run-config-save`, `-duplicate`, `-remove`, `-cancel`; `run-config-default-note`; errors `run-config-error-{field}`, `run-config-error`, `run-config-issues`.

## 9. Settings — ⌘,
- `settings-dialog`, `settings-scope`, `settings-search`, `settings-page-{appearance|keymap|editor|build|vcs}`, `settings-page`, `settings-page-title`, `settings-no-results`.
- Appearance: `settings-theme`, `settings-theme-note`, `settings-zoom`, `settings-density`.
- Keymap: `keymap-profile`, `keymap-active-profile`, `keymap-row-{id}`, `keymap-shortcut-{id}`, `keymap-change-{id}`, `keymap-reset-{id}`, `keymap-conflict-{id}`, `keymap-error`.
- Editor: `settings-font-family`, `settings-font-size`, `settings-indent-style`, `settings-indent-size`.
- Build: `settings-sdk-list`, `settings-analysis-host`, `settings-host-root`, `settings-redetect`, `settings-redetect-message`.
- VCS: `settings-git-path`, `settings-git-version`, `settings-git-message`.
- `settings-error`, `settings-save-error`, `settings-save`, `settings-cancel`.

## 10. Git
- Toolbar branch popup: `branch-upstream`, `branch-search`, `branch-error`, `branch-item-{name}`, `branch-create-open`, `branch-name`, `branch-create-submit`, `branch-create-cancel`, `branch-refresh`, `branch-commit`, `branch-update`, `branch-push`, reasons `branch-update-reason`, `branch-push-reason`, `branch-checkout-reason`.
- Commit tool: `commit-tool`, `commit-disabled-reason`, `commit-refresh`, `commit-count`, `commit-clean`, `commit-check-{path}`, `commit-file-{path}`, `commit-diff`, `commit-message`, `commit-amend`, `commit-submit`, `commit-state`.
- Git log: `git-log`, `git-log-disabled-reason`, `git-refresh`, `git-log-error`, `git-log-state`, `git-log-item-{shortHash}`, `git-log-files`, `git-log-files-empty`.
- Dialogs: `update-dialog` + `update-confirm`/`update-cancel`/`update-error`; `push-dialog` + `push-remote`, `push-branch`, `push-upstream`, `push-ahead`, `push-disabled-reason`, `push-confirm`/`push-cancel`/`push-error`.
- Editor diff modal: `editor-git-diff`, `editor-git-diff-state`, `editor-git-diff-message`, `editor-git-diff-content`, `editor-git-diff-close`.

## 11. NuGet
- Untrusted: `nuget-panel`, `nuget-disabled-reason`.
- Trusted: `nuget-project`, `nuget-refresh`, `nuget-sdk`, `nuget-probe`, `nuget-error`, `nuget-state`, `nuget-summary`, `nuget-sources`, `nuget-cpm`, `nuget-update-reason`.
- Per TFM: `nuget-tfm-{tfm}`, `nuget-package-{tfm}-{id}`, `nuget-latest-{tfm}-{id}`, `nuget-source-{tfm}-{id}`, `nuget-update-{tfm}-{id}`.
- Logs: `nuget-installed-log`, `nuget-outdated-log`. Plan: `nuget-plan-dialog`, `nuget-plan-project`, `nuget-plan-version`, `nuget-plan-command`, `nuget-plan-confirm`, `nuget-plan-cancel`, `nuget-plan-error`; after update `nuget-update-state`, `nuget-update-exit`, `nuget-update-log`, `nuget-restore-pending`, `nuget-restore`, `nuget-restore-reason`, `nuget-restore-state`, `nuget-cancelled`.

## 12. Tests
- `tests-project`, `tests-tfm`, `tests-discover`, `tests-cancel`, `tests-retry`, `tests-run-all`, `tests-run-selected`, `tests-debug-selected`, `tests-rerun-failed`, `tests-run-cancel`, `tests-run-retry`, `tests-state`, `tests-discovery-id`.
- Gates: `tests-disabled-reason`, `tests-restore-required`, `tests-mtp-message`, `tests-guidance`, `tests-sealed`, `tests-stale`, `tests-termination`, `tests-awaiting`, `tests-selection-blocked`.
- Meta: `tests-counts`, `tests-tree-state`, `tests-grouping`, `tests-scope`, `tests-command`, `tests-cwd`, `tests-sdk`, `tests-exit`, `tests-log`, `tests-log-truncated`; filter `tests-filter`, `tests-filter-count`, `tests-filter-empty`; tree `tests-items`, `test-item-{i}`, `test-checkbox-{i}`, `test-status-{i}`, `test-identity-{i}`, `test-ambiguous-{i}`.
- Run panel: `tests-run-panel`, `tests-run-state`, `tests-run-counts`, `tests-run-command`, `tests-run-exit`, `tests-run-trx`, `tests-run-results`, `tests-run-results-list`, `test-result-{i}`, `test-result-outcome-{i}`, `test-result-name-{i}`, `test-result-details-{i}`, `test-result-navigate-{i}`, `test-result-no-position-{i}`, `tests-run-log`.
- Quick list: ⇧⌥U → `tests-quick-list`, items `tests-quick-list-item-tests.*`.

## 13. Editor interactions
- Right-click `editor` → `editor-context-menu`, items `context-menu-{context-actions|find-usages|go-to-definition|rename|run-test|debug-test|git-diff|git-commit|copy-path}`.
- Alt+Enter: `code-actions`, `code-actions-list`, `code-action-{index}`, `code-actions-empty`, `code-actions-stale`, `code-actions-close`; pick → `rename-preview` data-origin=action.
- Rename: `inline-rename`, `rename-source`, `rename-input`, `rename-submit`, `rename-cancel`, `rename-error`; preview `rename-preview`, `rename-preview-summary`, `rename-stale`, `rename-index-{path}`, `rename-file-{path}`, `rename-count-{path}`, `rename-diff-{path}`, `rename-apply`, `rename-cancel-preview`.
- Find Usages: `find-usages`, `find-usages-summary`, `find-usages-list`, `find-usages-file-{path}`, `find-usage-{index}`, `find-usages-close`.
- Gutter: `gutter-run-{line}`, `gutter-test-{line}`; debug gutter `.cm-debug-gutter` (● ▶); editor `editor`, `editor-shell`, `editor-lsp-status`.

## 14. Debug
- `debug-tab`; `dap-panel`, `dap-state`, `dap-phase`, `debug-launch`, `debug-stop`, `debug-continue`, `debug-step-over`, `debug-step-into`, `dap-disabled-reason`, `dap-identity`, `dap-session`, `dap-build-id`, `dap-assembly`, `dap-adapter`, `dap-process`, `dap-exit`, `dap-termination`, `dap-thread-list`, `debug-frames`, `debug-locals`, `debug-breakpoints`, `dap-log`, `dap-output`.
- Adapter install: `debug-panel`, `debug-state`, `debug-plan-open`, `debug-install`, `debug-flag-checkbox`, `debug-flag-enable`, `debug-flag-disable`, `debug-plan`/`debug-plan-*`, `debug-log`.

## 15. Terminal
- `terminal-panel`, `terminal-disabled-reason`, `terminal-shell`, `terminal-cwd`, `terminal-size`, `terminal-state`, `terminal-process`, `terminal-start`, `terminal-close`, `terminal-interrupt`; dialog `terminal-confirm` + `-keep`/`-stop`; `terminal-output`, `terminal-exit`, `terminal-termination`, `terminal-input`, `terminal-send`.

## 16. Keyboard shortcuts / action registry
| id | title | Rider (macOS) | MVP enabled? |
|---|---|---|---|
| view.findAction | Find Action | ⇧⌘A | yes |
| view.keymap | Keymap: change shortcuts | — | yes |
| file.save | Save | ⌘S | open file |
| toolWindow.hideActive | Hide Active Tool Window | ⇧Esc | open slot |
| build.start | Build | ⌘F9 | gates |
| run.start | Run | ⌃F5 / ⌃R | needs build |
| debug.start | Debug | ⌥F5 / ⌃D | gates |
| run.stop | Stop | ⇧F5 | running |
| debug.stepOver / stepInto | Step Over/Into | F8 / F7 | paused |
| debug.toggleBreakpoint | Toggle Breakpoint | ⌘F8 | C# file |
| code.goToDeclaration | Go to Declaration | F12 / ⌘B | trust+analysis |
| code.findUsages | Find Usages | ⇧F12 | trust+analysis |
| code.contextActions | Show Context Actions | ⌥Enter | gates |
| code.rename | Rename Symbol | ⇧F6 | gates |
| search.everywhere | Search Everywhere | ⌘T / Shift Shift | yes |
| search.file/symbol/text | Go to File/Symbol/Find in Path | ⇧⌘T / ⌥⇧⌘T / ⌥⇧⌘E | yes |
| search.line | Go to Line | ⌘G | **disabled** "arrives in F3" |
| tests.runAll/runSelected/rerunFailed | tests | ⌘U L/R/F | gates |
| tests.debugSelected | Debug Selected Tests | ⌘U D | **always disabled** (P1) |
| tests.quickList | Show Test Quick List | ⇧⌥U | yes |
| git.commit/push/update/branchCreate | Git | ⌥⇧⌘K / ⇧⌘K / ⌥⌘W / ⌥⌘N | trust+repo |
| run.configure | Edit Run Configurations… | — | yes |
| settings.open | Open Settings | ⌘, | yes |
| sdk.redetect / sdk.openGlobalJson | SDK | — | yes / conditional |
| workspace.close | Close Workspace | — | yes |

## 17. Dialogs / menus inventory
- Project popup `toolbar-project-menu`; trust popup `trust-banner`.
- Close-unsaved dialog: `confirm-save`, `confirm-keep`, `confirm-discard`, `confirm-error`.
- Dirty build/run: `build-dirty-dialog` (`build-save-and-start`, `build-saved-only`, `build-decision-cancel`); `run-dirty-dialog` (`run-saved-build`, `run-save-rebuild`, `run-decision-cancel`); `run-dirty-warning`.
- Conflict: `conflict-banner`, `conflict-reload`, `conflict-overwrite`, `conflict-compare`, `conflict-compare-diff`.
- Processes: `processes-panel`, `processes-row-{id}`, `processes-terminate-{id}`, `processes-keep-{id}`.
- `app-log` notification with dismiss.

## Persisted keys
- localStorage `openridr.layout.v1`, `openridr.search`.
- App home `settings.json`, `run-configs/<fnv1a64(solution)>.json`.
