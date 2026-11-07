/**
 * F6 — real Git through the `git` on PATH. Every repository here is disposable and created
 * by the test; nothing runs git inside the app checkout.
 */
import { expect, test, type Page } from "@playwright/test";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const screenshots = path.resolve(process.env.OPENRIDR_SCREENSHOT_DIR ?? "docs/evidence/screenshots");
const program = "src/OrderDesk.Cli/Program.cs";
const order = "src/OrderDesk.Core/Order.cs";
const brandNew = "src/OrderDesk.Core/E2eNewFile.cs";
let base = "";
let repo = "";

function git(args: string[], cwd = repo) {
  return execFileSync("git", args, {
    cwd,
    encoding: "utf8",
    env: {
      ...process.env,
      HOME: base,
      GIT_CONFIG_NOSYSTEM: "1",
      GIT_TERMINAL_PROMPT: "0",
    },
  }).trim();
}

function setupRepo() {
  base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-git-"));
  repo = path.join(base, "orderdesk");
  fs.cpSync(fixture, repo, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  git(["init", "-q", "-b", "main"]);
  git(["config", "user.name", "OpenRidr E2E"]);
  git(["config", "user.email", "e2e@openridr.invalid"]);
  git(["add", "."]);
  git(["commit", "-qm", "Initial commit"]);
}

async function openWorkspace(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
}

async function openTrusted(page: Page, target: string) {
  await openWorkspace(page, target);
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function hostCall(page: Page, method: string, params: unknown = {}) {
  return page.request.post(`${host}/call`, { data: { method, params } });
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
  setupRepo();
});

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
  fs.rmSync(base, { recursive: true, force: true });
});

test("long current branch stays compact while its full name remains accessible", async ({ page }) => {
  const longBranch = `feature/${"long-branch-segment-".repeat(8)}end`;
  git(["switch", "-q", "-c", longBranch]);

  await openTrusted(page, path.join(repo, "OrderDesk.sln"));

  const branch = page.getByTestId("toolbar-branch");
  const label = page.getByTestId("branch-current");
  await expect(label).toHaveText(longBranch);
  // Middle truncation (ui-shape 02): the head ellipsizes and the branch tail stays whole.
  const head = label.locator(".mid-head");
  await expect(head).toHaveCSS("text-overflow", "ellipsis");
  expect(await head.evaluate((element) => element.scrollWidth > element.clientWidth)).toBe(true);
  await expect(branch).toHaveAttribute("title", /Current branch:/);
  await expect(branch).toHaveAccessibleName(/Current branch:/);
  expect(await branch.getAttribute("title")).toContain(longBranch);
  expect(await branch.getAttribute("aria-label")).toContain(longBranch);
});

