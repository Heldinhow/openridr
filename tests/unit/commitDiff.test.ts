import { describe, expect, test } from "bun:test";
import { parseCommitPatch } from "../../src/CommitDiff";

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
