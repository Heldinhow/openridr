import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

// Ticket 06 (SPEC-MEDIUM-NAV-SEARCH §9–10): docked Find Usages keeps the bottom tool window,
// notices overlay instead of pushing the editor, and the tools drawer never scrolls sideways.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const solution = path.join(fixture, "OrderDesk.sln");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(180_000);

function tempCopy(prefix: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  return { base, copy };
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "lsp.stop", params: {} } }).catch(() => {});
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } }).catch(() => {});
});

async function dockedFindUsages(page: Page, viewport: { width: number; height: number }) {
  await page.setViewportSize(viewport);
  const { base, copy } = tempCopy("openridr-layout-usages-");
  try {
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-trust").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    await page.getByTestId("workspace-tools-toggle").click();
    // tools.plan is inert. Only an already present launcher is reused; nothing is installed here.
    const planned = await (
      await page.request.post(`${host}/call`, {
        data: { method: "tools.plan", params: { tool: "csharp-ls" } },
      })
    ).json();
    const existing = planned.data.tools["csharp-ls"].plan.existing.state;
    test.skip(
      existing !== "present",
      `csharp-ls is not installed in this E2E home (existing: ${existing}); not verified, nothing is installed by this spec`,
    );
    const reused = await (
      await page.request.post(`${host}/call`, {
        data: {
          method: "tools.install",
          params: { planId: planned.data.tools["csharp-ls"].plan.planId, confirm: true },
        },
      })
    ).json();
    expect(reused.data.tools["csharp-ls"].reused, "the present launcher is reused").toBe(true);
    await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 120_000 });
    // Partial coverage still serves references for the loaded projects; "starting" does not.
    await expect(page.getByTestId("lsp-state")).toContainText(/Ready|Partial coverage/, { timeout: 90_000 });

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

    const output = page.getByRole("region", { name: "Bottom panel" });
    const outputBox = (await output.boundingBox())!;
    expect(outputBox.height, "the bottom tool window keeps its configured height, not 96px").toBeGreaterThanOrEqual(200);
    const panelBox = (await panel.boundingBox())!;
    expect(panelBox.y + panelBox.height, "the docked preview stays above the tool window").toBeLessThanOrEqual(outputBox.y + 1);
    await expect(page.getByTestId("bottom-tab-build")).toBeVisible();
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
}

for (const viewport of [{ width: 1440, height: 900 }, { width: 1100, height: 700 }]) {
  test(`docked Find Usages stays above the bottom tool window at ${viewport.width}x${viewport.height}`, async ({ page }) => {
    await dockedFindUsages(page, viewport);
  });
}

test("notifications overlay the editor without moving tabs or editor", async ({ page }) => {
  const { base, copy } = tempCopy("openridr-layout-notice-");
  try {
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-restricted").click();
    await expect(page.getByTestId("tree")).toBeVisible();

    const tabbar = page.locator(".tabbar");
    const editor = page.getByRole("region", { name: "Editor" });
    const tabbarBefore = (await tabbar.boundingBox())!;
    const editorBefore = (await editor.boundingBox())!;

    fs.rmSync(path.join(copy, "src/OrderDesk.Core/Order.cs"));
    await page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs").click();
    await expect(page.getByTestId("app-log")).toBeVisible();

    const tabbarAfter = (await tabbar.boundingBox())!;
    const editorAfter = (await editor.boundingBox())!;
    expect(Math.abs(tabbarAfter.y - tabbarBefore.y), "notices do not push the tab bar").toBeLessThanOrEqual(1);
    expect(Math.abs(editorAfter.y - editorBefore.y), "notices do not push the editor").toBeLessThanOrEqual(1);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

for (const viewport of [{ width: 1440, height: 900 }, { width: 1100, height: 700 }]) {
  test(`workspace tools drawer has no horizontal overflow at ${viewport.width}x${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(solution);
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-restricted").click();
    await page.getByTestId("workspace-tools-toggle").click();
    await expect(page.getByTestId("install-panel")).toBeVisible();
    await expect(page.getByTestId("install-reinstall")).toBeVisible();

    const drawer = page.getByTestId("workspace-tools");
    const metrics = await drawer.evaluate((element) => ({
      scrollWidth: element.scrollWidth,
      clientWidth: element.clientWidth,
      scrollLeft: element.scrollLeft,
    }));
    expect(metrics.scrollWidth, "the drawer does not overflow horizontally").toBeLessThanOrEqual(metrics.clientWidth + 1);
    expect(metrics.scrollLeft, "the drawer is not scrolled sideways").toBe(0);

    // The diagnostic panel scrolls in the drawer itself, never sideways and never inside a nested plan.
    const view = await drawer.evaluate((element) => ({
      scrolls: element.scrollHeight > element.clientHeight,
      overflowY: getComputedStyle(element).overflowY,
      top: element.getBoundingClientRect().top,
      bottom: element.getBoundingClientRect().bottom,
    }));
    if (viewport.height < 900) {
      expect(view.scrolls && view.overflowY !== "hidden", "the drawer scrolls vertically").toBe(true);
    }
    const panelBox = (await page.getByTestId("install-panel").boundingBox())!;
    expect(panelBox.y, "the panel starts inside the drawer").toBeGreaterThanOrEqual(view.top - 1);
  });
}