test("commit tool commits only the checked files, preserves the index and amends on demand", async ({ page }) => {
  // A pre-existing staged change must survive a commit of different paths untouched.
  fs.appendFileSync(path.join(repo, program), "// staged by the user\n");
  git(["add", program]);
  fs.appendFileSync(path.join(repo, order), "// e2e worktree change\n");
  fs.writeFileSync(path.join(repo, brandNew), "namespace OrderDesk.Core;\n\npublic static class E2eNewFile { }\n");

  await openTrusted(page, path.join(repo, "OrderDesk.sln"));
  await expect(page.getByTestId("toolbar-branch")).toHaveText(/main/);

  await page.getByTestId("toolband-commit").click();
  await expect(page.getByTestId("commit-tool")).toBeVisible();
  await expect(page.getByTestId("tree")).toHaveCount(0);
  await expect(page.getByTestId("toolband-commit")).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId(`commit-file-${program}`)).toContainText("Staged");
  await expect(page.getByTestId(`commit-file-${order}`)).toContainText("Modified");
  await expect(page.getByTestId(`commit-file-${brandNew}`)).toContainText("Untracked");
  const selectAll = page.getByRole("checkbox", { name: "Select all changes" });
  await expect(selectAll).toBeChecked();
  await page.getByTestId(`commit-check-${program}`).uncheck();
  expect(await selectAll.evaluate((element: HTMLInputElement) => element.indeterminate)).toBe(true);
  await selectAll.check();
  await expect(page.getByTestId(`commit-check-${program}`)).toBeChecked();
  await page.getByRole("button", { name: "Collapse changes", exact: true }).click();
  await expect(page.getByTestId(`commit-file-${program}`)).toHaveCount(0);
  await page.getByRole("button", { name: "Expand changes", exact: true }).click();

  // The diff is the real git diff of the selected file.
  await page.getByTestId(`commit-file-${order}`).click();
  await expect(page.getByTestId("commit-diff")).toContainText("// e2e worktree change");
  await expect(page.getByTestId("commit-diff-tab").getByRole("tab")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("commit-diff").locator(".cm-editor")).toHaveCount(2);
  await expect(page.getByTestId("commit-diff-file-count")).toContainText("/3 files");
  const nextFile = page.getByRole("button", { name: "Next changed file" });
  if (await nextFile.isEnabled()) {
    await nextFile.click();
    await expect(page.getByTestId("commit-diff-tab")).not.toContainText("Commit: Order.cs");
    await page.getByRole("button", { name: "Previous changed file" }).click();
    await expect(page.getByTestId("commit-diff-tab")).toContainText("Commit: Order.cs");
  }
  await page.getByRole("button", { name: "Close diff" }).click();

  await page.getByTestId(`commit-file-${program}`).click();
  await expect(page.getByTestId("commit-diff-left-label")).toContainText("HEAD");
  await expect(page.getByTestId("commit-diff-right-label")).toHaveText("Index");
  await expect(page.getByTestId("commit-diff-commit")).toContainText(`HEAD ${git(["rev-parse", "--short", "HEAD"])}`);
  await page.getByRole("button", { name: "Close diff" }).click();

  await page.getByTestId(`commit-file-${brandNew}`).click();
  await expect(page.getByTestId("commit-diff-left-label")).toHaveText("No previous file");
  await expect(page.getByTestId("commit-diff-right-label")).toHaveText("Worktree");
  await page.getByRole("button", { name: "Close diff" }).click();

  // Commit Order.cs and the new file, explicitly not the staged Program.cs.
  await page.getByTestId(`commit-check-${program}`).uncheck();
  await expect(page.getByTestId(`commit-check-${program}`)).not.toBeChecked();
  await page.getByTestId(`commit-check-${order}`).check();
  await page.getByTestId(`commit-check-${brandNew}`).check();
  await page.getByTestId("commit-message").fill("Selective commit from E2E");
  await page.getByTestId("commit-submit").click();
  await expect(page.getByTestId("commit-state")).toContainText("Committed");

  const message = git(["log", "-1", "--format=%s"]);
  expect(message).toBe("Selective commit from E2E");
  const committed = git(["show", "--name-only", "--format=", "HEAD"]).split("\n").filter(Boolean).sort();
  expect(committed).toEqual([brandNew, order].sort());
  expect(git(["diff", "--cached", "--name-only"])).toBe(program);
  expect(git(["status", "--porcelain", "--", program])).toBe("M  " + program);
  expect(git(["ls-files", "--error-unmatch", brandNew])).toBe(brandNew);

  // Amend keeps the explicit selection and replaces the message.
  fs.appendFileSync(path.join(repo, order), "// amended worktree change\n");
  await page.getByTestId("commit-refresh").click();
  await expect(page.getByTestId(`commit-file-${order}`)).toBeVisible();
  await page.getByTestId(`commit-check-${program}`).uncheck();
  await page.getByTestId(`commit-check-${order}`).check();
  await page.getByTestId("commit-amend").check();
  await page.getByTestId("commit-message").fill("Amended subject from E2E");
  await page.getByTestId("commit-submit").click();
  await expect(page.getByTestId("commit-state")).toContainText("Committed");

  expect(git(["log", "-1", "--format=%s"])).toBe("Amended subject from E2E");
  expect(git(["rev-list", "--count", "HEAD"])).toBe("2");
  expect(git(["show", "--name-only", "--format=", "HEAD"])).toContain(order);
  expect(git(["diff", "--cached", "--name-only"])).toBe(program);
  expect(git(["status", "--porcelain", "--", order])).toBe("");
});

