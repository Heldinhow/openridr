import { expect, test, type Page } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";

const solution = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
const programTab = "src/OrderDesk.Cli/Program.cs";

async function openTextOnly(page: Page) {
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("open-text-only").click();
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("tool windows open from the bands, toggle the active one and hide on Shift+Esc", async ({ page }) => {
  await page.goto("/");
  await openTextOnly(page);

  await expect(page.getByTestId("toolband-solution")).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("toolband-build")).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByTestId("tree")).toBeVisible();

  // One active window per slot: opening Problems replaces Build without a split.
  await page.getByTestId("toolband-problems").click();
  await expect(page.getByTestId("bottom-tab-problems")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("toolband-build")).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByTestId("toolband-problems")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#output-content")).toHaveCount(1);

  // Clicking the active icon hides it and leaves the other slot alone.
  await page.getByTestId("toolband-problems").click();
  await expect(page.getByTestId("toolband-problems")).toHaveAttribute("aria-pressed", "false");
  await expect(page.locator("#output-content")).toHaveCount(0);
  await expect(page.getByTestId("tree")).toBeVisible();

  // With a file open, Shift+Esc from the band hides the window and returns focus to the editor.
  await page.getByTestId(`tree-file-${programTab}`).click();
  await expect(page.getByTestId("editor")).toBeVisible();
  await page.getByTestId("toolband-tests").click();
  await page.keyboard.press("Shift+Escape");
  await expect(page.locator("#output-content")).toHaveCount(0);
  await expect(page.getByTestId("editor")).toBeFocused();
  await expect(page.getByTestId("tree")).toBeVisible();

  await page.getByTestId("toolband-solution").click();
  await expect(page.getByTestId("tree")).toHaveCount(0);
  await expect(page.getByTestId("toolband-solution")).toHaveAttribute("aria-pressed", "false");
  await page.getByTestId("toolband-solution").click();
  await expect(page.getByTestId("tree")).toBeVisible();
});

test("slot sizes and active tool windows persist and invalid values are corrected", async ({ page }) => {
  await page.goto("/");
  await openTextOnly(page);

  const width = page.getByRole("separator", { name: "Solution explorer width" });
  await width.focus();
  await page.keyboard.press("ArrowRight");
  await expect(width).toHaveAttribute("aria-valuenow", "296");
  await page.getByTestId("bottom-tab-tests").click();

  await page.reload();
  await openTextOnly(page);
  await expect(page.getByRole("separator", { name: "Solution explorer width" })).toHaveAttribute("aria-valuenow", "296");
  await expect(page.getByTestId("bottom-tab-tests")).toHaveAttribute("aria-selected", "true");

  await page.evaluate(() => localStorage.setItem(
    "openridr.layout.v1",
    JSON.stringify({
      left: { active: "missing", size: 9000, open: "yes" },
      bottom: { active: "missing", size: -10, open: false },
    }),
  ));
  await page.reload();
  await openTextOnly(page);
  await expect(page.getByRole("separator", { name: "Solution explorer width" })).toHaveAttribute("aria-valuenow", "420");
  await expect(page.locator("#output-content")).toHaveCount(0);
  await page.getByTestId("toolband-build").click();
  await expect(page.locator("#output-content")).toHaveCount(1);
  await expect(page.getByTestId("bottom-tab-build")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("toolband-build")).toHaveAttribute("aria-pressed", "true");
});

test("the single bar keeps placeholders disabled and the status bar reports the file", async ({ page }) => {
  await page.goto("/");
  await openTextOnly(page);
  await page.getByTestId(`tree-file-${programTab}`).click();

  const toolbar = page.locator(".toolbar");
  expect((await toolbar.boundingBox())!.height).toBe(44);
  await expect(page.getByTestId("toolbar-project")).toContainText("OrderDesk.sln");
  await expect(page.getByTestId("toolbar-branch")).toHaveText("Git not connected");
  await expect(page.getByTestId("toolbar-branch")).toBeDisabled();
  for (const id of ["toolbar-branch", "toolbar-run-config", "toolbar-settings"]) {
    await expect(page.getByTestId(id)).toBeDisabled();
    expect(await page.getByTestId(id).getAttribute("title")).toBeTruthy();
  }
  await expect(page.getByTestId("toolbar-search")).toBeEnabled();
  await expect(page.getByTestId("toolbar-search")).toHaveAttribute("title", /Cmd\+T/);
  await expect(page.getByTestId("toolbar-run-config")).toHaveAttribute("title", /Trust the workspace before choosing a run configuration/);
  await expect(page.getByTestId("toolbar-settings")).toHaveAttribute("aria-label", /Settings arrive in F8/);
  await expect(page.getByTestId("bottom-tab-build")).toBeVisible();
  await expect(page.getByTestId("toolbar-build")).toBeDisabled();
  await expect(page.getByTestId("toolbar-run")).toBeDisabled();

  await expect(page.getByTestId("status-breadcrumb")).toContainText("OrderDesk.sln");
  await expect(page.getByTestId("status-breadcrumb")).toContainText("OrderDesk.Cli");
  await expect(page.getByTestId("status-breadcrumb")).toContainText("Program.cs");
  await expect(page.getByTestId("status-cursor")).toContainText("Ln 1, Col 1");
  await expect(page.getByTestId("status-encoding")).toHaveText("UTF-8");
  await expect(page.getByTestId("status-indent")).toHaveText("4 spaces");
  await expect(page.getByTestId("status-keymap")).toHaveText("Keymap: IntelliJ / Rider");
});
