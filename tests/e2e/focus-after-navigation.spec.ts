import { expect, test, type Page } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";

const fixture = path.resolve("fixtures/OrderDesk");
const order = "src/OrderDesk.Core/Order.cs";
const program = "src/OrderDesk.Cli/Program.cs";

async function openRestricted(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(fixture, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-restricted").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function openSearch(page: Page) {
  await page.keyboard.press("Shift");
  await page.keyboard.press("Shift");
  await expect(page.getByTestId("find-action")).toBeVisible();
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("a file opened from Search Everywhere leaves the editor focused", async ({ page }) => {
  await openRestricted(page);
  await openSearch(page);
  await page.getByTestId("search-tab-files").click();
  await page.getByTestId("find-action-input").fill("Order.cs");
  const item = page
    .locator('[data-testid^="search-item-"]')
    .filter({ hasText: order })
    .first();
  await expect(item).toBeVisible();
  await item.click();

  await expect(page.getByTestId(`tab-${order}`)).toBeVisible();
  await expect(page.getByTestId("editor")).toBeFocused();
  await page.keyboard.press("Meta+f");
  await expect(page.locator(".cm-search")).toBeVisible();
});

test("a Search Everywhere location leaves the editor focused", async ({ page }) => {
  await openRestricted(page);
  await openSearch(page);
  await page.getByTestId("search-tab-text").click();
  await page.getByTestId("find-action-input").fill("A-104");
  const item = page.locator('[data-testid^="search-item-"]').first();
  await expect(item).toBeVisible();
  await item.click();

  await expect(page.getByTestId(`tab-${program}`)).toBeVisible();
  await expect(page.getByTestId("editor")).toBeFocused();
});

test("a Find in Files result leaves the editor focused", async ({ page }) => {
  await openRestricted(page);
  await page.getByTestId("tree-expand-all").click();
  await page.getByTestId(`tree-file-${program}`).click();
  await page.getByTestId("editor").click();

  await page.keyboard.press("Meta+Shift+F");
  const dialog = page.getByTestId("find-in-files");
  await expect(dialog).toBeVisible();
  await page.getByTestId("find-in-files-input").fill("A-104");
  await expect(page.getByTestId("find-in-files-item-0")).toBeVisible();
  await page.getByTestId("find-in-files-item-0").click();

  await expect(dialog).toHaveCount(0);
  await expect(page.getByTestId("editor")).toBeFocused();
  await page.keyboard.press("Meta+f");
  await expect(page.locator(".cm-search")).toBeVisible();
});

test("a plain tree click keeps the focus in the tree", async ({ page }) => {
  await openRestricted(page);
  await page.getByTestId("tree-expand-all").click();
  const file = page.getByTestId(`tree-file-${program}`);
  await file.click();

  await expect(file).toBeFocused();
  await expect(page.getByTestId("editor")).not.toBeFocused();
  await page.keyboard.press("Meta+f");
  await expect(page.locator(".cm-search")).toHaveCount(0);
});
