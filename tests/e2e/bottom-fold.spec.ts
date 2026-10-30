import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { e2eHome, host } from "./environment";

async function setBreakpoint(request: APIRequestContext, file: string) {
  await request.post(`${host}/call`, { data: { method: "debug.setBreakpoints", params: { path: file, lines: [3] } } });
}

// Ticket 04: the content a user came for is inside the first fold of the bottom panel.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");

for (const viewport of [{ width: 1100, height: 700 }, { width: 1440, height: 900 }]) {
  test.describe(`bottom panel first fold at ${viewport.width}x${viewport.height}`, () => {
    test.use({ viewport });

    async function openTrusted(page: Page) {
      await page.goto("/");
      await page.getByTestId("workspace-path-input").fill(path.join(fixture, "OrderDesk.sln"));
      await page.getByTestId("workspace-open").click();
      await page.getByTestId("trust-checkbox").check();
      await page.getByTestId("trust-grant").click();
      await expect(page.getByTestId("mode-trusted")).toBeVisible();
      await page.getByTestId("open-text-only").click();
      await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 120_000 });
    }

    // A row is in the first fold when its bottom edge is inside the bottom panel's visible box.
    async function inFirstFold(page: Page, testId: string) {
      const panel = (await page.getByRole("region", { name: "Bottom panel" }).boundingBox())!;
      const target = (await page.getByTestId(testId).boundingBox())!;
      expect(target.y, `${testId} starts above the panel`).toBeGreaterThanOrEqual(panel.y);
      expect(target.y + Math.min(target.height, 24), `${testId} is below the first fold`).toBeLessThanOrEqual(panel.y + panel.height);
    }

    test("tests show the tree and counts without scrolling", async ({ page, request }) => {
      await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } }).catch(() => undefined);
      await openTrusted(page);
      await page.getByTestId("bottom-tab-tests").click();
      await page.getByTestId("tests-discover").click();
      await expect(page.getByTestId("tests-state")).toContainText("Discovered 4 tests", { timeout: 120_000 });
      await inFirstFold(page, "tests-items");
      await inFirstFold(page, "tests-counts");
    });

    test("debug frames and locals are in the first fold while paused", async ({ page, request }) => {
      const realAdapter = path.join(process.env.HOME ?? "", "Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092");
      test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
      const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-fold-debug-"));
      const copy = path.join(base, "Order Desk fold");
      fs.cpSync(fixture, copy, { recursive: true, filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source) });
      const toolDir = path.join(e2eHome, "tools/netcoredbg");
      fs.rmSync(toolDir, { recursive: true, force: true });
      fs.mkdirSync(toolDir, { recursive: true });
      fs.cpSync(realAdapter, path.join(toolDir, "3.2.0-1092"), { recursive: true });
      try {
        await page.goto("/");
        await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
        await page.getByTestId("workspace-open").click();
        await page.getByTestId("trust-checkbox").check();
        await page.getByTestId("trust-grant").click();
        await page.getByTestId("open-text-only").click();
        await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 60_000 });
        await setBreakpoint(request, path.join(copy, "src/OrderDesk.Cli/Program.cs"));
        await page.getByTestId("toolbar-debug").click({ force: true });
        await page.getByTestId("debug-consent-accept").click();
        await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 120_000 });
        await page.getByTestId("bottom-tab-debug").click();
        await inFirstFold(page, "debug-frames");
        await inFirstFold(page, "debug-locals");
        await page.getByTestId("debug-stop").click();
      } finally {
        await request.post(`${host}/call`, { data: { method: "debug.stop", params: {} } });
        fs.rmSync(base, { recursive: true, force: true });
      }
    });
  });
}
