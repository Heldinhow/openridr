import { useEffect, useMemo, useRef, useState } from "react";
import { call, HostCallError } from "./host/client";
import { Icon } from "./Icon";

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

function messageOf(error: unknown) {
  return error instanceof HostCallError ? error.message : String(error);
}

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
    return `A ${status.operation} is in progress; finish or abort it before checking out or updating.`;
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

export type GitIntent = { kind: "push" | "update" | "branchCreate"; nonce: number };

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
  const [dialog, setDialog] = useState<"update" | "push" | null>(null);
  const [dialogError, setDialogError] = useState<string | null>(null);
  const [dialogBusy, setDialogBusy] = useState(false);
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [menuOpen]);

  useEffect(() => {
    if (!intent) return;
    setMenuOpen(true);
    setError(null);
    setDialogError(null);
    const blocked = status ? cleanReason(status) : null;
    if (intent.kind === "update") {
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [intent?.nonce]);

  if (!trusted) {
    return (
      <span className="toolbar-disabled" title={TRUST_MESSAGE}>
        <button type="button" data-testid="toolbar-branch" disabled aria-label={TRUST_MESSAGE} title={TRUST_MESSAGE}>
          <Icon name="git" /> {TRUST_MESSAGE}
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
          <Icon name="git" /> {text}
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
          <span data-testid="branch-current">{branchName(status)}</span>
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
          <div className="modal" role="dialog" aria-modal="true" aria-labelledby="update-title" data-testid="update-dialog">
            <h2 id="update-title">Update project</h2>
            <dl>
              <dt>Branch</dt>
              <dd>{branchName(status)}</dd>
              <dt>Upstream</dt>
              <dd>{status.upstream ?? "Not set"}</dd>
              <dt>Behind</dt>
              <dd>{status.behind}</dd>
            </dl>
            <p className="hint">
              Runs <code>git pull</code>. A dirty tree or an in-progress operation refuses the
              update; OpenRidr never stashes.
            </p>
            {dialogError && (
              <p className="error" role="alert" data-testid="update-error">{dialogError}</p>
            )}
            <div className="actions">
              <button type="button" data-testid="update-confirm" disabled={dialogBusy} onClick={() => void confirmUpdate()}>
                {dialogBusy ? "Updating…" : "Update"}
              </button>
              <button type="button" data-testid="update-cancel" onClick={() => setDialog(null)}>Cancel</button>
            </div>
          </div>
        </div>
      )}

      {dialog === "push" && (
        <div className="modal-backdrop">
          <div className="modal" role="dialog" aria-modal="true" aria-labelledby="push-title" data-testid="push-dialog">
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
              <button type="button" data-testid="push-confirm" disabled={dialogBusy || remote === null} onClick={() => void confirmPush()}>
                {dialogBusy ? "Pushing…" : "Push"}
              </button>
              <button type="button" data-testid="push-cancel" onClick={() => setDialog(null)}>Cancel</button>
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
  onMessage,
  onError,
}: {
  status: GitStatus | null;
  statusError: string | null;
  trusted: boolean;
  initialPath?: string | null;
  onRefresh: () => Promise<void>;
  onMessage: (text: string) => void;
  onError: (text: string) => void;
}) {
  const [selection, setSelection] = useState<Set<string> | null>(null);
  const [message, setMessage] = useState("");
  const [amend, setAmend] = useState(false);
  const [diff, setDiff] = useState<GitDiff | null>(null);
  const [diffPath, setDiffPath] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [state, setState] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const focused = useRef<string | null>(null);
  const reason = !trusted ? TRUST_MESSAGE : unavailable(status, statusError);
  const files = status?.state === "ok" ? status.files : [];
  const selected = useMemo(
    () => selection ?? new Set(files.filter((file) => !file.conflicted).map((file) => file.path)),
    [selection, files],
  );
  const commitPaths = files
    .filter((file) => selected.has(file.path) && !file.conflicted)
    .map((file) => file.path);

  useEffect(() => {
    if (!trusted) return;
    void onRefresh();
    // Refresh once when the panel opens; operations refresh explicitly.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function toggle(path: string) {
    const next = new Set(selected);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setSelection(next);
  }

  async function showDiff(file: GitFile) {
    setDiffPath(file.path);
    setDiff(null);
    try {
      const staged = file.staged && file.worktree === " ";
      setDiff(await call<GitDiff>("git.diff", { path: file.path, staged }));
    } catch (caught) {
      const text = messageOf(caught);
      setError(text);
      setDiff({
        state: "failed",
        message: text,
        path: file.path,
        staged: file.staged,
        diff: "",
        truncated: false,
      });
    }
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
        onMessage(result.message ?? "Commit created.");
        setMessage("");
        setAmend(false);
        setSelection(null);
        setDiff(null);
        setDiffPath(null);
      } else {
        setError(result.message ?? "git commit failed.");
      }
      await onRefresh();
    } catch (caught) {
      setError(messageOf(caught));
      onError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <nav className="side commit-tool" aria-label="Commit" data-testid="commit-tool">
      <div className="panel-title">
        Commit <span className="tag">{status?.state === "ok" ? branchName(status) : "Git"}</span>
      </div>
      {reason ? (
        <p className="tree-note" data-testid="commit-disabled-reason">{reason}</p>
      ) : (
        <>
          <div className="git-toolbar">
            <button type="button" data-testid="commit-refresh" disabled={busy} onClick={() => void onRefresh()}>
              Refresh
            </button>
            <span className="tag" data-testid="commit-count">
              {commitPaths.length} of {files.length} selected
            </span>
          </div>
          <div className="commit-files">
            {files.length === 0 && (
              <p className="tree-note" data-testid="commit-clean">
                The working tree is clean.
              </p>
            )}
            {files.map((file) => (
              <div className={`commit-row ${diffPath === file.path ? "selected" : ""}`} key={`${file.path}-${file.originalPath ?? ""}`}>
                <input
                  type="checkbox"
                  data-testid={`commit-check-${file.path}`}
                  aria-label={`Commit ${file.path}`}
                  checked={selected.has(file.path)}
                  disabled={busy || file.conflicted}
                  title={file.conflicted ? "Conflicted files cannot be committed; resolve the conflict first." : file.label}
                  onChange={() => toggle(file.path)}
                />
                <button
                  type="button"
                  className="commit-file"
                  data-testid={`commit-file-${file.path}`}
                  title={file.conflicted ? "Conflicted files cannot be committed; resolve the conflict first." : file.path}
                  onClick={() => void showDiff(file)}
                >
                  <span className="commit-path">{file.path}</span>
                  <span className={`tag ${file.conflicted ? "error" : ""}`}>{file.label}</span>
                  {file.untracked && <span className="menu-note">new file</span>}
                </button>
              </div>
            ))}
          </div>
          {diffPath && (
            <div className="commit-diff-wrap" data-testid="commit-diff">
              <div className="panel-title">{diffPath}</div>
              {diff ? (
                <>
                  <pre className="commit-diff">{diff.diff || diff.message || "No diff."}</pre>
                  {diff.truncated && <p className="menu-note">Diff truncated at the declared limit.</p>}
                  {diff.state !== "ok" && <p className="menu-note">{diff.message}</p>}
                </>
              ) : (
                <p className="menu-note">Loading diff…</p>
              )}
            </div>
          )}
          <div className="commit-form">
            <label className="commit-message-field">
              <span>Commit message</span>
              <textarea
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
              disabled={busy || message.trim() === "" || commitPaths.length === 0}
              onClick={() => void submit()}
            >
              {busy ? "Committing…" : amend ? "Amend commit" : "Commit"}
            </button>
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
            {log.commits.map((commit) => (
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
                  <span className="tag">{commit.parents.length === 0 ? "root" : `${commit.parents.length} parent`}</span>
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
