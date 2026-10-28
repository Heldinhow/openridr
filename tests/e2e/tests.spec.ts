import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const solution = path.join(fixture, "OrderDesk.sln");
const host = "http://127.0.0.1:4317";
const screenshots = path.resolve(process.cwd(), "docs/evidence/screenshots");

test.use({ viewport: { width: 1440, height: 900 } });

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-tests-"));
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return { base, copy };
}

function tempProject(name: string, extra: string, source = "public sealed class C { }\n") {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-tests-"));
  const copy = path.join(base, name);
  fs.mkdirSync(copy, { recursive: true });
  fs.writeFileSync(
    path.join(copy, "global.json"),
    '{"sdk":{"version":"8.0.408","rollForward":"disable"}}\n',
  );
  fs.writeFileSync(
    path.join(copy, `${name}.csproj`),
    `<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <IsTestProject>true</IsTestProject>
  </PropertyGroup>
  ${extra}
</Project>
`,
  );
  fs.writeFileSync(path.join(copy, "Code.cs"), source);
  return { base, copy };
}

async function openWorkspace(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
}

async function trustWorkspace(page: Page) {
  await page.getByTestId("trust-checkbox").check();
  await expect(page.getByTestId("trust-grant")).toBeEnabled();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
}

async function enterWorkspace(page: Page) {
  await page.getByTestId("open-text-only").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function waitForEvaluation(page: Page, revision = 1) {
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", {
    timeout: 20_000,
  });
  await expect(page.getByTestId("eval-state")).toContainText(`revision ${revision}`);
}

async function restoreAndReload(page: Page, revision = 2) {
  await page.getByTestId("restore-action").click();
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
    timeout: 90_000,
  });
  await page.getByTestId("restore-reload").click();
  await waitForEvaluation(page, revision);
}

