import { expect, test, type Page } from "@playwright/test";
import path from "node:path";
import { reveal } from "./disclosures";
import { host } from "./environment";

const solution = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
const programTab = "src/OrderDesk.Cli/Program.cs";

async function openProgram(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("open-text-only").click();
  await page.getByTestId(`tree-file-${programTab}`).click();
  await expect(page.getByTestId("editor")).toBeVisible();
}

async function openContextMenu(page: Page) {
  await page.getByTestId("editor").click({ button: "right" });
  await expect(page.getByTestId("editor-context-menu")).toBeVisible();
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("editor context menu lists F7 actions, copies a path and never offers Refactor This", async ({
  page,
}) => {
  await openProgram(page);
  await openContextMenu(page);
  const menu = page.getByTestId("editor-context-menu");
  for (const id of [
    "context-actions",
    "find-usages",
    "go-to-definition",
    "rename",
    "run-test",
    "debug-test",
    "git-diff",
    "git-commit",
    "copy-path",
  ]) {
    await expect(page.getByTestId(`context-menu-${id}`)).toBeVisible();
  }
  await expect(menu).not.toContainText("Refactor This");

  // Restricted mode disables analysis-backed entries with their reason, not silently.
  await expect(page.getByTestId("context-menu-context-actions")).toBeDisabled();
  await expect(page.getByTestId("context-menu-context-actions")).toHaveAttribute(
    "title",
    /Trust the workspace/,
  );
  await expect(page.getByTestId("context-menu-find-usages")).toBeDisabled();
  await expect(page.getByTestId("context-menu-run-test")).toBeDisabled();
  await expect(page.getByTestId("context-menu-debug-test")).toBeDisabled();
  await expect(page.getByTestId("context-menu-copy-path")).toBeEnabled();

  await page.getByTestId("context-menu-copy-path").click();
  await expect(menu).toHaveCount(0);
  await expect(page.getByTestId("app-log")).toContainText(`Copied path: ${programTab}`);

  // Escape closes the menu and the native menu never replaces it.
  await openContextMenu(page);
  await page.keyboard.press("Escape");
  await expect(menu).toHaveCount(0);
});

test("Alt+Enter stays the context action popup and reports the missing gate without inventing actions", async ({
  page,
}) => {
  await openProgram(page);
  await page.getByTestId("editor").click();
  await page.keyboard.press("Alt+Enter");
  await expect(page.getByTestId("code-actions")).toHaveCount(0);
  await expect(page.getByTestId("app-log")).toContainText(
    "Code actions unavailable: Trust the workspace before requesting code actions",
  );
});

test("trusted but unanalyzed editor explains Find Usages and Go To Definition gates", async ({
  page,
}) => {
  await openProgram(page);
  await reveal(page.getByTestId("trust-checkbox"));
  await page.getByTestId("trust-checkbox").check();
  await expect(page.getByTestId("trust-grant")).toBeEnabled();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();

  await openContextMenu(page);
  await expect(page.getByTestId("context-menu-find-usages")).toBeDisabled();
  await expect(page.getByTestId("context-menu-find-usages")).toHaveAttribute(
    "title",
    "Start analysis before finding usages.",
  );
  await expect(page.getByTestId("context-menu-go-to-definition")).toHaveAttribute(
    "title",
    "Start analysis before going to a declaration.",
  );
  await expect(page.getByTestId("context-menu-run-test")).toHaveAttribute(
    "title",
    "Place the cursor on a test method with analysis ready.",
  );
  await expect(page.getByTestId("context-menu-git-diff")).toBeVisible();
  await expect(page.getByTestId("context-menu-git-commit")).toBeVisible();
});
