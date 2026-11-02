/**
 * Medium nav 04: Push (⇧⌘K) and Update (⌥⌘W) dialogs take focus, keep typing out of the
 * hidden buffer and close with Escape. Every repository here is disposable and created by
 * the test; nothing runs git inside the app checkout.
 */
import { expect, test, type Page } from "@playwright/test";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const program = "src/OrderDesk.Cli/Program.cs";
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
  base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-git-dialogs-"));
  repo = path.join(base, "orderdesk");
  fs.cpSync(fixture, repo, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  const remote = path.join(base, "remote.git");
  git(["init", "-q", "-b", "main"]);
  git(["config", "user.name", "OpenRidr E2E"]);
  git(["config", "user.email", "e2e@openridr.invalid"]);
  git(["add", "."]);
  git(["commit", "-qm", "Initial commit"]);
  // Update needs an upstream, so the repository gets a real bare remote.
  git(["init", "-q", "--bare", remote], base);
  git(["remote", "add", "origin", remote]);
  git(["push", "-q", "-u", "origin", "main"]);
}

async function openEditor(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(repo, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("tree-expand-all").click();
  await page.getByTestId(`tree-file-${program}`).click();
  const editor = page.getByTestId("editor");
  await expect(editor).toContainText("Console.WriteLine");
  await editor.click();
  return editor;
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
  setupRepo();
});

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
  fs.rmSync(base, { recursive: true, force: true });
});

test("⇧⌘K focuses Push, typing stays out of the buffer and Escape closes", async ({ page }) => {
  const editor = await openEditor(page);
  const before = await editor.innerText();

  await page.keyboard.press("Meta+Shift+K");
  await expect(page.getByTestId("push-dialog")).toBeVisible();
  await expect(page.getByTestId("push-confirm")).toBeFocused();

  await page.keyboard.press("x");
  await expect.poll(() => editor.innerText()).toBe(before);
  await expect(page.getByTestId("dirty-marker")).toHaveCount(0);

  await page.keyboard.press("Escape");
  await expect(page.getByTestId("push-dialog")).toHaveCount(0);
});

test("⌥⌘W focuses Update and Escape closes", async ({ page }) => {
  await openEditor(page);

  await page.keyboard.press("Meta+Alt+W");
  await expect(page.getByTestId("update-dialog")).toBeVisible();
  await expect(page.getByTestId("update-confirm")).toBeFocused();

  await page.keyboard.press("Escape");
  await expect(page.getByTestId("update-dialog")).toHaveCount(0);
});

test("Push without a remote still takes focus inside the dialog and Escape closes", async ({ page }) => {
  // No remote means the confirm button is disabled, so the dialog itself must hold focus.
  git(["remote", "remove", "origin"]);
  git(["switch", "-q", "-c", "no-remote"]);
  const editor = await openEditor(page);
  const before = await editor.innerText();

  await page.keyboard.press("Meta+Shift+K");
  const dialog = page.getByTestId("push-dialog");
  await expect(dialog).toBeVisible();
  await expect(page.getByTestId("push-confirm")).toBeDisabled();
  expect(await dialog.evaluate((element) => element.contains(document.activeElement))).toBe(true);

  await page.keyboard.press("x");
  await expect.poll(() => editor.innerText()).toBe(before);
  await expect(page.getByTestId("dirty-marker")).toHaveCount(0);

  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
});

test("reopening Push keeps the hidden buffer untouched", async ({ page }) => {
  const editor = await openEditor(page);
  const before = await editor.innerText();

  for (const key of ["x", "y"]) {
    await page.keyboard.press("Meta+Shift+K");
    await expect(page.getByTestId("push-dialog")).toBeVisible();
    await page.keyboard.press(key);
    await expect.poll(() => editor.innerText()).toBe(before);
    await expect(page.getByTestId("dirty-marker")).toHaveCount(0);
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("push-dialog")).toHaveCount(0);
  }
});
