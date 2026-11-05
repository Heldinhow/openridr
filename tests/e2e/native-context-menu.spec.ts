import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { e2eHome, host } from "./environment";

// The acceptance run of 0.3.0-alpha.7 found that "Show menu" from accessibility (AXShowMenu, what a
// screen reader or agent-desktop sends) opened WebKit's own menu — "Reload / Share… / Services" — on
// rows that had no handler. Root, project, folder sources and files answered; the .sln folder row and
// every Dependencies row did not. Shift+F10 did nothing anywhere, because WebKit turns neither it nor
// the menu key into a contextmenu event.
// A browser cannot show WebKit's menu, so this spec proves the two things that suppress it: the row
// cancels the contextmenu event and the app menu opens for every row, by mouse and by keyboard.
// The native-menu guard itself is unit-tested (tests/unit/nativeMenu.test.ts) and proven on the real
// app in docs/evidence/native-context-menu.md.

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const settingsFile = path.join(e2eHome, "settings.json");
test.use({ viewport: { width: 1440, height: 900 } });

function tempCopy() {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-menu-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  return copy;
}

async function open(page: Page, root: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(root, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  // Packages and frameworks come from MSBuild, so the rows below need the evaluated model.
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 90_000 });
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
  // The keymap profile is host settings, shared by every spec: leave the default behind.
  fs.writeFileSync(settingsFile, "{}\n");
});

/** Right-click a row and expect the app menu, never a row without one. */
async function menuFrom(page: Page, testId: string, item: string) {
  await page.getByTestId(testId).click({ button: "right" });
  await expect(page.getByTestId("tree-context-menu")).toBeVisible();
  await expect(page.getByTestId(item)).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("tree-context-menu")).toHaveCount(0);
}

test("every row opens the app menu and cancels the native one", async ({ page }) => {
  test.setTimeout(180_000);
  await open(page, tempCopy());

  // The row cancels the event: this is what keeps WebKit from showing "Reload / Share… / Services".
  const cancelled = await page.evaluate(() => {
    const row = document.querySelector('[data-testid="tree-folder-src"]');
    if (!row) throw new Error("tree-folder-src is missing");
    return !row.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
  });
  expect(cancelled).toBe(true);
  await page.keyboard.press("Escape");

  // .sln folder: the solution's commands, without path actions a virtual folder cannot honour.
  await page.getByTestId("tree-folder-src").click({ button: "right" });
  await expect(page.getByTestId("tree-context-menu")).toHaveAccessibleName("Solution folder context menu");
  await expect(page.getByTestId("tree-menu-build-solution")).toBeVisible();
  await expect(page.getByTestId("tree-menu-copy-path")).toHaveCount(0);
  // "Add Project…" adds to the solution root, so it is not offered on a folder.
  await expect(page.getByTestId("tree-menu-add-project")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("tree-context-menu")).toHaveCount(0);

  // Root keeps the full solution menu (control: it always worked).
  await menuFrom(page, "tree-root", "tree-menu-add-project");

  // Dependencies, its groups and its leaves name the owning project, so they open the project menu.
  await page.getByTestId("tree-deps-OrderDesk.Cli").click();
  await menuFrom(page, "tree-deps-group-OrderDesk.Cli-projects", "tree-menu-reload");
  await page.getByTestId("tree-deps-group-OrderDesk.Cli-projects").click();
  await menuFrom(page, "tree-ref-OrderDesk.Cli-OrderDesk.Core", "tree-menu-reload");
  await page.getByTestId("tree-deps-group-OrderDesk.Cli-frameworks").click();
  await menuFrom(page, "tree-framework-OrderDesk.Cli-Microsoft.NETCore.App (net8.0)", "tree-menu-reload");

  await page.getByTestId("tree-deps-OrderDesk.Tests").click();
  await page.getByTestId("tree-deps-group-OrderDesk.Tests-packages").click();
  await menuFrom(page, "tree-package-OrderDesk.Tests-xunit", "tree-menu-nuget");

  // Keyboard: Shift+F10 and the menu key open the same menu, and Escape returns the focus to the row.
  const file = page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs");
  await file.focus();
  await page.keyboard.press("Shift+F10");
  await expect(page.getByTestId("tree-context-menu")).toHaveAccessibleName("File context menu");
  await expect(page.getByTestId("tree-menu-open")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(file).toBeFocused();

  await page.keyboard.press("ContextMenu");
  await expect(page.getByTestId("tree-context-menu")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(file).toBeFocused();

  // A mouse right-click need not move the focus; closing must still land on the row that opened it.
  await page.getByTestId("tree-project-OrderDesk.Core").click({ button: "right" });
  await expect(page.getByTestId("tree-context-menu")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("tree-project-OrderDesk.Core")).toBeFocused();
});

test("IntelliJ profile: a tree row keeps Shift+F10, outside the tree the keymap runs", async ({ page }) => {
  test.setTimeout(180_000);
  // IntelliJ maps Shift+F10 to Run All Tests on macOS; the tree must win while a row has the focus.
  fs.writeFileSync(settingsFile, JSON.stringify({ keymap: { profile: "intellij" } }));
  await open(page, tempCopy());

  const file = page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs");
  await file.focus();
  await page.keyboard.press("Shift+F10");
  await expect(page.getByTestId("tree-context-menu")).toHaveAccessibleName("File context menu");
  await expect(page.getByTestId("tree-menu-open")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("tree-context-menu")).toHaveCount(0);
  await expect(file).toBeFocused();

  // Outside the tree the same key stays the keymap's: the run selects the Tests tool window.
  await file.click();
  await expect(page.getByTestId("editor")).toBeVisible();
  await page.getByTestId("editor").click();
  await page.keyboard.press("Shift+F10");
  await expect(page.getByTestId("tree-context-menu")).toHaveCount(0);
  await expect(page.getByTestId("bottom-tab-tests")).toHaveAttribute("aria-selected", "true");
});
