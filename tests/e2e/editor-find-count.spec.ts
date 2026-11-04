import { expect, test, type Page } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";

const solution = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
const order = "src/OrderDesk.Core/Order.cs";

async function openOrder(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-restricted").click();
  await page.getByTestId("tree-file-" + order).click();
  await expect(page.getByTestId("editor")).toBeVisible();
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+ArrowUp");
}

async function openFind(page: Page) {
  await page.keyboard.press("Meta+f");
  const panel = page.locator(".cm-search");
  await expect(panel).toBeVisible();
  return panel;
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("the Find panel counts matches and follows navigation and document edits", async ({
  page,
}) => {
  await openOrder(page);
  const panel = await openFind(page);
  const count = page.getByTestId("editor-find-count");
  const search = panel.locator('input[name="search"]');

  await expect(count).toHaveText("0 of 0");
  await search.pressSequentially("Order");
  await expect(count).toHaveText("1 of 3");

  await panel.getByRole("button", { name: "next", exact: true }).click();
  await expect(count).toHaveText("1 of 3");
  await panel.getByRole("button", { name: "next", exact: true }).click();
  await expect(count).toHaveText("2 of 3");
  await panel.getByRole("button", { name: "previous", exact: true }).click();
  await expect(count).toHaveText("1 of 3");

  await panel.getByRole("button", { name: "all", exact: true }).click();
  await expect(count).toHaveText("1 of 3");

  // A live edit changes m; undo restores it.
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+ArrowUp");
  await page.keyboard.type("Order");
  await expect(count).toHaveText("1 of 4");
  await page.keyboard.press("Meta+z");
  await expect(count).toHaveText("1 of 3");

  await search.click();
  await search.press("Meta+a");
  await search.press("Backspace");
  await expect(count).toHaveText("0 of 0");
  await search.pressSequentially("zzz");
  await expect(count).toHaveText("0 of 0");
});

test("the counter follows case, regexp and whole-word options", async ({ page }) => {
  await openOrder(page);
  const panel = await openFind(page);
  const count = page.getByTestId("editor-find-count");
  const search = panel.locator('input[name="search"]');
  const caseBox = panel.locator('input[name="case"]');
  const regexpBox = panel.locator('input[name="re"]');
  const wordBox = panel.locator('input[name="word"]');

  // The default panel has exactly the three option checkboxes; no "in selection" box.
  await expect(panel.locator('input[type="checkbox"]')).toHaveCount(3);
  await expect(panel).not.toContainText("selection");

  await search.pressSequentially("order");
  await expect(count).toHaveText("1 of 3");

  await caseBox.check();
  await expect(count).toHaveText("0 of 0");
  await caseBox.uncheck();
  await expect(count).toHaveText("1 of 3");

  await wordBox.check();
  await expect(count).toHaveText("1 of 2");
  await wordBox.uncheck();

  await search.click();
  await search.press("Meta+a");
  await search.press("Backspace");
  await regexpBox.check();
  await search.pressSequentially("Order\\(");
  await expect(count).toHaveText("1 of 1");

  await search.press("Meta+a");
  await search.press("Backspace");
  await expect(count).toHaveText("0 of 0");
});

test("replace all keeps working and the counter follows the new text", async ({ page }) => {
  await openOrder(page);
  const panel = await openFind(page);
  const count = page.getByTestId("editor-find-count");
  const search = panel.locator('input[name="search"]');
  const replace = panel.locator('input[name="replace"]');

  await search.pressSequentially("Order");
  await expect(count).toHaveText("1 of 3");

  await replace.pressSequentially("Item");
  await panel.getByRole("button", { name: "replace all", exact: true }).click();
  await expect(count).toHaveText("0 of 0");
  await expect(page.getByTestId("editor")).toContainText("Item");

  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+z");
  await expect(count).toHaveText("1 of 3");
});
