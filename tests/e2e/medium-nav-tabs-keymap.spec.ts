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
  await expect(page.getByTestId(`tab-${program}`)).toHaveClass(/active/);
  await page.getByTestId(`tree-file-${order}`).click();
  await expect(page.getByTestId(`tab-${order}`)).toHaveClass(/active/);
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

test("⇧⌘O opens Go to File on the Files tab and ⌘T opens Search Everywhere", async ({ page }) => {
  await openTrusted(page);
  await page.getByTestId(`tree-file-${program}`).click();
  await page.getByTestId("editor").click();

  await page.keyboard.press("Meta+Shift+O");
  await expect(page.getByTestId("find-action")).toBeVisible();
  await expect(page.getByTestId("search-tab-files")).toHaveAttribute("aria-selected", "true");
  await page.getByTestId("find-action-input").press("Escape");
  await expect(page.getByTestId("find-action")).toHaveCount(0);

  await page.keyboard.press("Meta+t");
  await expect(page.getByTestId("find-action")).toBeVisible();
  await expect(page.getByTestId("search-tab-all")).toHaveAttribute("aria-selected", "true");
  await page.getByTestId("find-action-input").press("Escape");
});

test("⌘1..⌘8 focus the eight tool windows; repeating the active one toggles it", async ({ page }) => {
  await openTrusted(page);
  await page.getByTestId(`tree-file-${program}`).click();
  await page.getByTestId("editor").click();

  const band = [
    [1, "solution"], [2, "commit"], [3, "build"], [4, "run"],
    [5, "problems"], [6, "debug"], [7, "terminal"], [8, "tests"],
  ] as const;
  // Start from Commit and a non-default bottom tab so each ⌘N in the loop focuses, not toggles.
  await page.keyboard.press("Meta+2");
  await expect(page.getByTestId("toolband-commit")).toHaveAttribute("aria-pressed", "true");
  await page.keyboard.press("Meta+4");
  await expect(page.getByTestId("toolband-run")).toHaveAttribute("aria-pressed", "true");
  for (const [key, id] of band) {
    // The keymap ignores text fields by design; xterm focuses its own textarea, so return to the editor.
    await page.getByTestId("editor").click();
    await page.keyboard.press(`Meta+${key}`);
    await expect(page.getByTestId(`toolband-${id}`)).toHaveAttribute("aria-pressed", "true");
  }

  // The same shortcut on the active window closes it and opens it again.
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+8");
  await expect(page.getByTestId("toolband-tests")).toHaveAttribute("aria-pressed", "false");
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+8");
  await expect(page.getByTestId("toolband-tests")).toHaveAttribute("aria-pressed", "true");
  // Commit is still the active left window, so the first ⌘1 focuses Solution before toggling.
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+1");
  await expect(page.getByTestId("toolband-solution")).toHaveAttribute("aria-pressed", "true");
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+1");
  await expect(page.getByTestId("toolband-solution")).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByTestId("tree")).toHaveCount(0);
  await page.keyboard.press("Meta+1");
  await expect(page.getByTestId("toolband-solution")).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("tree")).toBeVisible();
});

test("Settings list the keymap keepers and the new defaults; the start hint names Double Shift", async ({
  page,
}) => {
  await openTrusted(page);

  // The empty editor hint shows the ⌘T binding plus the fixed gesture.
  const hint = page.locator(".empty-editor .shortcut-list");
  await expect(hint).toContainText("⌘T");
  await expect(hint).toContainText("Double Shift");

  await page.keyboard.press("Meta+,");
  await expect(page.getByTestId("settings-dialog")).toBeVisible();
  await page.getByTestId("settings-page-keymap").click();
  await expect(page.getByTestId("keymap-gestures")).toContainText("Double Shift");
  await expect(page.getByTestId("keymap-shortcut-search.everywhere")).toHaveText("⌘T");
  await expect(page.getByTestId("keymap-shortcut-code.findUsages")).toHaveText("⇧F12");
  await expect(page.getByTestId("keymap-shortcut-search.symbol")).toHaveText("⌥⇧⌘T");
  const band = ["solution", "commit", "build", "run", "problems", "debug", "terminal", "tests"];
  for (const [index, id] of band.entries()) {
    await expect(page.getByTestId(`keymap-shortcut-toolWindow.${id}`)).toHaveText(`⌘${index + 1}`);
    await expect(page.getByTestId(`keymap-conflict-toolWindow.${id}`)).toHaveCount(0);
  }
  await expect(page.getByTestId("keymap-conflict-search.everywhere")).toHaveCount(0);

  // The IntelliJ profile keeps ⌥F7 and gets the same Search Everywhere and tool-window keys.
  await page.getByTestId("settings-cancel").click();
  fs.writeFileSync(settingsFile, JSON.stringify({ keymap: { profile: "intellij" } }));
  await page.reload();
  await openTrusted(page);
  await page.keyboard.press("Meta+,");
  await page.getByTestId("settings-page-keymap").click();
  await expect(page.getByTestId("keymap-active-profile")).toContainText("IntelliJ");
  await expect(page.getByTestId("keymap-shortcut-code.findUsages")).toHaveText("⌥F7");
  await expect(page.getByTestId("keymap-shortcut-search.everywhere")).toHaveText("⌘T");
  await expect(page.getByTestId("keymap-shortcut-toolWindow.solution")).toHaveText("⌘1");
  await expect(page.getByTestId("keymap-shortcut-toolWindow.tests")).toHaveText("⌘8");

  // Both Search Everywhere gestures also work in the IntelliJ profile.
  await page.getByTestId("settings-cancel").click();
  await page.keyboard.press("Meta+t");
  await expect(page.getByTestId("find-action")).toBeVisible();
  await page.getByTestId("find-action-input").press("Escape");
  await page.keyboard.press("Shift");
  await page.keyboard.press("Shift");
  await expect(page.getByTestId("find-action")).toBeVisible();
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

test("Find Action applies the requested search tab and Recent Files is disabled without history", async ({
  page,
}) => {
  await openTrusted(page);

  await page.keyboard.press("Meta+Shift+A");
  await page.getByTestId("find-action-input").fill("Recent Files");
  const recent = page.getByTestId("find-action-item-search.recentFiles").getByRole("button");
  await expect(recent).toBeDisabled();
  await expect(recent).toHaveAttribute("title", "No recent files yet.");

  await page.getByTestId("find-action-input").fill("Search Everywhere");
  await page.getByTestId("find-action-input").press("Enter");
  await expect(page.getByTestId("find-action")).toBeVisible();
  await expect(page.getByTestId("search-tab-all")).toHaveAttribute("aria-selected", "true");
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
