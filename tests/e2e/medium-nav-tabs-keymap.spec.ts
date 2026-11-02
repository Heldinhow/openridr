import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { e2eHome, host } from "./environment";

const fixture = path.resolve("fixtures/OrderDesk");
const program = "src/OrderDesk.Cli/Program.cs";
const order = "src/OrderDesk.Core/Order.cs";
const settingsFile = path.join(e2eHome, "settings.json");

function tempCopy() {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-tabs-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  return copy;
}

function resetSettings() {
  fs.mkdirSync(e2eHome, { recursive: true });
  fs.writeFileSync(settingsFile, "{}\n");
}

async function openTrusted(page: Page, root = fixture) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(root, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function lineCount(page: Page) {
  return page.getByTestId("editor").locator(".cm-line").count();
}

test.beforeEach(async ({ request }) => {
  resetSettings();
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test.afterEach(() => {
  resetSettings();
});

test("⌘W closes the active tab and the tab shortcuts wrap in both profiles", async ({ page }) => {
  await openTrusted(page);
  await page.getByTestId(`tree-file-${program}`).click();
  await page.getByTestId(`tree-file-${order}`).click();
  await expect(page.getByTestId(`tab-${order}`)).toHaveClass(/active/);

  await page.keyboard.press("Meta+w");
  await expect(page.getByTestId(`tab-${order}`)).toHaveCount(0);
  await expect(page.getByTestId(`tab-${program}`)).toHaveClass(/active/);

  await page.getByTestId(`tree-file-${order}`).click();
  await expect(page.getByTestId(`tab-${order}`)).toHaveClass(/active/);

  // ⇧⌘] wraps from the last tab to the first.
  await page.keyboard.press("Meta+Shift+]");
  await expect(page.getByTestId(`tab-${program}`)).toHaveClass(/active/);
  await page.keyboard.press("Control+Tab");
  await expect(page.getByTestId(`tab-${order}`)).toHaveClass(/active/);

  await page.keyboard.press("Meta+Shift+[");
  await expect(page.getByTestId(`tab-${program}`)).toHaveClass(/active/);
  await page.keyboard.press("Control+Shift+Tab");
  await expect(page.getByTestId(`tab-${order}`)).toHaveClass(/active/);
});

test("⌘E lists the session's recent files and opens the chosen one", async ({ page }) => {
  await openTrusted(page);
  await page.getByTestId(`tree-file-${program}`).click();
  await page.getByTestId(`tree-file-${order}`).click();
  await page.getByTestId("editor").click();

  await page.keyboard.press("Meta+e");
  const dialog = page.getByTestId("recent-files-dialog");
  await expect(dialog).toBeVisible();
  await expect(page.getByTestId("recent-files")).toBeVisible();
  await expect(page.getByTestId("recent-file-0")).toContainText("Order.cs");
  await expect(page.getByTestId("recent-file-1")).toContainText("Program.cs");

  await page.getByTestId("recent-file-1").click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByTestId(`tab-${program}`)).toHaveClass(/active/);

  await page.keyboard.press("Meta+e");
  await expect(dialog).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
});

test("⇧⌘O opens Go to File on the Files tab and ⌘T opens nothing in Rider", async ({ page }) => {
  await openTrusted(page);
  await page.getByTestId(`tree-file-${program}`).click();
  await page.getByTestId("editor").click();

  await page.keyboard.press("Meta+Shift+O");
  await expect(page.getByTestId("find-action")).toBeVisible();
  await expect(page.getByTestId("search-tab-files")).toHaveAttribute("aria-selected", "true");
  await page.getByTestId("find-action-input").press("Escape");
  await expect(page.getByTestId("find-action")).toHaveCount(0);

  await page.keyboard.press("Meta+t");
  await expect(page.getByTestId("find-action")).toHaveCount(0);
});

test("Rider duplicates and deletes lines with ⌘D and ⌘⌫; IntelliJ keeps CodeMirror behavior", async ({
  page,
}) => {
  const root = tempCopy();
  await openTrusted(page, root);
  await page.getByTestId(`tree-file-${program}`).click();
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+ArrowUp");
  await page.keyboard.press("Home");

  const lines = page.getByTestId("editor").locator(".cm-line");
  const original = await lines.first().textContent();
  const before = await lineCount(page);
  await page.keyboard.press("Meta+d");
  await expect.poll(() => lineCount(page)).toBe(before + 1);
  await expect(lines.nth(0)).toHaveText(original ?? "");
  await expect(lines.nth(1)).toHaveText(original ?? "");

  await page.keyboard.press("Meta+Backspace");
  await expect.poll(() => lineCount(page)).toBe(before);

  // IntelliJ keeps the CodeMirror keymap: ⌘D selects the next occurrence, never edits.
  fs.writeFileSync(settingsFile, JSON.stringify({ keymap: { profile: "intellij" } }));
  await page.reload();
  await openTrusted(page, root);
  await page.getByTestId(`tree-file-${program}`).click();
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+ArrowUp");
  await page.keyboard.press("Home");
  const intellijBefore = await lineCount(page);
  const intellijFirst = await page.getByTestId("editor").locator(".cm-line").first().textContent();
  await page.keyboard.press("Meta+d");
  await expect.poll(() => lineCount(page)).toBe(intellijBefore);
  await expect(page.getByTestId("editor").locator(".cm-line").first()).toHaveText(intellijFirst ?? "");
});

test("Shift selection does not trigger Search Everywhere; two clean Shift taps do", async ({ page }) => {
  await openTrusted(page);
  await page.getByTestId(`tree-file-${program}`).click();
  await page.getByTestId("editor").click();

  await page.keyboard.press("Shift+ArrowUp");
  await page.keyboard.press("Shift+Home");
  await expect(page.getByTestId("find-action")).toHaveCount(0);

  await page.keyboard.press("Shift");
  await page.keyboard.press("Shift");
  await expect(page.getByTestId("find-action")).toBeVisible();
  await expect(page.getByTestId("search-tab-all")).toHaveAttribute("aria-selected", "true");
});
