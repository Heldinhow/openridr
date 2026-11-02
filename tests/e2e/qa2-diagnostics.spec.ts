import { expect, test } from "@playwright/test";
import fs from "node:fs";
import { cleanup, confirmToolPlan, hostState, openEvaluated, program, orderDesk, tempCopy } from "./qa-top4";

test.use({ viewport: { width: 1440, height: 900 } });
test.describe.configure({ timeout: 240_000 });

test.afterEach(async ({ request }) => cleanup(request));

test.describe("2 - diagnostics show in the editor and the status bar", () => {
  test("build errors are underlined, marked in the gutter and counted", async ({ page }) => {
    const { base, copy } = tempCopy(orderDesk, "Order Desk qa2");
    try {
      await openEvaluated(page, copy);
      await page.getByTestId(`tree-file-${program}`).click();
      await page.getByTestId("editor").click();
      await page.keyboard.press("Meta+A");
      await page.keyboard.insertText('int x = "a";\nFoo();\n');
      await page.keyboard.press("Meta+s");
      await expect(page.getByTestId("app-log")).toContainText(`Saved ${program}`);
      await page.getByTestId("bottom-tab-build").click();
      await page.getByTestId("build-action").click();
      await expect(page.getByTestId("build-state")).toContainText("Build failed", {
        timeout: 60_000,
      });

      await expect(page.locator(".cm-diagnostic-error").first()).toBeVisible();
      await expect(page.locator(".cm-diagnostic-marker")).toHaveCount(2);
      await expect(page.getByTestId("status-diagnostics")).toContainText("2 errors");
    } finally {
      fs.rmSync(base, { recursive: true, force: true });
    }
  });

  test("analysis diagnostics are underlined too", async ({ page, request }) => {
    const { base, copy } = tempCopy(orderDesk, "Order Desk qa2b");
    try {
      await openEvaluated(page, copy);
      await confirmToolPlan(request);
      await expect
        .poll(async () => (await hostState(request)).lsp.state, { timeout: 150_000 })
        .toMatch(/ready|partial|awaiting_diagnostics/);
      await page.getByTestId(`tree-file-${program}`).click();
      await page.getByTestId("editor").click();
      await page.keyboard.press("Meta+A");
      await page.keyboard.insertText('int x = "a";\n');
      await page.keyboard.press("Meta+s");
      await expect(page.locator(".cm-diagnostic-error").first()).toBeVisible({ timeout: 90_000 });
      await expect(page.locator(".cm-diagnostic-marker").first()).toBeVisible();
      await expect(page.getByTestId("status-diagnostics")).toContainText(/[1-9]\d* error/);
    } finally {
      fs.rmSync(base, { recursive: true, force: true });
    }
  });
});

