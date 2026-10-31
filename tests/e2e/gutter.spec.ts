import { expect, test } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { revisionOf, settleRestore } from "./automatic";
import { host } from "./environment";

// Gutter Run/Test (D-09): a real click on the test gutter runs that one test and shows its result.
// Needs a csharp-ls already installed in the E2E home (document symbols match the discovered tests); it never installs.
// The Run gutter is not covered: OrderDesk.Cli uses top-level statements, so there is no `Main` symbol to mark.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(240_000);

test("the test gutter runs one test from the editor and reports its result", async ({ page }) => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-gutter-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  try {
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-checkbox").check();
    await page.getByTestId("trust-grant").click();
    await expect(page.getByTestId("mode-trusted")).toBeVisible();
    await page.getByTestId("open-text-only").click();
    await page.getByTestId("workspace-tools-toggle").click();
    // tools.plan is inert; only an already present launcher is reused (nothing is downloaded here).
    const planned = await (await page.request.post(`${host}/call`, { data: { method: "tools.plan", params: { tool: "csharp-ls" } } })).json();
    const plan = planned.data.tools["csharp-ls"].plan;
    test.skip(plan.existing.state !== "present", `csharp-ls is not installed in this E2E home (existing: ${plan.existing.state}); not verified`);
    await page.request.post(`${host}/call`, { data: { method: "tools.install", params: { planId: plan.planId, confirm: true } } });
    await settleRestore(page);

    await page.getByTestId("bottom-tab-tests").click();
    const before = await revisionOf(page);
    await page.getByTestId("tests-tfm").selectOption("net8.0");
    await expect.poll(() => revisionOf(page), { timeout: 30_000 }).toBeGreaterThan(before);
    await expect(page.getByTestId("eval-state")).not.toContainText("stale");
    await expect(page.getByTestId("tests-discover")).toBeEnabled();
    await page.getByTestId("tests-discover").click();
    await expect(page.getByTestId("tests-state")).toContainText("Discovered 4 tests", { timeout: 120_000 });

    // Start analysis after the TFM re-evaluation, otherwise the session is stale for the new context.
    await page.getByTestId("lsp-start").click();
    await expect(page.getByTestId("lsp-state")).toContainText("Ready", { timeout: 90_000 });
    await page.getByTestId("tree-file-tests/OrderDesk.Tests/OrderTests.cs").click();
    await expect(page.getByTestId("editor")).toContainText("Id_is_preserved");
    const gutters = page.locator('[data-testid^="gutter-test-"]');
    await expect(gutters.first()).toBeVisible({ timeout: 15_000 });
    await page.screenshot({ path: "docs/evidence/screenshots/d09-gutter-test.png" });
    await gutters.first().click();
    await page.getByTestId("bottom-tab-tests").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Run completed", { timeout: 120_000 });
    await expect(page.getByTestId("tests-run-counts")).toContainText("1 result · 1 passed · 0 failed");
  } finally {
    await page.request.post(`${host}/call`, { data: { method: "lsp.stop", params: {} } }).catch(() => {});
    await page.request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } }).catch(() => {});
    fs.rmSync(base, { recursive: true, force: true });
  }
});
