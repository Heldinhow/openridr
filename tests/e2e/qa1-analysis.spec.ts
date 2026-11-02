import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { e2eHome } from "./environment";
import { cleanup, confirmToolPlan, hostCall, hostState, openTrusted, orderDesk, tempCopy } from "./qa-top4";

test.use({ viewport: { width: 1440, height: 900 } });
test.describe.configure({ timeout: 240_000 });

test.afterEach(async ({ request }) => cleanup(request));

test.describe("1 - analysis starts without the hidden shield path", () => {
  test("not installed: a visible Install analysis button, nothing installs without the plan", async ({
    page,
    request,
  }) => {
    const appHome = (await hostState(request)).app.appHome as string;
    test.skip(appHome !== e2eHome, "needs the isolated E2E app home; never touches the real one");
    fs.rmSync(path.join(appHome, "tools/csharp-ls"), { recursive: true, force: true });
    // The host caches the tool state; tools.plan is the inert re-probe the UI itself uses.
    await hostCall(request, "tools.plan", { tool: "csharp-ls" });
    const { base, copy } = tempCopy(orderDesk, "Order Desk qa1");
    try {
      await openTrusted(page, path.join(copy, "OrderDesk.sln"));
      const action = page.getByTestId("status-analysis-action");
      await expect(action).toBeVisible();
      await expect(action).toHaveText("Install analysis");

      // Clicking only opens the tools window; it never installs.
      await action.click();
      await expect(page.getByTestId("install-plan-open")).toBeVisible();
      await page.waitForTimeout(1500);
      expect((await hostState(request)).tools["csharp-ls"].state).not.toBe("ready");
      expect((await hostState(request)).lsp.state).toBe("unavailable");

      // The user confirms the plan; once installed the analysis starts by itself.
      await page.getByTestId("install-plan-open").click();
      await page.getByTestId("install-confirm").click();
      await expect
        .poll(async () => (await hostState(request)).tools["csharp-ls"].state, {
          timeout: 180_000,
        })
        .toBe("ready");
      await expect(page.getByTestId("status-analysis")).toContainText(
        /Starting|Loading|Ready|Partial|Awaiting/,
        { timeout: 120_000 },
      );
      await expect(page.getByTestId("status-analysis-action")).toHaveCount(0);
    } finally {
      fs.rmSync(base, { recursive: true, force: true });
    }
  });

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

