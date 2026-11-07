import { describe, expect, test } from "bun:test";
import { buildCommitRows, fullFileReason, loadDiffMode, parseCommitPatch, projectUnified } from "../../src/commitDiffRows";

describe("commit patch projection", () => {
  test("pairs replacements, preserves real line numbers and highlights only changed characters", () => {
    const result = parseCommitPatch("diff --git a/x b/x\n@@ -10,3 +10,4 @@\n same\n-old value\n+new value\n+added\n last\n");
    expect(result.differences).toBe(1);
    expect(result.rows.map((row) => [row.left.number, row.right.number])).toEqual([[10, 10], [11, 11], [null, 12], [12, 13]]);
    expect(result.rows[1].left.mark).toEqual([0, 3]);
    expect(result.rows[1].right.mark).toEqual([0, 3]);
    expect(result.rows[2].right.kind).toBe("added");
  });
  test("keeps separate hunks and deletion-only changes", () => {
    const result = parseCommitPatch("@@ -2,2 +2,1 @@\n-old\n context\n@@ -20,1 +19,0 @@\n-gone\n");
    expect(result.differences).toBe(2);
    expect(result.rows.map((row) => row.left.number)).toEqual([2, 3, 20]);
    expect(result.rows[2].right.number).toBe(null);
  });
});