test("full-file diff shows the whole file, connectors and the HEAD commit", async ({ page }) => {
  const longFile = "src/OrderDesk.Core/LongReviewFile.cs";
  const lines = Array.from({ length: 60 }, (_, index) => {
    const number = String(index + 1).padStart(2, "0");
    return `public static class Line${number} { public const string Value = "line-${number}"; }`;
  });
  fs.writeFileSync(path.join(repo, longFile), `namespace OrderDesk.Core;\n\n${lines.join("\n")}\n`);
  git(["add", longFile]);
  git(["commit", "-qm", "Add the long review file"]);
  lines[19] = `public static class Line20 { public const string Value = "changed-line-20"; }`;
  fs.writeFileSync(path.join(repo, longFile), `namespace OrderDesk.Core;\n\n${lines.join("\n")}\n`);
  const head = git(["rev-parse", "--short", "HEAD"]);

  await openTrusted(page, path.join(repo, "OrderDesk.sln"));
  await page.getByTestId("toolband-commit").click();
  await page.getByTestId(`commit-file-${longFile}`).click();

  const diff = page.getByTestId("commit-diff");
  const editors = diff.locator(".cm-editor");
  await expect(editors).toHaveCount(2);
  await expect(diff.getByTestId("commit-diff-note")).toHaveText("Showing the full file from Git.");
  await expect(editors.nth(1)).toContainText("line-01");
  await expect(editors.nth(1)).toContainText("changed-line-20");
  await expect(diff.locator(".cm-diff-connector").first()).toBeVisible();
  await expect(diff.getByTestId("commit-diff-commit")).toBeVisible();
  await expect(diff.getByTestId("commit-diff-commit")).toContainText(`HEAD ${head}`);
  await expect(diff.getByTestId("commit-diff-commit")).toContainText("Add the long review file");
  await expect(diff.getByTestId("commit-diff-commit")).toHaveAttribute("title", git(["rev-parse", "HEAD"]));
  await expect(diff.getByTestId("commit-diff-left-label")).toHaveText("Index");
  await expect(diff.getByTestId("commit-diff-right-label")).toHaveText("Worktree");
  await expect(diff.getByTestId("commit-diff-count")).toHaveText("1 difference");
  // The last line of the file is reachable, so the panel is not limited to the hunk.
  await editors.nth(1).locator(".cm-scroller").evaluate((element) => { element.scrollTop = element.scrollHeight; });
  await expect(editors.nth(1)).toContainText("line-60");
  await page.getByRole("button", { name: "Close diff" }).click();
});

test("diff labels added and deleted files and shows their full content", async ({ page }) => {
  const added = "src/OrderDesk.Core/AddedInDiff.cs";
  const deleted = "src/OrderDesk.Core/DeletedInDiff.cs";
  fs.writeFileSync(path.join(repo, deleted), "namespace OrderDesk.Core;\n\npublic static class DeletedInDiff { public const string Value = \"gone-line\"; }\n");
  git(["add", deleted]);
  git(["commit", "-qm", "Add the file the diff will see deleted"]);
  fs.rmSync(path.join(repo, deleted));
  fs.writeFileSync(path.join(repo, added), "namespace OrderDesk.Core;\n\npublic static class AddedInDiff { public const string Value = \"new-line\"; }\n");

  await openTrusted(page, path.join(repo, "OrderDesk.sln"));
  await page.getByTestId("toolband-commit").click();
  const diff = page.getByTestId("commit-diff");

  await page.getByTestId(`commit-file-${added}`).click();
  await expect(diff.getByTestId("commit-diff-left-label")).toHaveText("No previous file");
  await expect(diff.getByTestId("commit-diff-right-label")).toHaveText("Worktree");
  await expect(diff.locator(".cm-editor").nth(1)).toContainText("new-line");
  await expect(diff.getByTestId("commit-diff-note")).toHaveText("Showing the full file from Git.");
  await page.getByRole("button", { name: "Close diff" }).click();

  await page.getByTestId(`commit-file-${deleted}`).click();
  await expect(diff.getByTestId("commit-diff-left-label")).toHaveText("Index");
  await expect(diff.getByTestId("commit-diff-right-label")).toHaveText("No current file");
  await expect(diff.locator(".cm-editor").nth(0)).toContainText("gone-line");
  await page.getByRole("button", { name: "Close diff" }).click();
});

