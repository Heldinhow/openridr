import type { GitHunk } from "./git";

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
      // The lines between changed blocks are context by definition: if both texts disagree, the
      // ranges do not describe these files and the panel must fall back instead of guessing.
      if (left === undefined || right === undefined || left !== right) return null;
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
    if (left === undefined || right === undefined || left !== right) return null;
    rows.push(contextRow(left, right, oldLine, newLine));
    oldLine++; newLine++;
  }
  return { rows, differences: hunks.length };
}

export type UnifiedLine = { text: string; prefix: " " | "+" | "-"; number: number | null; kind: Cell["kind"]; difference: number | null; mark?: [number, number] };
export type UnifiedDoc = { lines: UnifiedLine[]; differenceLines: number[] };

/** Projects the paired rows onto one column: a paired modification becomes a removed line and
 * an added line; the gutter shows the new file's number, so removed lines have none. */
export function projectUnified(rows: DiffRow[]): UnifiedDoc {
  const lines: UnifiedLine[] = [];
  for (const row of rows) {
    const { left, right } = row;
    if (left.kind === "modified" && right.kind === "modified") {
      lines.push({ text: left.text, prefix: "-", number: null, kind: "removed", difference: row.difference, mark: left.mark });
      lines.push({ text: right.text, prefix: "+", number: right.number, kind: "added", difference: row.difference, mark: right.mark });
    } else if (left.kind === "removed") {
      lines.push({ text: left.text, prefix: "-", number: null, kind: "removed", difference: row.difference, mark: left.mark });
    } else if (right.kind === "added") {
      lines.push({ text: right.text, prefix: "+", number: right.number, kind: "added", difference: row.difference, mark: right.mark });
    } else if (left.kind !== "empty" || right.kind !== "empty") {
      const source = left.kind !== "empty" ? left : right;
      lines.push({ text: source.text, prefix: " ", number: right.number ?? left.number, kind: "context", difference: null });
    }
  }
  const differenceLines: number[] = [];
  lines.forEach((line, index) => {
    if (line.difference !== null && differenceLines[line.difference] === undefined) differenceLines[line.difference] = index;
  });
  return { lines, differenceLines };
}

export type DiffMode = "split" | "unified";
export const diffModeKey = "openridr.commitDiffMode.v1";

/** The explanation shown instead of the full file: a host `full` whose ranges the row builder
 * refused is `unaligned`, and that wins over a missing host reason. */
export function fullFileReason(full: boolean, aligned: boolean, hostReason: string | null): string | null {
  if (full && !aligned) return "unaligned";
  return hostReason;
}

export function loadDiffMode(): DiffMode {
  try {
    return globalThis.localStorage?.getItem(diffModeKey) === "unified" ? "unified" : "split";
  } catch {
    return "split";
  }
}

export function saveDiffMode(mode: DiffMode) {
  try {
    globalThis.localStorage?.setItem(diffModeKey, mode);
  } catch {
    // Private mode or a full quota only loses the choice; the diff keeps working.
  }
}