describe("full-file rows", () => {
  const base = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n";

  test("shows every line of a large file around a change in the middle", () => {
    const result = buildCommitRows(base, base.replace("six", "SIX"), [{ oldStart: 6, oldCount: 1, newStart: 6, newCount: 1 }]);
    expect(result).not.toBeNull();
    expect(result!.rows.map((row) => row.left.number)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    expect(result!.rows.map((row) => row.right.number)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    expect(result!.rows[0].left.kind).toBe("context");
    expect(result!.rows[5].left.kind).toBe("modified");
    expect(result!.rows[5].left.mark).toEqual([0, 3]);
    expect(result!.rows[5].right.mark).toEqual([0, 3]);
    expect(result!.rows[9].right.text).toBe("ten");
    expect(result!.differences).toBe(1);
  });

  test("keeps context between two hunks and names each difference", () => {
    const current = base.replace("two", "TWO").replace("five", "FIVE");
    const result = buildCommitRows(base, current, [
      { oldStart: 2, oldCount: 1, newStart: 2, newCount: 1 },
      { oldStart: 5, oldCount: 1, newStart: 5, newCount: 1 },
    ]);
    expect(result!.rows).toHaveLength(10);
    expect(result!.rows.map((row) => row.difference)).toEqual([null, 0, null, null, 1, null, null, null, null, null]);
    expect(result!.rows[2].left.text).toBe("three");
    expect(result!.differences).toBe(2);
  });

  test("renders an added file as added lines with no previous side", () => {
    const result = buildCommitRows(null, "first\nsecond\n", []);
    expect(result!.rows.map((row) => [row.left.kind, row.right.kind, row.right.number])).toEqual([
      ["empty", "added", 1],
      ["empty", "added", 2],
    ]);
    expect(result!.differences).toBe(1);
  });

  test("renders a deleted file as removed lines with no current side", () => {
    const result = buildCommitRows("first\nsecond\n", null, []);
    expect(result!.rows.map((row) => [row.left.kind, row.right.kind, row.left.number])).toEqual([
      ["removed", "empty", 1],
      ["removed", "empty", 2],
    ]);
    expect(result!.differences).toBe(1);
  });

  test("handles zero-count hunks at the top and at the end of the file", () => {
    const top = buildCommitRows("b\nc\n", "a\nb\nc\n", [{ oldStart: 0, oldCount: 0, newStart: 1, newCount: 1 }]);
    expect(top!.rows.map((row) => [row.left.kind, row.right.kind, row.right.text])).toEqual([
      ["empty", "added", "a"],
      ["context", "context", "b"],
      ["context", "context", "c"],
    ]);
    const end = buildCommitRows("a\n", "a\nb\n", [{ oldStart: 1, oldCount: 0, newStart: 2, newCount: 1 }]);
    expect(end!.rows.map((row) => [row.left.kind, row.right.kind, row.right.text])).toEqual([
      ["context", "context", "a"],
      ["empty", "added", "b"],
    ]);
  });

  test("refuses context lines whose two texts disagree", () => {
    // before the first hunk
    expect(buildCommitRows("alpha\nb\nc\n", "ALPHA\nb\nX\n", [{ oldStart: 3, oldCount: 1, newStart: 3, newCount: 1 }])).toBeNull();
    // between two hunks
    expect(buildCommitRows("a\nb\nc\nd\ne\n", "a\nB\ncd\nC\ne\n", [
      { oldStart: 2, oldCount: 1, newStart: 2, newCount: 1 },
      { oldStart: 4, oldCount: 1, newStart: 4, newCount: 1 },
    ])).toBeNull();
    // after the last hunk
    expect(buildCommitRows("a\nb\nc\n", "a\nB\ngone\n", [{ oldStart: 2, oldCount: 1, newStart: 2, newCount: 1 }])).toBeNull();
    // no hunks at all, yet the files disagree
    expect(buildCommitRows("a\n", "b\n", [])).toBeNull();
  });

  test("aligns a commit rename: the base is the original path's text and the current the new one", () => {
    // What git.commitDiff returns for a rename with an edit: base = parent:<original>, current = <commit>:<path>.
    const original = "one\ntwo\nthree\nfour\n";
    const renamed = original.replace("two", "TWO");
    const result = buildCommitRows(original, renamed, [{ oldStart: 2, oldCount: 1, newStart: 2, newCount: 1 }])!;
    expect(result.rows.map((row) => [row.left.text, row.right.text])).toEqual([
      ["one", "one"],
      ["two", "TWO"],
      ["three", "three"],
      ["four", "four"],
    ]);
    expect(result.rows[1].left.kind).toBe("modified");
    expect(result.differences).toBe(1);
  });

  test("keeps an unchanged file as context with no differences", () => {
    const result = buildCommitRows("a\nb\n", "a\nb\n", [])!;
    expect(result.differences).toBe(0);
    expect(result.rows.map((row) => row.left.text)).toEqual(["a", "b"]);
  });

  test("handles empty files and refuses ranges that do not fit the texts", () => {
    expect(buildCommitRows("", "", [])).toEqual({ rows: [], differences: 0 });
    expect(buildCommitRows(null, "", [])).toEqual({ rows: [], differences: 0 });
    expect(buildCommitRows("a\nb\n", "a\nc\n", [{ oldStart: 9, oldCount: 1, newStart: 9, newCount: 1 }])).toBeNull();
    expect(buildCommitRows("a\nb\n", "a\nc\n", [{ oldStart: 1, oldCount: 2, newStart: 1, newCount: 9 }])).toBeNull();
    expect(buildCommitRows("a\nb\n", "a\nc\nd\n", [{ oldStart: 2, oldCount: 1, newStart: 2, newCount: 1 }])).toBeNull();
  });
});

describe("unified projection", () => {
  test("splits a modified pair into removed and added lines with new-side numbers", () => {
    const rows = buildCommitRows("keep\nold\n", "keep\nnew\ntail\n", [
      { oldStart: 2, oldCount: 1, newStart: 2, newCount: 2 },
    ])!;
    const unified = projectUnified(rows.rows);
    expect(unified.lines.map((line) => `${line.prefix}${line.text}`)).toEqual([" keep", "-old", "+new", "+tail"]);
    expect(unified.lines.map((line) => line.number)).toEqual([1, null, 2, 3]);
    expect(unified.differenceLines).toEqual([1]);
  });

  test("maps every difference to its first document line", () => {
    const rows = buildCommitRows("a\nb\nc\n", "A\nb\nC\n", [
      { oldStart: 1, oldCount: 1, newStart: 1, newCount: 1 },
      { oldStart: 3, oldCount: 1, newStart: 3, newCount: 1 },
    ])!;
    const unified = projectUnified(rows.rows);
    expect(unified.lines.map((line) => line.prefix + line.text)).toEqual(["-a", "+A", " b", "-c", "+C"]);
    expect(unified.differenceLines).toEqual([0, 3]);
  });
});

describe("full-file fallback reason", () => {
  test("names unaligned when the host promised the full file but the rows did not fit", () => {
    expect(fullFileReason(true, false, null)).toBe("unaligned");
    expect(fullFileReason(true, false, "binary")).toBe("unaligned");
    expect(fullFileReason(false, false, "binary")).toBe("binary");
    expect(fullFileReason(false, false, null)).toBeNull();
    expect(fullFileReason(true, true, null)).toBeNull();
  });
});

describe("diff mode preference", () => {
  test("defaults to split and accepts only a stored unified value", () => {
    const store = new Map<string, string>();
    const original = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: { getItem: (key: string) => store.get(key) ?? null, setItem: (key: string, value: string) => store.set(key, value) },
    });
    try {
      expect(loadDiffMode()).toBe("split");
      store.set("openridr.commitDiffMode.v1", "unified");
      expect(loadDiffMode()).toBe("unified");
      store.set("openridr.commitDiffMode.v1", "sideways");
      expect(loadDiffMode()).toBe("split");
    } finally {
      if (original) Object.defineProperty(globalThis, "localStorage", original);
      else delete (globalThis as { localStorage?: unknown }).localStorage;
    }
  });
});
