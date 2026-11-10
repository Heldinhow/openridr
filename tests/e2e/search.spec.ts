import { expect, test, type Page } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";

const solution = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
const programTab = "src/OrderDesk.Cli/Program.cs";
const testsTab = "tests/OrderDesk.Tests/OrderTests.cs";

async function openTextOnly(page: Page) {
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-restricted").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

/** Search Everywhere opens on two clean Shift taps or the ⌘T binding. */
async function openSearch(page: Page) {
  await page.keyboard.press("Shift");
  await page.keyboard.press("Shift");
  await expect(page.getByTestId("find-action")).toBeVisible();
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("a slow preview never shows the previous file as the newly selected result", async ({ page }) => {
  let release: (() => void) | undefined;
  let requested = false;
  const pending = new Promise<void>((resolve) => { release = resolve; });
  await page.route("**/call", async (route) => {
    const body = route.request().postDataJSON() as { method: string; params?: { path?: string } };
    if (body.method === "file.read" && body.params?.path === testsTab) {
      requested = true;
      await pending;
    }
    await route.continue();
  });

  try {
    await page.goto("/");
    await openTextOnly(page);
    await openSearch(page);
    await page.getByTestId("search-tab-files").click();
    const input = page.getByTestId("find-action-input");
    await input.fill("Program.cs");
    await expect(page.getByTestId("search-preview-path")).toHaveText(programTab);
    await input.fill("OrderTests.cs");
    await expect.poll(() => requested).toBe(true);
    await expect(page.getByTestId("search-preview-path")).toHaveCount(0);
    await expect(page.getByTestId("search-preview-loading")).toBeVisible();
    release?.();
    await expect(page.getByTestId("search-preview-path")).toHaveText(testsTab);
  } finally {
    release?.();
  }
});

test("⇧⇧ opens Search Everywhere, Files opens a file, and reopening returns to All with the query selected", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);
  await page.getByTestId(`tree-file-${programTab}`).click();
  await page.getByTestId("editor").click();

  await openSearch(page);
  const dialog = page.getByTestId("find-action");
  await expect(page.getByRole("heading", { name: "Search Everywhere" })).toBeVisible();
  await expect(page.getByTestId("find-action-profile")).toContainText("Rider");
  await expect(page.getByTestId("search-tab-all")).toHaveAttribute("aria-selected", "true");
  for (const tab of ["all", "classes", "files", "symbols", "actions", "text"]) {
    await expect(page.getByTestId(`search-tab-${tab}`)).toBeVisible();
  }

  await page.getByTestId("search-tab-files").click();
  await expect(page.getByTestId("search-tab-files")).toHaveAttribute("aria-selected", "true");
  await page.getByTestId("find-action-input").fill("OrderTests.cs");
  await expect(page.getByTestId("search-item-0")).toContainText(testsTab);
  await expect(page.getByTestId("search-preview-path")).toHaveText(testsTab);
  await expect(page.getByTestId("search-preview-code")).toContainText("class OrderTests");
  const resultBox = (await page.getByTestId("find-action-list").boundingBox())!;
  const previewBox = (await page.getByTestId("search-preview").boundingBox())!;
  // ui-shape 10: the preview docks beside the list, so neither covers the other.
  expect(previewBox.x).toBeGreaterThanOrEqual(resultBox.x + resultBox.width - 1);
  if (process.env.OPENRIDR_SEARCH_SCREENSHOT) {
    await page.screenshot({ path: process.env.OPENRIDR_SEARCH_SCREENSHOT });
  }

  await page.getByTestId("find-action-input").press("Enter");
  await expect(dialog).toHaveCount(0);
  await expect(page.getByTestId(`tab-${testsTab}`)).toBeVisible();
  await expect(page.getByTestId("editor")).toContainText("class OrderTests");

  // Reopening always comes back to All, with the previous query selected so typing replaces it.
  await openSearch(page);
  const input = page.getByTestId("find-action-input");
  await expect(page.getByTestId("search-tab-all")).toHaveAttribute("aria-selected", "true");
  await expect(input).toHaveValue("OrderTests.cs");
  expect(
    await input.evaluate((el) => {
      const field = el as HTMLInputElement;
      return [field.selectionStart, field.selectionEnd];
    }),
  ).toEqual([0, "OrderTests.cs".length]);
  await input.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(page.locator(".cm-content").first()).toBeFocused();
});

test("Tab and ⇧Tab change scope; Text groups matches, previews and opens the occurrence", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);

  await openSearch(page);
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
  await openSearch(page);

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

test("symbol search reports visible results, host truncation, and lists types in Symbols", async ({
  page,
}) => {
  await page.route("**/call", async (route) => {
    const request = route.request();
    const body = request.postDataJSON() as { method: string; params: { query?: string } };
    if (body.method !== "lsp.workspaceSymbol") return route.continue();
    const typeQuery = body.params.query === "OrderDeskType";
    const items = typeQuery
      ? [{ name: "OrderDeskType", kind: 5 }]
      : [
          { name: "OrderDeskMethod", kind: 6 },
          { name: "OrderDeskProperty", kind: 7 },
        ];
    return route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        ok: true,
        data: {
          state: "ok",
          message: null,
          missingAction: null,
          session: 1,
          items: items.map((item) => ({
            ...item,
            containerName: "OrderDesk",
            uri: "file:///workspace/OrderDesk.cs",
            path: programTab,
            range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
          })),
          total: typeQuery ? 1 : 523,
          truncated: !typeQuery,
        },
      }),
    });
  });
  await page.goto("/");
  await openTextOnly(page);
  await openSearch(page);
  await page.getByTestId("search-tab-symbols").click();
  const input = page.getByTestId("find-action-input");
  await input.fill("OrderDesk");
  await expect(page.getByTestId("search-item-0")).toContainText("OrderDeskMethod");
  await expect(page.getByTestId("search-status")).toContainText("2 visible results");
  await expect(page.getByTestId("search-status")).toContainText("results truncated");
  await expect(page.getByTestId("search-truncated")).toContainText("2 returned of 523 total symbols");

  await input.fill("OrderDeskType");
  await expect(page.getByTestId("search-item-0")).toContainText("OrderDeskType");
  await expect(page.getByTestId("search-item-0").locator(".search-kind")).toHaveText("class");
  await expect(page.getByTestId("search-suggest-classes")).toHaveCount(0);
  await page.getByTestId("search-tab-classes").click();
  await expect(page.getByTestId("search-tab-classes")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("search-item-0")).toContainText("OrderDeskType");
});

test("⇧⌘A opens on Actions, executes the selected action and keeps disabled reasons", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);
  await page.keyboard.press("Meta+Shift+A");
  await expect(page.getByTestId("search-tab-actions")).toHaveAttribute("aria-selected", "true");

  await page.getByTestId("find-action-input").fill("git.commit");
  await expect(page.getByTestId("find-action-reason-git.commit")).toContainText(
    "Trust the workspace to use Git",
  );
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
