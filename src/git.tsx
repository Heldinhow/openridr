import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { call, messageOf } from "./host/client";
import { Icon } from "./Icon";
import { MidText } from "./shell/MidText";
import { FileIcon } from "./FileIcon";
import { RefreshCw, ChevronDown, ChevronRight, Eye, ChevronsUpDown, Undo2 } from "lucide-react";

export type GitFile = {
  path: string;
  originalPath: string | null;
  index: string;
  worktree: string;
  untracked: boolean;
  staged: boolean;
  conflicted: boolean;
  label: string;
};

export type GitStatus = {
  state: "ok" | "not_a_repository" | "git_missing" | "failed";
  message: string | null;
  root: string | null;
  branch: string | null;
  detached: boolean;
  initial: boolean;
  upstream: string | null;
  upstreamGone: boolean;
  ahead: number;
  behind: number;
  operation: string | null;
  clean: boolean;
  files: GitFile[];
  remotes: string[];
};

export type GitBranch = {
  name: string;
  upstream: string | null;
  ahead: number;
  behind: number;
  current: boolean;
  remote: boolean;
  gone: boolean;
};

export type GitBranches = {
  state: "ok" | "not_a_repository" | "git_missing" | "failed";
  message: string | null;
  current: string | null;
  local: GitBranch[];
  remote: GitBranch[];
  remotes: string[];
};

export type GitCommitFile = {
  path: string;
  originalPath: string | null;
  status: string;
};

export type GitCommit = {
  hash: string;
  shortHash: string;
  author: string;
  date: string;
  subject: string;
  parents: string[];
  files: GitCommitFile[];
  filesTruncated: boolean;
};

export type GitLog = {
  state: "ok" | "no_commits" | "not_a_repository" | "git_missing" | "failed";
  message: string | null;
  limit: number;
  truncated: boolean;
  commits: GitCommit[];
};

export type GitDiff = {
  state: "ok" | "empty" | "not_a_repository" | "git_missing" | "failed";
  message: string | null;
  path: string;
  staged: boolean;
  diff: string;
  truncated: boolean;
};

export type GitDiffSide = {
  exists: boolean;
  text: string | null;
};

/** One changed block as `git diff -U0` reports it: a zero count means "inserted after start". */
export type GitHunk = {
  oldStart: number;
  oldCount: number;
  newStart: number;
  newCount: number;
};

export type GitDiffCommit = {
  hash: string;
  shortHash: string;
  subject: string;
};

/** The diff panel's read: full sides, the block ranges that align them, and the HEAD commit. */
export type GitFileDiff = {
  state: "ok" | "empty" | "not_a_repository" | "git_missing" | "failed";
  message: string | null;
  path: string;
  staged: boolean;
  base: GitDiffSide;
  current: GitDiffSide;
  hunks: GitHunk[];
  patch: string;
  truncated: boolean;
  full: boolean;
  fullReason: "binary" | "too_large" | "unsupported_encoding" | "unreadable" | "unaligned" | null;
  commit: GitDiffCommit | null;
};

export type GitOperation = {
  state: "ok" | "failed";
  message: string | null;
  output: string;
  status: GitStatus;
  commit: {
    hash: string;
    shortHash: string;
    summary: string;
    amend: boolean;
    paths: string[];
  } | null;
  remote: string | null;
  branch: string | null;
  upstream: string | null;
  ahead: number;
};

const TRUST_MESSAGE = "Trust the workspace to use Git";

function unavailable(status: GitStatus | null, error: string | null): string | null {
  if (error) return error;
  if (!status) return null;
  if (status.state === "not_a_repository") return "Not a git repository";
  if (status.state === "git_missing") return "git was not found on PATH.";
  if (status.state === "failed") return status.message ?? "Git is unavailable.";
  return null;
}

function cleanReason(status: GitStatus | null): string | null {
  if (!status || status.state !== "ok") return null;
  if (status.operation) {
    // Only a merge can be finished or aborted from the app (#114); the others are named as a
    // terminal job instead of promising a button that does not exist.
    return status.operation === "merge"
      ? "A merge is in progress; finish or abort it from the branch menu before checking out or updating."
      : `A ${status.operation} is in progress; finish or abort it in a terminal before checking out or updating.`;
  }
  if (!status.clean) {
    return "The working tree has uncommitted changes; commit them first. OpenRidr does not auto-stash.";
  }
  return null;
}

/** The shared reason a Git surface cannot act, in the wording the ticket requires. */
export function gitBlockedReason(
  status: GitStatus | null,
  error: string | null,
  trusted: boolean,
): string | null {
  if (!trusted) return TRUST_MESSAGE;
  return unavailable(status, error) ?? (status ? null : "Loading Git status…");
}

