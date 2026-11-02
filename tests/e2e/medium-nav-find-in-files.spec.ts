import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

const fixture = path.resolve("fixtures/OrderDesk");
const programTab = "src/OrderDesk.Cli/Program.cs";

function tempCopy() {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-find-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  return copy;
}

async function openTrusted(page: Page, root = fixture) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(root, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("⇧⌘F opens Find in Files and a result navigates to its line and column", async ({ page }) => {
  await openTrusted(page);
  await page.getByTestId("tree-expand-all").click();
  await page.getByTestId(`tree-file-${programTab}`).click();
  await page.getByTestId("editor").click();

  await page.keyboard.press("Meta+Shift+F");
  const dialog = page.getByTestId("find-in-files");
  await expect(dialog).toBeVisible();
  const input = page.getByTestId("find-in-files-input");
  await expect(input).toBeFocused();

  await input.fill("A-104");
  await expect(page.getByTestId("find-in-files-item-0")).toContainText("A-104");
  await page.getByTestId("find-in-files-item-0").click();

  await expect(dialog).toHaveCount(0);
  await expect(page.getByTestId(`tab-${programTab}`)).toBeVisible();
  await expect(page.getByTestId("editor-shell")).toHaveAttribute("data-cursor", "2:23");
});

test("the tree menu opens Find in Files with the clicked folder as scope", async ({ page }) => {
  const root = tempCopy();
  const domain = path.join(root, "src/OrderDesk.Core/Domain");
  fs.mkdirSync(domain, { recursive: true });
  fs.writeFileSync(
    path.join(domain, "OrderRules.cs"),
    'namespace OrderDesk.Core;\n\npublic static class OrderRules\n{\n    public const string Prefix = "Order";\n}\n',
  );
  await openTrusted(page, root);
  await page.getByTestId("tree-expand-all").click();
  await page.getByTestId("tree-dir-src/OrderDesk.Core/Domain").click({ button: "right" });
  await expect(page.getByTestId("tree-context-menu")).toBeVisible();
  await page.getByTestId("tree-menu-find").click();

  await expect(page.getByTestId("find-in-files-scope")).toContainText("src/OrderDesk.Core/Domain");
  await page.getByTestId("find-in-files-input").fill("Order");
  await expect(page.getByTestId("find-in-files-item-0")).toBeVisible();

  const paths = page.locator('[data-testid^="find-in-files-item-"] .action-group');
  const count = await paths.count();
  expect(count).toBeGreaterThan(0);
  for (let index = 0; index < count; index += 1) {
    await expect(paths.nth(index)).toHaveText(/^src\/OrderDesk\.Core\/Domain\//);
  }
});

test("the project menu scopes Find in Files to the project folder", async ({ page }) => {
  await openTrusted(page);
  await page.getByTestId("tree-expand-all").click();
  await page.getByTestId("tree-project-OrderDesk.Core").click({ button: "right" });
  await expect(page.getByTestId("tree-context-menu")).toBeVisible();
  await page.getByTestId("tree-menu-edit").hover();
  await page.getByTestId("tree-menu-find").click();

  await expect(page.getByTestId("find-in-files-scope")).toContainText("src/OrderDesk.Core");
  await page.getByTestId("find-in-files-input").fill("Order");

  const paths = page.locator('[data-testid^="find-in-files-item-"] .action-group');
  await expect(paths.filter({ hasText: "src/OrderDesk.Core/Order.cs" }).first()).toBeVisible();
  const count = await paths.count();
  expect(count).toBeGreaterThan(0);
  for (let index = 0; index < count; index += 1) {
    await expect(paths.nth(index)).toHaveText(/^src\/OrderDesk\.Core\//);
  }
});

test("Match case reaches the host: ORDER matches off and does not match on", async ({ page }) => {
  await openTrusted(page);
  await page.keyboard.press("Meta+Shift+F");
  const input = page.getByTestId("find-in-files-input");
  await input.fill("ORDER");
  await expect(page.getByTestId("find-in-files-item-0")).toBeVisible();

  const matchCase = page.getByTestId("find-in-files-case");
  await expect(matchCase).not.toBeChecked();
  await matchCase.check();
  await expect(page.getByTestId("find-in-files-empty")).toBeVisible();
});

test("Esc closes Find in Files", async ({ page }) => {
  await openTrusted(page);
  await page.keyboard.press("Meta+Shift+F");
  const dialog = page.getByTestId("find-in-files");
  await expect(dialog).toBeVisible();
  await page.getByTestId("find-in-files-input").press("Escape");
  await expect(dialog).toHaveCount(0);
});
