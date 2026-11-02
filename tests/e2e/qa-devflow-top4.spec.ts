import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import { revisionOf, settleRestore } from "./automatic";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { e2eHome, host } from "./environment";

/**
 * SPEC-DEVFLOW-TOP4.md, finding 1: the editor must scroll to the current debug line when it
 * changes and is off screen. A real netcoredbg session stops far below the fold with the
 * editor at the top; the marker only exists in the DOM when CodeMirror renders that line.
 */
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const debugToolDir = path.join(e2eHome, "tools/netcoredbg");
const realAdapter = path.join(
  process.env.HOME ?? "",
  "Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092",
);

test.use({ viewport: { width: 1440, height: 900 } });
test.describe.configure({ timeout: 180_000 });

async function hostCall(request: APIRequestContext, method: string, params = {}) {
  const response = await request.post(`${host}/call`, { data: { method, params } });
  return response.json();
}

async function selectEvaluationTfm(page: Page, value: string) {
  const summary = page.getByTestId("workspace-actions-menu").locator("summary");
  await summary.click();
  const tfm = page.getByTestId("eval-tfm");
  await expect(tfm).toBeVisible();
  await tfm.selectOption(value);
  await summary.click();
  await expect(tfm).toBeHidden();
  await expect(tfm).toHaveValue(value);
}

test("achado 1: the editor scrolls to the current execution line when it is off screen", async ({
  page,
  request,
}) => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-top4-scroll-"));
  const copy = path.join(base, "Order Desk scroll");
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  const program = path.join(copy, "src/OrderDesk.Cli/Program.cs");
  const comments = Array.from({ length: 70 }, (_, index) => `// filler ${index + 1}`);
  const stopLine = comments.length + 2;
  fs.writeFileSync(
    program,
    ["var label = \"bottom\";", ...comments, "Console.WriteLine(label);", ""].join("\n"),
  );
  fs.rmSync(debugToolDir, { recursive: true, force: true });
  fs.mkdirSync(debugToolDir, { recursive: true });
  fs.cpSync(realAdapter, path.join(debugToolDir, "3.2.0-1092"), { recursive: true });

  try {
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-trust").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    await page.getByTestId("workspace-tools-toggle").click();
    await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });

    expect((await hostCall(request, "debug.flag", { enabled: true, confirm: true })).ok).toBe(true);
    const planned = await hostCall(request, "debug.plan");
    const installed = await hostCall(request, "debug.install", {
      planId: planned.data.debug.plan.planId,
      confirm: true,
    });
    expect(installed.data.debug.adapterState).toBe("ready");

    await settleRestore(page);
    await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(2);
    await selectEvaluationTfm(page, "net8.0");
    await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(3);

    const breakpoint = await hostCall(request, "debug.setBreakpoints", {
      path: program,
      lines: [stopLine],
    });
    expect(breakpoint.ok, JSON.stringify(breakpoint.error)).toBe(true);

    // Open the file in the editor and leave it at the top: the stop line is far below the fold.
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await expect(page.getByTestId("editor")).toBeVisible();
    const scroller = page.locator(".cm-scroller");
    await scroller.evaluate((element) => {
      element.scrollTop = 0;
    });
    expect(await scroller.evaluate((element) => element.scrollTop)).toBe(0);

    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("build-action").click();
    await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
      timeout: 60_000,
    });

    // Launch from the Debug panel so the editor keeps its scroll position and focus stays out of it.
    await page.getByTestId("bottom-tab-debug").click();
    await page.getByTestId("debug-launch").click();
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });

    await expect.poll(() => scroller.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
    const current = page.locator(".cm-line-current");
    await expect(current).toHaveCount(1);
    await expect(current).toContainText("Console.WriteLine");
    expect(
      await current.evaluate((line) => {
        const viewport = line.closest(".cm-scroller")!.getBoundingClientRect();
        const content = line.getBoundingClientRect();
        return content.top >= viewport.top && content.bottom <= viewport.bottom;
      }),
    ).toBe(true);
    // The automatic scroll never pulls focus from the Debug panel into the editor.
    expect(
      await page
        .getByTestId("editor-shell")
        .evaluate((shell) => shell.contains(document.activeElement)),
    ).toBe(false);
  } finally {
    await hostCall(request, "debug.stop");
    await hostCall(request, "workspace.close");
    fs.rmSync(base, { recursive: true, force: true });
  }
});
