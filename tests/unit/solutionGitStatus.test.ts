import { describe, expect, test } from "bun:test";
import type { GitFile, GitStatus } from "../../src/git";
import {
  mergeIgnored,
  treeGitStatusMap,
  treeQueryId,
  treeStatusOf,
  workspacePathOf,
} from "../../src/solutionGitStatus";

const file = (path: string, overrides: Partial<GitFile> = {}): GitFile => ({
  path,
  originalPath: null,
  index: " ",
  worktree: " ",
  untracked: false,
  staged: false,
  conflicted: false,
  label: "Modified",
  ...overrides,
});

const snapshot = (files: GitFile[], root: string | null = "/repo"): GitStatus => ({
  state: "ok",
  message: null,
  root,
  branch: "main",
  detached: false,
  initial: false,
  upstream: null,
  upstreamGone: false,
  ahead: 0,
  behind: 0,
  operation: null,
  clean: files.length === 0,
  files,
  remotes: [],
});

describe("workspacePathOf", () => {
  test("keeps the repository path when the roots coincide", () => {
    expect(workspacePathOf("/repo", "src/App.cs", "/repo")).toBe("src/App.cs");
  });

  test("maps a workspace opened in a repository subfolder", () => {
    expect(workspacePathOf("/repo", "apps/web/Program.cs", "/repo/apps/web")).toBe("Program.cs");
    expect(workspacePathOf("/repo", "apps/web/src/Program.cs", "/repo/apps/web")).toBe("src/Program.cs");
  });

  test("drops paths outside the workspace", () => {
    expect(workspacePathOf("/repo", "apps/server/Program.cs", "/repo/apps/web")).toBeNull();
    expect(workspacePathOf("/repo", "README.md", "/repo/apps/web")).toBeNull();
  });

  test("normalizes Windows separators", () => {
    expect(workspacePathOf("C:\\repo", "src/App.cs", "C:\\repo")).toBe("src/App.cs");
    expect(workspacePathOf("C:\\repo", "src\\App.cs", "C:\\repo\\apps\\web")).toBeNull();
  });

  test("a workspace at the filesystem root keeps the path without the leading slash", () => {
    expect(workspacePathOf("/", "src/App.cs", "/")).toBe("src/App.cs");
  });
});

describe("treeStatusOf", () => {
  test("a conflict wins over every other state", () => {
    expect(treeStatusOf(file("a.cs", { conflicted: true, index: "U", worktree: "U" }))).toBe("conflicted");
    expect(treeStatusOf(file("a.cs", { conflicted: true, untracked: true }))).toBe("conflicted");
  });

  test("untracked is not modified", () => {
    expect(treeStatusOf(file("a.cs", { index: "?", worktree: "?", untracked: true }))).toBe("untracked");
  });

  test("an A in the index is added, even with worktree changes", () => {
    expect(treeStatusOf(file("a.cs", { index: "A", worktree: " ", staged: true }))).toBe("added");
    expect(treeStatusOf(file("a.cs", { index: "A", worktree: "M", staged: true }))).toBe("added");
    expect(treeStatusOf(file("a.cs", { index: " ", worktree: "A" }))).toBe("added");
  });

  test("staged, unstaged, renamed and type-changed files are modified", () => {
    expect(treeStatusOf(file("a.cs", { index: "M", staged: true }))).toBe("modified");
    expect(treeStatusOf(file("a.cs", { worktree: "M" }))).toBe("modified");
    expect(treeStatusOf(file("a.cs", { index: "M", worktree: "M", staged: true }))).toBe("modified");
    expect(treeStatusOf(file("a.cs", { index: "R", staged: true }))).toBe("modified");
    expect(treeStatusOf(file("a.cs", { worktree: "T" }))).toBe("modified");
  });

  test("a clean pair says nothing", () => {
    expect(treeStatusOf(file("a.cs"))).toBeNull();
  });
});

describe("treeGitStatusMap", () => {
  test("maps files, keeps the granular label and ignores files outside the workspace", () => {
    const map = treeGitStatusMap(
      snapshot([
        file("apps/web/Program.cs", { worktree: "M", label: "Modified" }),
        file("apps/web/New.cs", { index: "A", staged: true, label: "Staged" }),
        file("apps/server/Other.cs", { worktree: "M" }),
      ]),
      "/repo/apps/web",
    );
    expect(Array.from(map.keys())).toEqual(["Program.cs", "New.cs"]);
    expect(map.get("Program.cs")).toEqual({ status: "modified", label: "Modified" });
    expect(map.get("New.cs")).toEqual({ status: "added", label: "Staged" });
  });

  test("a snapshot without repository, or not ok, yields no entries", () => {
    expect(treeGitStatusMap(null, "/repo").size).toBe(0);
    expect(treeGitStatusMap(snapshot([file("a.cs", { worktree: "M" })], null), "/repo").size).toBe(0);
    expect(
      treeGitStatusMap({ ...snapshot([file("a.cs", { worktree: "M" })]), state: "failed" }, "/repo").size,
    ).toBe(0);
  });
});

describe("mergeIgnored", () => {
  test("adds ignored paths and never overwrites a state Git already reported", () => {
    const base = treeGitStatusMap(snapshot([file("src/App.cs", { worktree: "M" })]), "/repo");
    const merged = mergeIgnored(base, ["src/App.cs", "build/app.log"]);
    expect(merged.get("src/App.cs")?.status).toBe("modified");
    expect(merged.get("build/app.log")).toEqual({ status: "ignored", label: "Ignored" });
  });

  test("does not mutate the map it receives", () => {
    const base = treeGitStatusMap(snapshot([file("src/App.cs", { worktree: "M" })]), "/repo");
    mergeIgnored(base, ["build/app.log"]);
    expect(base.has("build/app.log")).toBe(false);
  });
});

describe("treeQueryId", () => {
  test("the same paths under a new revision are a new request", () => {
    const paths = ["src/App.cs", "build/app.log"];
    expect(treeQueryId("ignored", 1, paths)).not.toBe(treeQueryId("ignored", 2, paths));
    expect(treeQueryId("ignored", 1, paths)).toBe(treeQueryId("ignored", 1, [...paths]));
  });

  test("the workspace scope keeps the same paths apart", () => {
    expect(treeQueryId("/work/a", 1, ["App.cs"])).not.toBe(treeQueryId("/work/b", 1, ["App.cs"]));
  });

  test("a newline in a name is data, not a separator", () => {
    const newline = "line\nbreak.cs";
    const id = treeQueryId("symlink", 1, [newline]);
    expect(JSON.parse(id)[2]).toEqual([newline]);
    expect(id).not.toBe(treeQueryId("symlink", 1, ["line", "break.cs"]));
  });
});
