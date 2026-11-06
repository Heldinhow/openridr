import { useEffect, useMemo, useRef, useState } from "react";
import { EditorState, RangeSet } from "@codemirror/state";
import { Decoration, EditorView, GutterMarker, gutter, lineNumbers } from "@codemirror/view";
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp } from "lucide-react";
import { call, messageOf } from "./host/client";
import type { GitFile, GitFileDiff, GitHunk } from "./git";

type Cell = { text: string; number: number | null; kind: "context" | "modified" | "added" | "removed" | "empty"; mark?: [number, number] };
export type DiffRow = { left: Cell; right: Cell; difference: number | null };
export type DiffRows = { rows: DiffRow[]; differences: number };
const blank = (): Cell => ({ text: "", number: null, kind: "empty" });

function cell(text: string, number: number | null, kind: Cell["kind"]): Cell {
  return { text, number, kind };
}

// Pairs a changed block: the same index on both sides is one modified line (with the changed
// character range marked); the longer side keeps its extra lines alone.
function pairCells(removed: Cell[], added: Cell[], difference: number): DiffRow[] {
  const rows: DiffRow[] = [];
  for (let i = 0; i < Math.max(removed.length, added.length); i++) {
    const left = removed[i] ?? blank(), right = added[i] ?? blank();
    if (removed[i] && added[i]) {
      left.kind = right.kind = "modified";
      let start = 0, end = 0;
      while (start < left.text.length && start < right.text.length && left.text[start] === right.text[start]) start++;
      while (end < left.text.length - start && end < right.text.length - start && left.text[left.text.length - end - 1] === right.text[right.text.length - end - 1]) end++;
      left.mark = [start, left.text.length - end]; right.mark = [start, right.text.length - end];
    }
    rows.push({ left, right, difference });
  }
  return rows;
}

// Git returns a bounded unified patch, not full file contents. Only render lines in its hunks.
export function parseCommitPatch(patch: string): DiffRows {
  const rows: DiffRow[] = [];
  let oldLine = 0, newLine = 0, differences = 0, inHunk = false;
  let removed: Cell[] = [], added: Cell[] = [];
  const flush = () => {
    if (!removed.length && !added.length) return;
    rows.push(...pairCells(removed, added, differences++));
    removed = []; added = [];
  };
  for (const line of patch.split("\n")) {
    const hunk = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
    if (hunk) { flush(); oldLine = Number(hunk[1]); newLine = Number(hunk[2]); inHunk = true; continue; }
    if (!inHunk) continue;
    if (line.startsWith("-")) removed.push(cell(line.slice(1), oldLine++, "removed"));
    else if (line.startsWith("+")) added.push(cell(line.slice(1), newLine++, "added"));
    else if (line.startsWith(" ")) { flush(); const text = line.slice(1); rows.push({ left: cell(text, oldLine++, "context"), right: cell(text, newLine++, "context"), difference: null }); }
    else if (line.startsWith("\\")) continue;
    else { flush(); inHunk = false; }
  }
  flush();
  return { rows, differences };
}

/** One line per element, so a trailing newline does not invent an empty last line. */
function linesOf(text: string | null): string[] {
  if (text === null || text === "") return [];
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  return lines;
}

function contextRow(left: string, right: string, oldLine: number, newLine: number): DiffRow {
  return { left: cell(left, oldLine, "context"), right: cell(right, newLine, "context"), difference: null };
}

/**
 * Expands the changed block ranges against the two full texts. Git already decided what changed
 * (`-U0` hunks), so the lines between blocks are unchanged context read straight from the files;
 * any range that does not fit the texts returns null and the panel keeps the patch view.
 */
