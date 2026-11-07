import { describe, expect, test } from "bun:test";
import { branchTriggerLabel } from "../../src/git";
import type { GitStatus } from "../../src/git";

function status(overrides: Partial<GitStatus> = {}): GitStatus {
  return {
    state: "ok",
    message: null,
    root: "/repo",
    branch: "main",
    detached: false,
    initial: false,
    upstream: null,
    upstreamGone: false,
    ahead: 0,
    behind: 0,
    operation: null,
    clean: true,
    files: [],
    remotes: [],
    ...overrides,
  };
}

const mergeText = "merge in progress — complete or abort it here";

describe("branchTriggerLabel", () => {
  test("names the ref and says there is no upstream", () => {
    expect(branchTriggerLabel(status())).toBe("Current branch: main · no upstream");
  });

  test("names the upstream and the ahead/behind counters", () => {
    expect(branchTriggerLabel(status({ upstream: "origin/main", ahead: 2, behind: 1 }))).toBe(
      "Current branch: main · upstream origin/main · 2 ahead, 1 behind",
    );
  });

  test("says a merge is in progress and where to finish it", () => {
    const label = branchTriggerLabel(status({ upstream: "origin/main", operation: "merge" }));
    expect(label).toContain("Current branch: main");
    expect(label).toContain(mergeText);
  });

  test("an operation the app does not finish is not called a merge", () => {
    expect(branchTriggerLabel(status({ operation: "rebase" }))).not.toContain("merge");
  });

  test("a detached HEAD and a missing branch still have a name", () => {
    expect(branchTriggerLabel(status({ detached: true }))).toBe("Current branch: HEAD detached · no upstream");
    expect(branchTriggerLabel(status({ branch: null }))).toBe("Current branch: No branch · no upstream");
  });
});
