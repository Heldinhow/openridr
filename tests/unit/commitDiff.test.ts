import { describe, expect, test } from "bun:test";
import { buildCommitRows, parseCommitPatch } from "../../src/CommitDiff";

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

  test("handles empty files and refuses ranges that do not fit the texts", () => {
    expect(buildCommitRows("", "", [])).toEqual({ rows: [], differences: 0 });
    expect(buildCommitRows(null, "", [])).toEqual({ rows: [], differences: 0 });
    expect(buildCommitRows("a\nb\n", "a\nc\n", [{ oldStart: 9, oldCount: 1, newStart: 9, newCount: 1 }])).toBeNull();
    expect(buildCommitRows("a\nb\n", "a\nc\n", [{ oldStart: 1, oldCount: 2, newStart: 1, newCount: 9 }])).toBeNull();
    expect(buildCommitRows("a\nb\n", "a\nc\nd\n", [{ oldStart: 2, oldCount: 1, newStart: 2, newCount: 1 }])).toBeNull();
  });
});