export function buildCommitRows(base: string | null, current: string | null, hunks: GitHunk[]): DiffRows | null {
  if (base === null && current === null) return null;
  const baseLines = linesOf(base), currentLines = linesOf(current);
  const rows: DiffRow[] = [];
  if (base === null) {
    currentLines.forEach((text, index) => rows.push({ left: blank(), right: cell(text, index + 1, "added"), difference: 0 }));
    return { rows, differences: rows.length ? 1 : 0 };
  }
  if (current === null) {
    baseLines.forEach((text, index) => rows.push({ left: cell(text, index + 1, "removed"), right: blank(), difference: 0 }));
    return { rows, differences: rows.length ? 1 : 0 };
  }
  let oldLine = 1, newLine = 1;
  for (let index = 0; index < hunks.length; index++) {
    const hunk = hunks[index];
    // A zero count means "inserted after this line": `0` is before the first line.
    const gapOld = (hunk.oldCount > 0 ? hunk.oldStart : hunk.oldStart + 1) - oldLine;
    const gapNew = (hunk.newCount > 0 ? hunk.newStart : hunk.newStart + 1) - newLine;
    if (gapOld < 0 || gapNew < 0 || gapOld !== gapNew) return null;
    for (let i = 0; i < gapOld; i++, oldLine++, newLine++) {
      const left = baseLines[oldLine - 1], right = currentLines[newLine - 1];
      if (left === undefined || right === undefined) return null;
      rows.push(contextRow(left, right, oldLine, newLine));
    }
    const removed: Cell[] = [], added: Cell[] = [];
    for (let i = 0; i < hunk.oldCount; i++, oldLine++) {
      const text = baseLines[oldLine - 1];
      if (text === undefined) return null;
      removed.push(cell(text, oldLine, "removed"));
    }
    for (let i = 0; i < hunk.newCount; i++, newLine++) {
      const text = currentLines[newLine - 1];
      if (text === undefined) return null;
      added.push(cell(text, newLine, "added"));
    }
    rows.push(...pairCells(removed, added, index));
  }
  for (;;) {
    const left = baseLines[oldLine - 1], right = currentLines[newLine - 1];
    if (left === undefined && right === undefined) break;
    if (left === undefined || right === undefined) return null;
    rows.push(contextRow(left, right, oldLine, newLine));
    oldLine++; newLine++;
  }
  return { rows, differences: hunks.length };
}

// One marker per changed line, on the right edge of the base pane: half a bar for a one-sided
// line, the full bar for a modified pair. The gutter aligns by construction, no pixel math.
class ConnectorMarker extends GutterMarker {
  constructor(private readonly kind: "removed" | "added" | "modified") { super(); }
  eq(other: ConnectorMarker) { return other.kind === this.kind; }
  toDOM() {
    const bar = document.createElement("div");
    bar.className = `cm-diff-connector cm-diff-connector-${this.kind}`;
    return bar;
  }
}

function connectorGutter(rows: DiffRow[]) {
  return gutter({
    class: "cm-diff-connectors",
    side: "after",
    markers: (view) => {
      const markers: ReturnType<ConnectorMarker["range"]>[] = [];
      rows.forEach((row, index) => {
        if (index >= view.state.doc.lines) return;
        const kind = row.left.kind === "modified" && row.right.kind === "modified" ? "modified"
          : row.left.kind === "removed" ? "removed"
          : row.right.kind === "added" ? "added" : null;
        if (!kind) return;
        markers.push(new ConnectorMarker(kind).range(view.state.doc.line(index + 1).from));
      });
      return RangeSet.of(markers, true);
    },
  });
}

const editorTheme = EditorView.theme({
  "&": { height: "100%" },
  ".cm-scroller": { overflow: "auto", fontFamily: "var(--font-mono, monospace)" },
});

function SplitDiff({ rows, difference }: { rows: DiffRow[]; difference: number }) {
  const left = useRef<HTMLDivElement>(null), right = useRef<HTMLDivElement>(null);
  const views = useRef<EditorView[]>([]);
  useEffect(() => {
    const parents = [left.current, right.current];
    const instances = parents.map((parent, side) => {
      const cells = rows.map((row) => side === 0 ? row.left : row.right);
      const doc = cells.map((cell) => cell.text).join("\n");
      const ranges: ReturnType<ReturnType<typeof Decoration.line>["range"]>[] = [];
      let pos = 0;
      for (const cell of cells) {
        if (cell.kind !== "context") ranges.push(Decoration.line({ class: `git-diff-line-${cell.kind}` }).range(pos));
        if (cell.mark && cell.mark[0] < cell.mark[1]) ranges.push(Decoration.mark({ class: "git-diff-intraline" }).range(pos + cell.mark[0], pos + cell.mark[1]));
        pos += cell.text.length + 1;
      }
      return new EditorView({ parent: parent!, state: EditorState.create({ doc, extensions: [
        EditorState.readOnly.of(true), EditorView.editable.of(false),
        lineNumbers({ formatNumber: (number) => String(cells[number - 1]?.number ?? "") }),
        EditorView.decorations.of(Decoration.set(ranges, true)),
        editorTheme,
        ...(side === 0 ? [connectorGutter(rows)] : []),
      ] }) });
    });
    views.current = instances;
    let syncing = false;
    const handlers = instances.map((view, i) => () => {
      if (syncing) return;
      syncing = true;
      instances[1 - i].scrollDOM.scrollTop = view.scrollDOM.scrollTop;
      syncing = false;
    });
    instances.forEach((view, i) => view.scrollDOM.addEventListener("scroll", handlers[i]));
    return () => { instances.forEach((view, i) => { view.scrollDOM.removeEventListener("scroll", handlers[i]); view.destroy(); }); views.current = []; };
  }, [rows]);
  useEffect(() => {
    const index = rows.findIndex((row) => row.difference === difference);
    if (index < 0) return;
    for (const view of views.current) {
      const line = view.state.doc.line(Math.min(index + 1, view.state.doc.lines));
      view.dispatch({ effects: EditorView.scrollIntoView(line.from, { y: "center" }) });
    }
  }, [rows, difference]);
  return <div className="git-diff-editors"><div ref={left} aria-label="Base diff editor" /><div ref={right} aria-label="Current diff editor" /></div>;
}