test("diff falls back to the patch for a binary file and says why", async ({ page }) => {
  const binary = "src/OrderDesk.Core/binary.dat";
  fs.writeFileSync(path.join(repo, binary), Buffer.from([0x41, 0x00, 0x42]));

  await openTrusted(page, path.join(repo, "OrderDesk.sln"));
  await page.getByTestId("toolband-commit").click();
  await page.getByTestId(`commit-file-${binary}`).click();

  const diff = page.getByTestId("commit-diff");
  await expect(diff.getByTestId("commit-diff-note")).toContainText("binary file");
  await expect(diff.locator(".cm-editor")).toHaveCount(0);
  await page.getByRole("button", { name: "Close diff" }).click();
});

test("unified mode shows one editor and remembers the choice", async ({ page }) => {
  fs.appendFileSync(path.join(repo, order), "// unified mode change\n");

  await openTrusted(page, path.join(repo, "OrderDesk.sln"));
  await page.getByTestId("toolband-commit").click();
  await page.getByTestId(`commit-file-${order}`).click();

  const diff = page.getByTestId("commit-diff");
  const mode = diff.getByTestId("commit-diff-mode");
  await expect(diff.locator(".cm-editor")).toHaveCount(2);
  await expect(mode.getByRole("button", { name: "Split" })).toHaveAttribute("aria-pressed", "true");
  await expect(mode.getByRole("button", { name: "Unified" })).toHaveAttribute("aria-pressed", "false");
  await expect(diff.getByTestId("commit-diff-count")).toHaveText("1 difference");

  await mode.getByRole("button", { name: "Unified" }).click();
  await expect(mode.getByRole("button", { name: "Unified" })).toHaveAttribute("aria-pressed", "true");
  await expect(diff.locator(".cm-editor")).toHaveCount(1);
  await expect(diff.getByTestId("commit-diff-unified")).toBeVisible();
  await expect(diff.getByTestId("commit-diff-unified")).toContainText("+// unified mode change");
  await expect(diff.getByTestId("commit-diff-count")).toHaveText("1 difference");
  await expect(diff.locator(".cm-diff-connector")).toHaveCount(0);

  // The choice outlives the diff tab, not just the file.
  await page.getByRole("button", { name: "Close diff" }).click();
  await page.getByTestId(`commit-file-${order}`).click();
  await expect(diff.locator(".cm-editor")).toHaveCount(1);
  await expect(mode.getByRole("button", { name: "Unified" })).toHaveAttribute("aria-pressed", "true");

  await mode.getByRole("button", { name: "Split" }).click();
  await expect(diff.locator(".cm-editor")).toHaveCount(2);
  await expect(diff.locator(".cm-diff-connector").first()).toBeVisible();
  await page.getByRole("button", { name: "Close diff" }).click();
});

