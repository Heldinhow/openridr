import { expect, test, type Page } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";

// Ticket 11: a breakpoint on a blank or comment-only line is accepted, marked as "no code"
// and never moved to the next line.
const fixture = path.resolve("fixtures/OrderDesk");
const program = "src/OrderDesk.Cli/Program.cs";
const hint = "No code on this line; the debugger will not stop here.";

async function openTrusted(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(fixture, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("tree-expand-all").click();
  await page.getByTestId(`tree-file-${program}`).click();
  await expect(page.getByTestId("editor")).toBeVisible();
}

/** Clicks the debug gutter next to the 0-based line index. */
async function clickGutter(page: Page, index: number) {
  const line = (await page.locator(".cm-line").nth(index).boundingBox())!;
  const gutter = (await page.locator(".cm-debug-gutter").boundingBox())!;
  await page.mouse.click(gutter.x + 5, line.y + line.height / 2);
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("a blank line breakpoint stays muted until the adapter binds it", async ({ page }) => {
  await openTrusted(page);

  // Program.cs line 2 is blank.
  await clickGutter(page, 1);
  await expect(page.locator(".cm-debug-no-code")).toHaveCount(1);
  await expect(page.locator(".cm-debug-no-code")).toHaveAttribute("title", hint);
  await expect(page.locator(".cm-line-breakpoint")).toHaveCount(0);
  await page.getByTestId("bottom-tab-debug").click();
  await expect(page.getByTestId("debug-breakpoints")).toContainText(`${program}:2`);

  // A code line keeps the normal filled marker and line tint.
  await clickGutter(page, 2);
  await expect(page.locator(".cm-debug-no-code")).toHaveCount(1);
  await expect(page.locator(".cm-debug-breakpoint")).toHaveCount(1);
  await expect(page.locator(".cm-line-breakpoint")).toHaveCount(1);
  await expect(page.getByTestId("debug-breakpoints")).toContainText(`${program}:3`);

  // Removing the blank-line breakpoint leaves only the code one.
  await clickGutter(page, 1);
  await expect(page.locator(".cm-debug-no-code")).toHaveCount(0);
  await expect(page.getByTestId("debug-breakpoints")).not.toContainText(`${program}:2`);
});

test("a comment-only line is also accepted as no code", async ({ page }) => {
  await openTrusted(page);
  await page.locator(".cm-line").nth(1).click();
  await page.keyboard.type("// note");
  await clickGutter(page, 1);
  await expect(page.locator(".cm-debug-no-code")).toHaveCount(1);
  await expect(page.locator(".cm-debug-no-code")).toHaveAttribute("title", hint);
  await expect(page.locator(".cm-line-breakpoint")).toHaveCount(0);
});