const FULL_REASONS: Record<string, string> = {
  binary: "full file not available: binary file.",
  too_large: "full file not available: the file is too large (limit 1 MB or 30,000 lines).",
  unsupported_encoding: "full file not available: not text the editor can show.",
  unreadable: "full file not available: the file could not be read.",
  unaligned: "full file not available: the changed blocks could not be aligned.",
};

export function CommitDiff({ path, files, onNavigate }: { path: string; files: GitFile[]; onNavigate: (path: string) => void }) {
  const [diff, setDiff] = useState<GitFileDiff | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [difference, setDifference] = useState(0);
  const file = files.find((entry) => entry.path === path);
  const staged = !!file?.staged && file.worktree === " ";
  const index = files.findIndex((entry) => entry.path === path);
  useEffect(() => {
    let live = true;
    setDiff(null); setError(null); setDifference(0);
    call<GitFileDiff>("git.fileDiff", { path, staged }).then((result) => { if (live) setDiff(result); }).catch((caught) => { if (live) setError(messageOf(caught)); });
    return () => { live = false; };
  }, [path, staged, file?.index, file?.worktree]);
  const built = useMemo(
    () => (diff?.full ? buildCommitRows(diff.base.text, diff.current.text, diff.hunks) : null),
    [diff],
  );
  const parsed = useMemo<DiffRows>(() => (diff ? built ?? parseCommitPatch(diff.patch) : { rows: [], differences: 0 }), [diff, built]);
  const fullFile = built !== null;
  const leftLabel = !diff ? "" : !diff.base.exists ? "No previous file" : staged ? `HEAD${diff.commit ? ` · ${diff.commit.shortHash}` : ""}` : "Index";
  const rightLabel = !diff ? "" : !diff.current.exists ? "No current file" : staged ? "Index" : "Worktree";
  const reason = FULL_REASONS[fullFile ? "" : diff?.fullReason ?? ""] ?? "";
  const note = !diff || !parsed.rows.length
    ? null
    : fullFile
      ? "Showing the full file from Git."
      : `Showing changed hunks and context from Git${diff.truncated ? " · patch truncated by host" : ""}.${reason ? ` ${reason}` : ""}`;
  return <section className="git-diff-view" data-testid="commit-diff" aria-label={`Git diff for ${path}`} role="tabpanel" id="commit-diff-content">
    <div className="git-diff-toolbar">
      <button aria-label="Previous difference" title="Previous difference" disabled={difference <= 0} onClick={() => setDifference(difference - 1)}><ArrowUp size={16} /></button>
      <button aria-label="Next difference" title="Next difference" disabled={difference >= parsed.differences - 1} onClick={() => setDifference(difference + 1)}><ArrowDown size={16} /></button>
      <span className="git-diff-separator" />
      <button aria-label="Previous changed file" disabled={index <= 0} onClick={() => onNavigate(files[index - 1].path)}><ArrowLeft size={16} /></button>
      <span data-testid="commit-diff-file-count">{index < 0 ? 0 : index + 1}/{files.length} files</span>
      <button aria-label="Next changed file" disabled={index < 0 || index >= files.length - 1} onClick={() => onNavigate(files[index + 1].path)}><ArrowRight size={16} /></button>
      <span className="git-diff-count" data-testid="commit-diff-count">{parsed.differences} {parsed.differences === 1 ? "difference" : "differences"}</span>
    </div>
    <div className="git-diff-labels">
      <span data-testid="commit-diff-left-label">{leftLabel}</span>
      <span data-testid="commit-diff-right-label">{rightLabel}</span>
    </div>
    {diff?.commit && <p className="git-diff-commit" data-testid="commit-diff-commit" title={diff.commit.hash}>HEAD <code>{diff.commit.shortHash}</code> · {diff.commit.subject}</p>}
    {!diff && !error && <p className="tree-note">Loading diff…</p>}
    {error && <p className="error" role="alert">{error}</p>}
    {diff && (parsed.rows.length
      ? <>{note && <p className="git-diff-note" data-testid="commit-diff-note">{note}</p>}<SplitDiff rows={parsed.rows} difference={difference} /></>
      : <pre className="git-diff-note" data-testid="commit-diff-note">{[diff.message || diff.patch.trim() || "No diff.", reason].filter(Boolean).join("\n")}</pre>)}
  </section>;
}