test("Git review keeps long diffs, paths, log metadata and commit feedback readable", async ({ page }) => {
  const longPath = `${"src/OrderDesk.Core/"}${"deeply-nested-review-folder/".repeat(3)}ReadablePath.cs`;
  const absolutePath = path.join(repo, longPath);
  fs.mkdirSync(path.dirname(absolutePath), { recursive: true });
  fs.writeFileSync(absolutePath, `namespace OrderDesk.Core;\n\npublic static class ReadablePath { public static string Value => "${"before ".repeat(30)}"; }\n`);
  git(["add", longPath]);
  git(["commit", "-qm", "A deliberately descriptive history entry for the Git log layout"]);
  fs.writeFileSync(absolutePath, `namespace OrderDesk.Core;\n\npublic static class ReadablePath { public static string Value => "${"after ".repeat(30)}"; }\n`);

  await openTrusted(page, path.join(repo, "OrderDesk.sln"));
  await page.getByTestId("toolband-commit").click();
  const file = page.getByTestId(`commit-file-${longPath}`);
  await expect(file).toBeVisible();
  await file.click();

  const diff = page.getByTestId("commit-diff");
  await expect(diff).toBeVisible();
  await expect(diff).toHaveAttribute("aria-label", `Git diff for ${longPath}`);
  await expect(page.getByTestId("commit-diff-tab")).toContainText("Commit: ReadablePath.cs");
  const diffText = diff.locator(".cm-editor");
  await expect(diff.getByTestId("commit-diff-left-label")).toContainText("Index");
  await expect(diff.getByTestId("commit-diff-right-label")).toHaveText("Worktree");
  await expect(diffText).toHaveCount(2);
  await expect(diffText.nth(1)).toContainText("after after after");
  await expect(diff.locator(".git-diff-intraline").first()).toBeVisible();
  await expect(diff.getByTestId("commit-diff-count")).toHaveText("1 difference");
  if (process.env.OPENRIDR_CAPTURE_GIT_UI === "1") {
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({ path: path.join(screenshots, "git-diff.png") });
  }
  const scroller = diffText.nth(1).locator(".cm-scroller");
  expect(await scroller.evaluate((element) => element.scrollWidth)).toBeGreaterThan(
    await scroller.evaluate((element) => element.clientWidth),
  );
  await page.getByRole("button", { name: "Close diff" }).click();

  await page.getByTestId("commit-message").fill("Readable commit result");
  await page.getByTestId("commit-submit").click();
  await expect(page.getByTestId("commit-state")).toContainText("Committed");
  await expect(page.getByTestId("commit-state")).toContainText("Readable commit result");
  await expect(page.getByTestId("app-log")).toHaveCount(0);

  await page.getByTestId("bottom-tab-git").click();
  const commits = page.getByTestId(/^git-log-item-/);
  await expect(commits.first()).toContainText("Readable commit result");
  await expect(commits.first()).toContainText("OpenRidr E2E");
  await commits.nth(1).click();
  await expect(page.getByTestId("git-log-files")).toContainText(longPath);
  await expect(page.getByTestId("git-log-files").locator("code").first()).toHaveText(longPath);
  if (process.env.OPENRIDR_CAPTURE_GIT_UI === "1") {
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({ path: path.join(screenshots, "git-log.png") });
  }
});

