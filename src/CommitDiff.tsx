import { useEffect, useMemo, useRef, useState } from "react";
import { EditorState } from "@codemirror/state";
import { Decoration, EditorView, lineNumbers } from "@codemirror/view";
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp } from "lucide-react";
import { call, messageOf } from "./host/client";
import type { GitDiff, GitFile } from "./git";

type Cell = { text: string; number: number | null; kind: "context" | "modified" | "added" | "removed" | "empty"; mark?: [number, number] };
type Pair = { left: Cell; right: Cell; difference: number | null };
const blank = (): Cell => ({ text: "", number: null, kind: "empty" });

// Git returns a bounded unified patch, not full file contents. Only render lines in its hunks.
export function parseCommitPatch(patch: string): { rows: Pair[]; differences: number } {
  const rows: Pair[] = [];
  let oldLine = 0, newLine = 0, differences = 0, inHunk = false;
  let removed: Cell[] = [], added: Cell[] = [];
  const flush = () => {
    if (!removed.length && !added.length) return;
    const difference = differences++;
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
    removed = []; added = [];
  };
  for (const line of patch.split("\n")) {
    const hunk = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
    if (hunk) { flush(); oldLine = Number(hunk[1]); newLine = Number(hunk[2]); inHunk = true; continue; }
    if (!inHunk) continue;
    if (line.startsWith("-")) removed.push({ text: line.slice(1), number: oldLine++, kind: "removed" });
    else if (line.startsWith("+")) added.push({ text: line.slice(1), number: newLine++, kind: "added" });
    else if (line.startsWith(" ")) { flush(); const text = line.slice(1); rows.push({ left: { text, number: oldLine++, kind: "context" }, right: { text, number: newLine++, kind: "context" }, difference: null }); }
    else if (line.startsWith("\\")) continue;
    else { flush(); inHunk = false; }
  }
  flush();
  return { rows, differences };
}

function DiffEditors({ rows, difference }: { rows: Pair[]; difference: number }) {
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
        EditorView.theme({ "&": { height: "100%" }, ".cm-scroller": { overflow: "auto", fontFamily: "var(--font-mono, monospace)" } }),
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

export function CommitDiff({ path, files, onNavigate }: { path: string; files: GitFile[]; onNavigate: (path: string) => void }) {
  const [diff, setDiff] = useState<GitDiff | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [difference, setDifference] = useState(0);
  const file = files.find((entry) => entry.path === path);
  const staged = !!file?.staged && file.worktree === " ";
  const index = files.findIndex((entry) => entry.path === path);
  useEffect(() => {
    let live = true;
    setDiff(null); setError(null); setDifference(0);
    call<GitDiff>("git.diff", { path, staged }).then((result) => { if (live) setDiff(result); }).catch((caught) => { if (live) setError(messageOf(caught)); });
    return () => { live = false; };
  }, [path, staged, file?.index, file?.worktree]);
  const parsed = useMemo(() => parseCommitPatch(diff?.diff ?? ""), [diff]);
  const hash = /^index ([0-9a-f]+)\.\./m.exec(diff?.diff ?? "")?.[1];
  const noPrevious = diff?.diff.includes("--- /dev/null");
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
    <div className="git-diff-labels"><span data-testid="commit-diff-left-label">{noPrevious ? "No previous file" : staged ? `HEAD${hash ? ` · ${hash}` : ""}` : `Index${hash ? ` · ${hash}` : ""}`}</span><span data-testid="commit-diff-right-label">{staged ? "Index (staged version)" : "Current version"}</span></div>
    {!diff && !error && <p className="tree-note">Loading diff…</p>}
    {error && <p className="error" role="alert">{error}</p>}
    {diff && (parsed.rows.length ? <><p className="git-diff-note">Showing changed hunks and context from Git{diff.truncated ? " · patch truncated by host" : ""}</p><DiffEditors rows={parsed.rows} difference={difference} /></> : <pre className="git-diff-note">{diff.message || diff.diff || "No diff."}</pre>)}
  </section>;
}
