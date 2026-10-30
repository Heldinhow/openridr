import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

// Text of the Build/Run/Debug gates in src/executionGates.ts, asserted at real seams.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(180_000);

async function openTrustedRestored(page: Page) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-gates-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-checkbox").check();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", { timeout: 60_000 });
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });
  return base;
}

test("Run is gated until the startup project is built", async ({ page }) => {
  const base = await openTrustedRestored(page);
  try {
    await page.getByTestId("bottom-tab-build").click();
    await expect(page.getByTestId("run-disabled-reason")).toContainText(
      "Build the startup project first; Run never builds or restores implicitly.",
    );
    await expect(page.getByTestId("run-action")).toBeDisabled();
    await page.getByTestId("build-action").click();
    await expect(page.getByTestId("build-state")).toContainText("Build succeeded", { timeout: 60_000 });
    await expect(page.getByTestId("run-action")).toBeEnabled();
    await page.getByTestId("build-configuration").selectOption("Release");
    // Switching configuration re-evaluates the model, which stales the saved build.
    await expect(page.getByTestId("run-disabled-reason")).toContainText(
      "The saved build is stale: The project model was re-evaluated",
      { timeout: 30_000 },
    );
    await expect(page.getByTestId("run-action")).toBeDisabled();
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("Build and Run name Restricted mode as the reason", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(fixture, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("bottom-tab-build").click();
  await expect(page.getByTestId("build-disabled-reason")).toContainText(
    "Trust the workspace before building; nothing runs in Restricted mode.",
  );
  await expect(page.getByTestId("run-disabled-reason")).toContainText(
    "Trust the workspace before running; nothing runs in Restricted mode.",
  );
});
