import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const solution = path.join(fixture, "OrderDesk.sln");
const brokenFixture = path.resolve(process.cwd(), "fixtures/BrokenImport");
import { host } from "./environment";
const screenshots = path.resolve(process.cwd(), "docs/evidence/screenshots");

test.use({ viewport: { width: 1440, height: 900 } });

function copyFixture(from: string, to: string) {
  fs.cpSync(from, to, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
}

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-eval-"));
  const copy = path.join(base, name);
  copyFixture(fixture, copy);
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
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function waitForEvaluation(page: Page, revision = 1) {
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", {
    timeout: 20_000,
  });
  await expect(page.getByTestId("eval-state")).toContainText(`revision ${revision}`);
}

async function cleanupHost(request: APIRequestContext) {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
}

test.afterEach(async ({ request }) => {
  await cleanupHost(request);
});

test("trust evaluates every project and the evaluated TargetPath follows the selection", async ({
  page,
}) => {
  await openWorkspace(page, solution);
  await trustWorkspace(page);
  await enterWorkspace(page);
  await waitForEvaluation(page);

  await expect(page.getByTestId("tree")).toContainText("MSBuild evaluated");
  await expect(page.getByTestId("tree-folder-src")).toBeVisible();
  await expect(page.getByTestId("tree-folder-tests")).toBeVisible();
  await expect(page.getByTestId("tree-project-OrderDesk.Cli")).toContainText("console");
  await expect(page.getByTestId("tree-project-OrderDesk.Cli")).toContainText("net8.0");
  await expect(page.getByTestId("tree-project-OrderDesk.Core")).toContainText("library");
  await expect(page.getByTestId("tree-project-OrderDesk.Tests")).toContainText("test");
  await expect(page.getByTestId("tree")).toContainText("References: OrderDesk.Core");
  await expect(page.getByTestId("tree")).toContainText("Packages: Microsoft.NET.Test.Sdk 17.11.1");
  await expect(page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs")).toBeVisible();
  await expect(page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs")).toBeVisible();
  await expect(page.getByTestId("tree-file-tests/OrderDesk.Tests/OrderTests.cs")).toBeVisible();
  const sourceIds = await page
    .getByTestId("tree")
    .locator('[data-testid^="tree-file-"]')
    .evaluateAll((nodes) => nodes.map((node) => node.getAttribute("data-testid") ?? ""));
  expect(sourceIds.length).toBeGreaterThan(0);
  for (const source of sourceIds) {
    expect(source).not.toContain("/bin/");
    expect(source).not.toContain("/obj/");
  }
  await expect(page.getByTestId("eval-restore-banner")).toHaveCount(0);
  await page.getByTestId("workspace-actions-menu").locator("summary").click();
  await page.getByTestId("eval-configuration").selectOption("Debug");
  await waitForEvaluation(page, 2);
  await expect(page.getByTestId("eval-state")).not.toContainText("stale");
  await page.getByTestId("workspace-actions-menu").locator("summary").click();
  await expect(page.getByTestId("eval-configuration")).toHaveValue("Debug");

  await page.getByTestId("tree-project-OrderDesk.Cli").click();
  await page.getByRole("button", { name: "Project details", exact: true }).click();
  await expect(page.getByTestId("eval-details")).toBeVisible();
  await expect(page.getByTestId("eval-details-state")).toHaveText("Evaluated");
  const target = (await page.getByTestId("eval-target-path").textContent()) ?? "";
  expect(target).toContain("src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll");
  expect(path.isAbsolute(target.trim())).toBe(true);
  expect(target.trim().startsWith(fixture)).toBe(true);
  await expect(page.getByTestId("eval-details")).toContainText("Evaluated TFM");
  await expect(page.getByTestId("eval-packages")).toContainText("0 packages");
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored");
  await expect(page.getByTestId("eval-revision")).toContainText("revision 2");

  const configuration = page.getByTestId("eval-configuration");
  await page.getByTestId("workspace-actions-menu").locator("summary").click();
  await configuration.focus();
  await expect(configuration).toBeFocused();
  await page.keyboard.press("r");
  await waitForEvaluation(page, 3);
  await expect(page.getByTestId("eval-state")).not.toContainText("stale");
  await expect(page.getByTestId("eval-details-state")).toHaveText("Evaluated");
  await expect(page.getByTestId("eval-target-path")).toContainText(
    "/bin/Release/net8.0/OrderDesk.Cli.dll",
  );
  await expect(page.getByTestId("eval-target-path")).not.toContainText("/bin/Debug/");

  fs.mkdirSync(screenshots, { recursive: true });
  await page.screenshot({
    path: path.join(screenshots, "03-evaluated-tree.png"),
    fullPage: false,
  });
});

test("a broken import stays in the tree with cause, log, open and retry", async ({ page }) => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-eval-"));
  const copy = path.join(base, "Broken Import copy");
  copyFixture(brokenFixture, copy);
  try {
    await openWorkspace(page, path.join(copy, "BrokenImport.csproj"));
    await trustWorkspace(page);
    await enterWorkspace(page);
    await expect(page.getByTestId("eval-state")).toContainText("Evaluation failed", {
      timeout: 20_000,
    });

    await expect(page.getByTestId("tree-project-BrokenImport")).toContainText("error");
    await page.getByRole("button", { name: "Project details", exact: true }).click();
    await expect(page.getByTestId("eval-details")).toBeVisible();
    await expect(page.getByTestId("eval-error")).toContainText("MSB4019");
    await expect(page.getByTestId("eval-project-log")).toContainText("Missing.props");
    await expect(page.getByTestId("eval-open-project")).toBeVisible();
    await expect(page.getByTestId("eval-retry")).toBeEnabled();
    await expect(page.getByTestId("eval-restore")).toContainText("Not restored");
    await expect(page.getByTestId("eval-restore-banner")).toContainText(
      "Restore not requested — package dependencies and analysis are incomplete",
    );

    await page.getByTestId("eval-retry").click();
    await expect(page.getByTestId("eval-state")).toContainText("revision 2", {
      timeout: 20_000,
    });
    await expect(page.getByTestId("eval-error")).toContainText("MSB4019");

    await page.getByTestId("eval-open-project").click();
    await expect(page.getByTestId("editor")).toContainText("build/Missing.props");
    expect(fs.existsSync(path.join(copy, "obj"))).toBe(false);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("saving a project file in a temp copy marks the model stale and re-evaluates it", async ({
  page,
}) => {
  const { base, copy } = tempCopy("Order Desk stale");
  const projectFile = "src/OrderDesk.Cli/OrderDesk.Cli.csproj";
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await trustWorkspace(page);
    await enterWorkspace(page);
    await waitForEvaluation(page);
    await expect(page.getByTestId("eval-restore-banner")).toContainText(
      "Restore not requested",
    );
    expect(
      fs.existsSync(path.join(copy, "src/OrderDesk.Cli/obj/project.assets.json")),
    ).toBe(false);

    await page.getByTestId("tree-project-OrderDesk.Cli").click();
    await page.getByRole("button", { name: "Project details", exact: true }).click();
    await page.getByTestId("eval-open-project").click();
    await expect(page.getByTestId("editor")).toContainText("ProjectReference");
    const original = fs.readFileSync(path.join(copy, projectFile), "utf8");
    const edited = `${original}\n`;
    const editor = page.getByTestId("editor");
    await editor.click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(edited);
    await page.keyboard.press("Meta+s");
    await expect(page.getByTestId("app-log")).toContainText(`Saved ${projectFile}`);

    await expect(page.getByTestId("eval-restore")).toContainText("Not restored");
    await expect(page.getByTestId("eval-stale")).toBeVisible();
    await expect(page.getByTestId("eval-state")).toContainText("stale");
    await expect(page.getByTestId("eval-target-path")).toContainText(
      "Previous evaluation (stale)",
    );

    await page.getByTestId("eval-stale-run").click();
    await expect(page.getByTestId("eval-state")).toContainText("revision 2", {
      timeout: 20_000,
    });
    await expect(page.getByTestId("eval-stale")).toHaveCount(0);
    await expect(page.getByTestId("eval-details-state")).toHaveText("Evaluated");
    await expect(page.getByTestId("eval-target-path")).toContainText(
      "src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll",
    );
    expect(
      fs.existsSync(path.join(copy, "src/OrderDesk.Cli/obj/project.assets.json")),
    ).toBe(false);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a direct evaluation call is denied before trust", async ({ request }) => {
  const opened = await request.post(`${host}/call`, {
    data: { method: "workspace.open", params: { path: solution } },
  });
  const openedBody = await opened.json();
  expect(openedBody.ok).toBe(true);
  expect(openedBody.data.workspace.mode).toBe("restricted");
  expect(openedBody.data.evaluation.state).toBe("idle");
  expect(openedBody.data.evaluation.projects).toEqual([]);

  const denied = await request.post(`${host}/call`, {
    data: { method: "evaluation.run", params: {} },
  });
  const body = await denied.json();
  expect(body.ok).toBe(false);
  expect(body.error.code).toBe("trust_required");
  expect(body.error.message).toContain("no process was started");

  const state = await request.post(`${host}/call`, {
    data: { method: "workspace.state", params: {} },
  });
  const stateBody = await state.json();
  expect(stateBody.data.evaluation.state).toBe("idle");
  expect(stateBody.data.evaluation.projects).toEqual([]);
});
