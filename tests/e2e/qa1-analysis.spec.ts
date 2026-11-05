import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { cleanup, confirmToolPlan, hostCall, hostState, openTrusted, orderDesk, tempCopy } from "./qa-top4";

test.use({ viewport: { width: 1440, height: 900 } });
test.describe.configure({ timeout: 240_000 });

test.afterEach(async ({ request }) => cleanup(request));

test.describe("1 - analysis starts without the hidden shield path", () => {
  test("tool ready: the trusted project starts the analysis on its own", async ({
    page,
    request,
  }) => {
    const { base, copy } = tempCopy(orderDesk, "Order Desk qa1b");
    try {
      await openTrusted(page, path.join(copy, "OrderDesk.sln"));
      // The host only runs an existing launcher after the plan is confirmed (decisions: tools).
      await confirmToolPlan(request);
      await expect
        .poll(async () => (await hostState(request)).lsp.state, { timeout: 150_000 })
        .toMatch(/starting|loading|ready|partial|awaiting_diagnostics/);
      await expect(page.getByTestId("status-analysis")).toContainText(
        /Starting|Loading|Ready|Partial|Awaiting/,
      );

      // A user stop is respected: no silent restart, and the visible button offers it back.
      await expect
        .poll(async () => (await hostState(request)).lsp.state, { timeout: 150_000 })
        .toMatch(/ready|partial|awaiting_diagnostics/);
      await hostCall(request, "lsp.stop");
      await expect(page.getByTestId("status-analysis-action")).toHaveText("Start analysis");
      await page.waitForTimeout(1500);
      expect((await hostState(request)).lsp.state).toBe("stopped");
    } finally {
      fs.rmSync(base, { recursive: true, force: true });
    }
  });
});