/** A Push action is unavailable when no remote exists; a dirty tree is fine for push. */
export function gitPushBlockedReason(
  status: GitStatus | null,
  error: string | null,
  trusted: boolean,
): string | null {
  if (!trusted || error || !status || status.state !== "ok") {
    return gitBlockedReason(status, error, trusted);
  }
  if (status.upstream === null && status.remotes.length === 0) {
    return "No remote is configured; add one with git remote add.";
  }
  return null;
}

/**
 * A Fetch action is unavailable only when no remote exists. Unlike Checkout and Update it never
 * refuses a dirty tree or an in-progress operation: fetch moves refs, not the tree.
 */
export function gitFetchBlockedReason(
  status: GitStatus | null,
  error: string | null,
  trusted: boolean,
): string | null {
  if (!trusted || error || !status || status.state !== "ok") {
    return gitBlockedReason(status, error, trusted);
  }
  if (status.remotes.length === 0) {
    return "No remote is configured; add one with git remote add.";
  }
  return null;
}

/** Update (pull) needs a clean tree and an upstream; it never auto-stashes. */
export function gitUpdateBlockedReason(
  status: GitStatus | null,
  error: string | null,
  trusted: boolean,
): string | null {
  if (!trusted || error || !status || status.state !== "ok") {
    return gitBlockedReason(status, error, trusted);
  }
  if (status.upstream === null) return "No upstream is configured; nothing to pull.";
  return cleanReason(status);
}

function branchName(status: GitStatus) {  if (status.detached) return "HEAD detached";
  return status.branch ?? "No branch";
}

function counters(status: GitStatus) {
  const parts = [];
  if (status.ahead > 0) parts.push(`↑${status.ahead}`);
  if (status.behind > 0) parts.push(`↓${status.behind}`);
  return parts.join(" ");
}

function shortDate(value: string) {
  return value.slice(0, 16).replace("T", " ");
}

export type GitIntent = { kind: "push" | "update" | "fetch" | "branchCreate"; nonce: number };

