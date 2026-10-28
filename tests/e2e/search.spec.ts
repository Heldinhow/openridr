import { expect, test, type Page } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";

const solution = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
const programTab = "src/OrderDesk.Cli/Program.cs";
const testsTab = "tests/OrderDesk.Tests/OrderTests.cs";

async function openTextOnly(page: Page) {
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("open-text-only").click();
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("⌘T opens Search Everywhere, Files opens a file, and the last tab and text persist", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);
  await page.getByTestId(`tree-file-${programTab}`).click();
  await page.getByTestId("editor").click();

  await page.keyboard.press("Meta+t");
  const dialog = page.getByTestId("find-action");
  await expect(dialog).toBeVisible();
  await expect(page.getByRole("heading", { name: "Search Everywhere" })).toBeVisible();
  await expect(page.getByTestId("find-action-profile")).toContainText("Rider (macOS)");
  for (const tab of ["all", "classes", "files", "symbols", "actions", "text"]) {
    await expect(page.getByTestId(`search-tab-${tab}`)).toBeVisible();
  }

  await page.getByTestId("search-tab-files").click();
  await expect(page.getByTestId("search-tab-files")).toHaveAttribute("aria-selected", "true");
  await page.getByTestId("find-action-input").fill("OrderTests.cs");
  await expect(page.getByTestId("search-item-0")).toContainText(testsTab);
  await expect(page.getByTestId("search-preview-path")).toHaveText(testsTab);
  await expect(page.getByTestId("search-preview-code")).toContainText("class OrderTests");

  await page.getByTestId("find-action-input").press("Enter");
  await expect(dialog).toHaveCount(0);
  await expect(page.getByTestId(`tab-${testsTab}`)).toBeVisible();
  await expect(page.getByTestId("editor")).toContainText("class OrderTests");

  // Reopening comes back to the last tab and the last text.
  await page.keyboard.press("Meta+t");
  await expect(dialog).toBeVisible();
  await expect(page.getByTestId("search-tab-files")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("find-action-input")).toHaveValue("OrderTests.cs");
  await page.getByTestId("find-action-input").press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(page.locator(".cm-content").first()).toBeFocused();

  // Shift Shift is the second entry point and also opens on the last tab.
  await page.keyboard.press("Shift");
  await page.keyboard.press("Shift");
  await expect(dialog).toBeVisible();
  await expect(page.getByTestId("search-tab-files")).toHaveAttribute("aria-selected", "true");
  await page.getByTestId("find-action-input").press("Escape");
  await expect(dialog).toHaveCount(0);
});

test("Tab and ⇧Tab change scope; Text groups matches, previews and opens the occurrence", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);

  await page.keyboard.press("Meta+t");
  const input = page.getByTestId("find-action-input");
  await expect(input).toBeFocused();
  // Starting from the default All, Tab walks the tabs; ⇧Tab walks back.
  await input.press("Tab");
  await expect(page.getByTestId("search-tab-classes")).toHaveAttribute("aria-selected", "true");
  await input.press("Tab");
  await expect(page.getByTestId("search-tab-files")).toHaveAttribute("aria-selected", "true");
  await input.press("Shift+Tab");
  await expect(page.getByTestId("search-tab-classes")).toHaveAttribute("aria-selected", "true");

  await page.getByTestId("search-tab-text").click();
  await input.fill("A-104");
  await expect(page.getByTestId("search-item-0")).toContainText("A-104");
  await expect(page.getByTestId("search-preview-path")).toHaveText(programTab);
  await expect(page.getByTestId("search-preview-code").locator("mark.search-hit").first()).toHaveText(
    "A-104",
  );
  await expect(page.getByTestId("search-preview-code")).toContainText("var order = new Order");
  await input.press("Enter");
  await expect(page.getByTestId("find-action")).toHaveCount(0);
  await expect(page.getByTestId(`tab-${programTab}`)).toBeVisible();
});

test("Classes and Symbols say Analysis not ready without trust; No results needs a real search", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);
  await page.keyboard.press("Meta+t");

  await page.getByTestId("search-tab-symbols").click();
  await page.getByTestId("find-action-input").fill("Order");
  await expect(page.getByTestId("search-state")).toContainText("Analysis not ready");
  await expect(page.getByTestId("search-state")).toContainText("Trust the workspace");
  await expect(page.getByTestId("search-empty")).toHaveCount(0);

  await page.getByTestId("search-tab-classes").click();
  await expect(page.getByTestId("search-state")).toContainText("Analysis not ready");
  await expect(page.getByTestId("search-empty")).toHaveCount(0);

  await page.getByTestId("search-tab-files").click();
  await page.getByTestId("find-action-input").fill("definitely-not-a-file-xyz");
  await expect(page.getByTestId("search-empty")).toHaveText("No results");
  await expect(page.getByTestId("search-state")).toHaveCount(0);

  await page.getByTestId("search-tab-text").click();
  await page.getByTestId("find-action-input").fill("definitely-not-in-the-workspace-xyz");
  await expect(page.getByTestId("search-empty")).toHaveText("No results");
  await expect(page.getByTestId("search-state")).toHaveCount(0);
});

test("⇧⌘A opens on Actions, executes the selected action and keeps disabled reasons", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);
  await page.keyboard.press("Meta+Shift+A");
  await expect(page.getByTestId("search-tab-actions")).toHaveAttribute("aria-selected", "true");

  await page.getByTestId("find-action-input").fill("git.commit");
  await expect(page.getByTestId("find-action-reason-git.commit")).toContainText("F6");
  await expect(
    page.getByTestId("find-action-item-git.commit").getByRole("button"),
  ).toBeDisabled();
  await page.getByTestId("find-action-input").press("Enter");
  await expect(page.getByTestId("find-action")).toBeVisible();

  await page.getByTestId("find-action-input").fill("problems");
  await page.getByTestId("find-action-input").press("Enter");
  await expect(page.getByTestId("find-action")).toHaveCount(0);
  await expect(page.getByTestId("bottom-tab-problems")).toHaveAttribute("aria-selected", "true");
});
