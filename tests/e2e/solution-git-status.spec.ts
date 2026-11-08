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

/**
 * #264 — the first host answer for `method` that carries `path` is refused; every other call passes.
 * A tree query that gives up on the first failure leaves the row without its marker, so the marker
 * itself is the assertion.
 *
 * The `workspace.state` poll and the `git.status` refresh are stopped here: both re-render the tree,
 * and a re-render asks the queries again — which would hide whether the query was repeated. The
 * returned `settle` waits for the calls already in the air to land before the identity changes, so
 * the refusal lands on the request under test instead of on a stale one.
 */
async function refuseFirstAnswerFor(page: Page, method: string, path: string) {
  let refused = false;
  let lastPassed = Date.now();
  await page.route("**/call", async (route) => {
    const body = route.request().postDataJSON() as { method?: string; params?: { paths?: string[] } };
    if (body.method === "workspace.state" || body.method === "git.status") {
      await route.abort();
      return;
    }
    lastPassed = Date.now();
    const carries = body.method === method && (body.params?.paths ?? []).includes(path);
    if (!carries || refused) {
      await route.continue();
      return;
    }
    refused = true;
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ ok: false, error: { code: "host_error", message: "injected failure" } }),
    });
  });
  return {
    refused: () => refused,
    settle: () => expect.poll(() => Date.now() - lastPassed).toBeGreaterThan(1000),
  };
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
  // One glyph per row: the link is the row's own icon here, so no corner badge on top of it.
  await expect(linkDir.locator(".symlink-mark")).toHaveCount(0);
  await linkDir.click();
  await expect(page.getByTestId("tree-file-LinkDir/Order.cs")).toHaveCount(0);
});

// #264: a tree query that fails once must ask again, not leave the row without its marker.
// The badge on an evaluated source is the signal that the model (and the tree it paints) settled:
// the query under test is the only one a new Git revision or evaluation asks for afterwards.
async function waitForEvaluatedTree(page: Page) {
  await expect(page.getByTestId(`tree-file-${order}`)).toHaveAttribute("data-symbol-kind", "class");
}

test("an ignored answer that fails once is asked again instead of dropping the marker", async ({ page }) => {
  // A source covered by `.gitignore`: the evaluated tree shows it, so the query has one identity.
  const file = "src/OrderDesk.Core/Ignored.cs";
  fs.writeFileSync(path.join(repo, file), "namespace OrderDesk.Core;\n\nclass Ignored {}\n");
  fs.appendFileSync(path.join(repo, ".gitignore"), "Ignored.cs\n");
  await openTrusted(page);
  await waitForEvaluatedTree(page);
  const ignored = page.getByTestId(`tree-file-${file}`);
  await expect(ignored).toHaveAttribute("data-git-status", "ignored");

  // Collapsing drops the row from the request and clears the marker; expanding asks again, and the
  // first answer to that request is lost, so the marker can only come back from a repeated query.
  const project = page.getByTestId("tree-project-OrderDesk.Core");
  await project.click();
  const { refused, settle } = await refuseFirstAnswerFor(page, "git.ignored", file);
  await settle();
  await project.click();
  await expect.poll(refused).toBe(true);
  await expect(ignored).toHaveAttribute("data-git-status", "ignored");
});

test("a symlink answer that fails once is asked again instead of dropping the marker", async ({ page }) => {
  const file = "src/OrderDesk.Core/Linked.cs";
  fs.symlinkSync(path.join(repo, order), path.join(repo, file));
  await openTrusted(page);
  await waitForEvaluatedTree(page);
  const linked = page.getByTestId(`tree-file-${file}`);
  await expect(linked).toHaveAttribute("data-symlink", "true");

  // "Show all files" turns the evaluated query off and clears its markers; turning it back on asks
  // again, and the first answer to that request is lost.
  await page.getByTestId("tree-show-all").click();
  const { refused, settle } = await refuseFirstAnswerFor(page, "workspace.symlinks", file);
  await settle();
  await page.getByTestId("tree-show-all").click();
  await expect.poll(refused).toBe(true);
  await expect(linked).toHaveAttribute("data-symlink", "true");
});

test("a symbol-kind answer that fails once is asked again instead of dropping the badge", async ({ page }) => {
  await openTrusted(page);
  await waitForEvaluatedTree(page);
  // A source the badge scan never answered for: only a fresh answer can paint its badge. The
  // re-evaluation below is the first scan that asks for it; the automatic one for the model that
  // changed on disk asks again, so this test asserts the badge and not the retry (see the evidence).
  const file = "src/OrderDesk.Core/RetryBadge.cs";
  fs.writeFileSync(path.join(repo, file), "namespace OrderDesk;\n\nclass RetryBadge {}\n");
  const { refused, settle } = await refuseFirstAnswerFor(page, "workspace.symbolKinds", file);
  await settle();
  await page.getByTestId("tree-more").click();
  await page.getByTestId("tree-more-reevaluate").click();
  await expect.poll(refused).toBe(true);
  await expect(page.getByTestId(`tree-file-${file}`)).toHaveAttribute("data-symbol-kind", "class");
});