export function BranchWidget({
  status,
  statusError,
  trusted,
  intent,
  onRefresh,
  onOpenCommit,
  onMessage,
}: {
  status: GitStatus | null;
  statusError: string | null;
  trusted: boolean;
  intent: GitIntent | null;
  onRefresh: () => Promise<void>;
  onOpenCommit: () => void;
  onMessage: (text: string) => void;
}) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [branches, setBranches] = useState<GitBranches | null>(null);
  const [filter, setFilter] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [dialog, setDialog] = useState<"update" | "push" | "mergeAbort" | null>(null);
  const [dialogError, setDialogError] = useState<string | null>(null);
  const [dialogBusy, setDialogBusy] = useState(false);
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [fetching, setFetching] = useState(false);
  // A fetch takes seconds; the command palette can ask for another one while the first is still
  // running. The ref is the guard the state cannot be: it is read at call time, not at render time.
  const fetchingRef = useRef(false);
  const detailsRef = useRef<HTMLDetailsElement>(null);

  async function loadBranches() {
    try {
      const listed = await call<GitBranches>("git.branches");
      setBranches(listed);
      if (listed.state !== "ok") setError(listed.message ?? "Branches are unavailable.");
    } catch (caught) {
      setError(messageOf(caught));
    }
  }

  useEffect(() => {
    if (!menuOpen) return;
    void loadBranches();
    void onRefresh();
    // Only when the popup opens; refresh() is stable per trust state.
  }, [menuOpen]);

  useEffect(() => {
    if (!intent) return;
    setMenuOpen(true);
    setError(null);
    setDialogError(null);
    setFetchError(null);
    const blocked = status ? cleanReason(status) : null;
    if (intent.kind === "fetch") {
      // Fetch is the one action that runs straight from the command: it needs no confirmation
      // because it writes no branch, no index and no file.
      if (status && status.state === "ok" && status.remotes.length === 0) {
        setFetchError(gitFetchBlockedReason(status, null, true));
      } else if (!fetchingRef.current) {
        void runFetch();
      }
    } else if (intent.kind === "update") {
      if (status && status.state === "ok" && status.upstream === null) {
        setError("No upstream is configured; nothing to pull.");
      } else if (blocked) {
        setError(blocked);
      } else {
        setDialog("update");
      }
    } else if (intent.kind === "push") {
      setDialog("push");
    } else {
      setCreating(true);
    }
    // A new command runs once per shortcut press.
  }, [intent?.nonce]);

  if (!trusted) {
    return (
      <span className="toolbar-disabled" title={TRUST_MESSAGE}>
        <button type="button" data-testid="toolbar-branch" disabled aria-label={TRUST_MESSAGE} title={TRUST_MESSAGE}>
          <Icon name="git" /> Git
        </button>
      </span>
    );
  }
  const reason = unavailable(status, statusError);
  if (reason || !status) {
    const text = reason ?? "Loading Git status…";
    return (
      <span className="toolbar-disabled" title={text}>
        <button type="button" data-testid="toolbar-branch" disabled aria-label={text} title={text}>
          <Icon name="git" /> {status?.state === "not_a_repository" ? "No repo" : "Git unavailable"}
        </button>
      </span>
    );
  }

  const ahead = counters(status);
  const title = `Current branch: ${branchName(status)}${
    status.upstream ? ` · upstream ${status.upstream}` : " · no upstream"
  }${ahead ? ` · ${status.ahead} ahead, ${status.behind} behind` : ""}`;
  const remoteReason =
    status.upstream === null && status.remotes.length === 0
      ? "No remote is configured; add one with git remote add."
      : null;
  // Fetch needs a remote and nothing else: it moves refs, so a dirty tree or an operation in
  // progress is not a reason to disable it.
  const fetchReason =
    status.remotes.length === 0 ? "No remote is configured; add one with git remote add." : null;
  const updateReason =
    status.upstream === null
      ? "No upstream is configured; nothing to pull."
      : cleanReason(status);
  const checkoutBlocked = cleanReason(status);
  const query = filter.trim().toLowerCase();
  const matches = (branch: GitBranch) =>
    query === "" || branch.name.toLowerCase().includes(query);

  async function checkout(target: string) {
    setBusy(true);
    setError(null);
    try {
      const result = await call<GitOperation>("git.checkout", { name: target });
      if (result.state === "ok") {
        onMessage(result.message ?? `Checked out ${target}.`);
      } else {
        setError(result.message ?? `Could not check out ${target}.`);
      }
      await Promise.all([onRefresh(), loadBranches()]);
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  async function createBranch() {
    const target = name.trim();
    if (target === "") return;
    setBusy(true);
    setError(null);
    try {
      const result = await call<GitOperation>("git.branchCreate", { name: target, checkout: true });
      if (result.state === "ok") {
        onMessage(result.message ?? `Created ${target}.`);
        setCreating(false);
        setName("");
      } else {
        setError(result.message ?? `Could not create ${target}.`);
      }
      await Promise.all([onRefresh(), loadBranches()]);
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  async function runFetch() {
    if (fetchingRef.current) return;
    fetchingRef.current = true;
    setFetching(true);
    setBusy(true);
    setFetchError(null);
    try {
      const result = await call<GitOperation>("git.fetch");
      if (result.state === "ok") {
        onMessage(result.message ?? "Fetched the remote.");
      } else {
        setFetchError(result.message ?? "git fetch failed.");
      }
      await Promise.all([onRefresh(), loadBranches()]);
    } catch (caught) {
      setFetchError(messageOf(caught));
    } finally {
      fetchingRef.current = false;
      setFetching(false);
      setBusy(false);
    }
  }

  async function confirmUpdate() {
    setDialogBusy(true);
    setDialogError(null);
    try {
      const result = await call<GitOperation>("git.pull");
      if (result.state === "ok") {
        onMessage(result.message ?? "Update completed.");
        setDialog(null);
      } else {
        setDialogError(result.message ?? "git pull failed.");
      }
      await Promise.all([onRefresh(), loadBranches()]);
    } catch (caught) {
      setDialogError(messageOf(caught));
    } finally {
      setDialogBusy(false);
    }
  }

  const remote = status.upstream?.split("/")[0] ?? status.remotes[0] ?? null;

  async function completeMerge() {
    setBusy(true);
    setError(null);
    try {
      const result = await call<GitOperation>("git.mergeContinue");
      if (result.state === "ok") {
        onMessage(result.message ?? "Merge completed.");
      } else {
        setError(result.message ?? "git commit failed.");
      }
      await Promise.all([onRefresh(), loadBranches()]);
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  async function confirmMergeAbort() {
    setDialogBusy(true);
    setDialogError(null);
    try {
      const result = await call<GitOperation>("git.mergeAbort");
      if (result.state === "ok") {
        onMessage(result.message ?? "Merge aborted.");
        setDialog(null);
      } else {
        setDialogError(result.message ?? "git merge --abort failed.");
      }
      await Promise.all([onRefresh(), loadBranches()]);
    } catch (caught) {
      setDialogError(messageOf(caught));
    } finally {
      setDialogBusy(false);
    }
  }

  async function confirmPush() {
    setDialogBusy(true);
    setDialogError(null);
    try {
      const result = await call<GitOperation>("git.push", {
        confirm: true,
        remote: remote ?? undefined,
      });
      if (result.state === "ok") {
        onMessage(result.message ?? "Push completed.");
        setDialog(null);
      } else {
        setDialogError(result.message ?? "git push failed.");
      }
      await Promise.all([onRefresh(), loadBranches()]);
    } catch (caught) {
      setDialogError(messageOf(caught));
    } finally {
      setDialogBusy(false);
    }
  }

  function closeMenu() {
    setMenuOpen(false);
    detailsRef.current?.removeAttribute("open");
  }

  // Escape dismisses the dialog unless an operation is in flight.
  function dialogKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key !== "Escape" || dialogBusy) return;
    event.preventDefault();
    event.stopPropagation();
    setDialog(null);
  }

  return (
    <>
      <details
        ref={detailsRef}
        className="toolbar-menu branch-menu"
        data-testid="branch-menu"
        open={menuOpen}
        onToggle={(event) => {
          const isOpen = event.currentTarget.open;
          setMenuOpen(isOpen);
          if (!isOpen) {
            setError(null);
            setCreating(false);
          }
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            closeMenu();
            event.currentTarget.querySelector<HTMLElement>("summary")?.focus();
          }
        }}
      >
        <summary data-testid="toolbar-branch" title={title} aria-label={title}>
          <Icon name="git" />
          <span data-testid="branch-current"><MidText text={branchName(status)} /></span>
          {ahead && <span className="branch-counters">{ahead}</span>}
        </summary>
        <div className="menu-content branch-menu-content">
          <span className="menu-heading">Branches</span>
          <span className="menu-note" data-testid="branch-upstream">
            {status.upstream
              ? `Upstream ${status.upstream}${status.upstreamGone ? " (gone)" : ""}`
              : "No upstream"}
            {status.operation ? ` · ${status.operation} in progress` : ""}
          </span>
          {status.operation === "merge" && (
            <div className="branch-actions" data-testid="branch-merge">
              <span className="menu-note" data-testid="branch-merge-note">
                A merge is in progress: resolve the conflicts in the editor, then finish or abort it here.
              </span>
              <button
                type="button"
                data-testid="branch-merge-complete"
                disabled={busy}
                onClick={() => void completeMerge()}
              >
                Complete merge
              </button>
              <button
                type="button"
                data-testid="branch-merge-abort"
                disabled={busy}
                onClick={() => { setDialogError(null); setDialog("mergeAbort"); }}
              >
                Abort merge…
              </button>
            </div>
          )}
          <input
            className="branch-search"
            data-testid="branch-search"
            aria-label="Search branches"
            placeholder="Search branches"
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
          />
          {error && (
            <p className="error" role="alert" data-testid="branch-error">
              {error}
            </p>
          )}
          <div className="branch-list">
            {branches?.local.filter(matches).map((branch) => (
              <button
                key={`local-${branch.name}`}
                type="button"
                data-testid={`branch-item-${branch.name}`}
                aria-current={branch.current ? "true" : undefined}
                title={checkoutBlocked ?? `Check out ${branch.name}`}
                disabled={busy || checkoutBlocked !== null}
                onClick={() => void checkout(branch.name)}
              >
                {branch.name}
                {branch.current ? " (current)" : ""}
                {branch.ahead > 0 ? ` ↑${branch.ahead}` : ""}
                {branch.behind > 0 ? ` ↓${branch.behind}` : ""}
              </button>
            ))}
            {branches?.remote.filter(matches).map((branch) => (
              <button
                key={`remote-${branch.name}`}
                type="button"
                data-testid={`branch-item-${branch.name}`}
                title={checkoutBlocked ?? `Check out ${branch.name} as a local tracking branch`}
                disabled={busy || checkoutBlocked !== null}
                onClick={() => void checkout(branch.name)}
              >
                {branch.name}
                <span className="tag">remote</span>
              </button>
            ))}
            {branches && branches.state === "ok" && branches.local.length + branches.remote.length === 0 && (
              <span className="menu-note">No branches are listed yet.</span>
            )}
          </div>
          <div className="branch-actions">
            {creating ? (
              <form
                className="branch-create-form"
                onSubmit={(event) => {
                  event.preventDefault();
                  void createBranch();
                }}
              >
                <input
                  data-testid="branch-name"
                  aria-label="New branch name"
                  placeholder="New branch name"
                  value={name}
                  onChange={(event) => setName(event.target.value)}
                  autoFocus
                />
                <button type="submit" data-testid="branch-create-submit" disabled={busy || name.trim() === ""}>
                  Create
                </button>
                <button type="button" data-testid="branch-create-cancel" onClick={() => setCreating(false)}>
                  Cancel
                </button>
              </form>
            ) : (
              <button type="button" data-testid="branch-create-open" disabled={busy} onClick={() => setCreating(true)}>
                New branch…
              </button>
            )}
            <button
              type="button"
              data-testid="branch-refresh"
              disabled={busy}
              onClick={() => void Promise.all([onRefresh(), loadBranches()])}
            >
              Refresh
            </button>
            <button
              type="button"
              data-testid="branch-fetch"
              title={
                fetchReason ??
                `Fetch ${remote ?? "the remote"} and prune branches deleted there (git fetch --prune). OpenRidr never touches your tree, index or local branches.`
              }
              disabled={busy || fetchReason !== null}
              aria-busy={fetching || undefined}
              onClick={() => void runFetch()}
            >
              {fetching ? "Fetching…" : "Fetch"}
            </button>
            <button type="button" data-testid="branch-commit" disabled={busy} onClick={() => { closeMenu(); onOpenCommit(); }}>
              Commit…
            </button>
            <button
              type="button"
              data-testid="branch-update"
              title={updateReason ?? "Pull the upstream branch. OpenRidr never auto-stashes."}
              disabled={busy || updateReason !== null}
              onClick={() => { setDialogError(null); setDialog("update"); }}
            >
              Update…
            </button>
            <button
              type="button"
              data-testid="branch-push"
              title={remoteReason ?? "Push after an explicit confirmation. OpenRidr never forces."}
              disabled={busy || remoteReason !== null}
              onClick={() => { setDialogError(null); setDialog("push"); }}
            >
              Push…
            </button>
            {fetchReason && (
              <span className="menu-note" data-testid="branch-fetch-reason">{fetchReason}</span>
            )}
            {fetchError && (
              <p className="error" role="alert" data-testid="branch-fetch-error">
                {fetchError}
              </p>
            )}
            {updateReason && updateReason !== checkoutBlocked && (
              <span className="menu-note" data-testid="branch-update-reason">{updateReason}</span>
            )}
            {remoteReason && (
              <span className="menu-note" data-testid="branch-push-reason">{remoteReason}</span>
            )}
            {checkoutBlocked && (
              <span className="menu-note" data-testid="branch-checkout-reason">{checkoutBlocked}</span>
            )}
          </div>
        </div>
      </details>

      {dialog === "update" && (
        <div className="modal-backdrop">
          <div className="modal" role="dialog" aria-modal="true" aria-labelledby="update-title" data-testid="update-dialog" onKeyDown={dialogKeyDown}>
            <h2 id="update-title">Update project</h2>
            <dl>
              <dt>Branch</dt>
              <dd>{branchName(status)}</dd>
              <dt>Upstream</dt>
              <dd>{status.upstream ?? "Not set"}</dd>
              <dt>Ahead</dt>
              <dd data-testid="update-ahead">{status.ahead}</dd>
              <dt>Behind</dt>
              <dd>{status.behind}</dd>
            </dl>
            <p className="hint" data-testid="update-policy">
              When the branch and its upstream both moved, Update fetches and merges the upstream
              into a merge commit (<code>git fetch</code>, then{" "}
              <code>git merge --ff --no-edit</code>); a fast-forward stays a fast-forward. The
              repository's <code>pull.rebase</code>, <code>pull.ff</code> and{" "}
              <code>merge.ff</code> settings do not change that. If the merge conflicts, it stays in
              progress: resolve the files in the editor, then finish or abort it from the branch
              menu. Nothing is pushed.
            </p>
            <p className="hint">
              A dirty tree or an in-progress operation refuses the update; OpenRidr never stashes.
            </p>
            {dialogError && (
              <p className="error" role="alert" data-testid="update-error">{dialogError}</p>
            )}
            <div className="actions">
              <button type="button" data-testid="update-confirm" autoFocus disabled={dialogBusy} onClick={() => void confirmUpdate()}>
                {dialogBusy ? "Updating…" : "Update"}
              </button>
              <button type="button" data-testid="update-cancel" onClick={() => setDialog(null)}>Cancel</button>
            </div>
          </div>
        </div>
      )}

      {dialog === "mergeAbort" && (
        <div className="modal-backdrop">
          <div className="modal" role="dialog" aria-modal="true" aria-labelledby="merge-abort-title" data-testid="merge-abort-dialog" onKeyDown={dialogKeyDown}>
            <h2 id="merge-abort-title">Abort merge</h2>
            <p className="hint">
              Undoes the merge: the branch, index and working tree go back to the state before the
              merge. Conflict resolutions and anything staged during the merge are discarded; your
              commits stay. Uncommitted changes that existed before the merge may not come back.
            </p>
            {dialogError && (
              <p className="error" role="alert" data-testid="merge-abort-error">{dialogError}</p>
            )}
            <div className="actions">
              <button type="button" data-testid="merge-abort-confirm" autoFocus disabled={dialogBusy} onClick={() => void confirmMergeAbort()}>
                {dialogBusy ? "Aborting…" : "Abort merge"}
              </button>
              <button type="button" data-testid="merge-abort-cancel" onClick={() => setDialog(null)}>Cancel</button>
            </div>
          </div>
        </div>
      )}

      {dialog === "push" && (
        <div className="modal-backdrop">
          <div className="modal" role="dialog" aria-modal="true" aria-labelledby="push-title" data-testid="push-dialog" onKeyDown={dialogKeyDown}>
            <h2 id="push-title">Push</h2>
            <dl>
              <dt>Remote</dt>
              <dd data-testid="push-remote">{remote ?? "No remote"}</dd>
              <dt>Branch</dt>
              <dd data-testid="push-branch">{branchName(status)}</dd>
              <dt>Upstream</dt>
              <dd data-testid="push-upstream">{status.upstream ?? "Not set (pushes with --set-upstream)"}</dd>
              <dt>Commits ahead</dt>
              <dd data-testid="push-ahead">{status.ahead}</dd>
            </dl>
            {remoteReason && (
              <p className="hint" data-testid="push-disabled-reason">{remoteReason}</p>
            )}
            <p className="hint">
              Runs <code>git push</code>. Never <code>--force</code>, never tags. Credentials
              come from your git credential helper; OpenRidr stores none.
            </p>
            {dialogError && (
              <p className="error" role="alert" data-testid="push-error">{dialogError}</p>
            )}
            <div className="actions">
              <button type="button" data-testid="push-confirm" autoFocus={remote !== null} disabled={dialogBusy || remote === null} onClick={() => void confirmPush()}>
                {dialogBusy ? "Pushing…" : "Push"}
              </button>
              <button type="button" data-testid="push-cancel" autoFocus={remote === null} onClick={() => setDialog(null)}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

export function CommitPanel({
  status,
  statusError,
  trusted,
  initialPath = null,
  onRefresh,
  onOpenDiff,
}: {
  status: GitStatus | null;
  statusError: string | null;
  trusted: boolean;
  initialPath?: string | null;
  onRefresh: () => Promise<void>;
  onOpenDiff: (path: string) => void;
}) {
  const [selection, setSelection] = useState<Set<string> | null>(null);
  const [message, setMessage] = useState("");
  const [amend, setAmend] = useState(false);
  const [diffPath, setDiffPath] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [state, setState] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const focused = useRef<string | null>(null);
  const groupCheck = useRef<HTMLInputElement>(null);
  const [expanded, setExpanded] = useState(true);
  const reason = !trusted ? TRUST_MESSAGE : unavailable(status, statusError);
  // Git refuses a partial commit during a merge, so the app refuses first and names the way out.
  const operationReason = status?.operation
    ? status.operation === "merge"
      ? "A merge is in progress; finish or abort it from the branch menu. A partial commit is not possible during a merge."
      : `A ${status.operation} is in progress; finish or abort it in a terminal. A partial commit is not possible during a ${status.operation}.`
    : null;
  const files = status?.state === "ok" ? status.files : [];
  const selected = useMemo(
    () => selection ?? new Set(files.filter((file) => !file.conflicted).map((file) => file.path)),
    [selection, files],
  );
  const commitPaths = files
    .filter((file) => selected.has(file.path) && !file.conflicted)
    .map((file) => file.path);

  useEffect(() => {
    if (groupCheck.current) groupCheck.current.indeterminate = commitPaths.length > 0 && commitPaths.length < files.filter((file) => !file.conflicted).length;
  }, [commitPaths.length, files]);

  useEffect(() => {
    if (!trusted) return;
    void onRefresh();
    // Refresh once when the panel opens; operations refresh explicitly.
  }, []);

  function toggle(path: string) {
    const next = new Set(selected);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setSelection(next);
  }

  function showDiff(file: GitFile) {
    setDiffPath(file.path);
    onOpenDiff(file.path);
  }

  useEffect(() => {
    if (!initialPath || focused.current === initialPath) return;
    const file = files.find((entry) => entry.path === initialPath && !entry.conflicted);
    if (!file) return;
    focused.current = initialPath;
    setSelection(new Set([initialPath]));
    void showDiff(file);
    // The path comes from the editor context menu; later refreshes must not replace the
    // user's selection.
  }, [initialPath, files]);

  async function submit() {
    if (busy) return;
    setBusy(true);
    setError(null);
    setState(null);
    try {
      const result = await call<GitOperation>("git.commit", {
        paths: commitPaths,
        message,
        amend,
      });
      if (result.state === "ok") {
        setState(result.message ?? "Commit created.");
        setMessage("");
        setAmend(false);
        setSelection(null);
        setDiffPath(null);
      } else {
        setError(result.message ?? "git commit failed.");
      }
      await onRefresh();
    } catch (caught) {
      setError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <nav className="side commit-tool" aria-label="Commit" data-testid="commit-tool">
      <div className="panel-title">Commit</div>
      {reason ? (
        <p className="tree-note" data-testid="commit-disabled-reason">{reason}</p>
      ) : (
        <>
          <div className="git-toolbar commit-toolbar">
            <button type="button" data-testid="commit-refresh" aria-label="Refresh" title="Refresh" disabled={busy} onClick={() => void onRefresh()}><RefreshCw size={16} /></button>
            <button type="button" aria-label="Rollback" title="Rollback is not available in the Commit tool" disabled><Undo2 size={16} /></button>
            <button type="button" aria-label="Expand or collapse changes" title="Expand or collapse changes" onClick={() => setExpanded(!expanded)}><ChevronsUpDown size={16} /></button>
            <button type="button" aria-label="Show selected diff" title="Show selected diff" disabled={!diffPath} onClick={() => { const file = files.find((entry) => entry.path === diffPath); if (file) void showDiff(file); }}><Eye size={16} /></button>
            <span className="commit-selection-count" data-testid="commit-count">{commitPaths.length} of {files.length} selected</span>
          </div>
          {operationReason && (
            <p className="tree-note" data-testid="commit-operation-note">{operationReason}</p>
          )}
          <div className="commit-group">
            <button type="button" aria-label={expanded ? "Collapse changes" : "Expand changes"} aria-expanded={expanded} onClick={() => setExpanded(!expanded)}>{expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />}</button>
            <input ref={groupCheck} type="checkbox" aria-label="Select all changes" checked={files.some((file) => !file.conflicted) && commitPaths.length === files.filter((file) => !file.conflicted).length} disabled={busy || !files.some((file) => !file.conflicted)} onChange={(event) => setSelection(new Set(event.target.checked ? files.filter((file) => !file.conflicted).map((file) => file.path) : []))} />
            <strong>Changes</strong><span>{files.length} {files.length === 1 ? "file" : "files"}</span>
          </div>
          <div className="commit-files">
            {files.length === 0 && (
              <p className="tree-note" data-testid="commit-clean">
                The working tree is clean.
              </p>
            )}
            {expanded && files.map((file) => (
              <div className={`commit-row ${diffPath === file.path ? "selected" : ""}`}  key={`${file.path}-${file.originalPath ?? ""}`}>
                <input
                  type="checkbox"
                  data-testid={`commit-check-${file.path}`}
                  aria-label={`Commit ${file.path}`}
                  checked={selected.has(file.path)}
                  disabled={busy || file.conflicted}
                  title={file.conflicted ? "Conflicted files cannot be committed; resolve the conflict in the editor, then finish or abort the merge from the branch menu." : file.label}
                  onChange={() => toggle(file.path)}
                />
                <button
                  type="button"
                  className="commit-file"
                  data-testid={`commit-file-${file.path}`}
                  title={file.conflicted ? "Conflicted files cannot be committed; resolve the conflict in the editor, then finish or abort the merge from the branch menu." : file.path}
                  onClick={() => void showDiff(file)}
                >
                  <FileIcon path={file.path} />
                  <span className="commit-path" title={file.path}>
                    <span className={`commit-name ${file.untracked ? "untracked" : file.index === "D" || file.worktree === "D" ? "deleted" : file.index === "A" ? "added" : "modified"}`}>{file.path.slice(file.path.lastIndexOf("/") + 1)}</span>
                    <span className="commit-dir">{file.path.includes("/") ? file.path.slice(0, file.path.lastIndexOf("/")) : ""}</span>
                  </span>
                  <span className="commit-status">{file.label}</span>
                </button>
              </div>
            ))}
          </div>
          <div className="commit-form">
            <label className="commit-message-field">
              <textarea
                placeholder="Commit Message"
                data-testid="commit-message"
                aria-label="Commit message"
                value={message}
                onChange={(event) => setMessage(event.target.value)}
                rows={3}
                disabled={busy}
              />
            </label>
            <label className="commit-amend-field">
              <input
                type="checkbox"
                data-testid="commit-amend"
                checked={amend}
                disabled={busy}
                onChange={(event) => setAmend(event.target.checked)}
              />
              Amend the last commit
            </label>
            {amend && (
              <p className="hint">
                Only the checked files are amended into the previous commit; the message above
                replaces it.
              </p>
            )}
            <button
              type="button"
              data-testid="commit-submit"
              disabled={busy || message.trim() === "" || commitPaths.length === 0 || operationReason !== null}
              title={operationReason ?? undefined}
              onClick={() => void submit()}
            >
              {busy ? "Committing…" : amend ? "Amend commit" : "Commit"}
            </button>
            <button type="button" className="commit-push" disabled title="Combined commit and push is not available; push separately from the branch menu.">Commit and Push…</button>
            {(state || error) && (
              <p
                role={error ? "alert" : "status"}
                className={error ? "error" : "hint"}
                data-testid="commit-state"
              >
                {error ?? state}
              </p>
            )}
          </div>
        </>
      )}
    </nav>
  );
}

export function GitLogPanel({
  trusted,
  onError,
}: {
  trusted: boolean;
  onError: (text: string) => void;
}) {
  const [log, setLog] = useState<GitLog | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");

  async function load() {
    if (!trusted) return;
    setBusy(true);
    setError(null);
    try {
      const answer = await call<GitLog>("git.log");
      setLog(answer);
      setSelected((current) =>
        current && answer.commits.some((commit) => commit.hash === current) ? current : null,
      );
    } catch (caught) {
      setError(messageOf(caught));
      onError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void load();
    // Load when the tool window opens or trust changes; there is no polling.
  }, [trusted]);

  if (!trusted) {
    return (
      <div data-testid="git-log">
        <p className="hint" data-testid="git-log-disabled-reason">{TRUST_MESSAGE}</p>
      </div>
    );
  }
  if (log && log.state !== "ok" && log.state !== "no_commits") {
    return (
      <div data-testid="git-log">
        <p className="hint" data-testid="git-log-disabled-reason">
          {log.message ?? "Git log is unavailable."}
        </p>
      </div>
    );
  }
  const selectedCommit = log?.commits.find((commit) => commit.hash === selected) ?? null;
  return (
    <div className="git-log-panel" data-testid="git-log">
      <div className="git-toolbar">
        <span className="panel-title">Git log</span>
        <button type="button" data-testid="git-refresh" disabled={busy} onClick={() => void load()}>
          Refresh
        </button>
        <input
          type="search"
          data-testid="git-log-filter"
          aria-label="Filter commits by text, author or hash"
          placeholder="Text, author or hash"
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
        />
        <span className="menu-note">Up to {log?.limit ?? 200} commits on the current branch; no graph or pagination.</span>
      </div>
      {error && <p className="error" role="alert" data-testid="git-log-error">{error}</p>}
      {log && log.state === "no_commits" && (
        <p className="hint" data-testid="git-log-state">No commits yet.</p>
      )}
      {!log && !error && <p className="hint" data-testid="git-log-state">Loading commits…</p>}
      {log && log.state === "ok" && (
        <div className="git-log-body">
          <ul className="git-commit-list" aria-label="Commits">
            {log.commits.filter((commit) => {
              const needle = filter.trim().toLowerCase();
              return needle === "" || `${commit.hash} ${commit.subject} ${commit.author}`.toLowerCase().includes(needle);
            }).map((commit) => (
              <li key={commit.hash}>
                <button
                  type="button"
                  className={`git-commit ${selected === commit.hash ? "selected" : ""}`}
                  data-testid={`git-log-item-${commit.shortHash}`}
                  aria-current={selected === commit.hash ? "true" : undefined}
                  title={`${commit.hash}\n${commit.parents.length} parent(s)`}
                  onClick={() => setSelected(commit.hash)}
                >
                  <code>{commit.shortHash}</code>
                  <span className="git-subject">{commit.subject}</span>
                  <span className="menu-note">{commit.author} · {shortDate(commit.date)}</span>
                  {commit.parents.length !== 1 && <span className="tag">{commit.parents.length === 0 ? "root" : `${commit.parents.length} parents`}</span>}
                </button>
              </li>
            ))}
          </ul>
          <section className="git-commit-details" aria-label="Commit files">
            {selectedCommit ? (
              <>
                <div className="panel-title">{selectedCommit.shortHash} · {selectedCommit.subject}</div>
                <p className="menu-note">
                  {selectedCommit.author} · {shortDate(selectedCommit.date)} · parents{" "}
                  {selectedCommit.parents.length === 0
                    ? "none (root commit)"
                    : selectedCommit.parents.map((parent) => parent.slice(0, 7)).join(", ")}
                </p>
                <ul className="git-file-list" data-testid="git-log-files">
                  {selectedCommit.files.map((file) => (
                    <li key={`${file.status}-${file.path}`}>
                      <span className="tag">{file.status}</span>
                      <code>{file.path}</code>
                      {file.originalPath && <span className="menu-note">from {file.originalPath}</span>}
                    </li>
                  ))}
                </ul>
                {selectedCommit.files.length === 0 && (
                  <p className="menu-note" data-testid="git-log-files-empty">
                    No changed files reported for this commit (merge commits are not diffed).
                  </p>
                )}
                {selectedCommit.filesTruncated && (
                  <p className="menu-note">Files truncated at the declared limit.</p>
                )}
              </>
            ) : (
              <p className="menu-note">Select a commit to list its changed files.</p>
            )}
          </section>
        </div>
      )}
    </div>
  );
}
