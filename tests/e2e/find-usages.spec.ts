import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

// Find Usages against the real csharp-ls session (lsp.references). Needs csharp-ls already installed in the E2E home; it never installs.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(180_000);

async function state(page: Page) {
  const response = await page.request.post(`${host}/call`, { data: { method: "workspace.state", params: {} } });
  return (await response.json()).data;
}

test("Find Usages lists the real references of Order and opens one", async ({ page }) => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-usages-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  try {
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-trust").click();
    await expect(page.getByTestId("mode-badge")).toHaveText("Trusted mode");
    await page.getByTestId("workspace-tools-toggle").click();
    // tools.plan is inert. Only an already present launcher is reused (a valid one is never replaced); nothing is downloaded here.
    const planned = await (await page.request.post(`${host}/call`, { data: { method: "tools.plan", params: { tool: "csharp-ls" } } })).json();
    const existing = planned.data.tools["csharp-ls"].plan.existing.state;
    test.skip(existing !== "present", `csharp-ls is not installed in this E2E home (existing: ${existing}); not verified, nothing is installed by this spec`);
    const reused = await (await page.request.post(`${host}/call`, { data: { method: "tools.install", params: { planId: planned.data.tools["csharp-ls"].plan.planId, confirm: true } } })).json();
    expect(reused.data.tools["csharp-ls"].reused, "the present launcher is reused, not reinstalled").toBe(true);
    await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 120_000 });
    await page.getByTestId("lsp-start").click();
    await expect(page.getByTestId("lsp-state")).toContainText("Ready", { timeout: 90_000 });

    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await expect(page.getByTestId("editor")).toContainText("OrderDesk.Core");
    // Cursor inside `Order` of `var order = new Order(...)` (line index 2, character 17).
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+ArrowUp");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Home");
    for (let i = 0; i < 17; i += 1) await page.keyboard.press("ArrowRight");
    await expect(page.getByTestId("editor-shell")).toHaveAttribute("data-cursor", "2:17");

    await page.keyboard.press("Shift+F12");
    const panel = page.getByTestId("find-usages");
    await expect(panel).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("find-usages-summary")).toContainText(/reference/i, { timeout: 60_000 });
    await expect(page.getByTestId("find-usages-empty")).toHaveCount(0);
    // Uses across projects, grouped by file: the console project once and the test project three times.
    const files = panel.locator('[data-testid^="find-usages-file-"]');
    await expect(files.filter({ hasText: "tests/OrderDesk.Tests/OrderTests.cs (3)" })).toBeVisible();
    await expect(files.filter({ hasText: "src/OrderDesk.Cli/Program.cs (1)" })).toBeVisible();
    await expect(panel.getByTestId("find-usages-summary")).toContainText("4 references");
    await page.screenshot({ path: "docs/evidence/screenshots/d05-find-usages.png" });
    // find-usage-N restarts at 0 in each file group, so pick by the unique title (file:line:column).
    await panel.getByTitle("tests/OrderDesk.Tests/OrderTests.cs:11:25").click();
    await expect(page.getByTestId("tab-tests/OrderDesk.Tests/OrderTests.cs")).toBeVisible();
    await expect(page.getByTestId("editor-shell")).toHaveAttribute("data-cursor", "10:24");
  } finally {
    await page.request.post(`${host}/call`, { data: { method: "lsp.stop", params: {} } }).catch(() => {});
    await page.request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } }).catch(() => {});
    fs.rmSync(base, { recursive: true, force: true });
  }
});