test("log lists commits and files; checkout and pull explain refusals; push needs a remote and confirmation", async ({
  page,
}) => {
  fs.appendFileSync(path.join(repo, order), "// second commit\n");
  git(["add", "."]);
  git(["commit", "-qm", "Second commit for log"]);
  git(["branch", "feature"]);

  await openTrusted(page, path.join(repo, "OrderDesk.sln"));
  await page.getByTestId("bottom-tab-git").click();
  await expect(page.getByTestId("git-log")).toBeVisible();
  const commits = page.getByTestId(/^git-log-item-/);
  await expect(commits).toHaveCount(2);
  await expect(commits.first()).toContainText("Second commit for log");
  // A single parent is the normal case and is not repeated on every row (ui-shape 11); root and merges still say so.
  await expect(commits.first()).not.toContainText("1 parent");
  await expect(commits.nth(1)).toContainText("root");
  await commits.first().click();
  await expect(page.getByTestId("git-log-files")).toContainText(order);

  // Branch popup: create a branch, then show the dirty-tree refusal.
  await page.getByTestId("toolbar-branch").click();
  await expect(page.getByTestId("toolbar-branch")).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByTestId("branch-item-main")).toBeVisible();
  await expect(page.getByTestId("branch-item-feature")).toBeVisible();
  await page.getByTestId("branch-create-open").click();
  await page.getByTestId("branch-name").fill("e2e/branch");
  await page.getByTestId("branch-create-submit").click();
  await expect(page.getByTestId("toolbar-branch")).toContainText("e2e/branch");

  fs.appendFileSync(path.join(repo, order), "// dirty while checking out\n");
  await page.getByTestId("branch-refresh").click();
  await expect(page.getByTestId("branch-checkout-reason")).toContainText("uncommitted changes");
  await expect(page.getByTestId("branch-item-main")).toBeDisabled();
  const denied = await hostCall(page, "git.checkout", { name: "main" });
  const deniedBody = await denied.json();
  expect(deniedBody.ok).toBe(false);
  expect(deniedBody.error.code).toBe("git_dirty_tree");
  expect(deniedBody.error.message).toContain("does not auto-stash");
  expect(git(["rev-parse", "--abbrev-ref", "HEAD"])).toBe("e2e/branch");

  // Update needs an upstream; Push needs a remote; neither runs a command.
  await expect(page.getByTestId("branch-update")).toBeDisabled();
  await expect(page.getByTestId("branch-update")).toHaveAttribute("title", /No upstream/);
  await expect(page.getByTestId("branch-push")).toBeDisabled();
  await expect(page.getByTestId("branch-push-reason")).toContainText("No remote is configured");
  const unconfirmed = await (await hostCall(page, "git.push")).json();
  expect(unconfirmed.error.code).toBe("confirmation_required");
  // The push is a start: it returns the host's id at once and the missing remote is the
  // operation's failure, published in the snapshot.
  const noRemote = await (await hostCall(page, "git.push", { confirm: true })).json();
  expect(noRemote.ok).toBe(true);
  expect(noRemote.data.id).toMatch(/^git:\d+$/);
  await expect
    .poll(
      async () => (await (await hostCall(page, "workspace.state")).json()).data.git,
      { timeout: 15_000 },
    )
    .toMatchObject({ state: "failed", message: expect.stringContaining("No remote is configured") });

  // A remote with credentials still never exposes them, and Push shows the confirmation
  // dialog with remote, branch, upstream and commits ahead.
  git(["remote", "add", "origin", "https://user:secret-token@example.invalid/repo.git"]);
  await page.keyboard.press("Escape");
  await page.getByTestId("toolbar-branch").click();
  await page.getByTestId("branch-refresh").click();
  await page.getByTestId("branch-push").click();
  await expect(page.getByTestId("push-dialog")).toBeVisible();
  await expect(page.getByTestId("push-remote")).toHaveText("origin");
  await expect(page.getByTestId("push-branch")).toHaveText("e2e/branch");
  await expect(page.getByTestId("push-upstream")).toContainText("Not set");
  await expect(page.getByTestId("push-ahead")).toHaveText("0");
  await page.getByTestId("push-cancel").click();

  const status = await (await hostCall(page, "git.status")).json();
  expect(JSON.stringify(status)).not.toContain("secret-token");
  const branches = await (await hostCall(page, "git.branches")).json();
  expect(JSON.stringify(branches)).not.toContain("secret-token");
});