async function discoveryState(request: APIRequestContext) {
  const response = await request.post(`${host}/call`, {
    data: { method: "workspace.state", params: {} },
  });
  const body = await response.json();
  expect(body.ok).toBe(true);
  return body.data;
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("real VSTest discovery lists four identified tests and blocks a stale selection", async ({
  page,
}) => {
  const { base, copy } = tempCopy("Order Desk discovery");
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await trustWorkspace(page);
    await enterWorkspace(page);
    await waitForEvaluation(page);
    await restoreAndReload(page);

    await page.getByTestId("bottom-tab-tests").click();
    await expect(page.getByTestId("tests-panel")).toBeVisible();
    await expect(page.getByTestId("tests-project")).toHaveValue(
      "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
    );
    await page.getByTestId("tests-tfm").selectOption("net8.0");
    await waitForEvaluation(page, 3);

    await page.getByTestId("tests-discover").click();
    await expect(page.getByTestId("tests-state")).toContainText("Discovered 4 tests", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("tests-state")).toContainText("exit 0");
    await expect(page.getByTestId("tests-scope")).toContainText("OrderDesk.Tests");
    await expect(page.getByTestId("tests-scope")).toContainText("net8.0");
    await expect(page.getByTestId("tests-command")).toContainText("dotnet test");
    await expect(page.getByTestId("tests-command")).toContainText(
      "--no-restore --list-tests --nologo -v:minimal -f net8.0",
    );
    await expect(page.getByTestId("tests-command")).toContainText(
      "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
    );
    await expect(page.getByTestId("tests-cwd")).toContainText(path.basename(copy));
    await expect(page.getByTestId("tests-sdk")).toContainText("8.0.408");
    await expect(page.getByTestId("tests-exit")).toHaveText("exit 0");
    await expect(page.getByTestId("tests-discovery-id")).toContainText(/discovery:\d+:1/);
    await expect(page.getByTestId("tests-counts")).toContainText("4 tests · 0 selected");
    await expect(page.getByTestId("tests-counts")).toContainText(/discovery:\d+:1 · revision 1/);
    await expect(page.getByTestId("tests-log")).toContainText(
      "The following Tests are available:",
    );

    const identities = [
      "OrderDesk.Tests.OrderTests.Id_is_preserved",
      "OrderDesk.Tests.OrderTests.Total_is_preserved",
      "OrderDesk.Tests.OrderTests.Deliberate_failure_for_runner_evidence",
      "OrderDesk.Tests.OrderTests.Skipped_until_fixture_changes",
    ];
    for (const [index, identity] of identities.entries()) {
      await expect(page.getByTestId(`test-identity-${index}`)).toHaveText(identity);
      await expect(page.getByTestId(`test-item-${index}`)).toContainText(
        "display name: " + identity,
      );
      await expect(page.getByTestId(`test-item-${index}`)).toContainText(
        "tests/OrderDesk.Tests/OrderDesk.Tests.csproj · net8.0 · vstest",
      );
      await expect(page.getByTestId(`test-checkbox-${index}`)).toBeEnabled();
    }
    await page.getByTestId("test-checkbox-0").check();
    await expect(page.getByTestId("tests-counts")).toContainText("4 tests · 1 selected");

    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({
      path: path.join(screenshots, "16-tests-discovery.png"),
      fullPage: false,
    });

    // An unsaved buffer edit makes the list stale and its checkboxes unusable.
    await page.getByTestId("tree-file-tests/OrderDesk.Tests/OrderTests.cs").click();
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+End");
    await page.keyboard.insertText("\n// stale discovery check\n");
    await expect(page.getByTestId("dirty-marker")).toBeVisible();
    await expect(page.getByTestId("tests-stale")).toContainText("stale, not failed");
    await expect(page.getByTestId("tests-stale")).toContainText("Discover again");
    for (const index of [0, 1, 2, 3]) {
      await expect(page.getByTestId(`test-checkbox-${index}`)).toBeDisabled();
    }
    await page.screenshot({
      path: path.join(screenshots, "16-tests-stale.png"),
      fullPage: false,
    });

    // Rediscovery uses the saved code and clears the stale block; the dirty buffer stays
    // dirty and is a ticket-17 concern, not a reason to keep the list unusable.
    await page.getByTestId("tests-discover").click();
    await expect(page.getByTestId("tests-state")).toContainText("Discovered 4 tests", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("tests-stale")).toHaveCount(0);
    await expect(page.getByTestId("test-checkbox-0")).toBeEnabled();
    await expect(page.getByTestId("tests-counts")).toContainText("revision 2");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("an MTP project shows the exact message and runs no VSTest command", async ({ page }) => {
  const { base, copy } = tempProject(
    "MtpGate",
    "<PropertyGroup><UseMicrosoftTestingPlatformRunner>true</UseMicrosoftTestingPlatformRunner></PropertyGroup>",
  );
  try {
    await openWorkspace(page, path.join(copy, "MtpGate.csproj"));
    await trustWorkspace(page);
    await enterWorkspace(page);
    await waitForEvaluation(page);
    await restoreAndReload(page);

    await page.getByTestId("bottom-tab-tests").click();
    await page.getByTestId("tests-discover").click();
    await expect(page.getByTestId("tests-mtp-message")).toHaveText(
      "Microsoft.Testing.Platform is not supported in this version. Use the terminal.",
    );
    await expect(page.getByTestId("tests-state")).toContainText("Microsoft.Testing.Platform");
    await expect(page.getByTestId("tests-command")).toHaveText("No command was run.");
    await expect(page.getByTestId("tests-exit")).toHaveText("exit unknown");
    await expect(page.getByTestId("tests-counts")).toContainText("0 tests");

    const state = await discoveryState(page.request);
    expect(state.tests.discovery.state).toBe("mtp_unsupported");
    expect(state.tests.discovery.command).toEqual([]);
    const processes = (state.processes as { kind: string; workspaceId: number }[]).filter(
      (process) =>
        process.workspaceId === state.workspace.id && process.kind.startsWith("tests:"),
    );
    expect(processes).toEqual([]);
    await page.screenshot({
      path: path.join(screenshots, "16-tests-mtp.png"),
      fullPage: false,
    });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a project without a VSTest adapter is adapter-missing, not empty", async ({ page }) => {
  const { base, copy } = tempProject("NoAdapter", "");
  try {
    await openWorkspace(page, path.join(copy, "NoAdapter.csproj"));
    await trustWorkspace(page);
    await enterWorkspace(page);
    await waitForEvaluation(page);
    await restoreAndReload(page);

    await page.getByTestId("bottom-tab-tests").click();
    await page.getByTestId("tests-discover").click();
    await expect(page.getByTestId("tests-state")).toContainText("No VSTest adapter");
    await expect(page.getByTestId("tests-guidance")).toContainText("Microsoft.NET.Test.Sdk");
    await expect(page.getByTestId("tests-guidance")).toContainText(
      "xunit.runner.visualstudio",
    );
    await expect(page.getByTestId("tests-guidance")).toContainText(
      "does not add or install packages",
    );
    await expect(page.getByTestId("tests-command")).toHaveText("No command was run.");
    await expect(page.getByTestId("tests-counts")).toContainText("0 tests");

    const state = await discoveryState(page.request);
    expect(state.tests.discovery.state).toBe("adapter_missing");
    expect(state.tests.discovery.command).toEqual([]);
    await page.screenshot({
      path: path.join(screenshots, "16-tests-adapter-missing.png"),
      fullPage: false,
    });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("real test run reports TRX outcomes, details, position, selected scope and infrastructure", async ({
  page,
}) => {
  test.setTimeout(240_000);
  const { base, copy } = tempCopy("Order Desk run");
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await trustWorkspace(page);
    await enterWorkspace(page);
    await waitForEvaluation(page);
    await restoreAndReload(page);

    await page.getByTestId("bottom-tab-tests").click();
    await expect(page.getByTestId("tests-project")).toHaveValue(
      "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
    );
    await page.getByTestId("tests-tfm").selectOption("net8.0");
    await waitForEvaluation(page, 3);
    await page.getByTestId("tests-discover").click();
    await expect(page.getByTestId("tests-state")).toContainText("Discovered 4 tests", {
      timeout: 120_000,
    });
    const discoveryIdText = await page.getByTestId("tests-discovery-id").innerText();
    const discoveryId = discoveryIdText.split(" ")[0];
    const discoveryRevision = discoveryIdText.match(/revision (\d+)/)?.[1];

    await page.getByTestId("tests-run-all").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Run completed", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("tests-run-exit")).toHaveText("exit 1");
    await expect(page.getByTestId("tests-run-counts")).toContainText(
      "4 results · 2 passed · 1 failed · 1 skipped · 0 not executed · 0 unknown",
    );
    await expect(page.getByTestId("tests-run-id")).toContainText(/testrun:\d+:1/);
    await expect(page.getByTestId("tests-run-id")).toContainText(discoveryId);
    await expect(page.getByTestId("tests-run-id")).toContainText(
      `discovery revision ${discoveryRevision}`,
    );
    await expect(page.getByTestId("tests-run-command")).toContainText(
      "--no-build --no-restore --nologo -v:minimal -f net8.0 --logger trx;LogFileName=openridr.trx",
    );
    await expect(page.getByTestId("tests-run-command")).toContainText(
      "--results-directory",
    );
    await expect(page.getByTestId("tests-run-trx")).toContainText("openridr.trx");
    await expect(page.getByTestId("tests-run-trx-state")).toHaveText("ok");
    await expect(page.getByTestId("tests-run-log-path")).toContainText("run.log");
    await expect(page.getByTestId("tests-run-log")).toContainText("Test run for");

    const results = page.getByTestId("tests-run-results-list").locator("li");
    await expect(results).toHaveCount(4);
    const failedRow = results.filter({ hasText: "Deliberate_failure_for_runner_evidence" });
    await expect(
      failedRow.locator('[data-testid^="test-result-outcome-"]'),
    ).toHaveText("failed");
    await expect(failedRow).toContainText("source Failed");
    await failedRow.locator("summary").click();
    await expect(
      failedRow.locator('[data-testid^="test-result-message-"]'),
    ).toContainText("Assert.Equal() Failure");
    await expect(
      failedRow.locator('[data-testid^="test-result-message-"]'),
    ).toContainText("Expected: 43");
    await expect(
      failedRow.locator('[data-testid^="test-result-message-"]'),
    ).toContainText("Actual:   42");
    await expect(
      failedRow.locator('[data-testid^="test-result-stack-"]'),
    ).toContainText("OrderTests.cs:line 29");
    await failedRow.getByRole("button", { name: "Open source position" }).click();
    await expect(page.getByTestId("editor-shell")).toHaveAttribute("data-cursor", "28:0");
    await expect(
      page.getByTestId("tab-tests/OrderDesk.Tests/OrderTests.cs"),
    ).toBeVisible();

    const skippedRow = results.filter({ hasText: "Skipped_until_fixture_changes" });
    await expect(
      skippedRow.locator('[data-testid^="test-result-outcome-"]'),
    ).toHaveText("skipped");
    await expect(skippedRow).toContainText("source NotExecuted");
    const passedRow = results.filter({ hasText: "Id_is_preserved" });
    await expect(
      passedRow.locator('[data-testid^="test-result-outcome-"]'),
    ).toHaveText("passed");

    await page.screenshot({
      path: path.join(screenshots, "17-tests-run-all.png"),
      fullPage: false,
    });

    // Run selected sends an exact filter and reports only that identity.
    await page.getByTestId("test-checkbox-0").check();
    await page.getByTestId("tests-run-selected").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Run completed", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("tests-run-counts")).toContainText(
      "1 result · 1 passed · 0 failed · 0 skipped",
    );
    await expect(page.getByTestId("tests-run-command")).toContainText(
      "--filter FullyQualifiedName=OrderDesk.Tests.OrderTests.Id_is_preserved",
    );
    await expect(page.getByTestId("tests-run-id")).toContainText(/testrun:\d+:2/);
    await expect(page.getByTestId("tests-run-results-list").locator("li")).toHaveCount(1);

    // Real infrastructure failure: the built test assembly is gone before --no-build.
    fs.rmSync(path.join(copy, "tests/OrderDesk.Tests/bin/Debug/net8.0/OrderDesk.Tests.dll"));
    await page.getByTestId("tests-run-all").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Run failed", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("tests-run-infrastructure")).toContainText(
      "infrastructure failure",
    );
    await expect(page.getByTestId("tests-run-infrastructure")).toContainText("no test result");
    await expect(page.getByTestId("tests-run-trx-state")).toHaveText("missing");
    await page.screenshot({
      path: path.join(screenshots, "17-tests-run-infrastructure.png"),
      fullPage: false,
    });
    // Retry is explicit and repeats the same selection; it fails again while the assembly is gone.
    await page.getByTestId("tests-run-retry").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Run failed", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("tests-run-id")).toContainText(/testrun:\d+:4/);

    // An unsaved edit keeps results but marks them obsolete, never corrected.
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+End");
    await page.keyboard.insertText("\n// run stale check\n");
    await expect(page.getByTestId("dirty-marker")).toBeVisible();
    await expect(page.getByTestId("tests-run-stale")).toContainText(
      "preserved but not current",
    );
    await expect(page.getByTestId("tests-run-counts")).toBeVisible();
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a long test run is cancelled with partial state and sealed on revoke", async ({ page }) => {
  test.setTimeout(300_000);
  const { base, copy } = tempCopy("Order Desk cancel");
  fs.writeFileSync(
    path.join(copy, "tests/OrderDesk.Tests/LongTests.cs"),
    `using System.Threading.Tasks;
using Xunit;

namespace OrderDesk.Tests;

public sealed class LongTests
{
    [Fact]
    public async Task Waits_for_cancellation()
    {
        await Task.Delay(60000);
    }
}
`,
  );
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await trustWorkspace(page);
    await enterWorkspace(page);
    await waitForEvaluation(page);
    await restoreAndReload(page);

    await page.getByTestId("bottom-tab-tests").click();
    await page.getByTestId("tests-tfm").selectOption("net8.0");
    await waitForEvaluation(page, 3);
    await page.getByTestId("tests-discover").click();
    await expect(page.getByTestId("tests-state")).toContainText("Discovered 5 tests", {
      timeout: 120_000,
    });
    const longItem = page
      .getByTestId("tests-items")
      .locator("li")
      .filter({ hasText: "Waits_for_cancellation" });
    await longItem.locator("input[type=checkbox]").check();
    await page.getByTestId("tests-run-selected").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Running tests", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("tests-run-cancel")).toBeEnabled();
    await page.getByTestId("tests-run-cancel").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Run cancelled", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("tests-run-termination")).toContainText(
      "Observed: exited",
    );
    await expect(page.getByTestId("tests-run-counts")).toContainText("0 passed");
    await expect(page.getByTestId("tests-run-log")).not.toBeEmpty();

    let state = await discoveryState(page.request);
    expect(state.tests.run.state).toBe("cancelled");
    expect(state.tests.run.counts.passed).toBe(0);
    expect(state.tests.run.termination.observed).toBe("exited");
    expect(
      (state.tests.run.results as { outcome: string }[]).every(
        (result) => result.outcome !== "passed",
      ),
    ).toBe(true);
    await page.screenshot({
      path: path.join(screenshots, "17-tests-run-cancel.png"),
      fullPage: false,
    });

    // Revocation during a second long run seals the record and keeps the log.
    await page.getByTestId("tests-run-selected").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Running tests", {
      timeout: 60_000,
    });
    await page.getByTestId("trust-revoke").click();
    await expect(page.getByTestId("tests-run-sealed")).toContainText("not current", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("tests-run-state")).toContainText("Run cancelled");
    await expect(page.getByTestId("tests-run-log")).not.toBeEmpty();
    state = await discoveryState(page.request);
    expect(state.trust.granted).toBe(false);
    expect(state.tests.run.sealed).toBe(true);
    expect(state.tests.run.stale).toBe(true);
    expect(state.tests.run.state).toBe("cancelled");
    const finishedAt = state.tests.run.finishedAt;
    await page.waitForTimeout(700);
    state = await discoveryState(page.request);
    expect(state.tests.run.state).toBe("cancelled");
    expect(state.tests.run.finishedAt).toBe(finishedAt);
    await page.screenshot({
      path: path.join(screenshots, "17-tests-run-revoke.png"),
      fullPage: false,
    });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a direct discovery call is denied before trust", async ({
  request,
}: {
  request: APIRequestContext;
}) => {
  const opened = await request.post(`${host}/call`, {
    data: { method: "workspace.open", params: { path: solution } },
  });
  const openedBody = await opened.json();
  expect(openedBody.ok).toBe(true);
  expect(openedBody.data.tests.discovery.state).toBe("idle");
  expect(openedBody.data.tests.discovery.revision).toBe(0);

  for (const method of ["tests.discover", "tests.run"]) {
    const denied = await request.post(`${host}/call`, { data: { method, params: {} } });
    const body = await denied.json();
    expect(body.ok).toBe(false);
    expect(body.error.code).toBe("trust_required");
  }
  const cancelled = await request.post(`${host}/call`, {
    data: { method: "tests.discover_cancel", params: {} },
  });
  const cancelBody = await cancelled.json();
  expect(cancelBody.ok).toBe(false);
  expect(cancelBody.error.code).toBe("invalid_state");
  const runCancelled = await request.post(`${host}/call`, {
    data: { method: "tests.run_cancel", params: {} },
  });
  const runCancelBody = await runCancelled.json();
  expect(runCancelBody.ok).toBe(false);
  expect(runCancelBody.error.code).toBe("invalid_state");

  const state = await discoveryState(request);
  expect(state.tests.discovery.revision).toBe(0);
  expect(state.tests.run.revision).toBe(0);
  const workspaceId = openedBody.data.workspace.id;
  const testProcesses = (state.processes as { workspaceId: number; kind: string }[]).filter(
    (process) => process.workspaceId === workspaceId && process.kind.startsWith("tests:"),
  );
  expect(testProcesses).toEqual([]);
});
