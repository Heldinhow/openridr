import { expect, test, type Page } from "@playwright/test";
import path from "node:path";

const solution = path.resolve(process.cwd(), "fixtures/OrderDesk/OrderDesk.sln");

async function openWorkspace(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
  const trust = page.getByTestId("trust-prompt-trust");
  await trust.waitFor({ state: "visible", timeout: 60_000 }).catch(() => undefined);
  if (await trust.isVisible().catch(() => false)) await trust.click();
  await page.getByTestId("tool-windows").waitFor({ state: "visible", timeout: 90_000 });
}

// #336: an empty list teaches only what works, and a failed open names the repair next to the
// host's own message.
test("empty welcome teaches only the open shortcut and a bad path names the repair", async ({ page }) => {
  await page.goto("/");
  const keys = page.getByTestId("welcome-keys");
  await expect(keys).toContainText("open project");
  await expect(keys).not.toContainText("select");
  await expect(keys).not.toContainText("search");

  await page.getByTestId("workspace-path-input").fill("/tmp/openridr-336-missing.sln");
  await page.getByTestId("workspace-open").click();
  const error = page.getByTestId("workspace-error");
  await expect(error).toContainText("not found");
  await expect(error).toContainText("Check the path and that the file exists, then try again.");
});

// The empty editor lists only shortcuts that can act without a file.
test("empty editor does not teach actions that need a file", async ({ page }) => {
  await openWorkspace(page, solution);
  const empty = page.locator(".empty-editor");
  await expect(empty).toContainText("Search Everywhere");
  await expect(empty).not.toContainText("Save file");
  await expect(empty).not.toContainText("Go to definition");
  await expect(empty).not.toContainText("Rename symbol");
});

// A disabled menu item explains itself in title/aria-description, not as a paragraph in the menu.
test("disabled context-menu items explain themselves outside the menu body", async ({ page }) => {
  await openWorkspace(page, solution);
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await expect(page.getByTestId("editor")).toBeVisible();
  await page.getByTestId("editor").click({ button: "right" });
  const menu = page.locator(".editor-context-menu");
  await expect(menu).toBeVisible();
  await expect(menu.locator(".context-menu-reason")).toHaveCount(0);
  const explained = menu.locator("button[aria-description]");
  await expect(explained.first()).toBeAttached();
});
