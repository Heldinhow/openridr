import { useEffect, useMemo, useState } from "react";
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp } from "lucide-react";
import { call, messageOf } from "./host/client";
import type { GitCommitDiff, GitCommitFile, GitFile, GitFileDiff } from "./git";
import { buildCommitRows, fullFileReason, loadDiffMode, parseCommitPatch, saveDiffMode, type DiffMode, type DiffRows } from "./commitDiffRows";
import { SplitDiff } from "./SplitDiff";
import { UnifiedDiff } from "./UnifiedDiff";

const FULL_REASONS: Record<string, string> = {
  binary: "full file not available: binary file.",
  too_large: "full file not available: the file is too large (limit 1 MB or 30,000 lines).",
  unsupported_encoding: "full file not available: not text the editor can show.",
  unreadable: "full file not available: the file could not be read.",
  unaligned: "full file not available: the changed blocks could not be aligned.",
};

/** What a commit diff tab knows about the commit it shows. */
export type CommitDiffTarget = {
  hash: string;
  shortHash: string;
  subject: string;
  originalPath: string | null;
};

type WorktreeDiffProps = { path: string; files: GitFile[]; onNavigate: (path: string) => void };
type CommitFileDiffProps = { path: string; files: GitCommitFile[]; onNavigate: (path: string) => void; target: CommitDiffTarget };

/** One panel for both diffs: the working tree's (index/HEAD against the worktree) and a commit's
 * (the commit's blobs against its first parent). Only the read and the labels differ. */
export function CommitDiff(props: WorktreeDiffProps | CommitFileDiffProps) {
  const { path, files, onNavigate } = props;
  const worktree = "target" in props ? null : props;
  const target = "target" in props ? props.target : null;
  const [diff, setDiff] = useState<GitFileDiff | GitCommitDiff | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [difference, setDifference] = useState(0);
  const [mode, setMode] = useState<DiffMode>(loadDiffMode);
  const file = worktree?.files.find((entry) => entry.path === path);
  const staged = !!file?.staged && file.worktree === " ";
  const index = files.findIndex((entry) => entry.path === path);
  useEffect(() => {
    let live = true;
    setDiff(null); setError(null); setDifference(0);
    const request = target
      ? call<GitCommitDiff>("git.commitDiff", { hash: target.hash, path, originalPath: target.originalPath })
      : call<GitFileDiff>("git.fileDiff", { path, staged });
    request.then((result) => { if (live) setDiff(result); }).catch((caught) => { if (live) setError(messageOf(caught)); });
    return () => { live = false; };
  }, [path, staged, file?.index, file?.worktree, target?.hash, target?.originalPath]);
  const built = useMemo(
    () => (diff?.full ? buildCommitRows(diff.base.text, diff.current.text, diff.hunks) : null),
    [diff],
  );
  const parsed = useMemo<DiffRows>(() => (diff ? built ?? parseCommitPatch(diff.patch) : { rows: [], differences: 0 }), [diff, built]);
  const fullFile = built !== null;
  const parentShort = diff && "parent" in diff ? diff.parent?.shortHash ?? null : null;
  const leftLabel = !diff ? "" : !diff.base.exists ? "No previous file" : target ? `parent${parentShort ? ` · ${parentShort}` : ""}` : staged ? `HEAD${diff.commit ? ` · ${diff.commit.shortHash}` : ""}` : "Index";
  const rightLabel = !diff ? "" : !diff.current.exists ? "No current file" : target ? diff.commit?.shortHash ?? "" : staged ? "Index" : "Worktree";
  const reasonKey = fullFile
    ? null
    : fullFileReason(!!diff?.full, built !== null, diff?.fullReason ?? null);
  const reason = FULL_REASONS[reasonKey ?? ""] ?? "";
  const note = !diff || !parsed.rows.length
    ? null
    : fullFile
      ? "Showing the full file from Git."
      : `Showing changed hunks and context from Git${diff.truncated ? " · patch truncated by host" : ""}.${reason ? ` ${reason}` : ""}`;
  function chooseMode(next: DiffMode) {
    setMode(next);
    saveDiffMode(next);
  }
  return <section
    className="git-diff-view"
    data-testid="commit-diff"
    aria-label={target ? `Commit diff for ${path} at ${target.shortHash}` : `Git diff for ${path}`}
    role="tabpanel"
    id={target ? "commit-file-diff-content" : "commit-diff-content"}
  >
    <div className="git-diff-toolbar">
      <button aria-label="Previous difference" title="Previous difference" disabled={difference <= 0} onClick={() => setDifference(difference - 1)}><ArrowUp size={16} /></button>
      <button aria-label="Next difference" title="Next difference" disabled={difference >= parsed.differences - 1} onClick={() => setDifference(difference + 1)}><ArrowDown size={16} /></button>
      <span className="git-diff-separator" />
      <button aria-label="Previous changed file" disabled={index <= 0} onClick={() => onNavigate(files[index - 1].path)}><ArrowLeft size={16} /></button>
      <span data-testid="commit-diff-file-count">{index < 0 ? 0 : index + 1}/{files.length} files</span>
      <button aria-label="Next changed file" disabled={index < 0 || index >= files.length - 1} onClick={() => onNavigate(files[index + 1].path)}><ArrowRight size={16} /></button>
      <span className="git-diff-mode" role="group" aria-label="Diff view mode" data-testid="commit-diff-mode">
        <button type="button" aria-pressed={mode === "split"} className={mode === "split" ? "active" : ""} onClick={() => chooseMode("split")}>Split</button>
        <button type="button" aria-pressed={mode === "unified"} className={mode === "unified" ? "active" : ""} onClick={() => chooseMode("unified")}>Unified</button>
      </span>
      <span className="git-diff-count" data-testid="commit-diff-count">{parsed.differences} {parsed.differences === 1 ? "difference" : "differences"}</span>
    </div>
    <div className="git-diff-labels">
      <span data-testid="commit-diff-left-label">{leftLabel}</span>
      <span data-testid="commit-diff-right-label">{rightLabel}</span>
    </div>
    {diff?.commit && <p className="git-diff-commit" data-testid="commit-diff-commit" title={diff.commit.hash}>{target ? <><code>{diff.commit.shortHash}</code> · {diff.commit.subject}</> : <>HEAD <code>{diff.commit.shortHash}</code> · {diff.commit.subject}</>}</p>}
    {!diff && !error && <p className="tree-note">Loading diff…</p>}
    {error && <p className="error" role="alert">{error}</p>}
    {diff && (parsed.rows.length
      ? <>{note && <p className="git-diff-note" data-testid="commit-diff-note">{note}</p>}{mode === "split" ? <SplitDiff rows={parsed.rows} difference={difference} /> : <UnifiedDiff rows={parsed.rows} difference={difference} />}</>
      : <pre className="git-diff-note" data-testid="commit-diff-note">{[diff.message || diff.patch.trim() || "No diff.", reason].filter(Boolean).join("\n")}</pre>)}
  </section>;
}