test("restricted mode and a non-repository explain themselves and every git method needs trust", async ({
  page,
}) => {
  await openWorkspace(page, path.join(repo, "OrderDesk.sln"));
  await page.getByTestId("trust-prompt-restricted").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  const restrictedGit = page.getByTestId("toolbar-branch");
  await expect(restrictedGit).toBeDisabled();
  await expect(restrictedGit).toHaveAccessibleName("Trust the workspace to use Git");
  await expect(restrictedGit).toHaveAttribute("title", "Trust the workspace to use Git");

  await page.getByTestId("toolband-commit").click();
  await expect(page.getByTestId("commit-disabled-reason")).toHaveText("Trust the workspace to use Git");
  await page.getByTestId("bottom-tab-git").click();
  await expect(page.getByTestId("git-log-disabled-reason")).toHaveText("Trust the workspace to use Git");

  for (const [method, params] of [
    ["git.status", {}],
    ["git.diff", { path: program }],
    ["git.branches", {}],
    ["git.branchCreate", { name: "x" }],
    ["git.checkout", { name: "main" }],
    ["git.log", {}],
    ["git.commit", { paths: [program], message: "x" }],
    ["git.pull", {}],
    ["git.push", { confirm: true }],
  ] as const) {
    const body = await (await hostCall(page, method, params)).json();
    expect(body.ok, method).toBe(false);
    expect(body.error.code, method).toBe("trust_required");
  }

  // A trusted workspace outside any repository is data, not an error.
  const plain = path.join(base, "plain");
  fs.mkdirSync(plain);
  fs.writeFileSync(
    path.join(plain, "Plain.csproj"),
    '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n',
  );
  fs.writeFileSync(path.join(plain, "Class.cs"), "class C {}\n");
  // The shell layout persisted the Commit/Git tool windows from the repository workspace.
  await page.evaluate(() => localStorage.setItem("openridr.layout.v1", JSON.stringify({ bottom: { active: "build", size: 220, open: true } })));
  await openTrusted(page, path.join(plain, "Plain.csproj"));
  const noRepoGit = page.getByTestId("toolbar-branch");
  await expect(noRepoGit).toBeDisabled();
  await expect(noRepoGit).toHaveAccessibleName("Not a git repository");
  await expect(noRepoGit).toHaveAttribute("title", "Not a git repository");
  await page.getByTestId("toolband-commit").click();
  await expect(page.getByTestId("commit-disabled-reason")).toHaveText("Not a git repository");
  await page.getByTestId("bottom-tab-git").click();
  await expect(page.getByTestId("git-log-disabled-reason")).toHaveText("Not a git repository");
  const status = await (await hostCall(page, "git.status")).json();
  expect(status.ok).toBe(true);
  expect(status.data.state).toBe("not_a_repository");
});

/**
 * A remote that accepts the connection and never answers — the same scenario the host tests and
 * #136 use. Only the cancel can end the wait before the deadline, so a passing assertion here is
 * the cancel doing its job.
 */
function silentRemote() {
  const sockets: net.Socket[] = [];
  const server = net.createServer((socket) => sockets.push(socket));
  return new Promise<{ port: number; close: () => void }>((resolve) => {
    server.listen(0, "127.0.0.1", () => {
      const port = (server.address() as net.AddressInfo).port;
      resolve({
        port,
        close: () => {
          for (const socket of sockets) socket.destroy();
          server.close();
        },
      });
    });
  });
}

/** The branch popover closes on any pointerdown outside it, including inside a dialog. */
async function openBranchMenu(page: Page) {
  if (!(await page.getByTestId("branch-refresh").isVisible())) {
    await page.getByTestId("toolbar-branch").click();
  }
  await expect(page.getByTestId("branch-refresh")).toBeVisible();
}

