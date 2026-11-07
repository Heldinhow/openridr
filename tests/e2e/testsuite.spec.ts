import { expect, test, type Page } from "@playwright/test";
import { clickDiscover, revisionOf, settleRestore } from "./automatic";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host, screenshotsDir as screenshots } from "./environment";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const solution = path.join(fixture, "OrderDesk.sln");

test.use({ viewport: { width: 1440, height: 900 } });

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-tests-tree-"));
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return { base, copy };
}

async function openWorkspace(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
}

async function enterWorkspace(page: Page) {
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function waitForEvaluation(page: Page, revision = 1) {
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 20_000 });
  await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(revision);
}

async function restoreAndReload(page: Page, revision = 2) {
  await settleRestore(page);
  await waitForEvaluation(page, revision);
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("a grouped tree filters without losing selection and rerun failed uses the last run", async ({
  page,
}) => {
  test.setTimeout(300_000);
  const { base, copy } = tempCopy("Order Desk tree");
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await enterWorkspace(page);
    await waitForEvaluation(page);
    await restoreAndReload(page);

    await page.getByTestId("bottom-tab-tests").click();
    await page.getByTestId("tests-tfm").selectOption("net8.0");
    await waitForEvaluation(page, 3);
    await clickDiscover(page);
    await expect(page.getByTestId("tests-state")).toContainText("Discovered 4 tests", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("tests-discover")).toHaveText("Refresh tests");

    // Grouping is declared and derived from the FQN, project > prefix > class > method.
    await expect(page.getByTestId("tests-grouping")).toContainText(
      "Grouping derived from test names",
    );
    await expect(page.getByTestId("tests-tree-state")).toContainText(
      "4 tests grouped from their fully qualified names",
    );
    await expect(
      page.getByTitle(/^Project: tests\/OrderDesk\.Tests\/OrderDesk\.Tests\.csproj$/),
    ).toHaveCount(1);
    // The prefix level equals the project name here, so the tree skips it instead of repeating "OrderDesk.Tests" (ui-shape 08).
    await expect(
      page.getByTitle(
        /^Namespace prefix derived from the fully qualified test name: OrderDesk\.Tests\.OrderTests\./,
      ),
    ).toHaveCount(0);
    await expect(
      page.getByTitle(/^Class derived from the fully qualified test name: OrderDesk\.Tests\./),
    ).toHaveCount(1);
    await expect(page.getByTestId("test-identity-0")).toHaveAttribute(
      "title",
      "OrderDesk.Tests.OrderTests.Id_is_preserved",
    );

    // The filter keeps the groups of the matches and never changes the selection.
    await page.getByTestId("test-checkbox-0").check();
    await expect(page.getByTestId("tests-counts")).toContainText("1 selected");
    await page.getByTestId("tests-filter").fill("Skipped");
    await expect(page.getByTestId("tests-filter-count")).toHaveText("1 of 4 match");
    await expect(page.getByTestId("test-item-3")).toBeVisible();
    await expect(page.getByTestId("test-item-0")).toHaveCount(0);
    await expect(
      page.getByTitle(/^Project: tests\/OrderDesk\.Tests\/OrderDesk\.Tests\.csproj$/),
    ).toHaveCount(1);
    // The prefix level equals the project name here, so the tree skips it instead of repeating "OrderDesk.Tests" (ui-shape 08).
    await expect(
      page.getByTitle(
        /^Namespace prefix derived from the fully qualified test name: OrderDesk\.Tests\.OrderTests\./,
      ),
    ).toHaveCount(0);
    await expect(page.getByTestId("tests-counts")).toContainText("1 selected");
    await page.getByTestId("tests-filter").fill("");
    await expect(page.getByTestId("test-item-0")).toBeVisible();

    await page.getByTestId("tests-run-all").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Run completed", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("test-status-0")).toHaveText("passed");
    await expect(page.getByTestId("test-status-2")).toHaveText("failed");
    await expect(page.getByTestId("test-status-3")).toHaveText("skipped");

    // Rerun failed sends only the failed identity of the last completed run.
    await expect(page.getByTestId("tests-rerun-failed")).toBeEnabled();
    await page.getByTestId("tests-rerun-failed").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Run completed", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("tests-run-command")).toContainText(
      "--filter FullyQualifiedName=OrderDesk.Tests.OrderTests.Deliberate_failure_for_runner_evidence",
    );
    await expect(page.getByTestId("tests-run-counts")).toContainText(
      "1 result · 0 passed · 1 failed",
    );
    await expect(page.getByTestId("tests-run-selection")).toContainText("1 selected identity");

    // Quick list: numbered actions with visible shortcuts and the reason for each unavailable one.
    await page.keyboard.press("Alt+Shift+U");
    await expect(page.getByTestId("tests-quick-list")).toBeVisible();
    await expect(page.getByTestId("tests-quick-list-item-tests.runAll")).toContainText("Run all");
    await expect(page.getByTestId("tests-quick-list-item-tests.runAll")).toContainText("⌘U L");
    await expect(page.getByTestId("tests-quick-list-item-tests.runSelected")).toContainText(
      "Run selected",
    );
    await expect(page.getByTestId("tests-quick-list-item-tests.debugSelected")).toBeDisabled();
    await expect(page.getByTestId("tests-quick-list-reason-tests.debugSelected")).toContainText(
      "Debugging tests needs a test-host attach the debugger does not support yet",
    );
    await expect(page.getByTestId("tests-quick-list-item-tests.rerunFailed")).toBeEnabled();
    await expect(page.getByTestId("tests-quick-list-item-tests.rerunFailed")).toContainText("⌘U F");
    await expect(page.getByTestId("tests-quick-list-item-tests.stop")).toBeDisabled();
    await expect(page.getByTestId("tests-quick-list-reason-tests.stop")).toContainText(
      "No test discovery or run is active",
    );
    await page.screenshot({
      path: path.join(screenshots, "f5-tests-quick-list.png"),
      fullPage: false,
    });
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("tests-quick-list")).toHaveCount(0);

    // An edit after the run makes both the tree and rerun failed stale, never passed.
    await page.getByTestId("tree-file-tests/OrderDesk.Tests/OrderTests.cs").click();
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+End");
    await page.keyboard.insertText("\n// f5 stale check\n");
    await expect(page.getByTestId("dirty-marker")).toBeVisible();
    await expect(page.getByTestId("tests-tree-state")).toContainText("Stale tree");
    await expect(page.getByTestId("tests-rerun-failed")).toBeDisabled();
    await expect(page.getByTestId("tests-rerun-failed")).toHaveAttribute(
      "title",
      /buffer was edited after discovery/,
    );

    // Switching tool windows keeps the snapshot-driven tree and the lifted selection.
    await page.getByTestId("bottom-tab-build").click();
    await expect(page.getByTestId("tests-panel")).toHaveCount(0);
    await page.getByTestId("bottom-tab-tests").click();
    await expect(page.getByTestId("tests-items").locator("li")).toHaveCount(4);
    await expect(page.getByTestId("tests-tree-state")).toContainText("Stale tree");
    await expect(page.getByTestId("tests-counts")).toContainText("1 selected");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("the quick list dims unavailable actions with their reason", async ({ page }) => {
  await openWorkspace(page, solution);
  await enterWorkspace(page);
  await page.keyboard.press("Alt+Shift+U");
  await expect(page.getByTestId("tests-quick-list")).toBeVisible();
  await expect(page.getByTestId("tests-quick-list-item-tests.runAll")).toBeDisabled();
  await expect(page.getByTestId("tests-quick-list-reason-tests.runAll")).not.toBeEmpty();
  await expect(page.getByTestId("tests-quick-list-item-tests.debugSelected")).toBeDisabled();
  await expect(page.getByTestId("tests-quick-list-reason-tests.debugSelected")).toContainText(
    "Debugging tests needs a test-host attach the debugger does not support yet",
  );
  await expect(page.getByTestId("tests-quick-list-item-tests.stop")).toBeDisabled();
  await expect(page.getByTestId("tests-quick-list-reason-tests.stop")).toContainText(
    "No test discovery or run is active",
  );
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("tests-quick-list")).toHaveCount(0);
});
