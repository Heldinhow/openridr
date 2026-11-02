import { expect, test } from "@playwright/test";
import fs from "node:fs";
import { cleanup, openEvaluated, order, orderDesk, tempCopy } from "./qa-top4";

test.use({ viewport: { width: 1440, height: 900 } });
test.describe.configure({ timeout: 240_000 });

test.afterEach(async ({ request }) => cleanup(request));

test("3 - undo back to the saved text clears modified and the Build/Run prompts", async ({
  page,
}) => {
  const { base, copy } = tempCopy(orderDesk, "Order Desk qa3");
  try {
    await openEvaluated(page, copy);
    await page.getByTestId(`tree-file-${order}`).click();
    await page.getByTestId("editor").click();
    await page.keyboard.type("x");
    await expect(page.getByTestId("dirty-marker")).toBeVisible();
    await expect(page.getByTestId("status-dirty")).toContainText("1 modified buffer");

    await page.keyboard.press("Meta+z");
    await expect(page.getByTestId("dirty-marker")).toHaveCount(0);
    await expect(page.getByTestId("status-dirty")).toHaveCount(0);

    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("build-action").click();
    await expect(page.getByTestId("build-dirty-dialog")).toHaveCount(0);
    await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
      timeout: 60_000,
    });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