test("cancelling a push, an update and a fetch returns in seconds while the app answers", async ({
  page,
}) => {
  const silent = await silentRemote();
  try {
    git(["checkout", "-qb", "e2e/cancel"]);
    git(["remote", "add", "origin", `http://127.0.0.1:${silent.port}/repo.git`]);
    // An upstream that exists without touching the network: Update needs one to be offered.
    git(["update-ref", "refs/remotes/origin/e2e/cancel", "HEAD"]);
    git(["branch", "--set-upstream-to=origin/e2e/cancel"]);

    await openTrusted(page, path.join(repo, "OrderDesk.sln"));

    // Push → Cancel push: the dialog stays open with the app's message.
    await openBranchMenu(page);
    await page.getByTestId("branch-push").click();
    await expect(page.getByTestId("push-dialog")).toBeVisible();
    await page.getByTestId("push-confirm").click();
    await expect(page.getByTestId("push-stop")).toBeVisible();
    // The app keeps answering while the push waits: the same bridge the panel polls.
    const began = Date.now();
    const during = await (await hostCall(page, "git.status")).json();
    expect(during.data.state).toBe("ok");
    // The bridge is sequential: this proves the push is not holding it, not that the answer is instant.
    expect(Date.now() - began).toBeLessThan(5000);
    await page.getByTestId("push-stop").click();
    await expect(page.getByTestId("push-error")).toContainText("Push cancelled", { timeout: 5000 });
    await expect(page.getByTestId("push-error")).toContainText("may already have reached the remote");
    await expect(page.getByTestId("push-dialog")).toBeVisible();
    await page.getByTestId("push-cancel").click();

    // Escape cancels the operation in flight instead of closing the dialog.
    await openBranchMenu(page);
    await page.getByTestId("branch-push").click();
    await page.getByTestId("push-confirm").click();
    await expect(page.getByTestId("push-stop")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("push-error")).toContainText("Push cancelled", { timeout: 5000 });
    await page.getByTestId("push-cancel").click();

    // Update → Cancel update: nothing was merged and no merge is left behind.
    await openBranchMenu(page);
    await page.getByTestId("branch-update").click();
    await expect(page.getByTestId("update-dialog")).toBeVisible();
    await page.getByTestId("update-confirm").click();
    await expect(page.getByTestId("update-stop")).toBeVisible();
    await page.getByTestId("update-stop").click();
    await expect(page.getByTestId("update-error")).toContainText("Update cancelled", { timeout: 5000 });
    await expect(page.getByTestId("update-error")).toContainText("nothing was merged");
    expect(fs.existsSync(path.join(repo, ".git", "MERGE_HEAD"))).toBe(false);
    await page.getByTestId("update-cancel").click();

    // Fetch → Cancel fetch.
    await openBranchMenu(page);
    await page.getByTestId("branch-fetch").click();
    await expect(page.getByTestId("branch-fetch-stop")).toBeVisible();
    await page.getByTestId("branch-fetch-stop").click();
    await expect(page.getByTestId("branch-fetch-error")).toContainText("Fetch cancelled", {
      timeout: 5000,
    });
  } finally {
    silent.close();
  }
});

test("an update whose fetch already finished says there is nothing to cancel", async ({ page }) => {
  // A real remote, so the fetch completes; the merge is local and cannot be cancelled, and the
  // post-merge hook holds it open long enough for the click to land after the network step.
  const bare = path.join(base, "origin.git");
  git(["init", "-q", "--bare", "-b", "main", bare], base);
  git(["remote", "add", "origin", bare]);
  git(["push", "-q", "-u", "origin", "main"]);
  const peer = path.join(base, "peer");
  git(["clone", "-q", bare, peer], base);
  fs.writeFileSync(path.join(peer, "peer.txt"), "peer\n");
  git(["add", "."], peer);
  git(["commit", "-qm", "peer"], peer);
  git(["push", "-q", "origin", "main"], peer);
  fs.writeFileSync(path.join(repo, "local.txt"), "local\n");
  git(["add", "."]);
  git(["commit", "-qm", "local"]);
  const merging = path.join(base, "merging");
  const hook = path.join(repo, ".git", "hooks", "post-merge");
  fs.writeFileSync(hook, `#!/bin/sh\ntouch '${merging}'\nsleep 3\n`);
  fs.chmodSync(hook, 0o755);

  await openTrusted(page, path.join(repo, "OrderDesk.sln"));
  await openBranchMenu(page);
  await page.getByTestId("branch-update").click();
  await page.getByTestId("update-confirm").click();
  await expect.poll(() => fs.existsSync(merging), { timeout: 30_000 }).toBe(true);
  await page.getByTestId("update-stop").click();
  await expect(page.getByTestId("update-error")).toContainText(
    "Nothing to cancel: the update is already merging.",
  );
  // The merge finishes on its own and the dialog closes with the app's message.
  await expect(page.getByTestId("update-dialog")).toBeHidden({ timeout: 30_000 });
  expect(fs.existsSync(path.join(repo, ".git", "MERGE_HEAD"))).toBe(false);
});
