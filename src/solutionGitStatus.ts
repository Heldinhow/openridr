// Pure mapping from the Git status snapshot to the Solution Explorer's per-row state.
// Paths are compared as workspace-relative strings; the repository root comes from the snapshot,
// so a solution opened in a subfolder of the repository keeps its rows. Colors live in styles.css
// (`--git-*`); this module only names the state, never a hex value.
import type { GitFile, GitStatus } from "./git";

/** The states a tree file row can show; `deleted` never applies (the file is not on disk). */
export type TreeGitStatus = "modified" | "added" | "untracked" | "ignored" | "conflicted";

export type TreeGitEntry = {
  status: TreeGitStatus;
  /** The commit tool's granular label (`Staged + modified`, `Renamed`…), kept for the title. */
  label: string;
};

const normalize = (path: string) => path.replace(/\\/g, "/");

/** Absolute path of a `git.status` file, whose `path` is relative to the repository root. */
function repoAbsolute(root: string, path: string): string {
  const base = normalize(root).replace(/\/+$/, "");
  const relative = normalize(path).replace(/^\/+/, "");
  return base ? `${base}/${relative}` : `/${relative}`;
}

/**
 * Workspace-relative path for a repository path, or `null` when it falls outside the workspace
 * (changes in other projects of the same repository never reach this tree).
 */
export function workspacePathOf(
  root: string,
  path: string,
  workspaceDir: string,
): string | null {
  const absolute = repoAbsolute(root, path);
  const base = normalize(workspaceDir).replace(/\/+$/, "");
  if (!base) return absolute.startsWith("/") ? absolute.slice(1) : absolute;
  if (absolute === base) return "";
  return absolute.startsWith(`${base}/`) ? absolute.slice(base.length + 1) : null;
}

/**
 * One state per file, with the precedence the spec fixes: a conflict wins over everything, an
 * untracked file is not "modified", and an `A` in the index is "added" even when it also has
 * worktree changes. `null` means the snapshot says nothing about this path.
 */
export function treeStatusOf(file: GitFile): TreeGitStatus | null {
  if (file.conflicted) return "conflicted";
  if (file.untracked) return "untracked";
  if (file.index === "A" || file.worktree === "A") return "added";
  if (file.index !== " " || file.worktree !== " ") return "modified";
  return null;
}

/** Snapshot → one entry per workspace path; a snapshot without a repository root yields none. */
export function treeGitStatusMap(
  status: GitStatus | null,
  workspaceDir: string,
): Map<string, TreeGitEntry> {
  const map = new Map<string, TreeGitEntry>();
  if (!status || status.state !== "ok" || !status.root) return map;
  for (const file of status.files) {
    const path = workspacePathOf(status.root, file.path, workspaceDir);
    if (!path) continue;
    const tree = treeStatusOf(file);
    if (!tree) continue;
    map.set(path, { status: tree, label: file.label });
  }
  return map;
}

/**
 * Adds `ignored` for the paths Git's ignore rules matched. A path already in the snapshot is never
 * overwritten: a tracked file is not ignored, and the porcelain already named its real state.
 */
export function mergeIgnored(
  map: Map<string, TreeGitEntry>,
  ignored: Iterable<string>,
): Map<string, TreeGitEntry> {
  const next = new Map(map);
  for (const path of ignored) {
    if (next.has(path)) continue;
    next.set(path, { status: "ignored", label: "Ignored" });
  }
  return next;
}
