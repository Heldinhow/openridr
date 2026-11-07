/**
 * #49 — Git status and symlink markers in the Solution Explorer.
 *
 * Every repository here is disposable and created by the test; nothing runs git inside the app
 * checkout. Nightly smoke only (decision #67): the spec is in `scripts/e2e-smoke.txt` and is never
 * part of the per-PR or per-session verification.
 */
import { expect, test, type Page } from "@playwright/test";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const order = "src/OrderDesk.Core/Order.cs";
const newFile = "src/OrderDesk.Core/E2eNewFile.cs";
let base = "";
let repo = "";

function git(args: string[], cwd = repo) {
  return execFileSync("git", args, {
    cwd,
    encoding: "utf8",
    env: { ...process.env, HOME: base, GIT_CONFIG_NOSYSTEM: "1", GIT_TERMINAL_PROMPT: "0" },
  }).trim();
}

function setupRepo() {
  base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-tree-git-"));
  repo = path.join(base, "orderdesk");
  fs.cpSync(fixture, repo, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  fs.writeFileSync(path.join(repo, ".gitignore"), "*.log\n");
  git(["init", "-q", "-b", "main"]);
  git(["config", "user.name", "OpenRidr E2E"]);
  git(["config", "user.email", "e2e@openridr.invalid"]);
  git(["add", "."]);
  git(["commit", "-qm", "Initial commit"]);
}

async function openTrusted(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(repo, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await expect(page.getByTestId("tree")).toBeVisible();
}

/** The window regaining focus is the trigger for an out-of-app commit or checkout. */
async function refocus(page: Page) {
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
  setupRepo();
});

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
  fs.rmSync(base, { recursive: true, force: true });
});

test("file rows show the Git status and refresh after commit and checkout", async ({ page }) => {
  // Created before the workspace opens, so the evaluated sources include it.
  fs.writeFileSync(path.join(repo, newFile), "namespace OrderDesk.Core;\n");
  await openTrusted(page);
  await page.getByTestId("tree-project-OrderDesk.Core").click();

  const row = page.getByTestId(`tree-file-${order}`);
  const added = page.getByTestId(`tree-file-${newFile}`);
  await expect(row).not.toHaveAttribute("data-git-status", /.+/);
  await expect(added).toHaveAttribute("data-git-status", "untracked");

  fs.appendFileSync(path.join(repo, order), "\n// changed\n");
  await refocus(page);
  await expect(row).toHaveAttribute("data-git-status", "modified");
  await expect(row.locator(".tag.git-status")).toHaveText("· modified");
  // The granular commit-tool label stays available without a second colour.
  await expect(row.locator(".tag.git-status")).toHaveAttribute("title", "Modified");

  git(["add", "-A"]);
  git(["commit", "-qm", "change"]);
  await refocus(page);
  await expect(row).not.toHaveAttribute("data-git-status", /.+/);
  await expect(added).not.toHaveAttribute("data-git-status", /.+/);

  git(["checkout", "-qb", "feature"]);
  fs.appendFileSync(path.join(repo, order), "\n// feature\n");
  await refocus(page);
  await expect(row).toHaveAttribute("data-git-status", "modified");
});

test("a .gitignore match is marked ignored and never overrides a real status", async ({ page }) => {
  fs.writeFileSync(path.join(repo, "notes.log"), "log\n");
  await openTrusted(page);
  await page.getByTestId("tree-show-all").click();

  const ignored = page.getByTestId("tree-file-notes.log");
  await expect(ignored).toHaveAttribute("data-git-status", "ignored");
  await expect(ignored.locator(".tag.git-status")).toHaveText("· ignored");
  // A tracked, clean file is never "ignored", even when a pattern would match it.
  const tracked = page.getByTestId("tree-file-global.json");
  await expect(tracked).not.toHaveAttribute("data-git-status", /.+/);
});

test("symlinks are marked and a symlinked folder is not expanded", async ({ page }) => {
  fs.symlinkSync(path.join(repo, order), path.join(repo, "src/OrderDesk.Core/Linked.cs"));
  fs.symlinkSync(path.join(repo, "src/OrderDesk.Core"), path.join(repo, "LinkDir"));
  await openTrusted(page);

  // Evaluated sources: the link is a source file, marked without reading the target.
  await page.getByTestId("tree-project-OrderDesk.Core").click();
  const linked = page.getByTestId("tree-file-src/OrderDesk.Core/Linked.cs");
  await expect(linked).toHaveAttribute("data-symlink", "true");
  await expect(linked.locator(".symlink-mark")).toHaveCount(1);

  // "Show all files": a link is its own object (no chevron, link glyph) and never expands. The
  // listing never stats the target, so a link to a folder is listed exactly like a link to a file.
  await page.getByTestId("tree-show-all").click();
  const linkDir = page.getByTestId("tree-file-LinkDir");
  await expect(linkDir).toHaveAttribute("data-symlink", "true");
  await expect(linkDir.locator(".tree-chevron.open")).toHaveCount(0);
  await expect(linkDir.locator(".icon-link")).toHaveCount(1);
  await linkDir.click();
  await expect(page.getByTestId("tree-file-LinkDir/Order.cs")).toHaveCount(0);
});
