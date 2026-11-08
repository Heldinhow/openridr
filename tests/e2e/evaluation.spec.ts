import { openProjectDetails, revisionOf } from "./automatic";
import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const solution = path.join(fixture, "OrderDesk.sln");
const brokenFixture = path.resolve(process.cwd(), "fixtures/BrokenImport");
import { host, screenshotsDir as screenshots } from "./environment";

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
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
}

async function enterWorkspace(page: Page) {
  // After trustWorkspace the modal is gone and the IDE is open; otherwise choose Restricted mode.
  const prompt = page.getByTestId("trust-prompt-dialog");
  await prompt.or(page.getByTestId("tree")).first().waitFor();
  if (await prompt.isVisible()) await page.getByTestId("trust-prompt-restricted").click();
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
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });

  await expect(page.getByTestId("status-evaluation")).toContainText("Evaluated");
  await expect(page.getByTestId("tree-folder-src")).toBeVisible();
  await expect(page.getByTestId("tree-folder-tests")).toBeVisible();
  await expect(page.getByTestId("tree-project-OrderDesk.Cli")).toHaveAttribute("data-project-kind", "console");
  await expect(page.getByTestId("tree-project-OrderDesk.Cli")).toHaveAttribute("data-project-tfms", "net8.0");
  await expect(page.getByTestId("tree-project-OrderDesk.Core")).toHaveAttribute("data-project-kind", "library");
  await expect(page.getByTestId("tree-project-OrderDesk.Tests")).toHaveAttribute("data-project-kind", "test");
  // Dependencies is a tree node (closed by default); its groups are closed until opened.
  const cliDeps = page.getByTestId("tree-deps-OrderDesk.Cli");
  await expect(cliDeps).toHaveAttribute("aria-expanded", "false");
  await cliDeps.click();
  await expect(cliDeps).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByTestId("tree-deps-group-OrderDesk.Cli-frameworks")).toBeVisible();
  await expect(page.getByTestId("tree-deps-group-OrderDesk.Cli-packages")).toHaveCount(0); // Cli has no packages
  const cliProjects = page.getByTestId("tree-deps-group-OrderDesk.Cli-projects");
  await expect(cliProjects).toHaveAttribute("aria-expanded", "false");
  await cliProjects.click();
  const coreRef = page.getByTestId("tree-ref-OrderDesk.Cli-OrderDesk.Core");
  await expect(coreRef).toContainText("OrderDesk.Core");
  await expect(coreRef).not.toContainText("· project");
  await cliDeps.click();
  await page.getByTestId("tree-deps-OrderDesk.Tests").click();
  await page.getByTestId("tree-deps-group-OrderDesk.Tests-packages").click();
  if (process.env.OPENRIDR_EVALUATION_SCREENSHOT) {
    await page.screenshot({ path: process.env.OPENRIDR_EVALUATION_SCREENSHOT });
  }
  const testSdk = page.getByTestId("tree-package-OrderDesk.Tests-Microsoft.NET.Test.Sdk");
  await expect(testSdk).toContainText("Microsoft.NET.Test.Sdk");
  await expect(testSdk).toContainText("17.11.1");
  await expect(testSdk).not.toContainText("· package");
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
  await page.getByRole("button", { name: "Workspace actions" }).click();
  const beforeDebug = await revisionOf(page);
  await page.getByTestId("eval-configuration").selectOption("Debug");
  await expect(page.getByTestId("eval-state")).toContainText(`revision ${beforeDebug + 1}`, {
    timeout: 20_000,
  });
  await expect(page.getByTestId("eval-state")).not.toContainText("stale");
  await expect(page.getByTestId("eval-configuration")).toHaveValue("Debug");
  await page.getByRole("button", { name: "Workspace actions" }).click();

  await page.getByTestId("tree-project-OrderDesk.Cli").click();
  await openProjectDetails(page);
  await expect(page.getByTestId("eval-details")).toBeVisible();
  await expect(page.getByTestId("eval-details-state")).toHaveText("Evaluated");
  const target = (await page.getByTestId("eval-target-path").textContent()) ?? "";
  expect(target).toContain("src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll");
  expect(path.isAbsolute(target.trim())).toBe(true);
  expect(target.trim().startsWith(fixture)).toBe(true);
  await expect(page.getByTestId("eval-details")).toContainText("Evaluated TFM");
  await expect(page.getByTestId("eval-packages")).toContainText("0 packages");
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored");
  const configuration = page.getByTestId("eval-configuration");
  await page.getByRole("button", { name: "Workspace actions" }).click();
  await expect(configuration).toBeEnabled({ timeout: 60_000 });
  await configuration.focus();
  await expect(configuration).toBeFocused();
  const beforeRelease = await revisionOf(page);
  await page.keyboard.press("r");
  await expect(page.getByTestId("eval-state")).toContainText(`revision ${beforeRelease + 1}`, {
    timeout: 20_000,
  });
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
    await openProjectDetails(page);
    await expect(page.getByTestId("eval-details")).toBeVisible();
    await expect(page.getByTestId("eval-error")).toContainText("MSB4019");
    await expect(page.getByTestId("eval-project-log")).toContainText("Missing.props");
    await expect(page.getByTestId("eval-open-project")).toBeVisible();
    await expect(page.getByTestId("eval-retry")).toBeEnabled();
    await expect(page.getByTestId("eval-restore")).toContainText("Restored");

    await page.getByTestId("eval-retry").click();
    await expect(page.getByTestId("eval-state")).toContainText("Evaluation failed", {
      timeout: 20_000,
    });
    await expect(page.getByTestId("eval-error")).toContainText("MSB4019");

    await page.getByTestId("eval-open-project").click();
    await expect(page.getByTestId("editor")).toContainText("build/Missing.props");
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
    await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });
    // The restore may already have triggered a second evaluation; make sure this save has a real predecessor.
    await expect(page.getByTestId("eval-run")).toBeEnabled({ timeout: 20_000 });
    const beforeSave = await revisionOf(page);
    await page.getByTestId("eval-run").click();
    await expect
      .poll(() => revisionOf(page), { timeout: 20_000 })
      .toBeGreaterThan(beforeSave);
    await expect(page.getByTestId("eval-stale")).toHaveCount(0);

    await page.getByTestId("tree-project-OrderDesk.Cli").click();
    await openProjectDetails(page);
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

    await expect(page.getByTestId("eval-stale")).toBeVisible();
    await expect(page.getByTestId("eval-state")).toContainText("not current");
    // revision > 1 is the only case that may claim values from a previous evaluation.
    await expect(page.getByTestId("eval-stale")).toContainText(
      "Values shown are from the previous evaluation",
    );
    await expect(page.getByTestId("eval-target-path")).toContainText(
      "Previous evaluation (stale)",
    );

    const beforeStale = await revisionOf(page);
    await page.getByTestId("eval-stale-run").click();
    await expect(page.getByTestId("eval-state")).toContainText(`revision ${beforeStale + 1}`, {
      timeout: 20_000,
    });
    await expect(page.getByTestId("eval-stale")).toHaveCount(0);
    await expect(page.getByTestId("eval-details-state")).toHaveText("Evaluated");
    await expect(page.getByTestId("eval-target-path")).toContainText(
      "src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll",
    );
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a solution with no projects blames the file, not the SDK", async ({ page }) => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-eval-"));
  const target = path.join(base, "Broken.sln");
  fs.writeFileSync(target, "this file is not a solution\n");
  // Even with a resolvable project SDK, an empty solution blames the file.
  fs.copyFileSync(path.join(fixture, "global.json"), path.join(base, "global.json"));
  try {
    await openWorkspace(page, target);
    await trustWorkspace(page);
    await enterWorkspace(page);
    const reason = page.getByTestId("eval-disabled-reason");
    await expect(reason).toContainText("Broken.sln lists no projects");
    await expect(reason).toContainText("not valid or is empty");
    await expect(reason).not.toContainText("SDK");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a workspace without global.json resolves the SDK its TargetFramework needs", async ({
  page,
  request,
}) => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-eval-"));
  const project = path.join(base, "App.csproj");
  fs.writeFileSync(
    project,
    '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n',
  );
  try {
    await openWorkspace(page, project);
    await trustWorkspace(page);
    await enterWorkspace(page);
    const state = (await (
      await request.post(`${host}/call`, {
        data: { method: "workspace.state", params: {} },
      })
    ).json()) as { data: any };
    const sdk = state.data.sdk.project;
    test.skip(sdk.state !== "resolved", "this machine has no SDK 8 or newer");

    // No pin: the host derives the requirement from the TargetFramework and picks the SDK.
    expect(sdk.requirement.source).toBe("target_framework");
    expect(sdk.requirement.line).toBe("8.0");
    expect(sdk.globalJsonPath).toBeNull();
    expect(sdk.requestedVersion).toBeNull();
    await expect(page.getByTestId("status-sdk")).toContainText(
      `Project SDK ${sdk.resolvedVersion}`,
    );
    // Nothing is missing, so the tree note is gone and nothing blames global.json.
    await expect(page.getByTestId("eval-disabled-reason")).toHaveCount(0);
    await waitForEvaluation(page);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a first evaluation with an invalid TFM never claims a previous evaluation", async ({
  page,
}) => {
  const { base, copy } = tempCopy("Order Desk invalid TFM");
  const projectFiles = [
    "src/OrderDesk.Cli/OrderDesk.Cli.csproj",
    "src/OrderDesk.Core/OrderDesk.Core.csproj",
    "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
  ];
  try {
    for (const file of projectFiles) {
      const full = path.join(copy, file);
      fs.writeFileSync(full, fs.readFileSync(full, "utf8").replaceAll("net8.0", "net99.0"));
    }
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await trustWorkspace(page);
    await enterWorkspace(page);
    await expect(page.getByTestId("eval-state")).toContainText("revision 1", {
      timeout: 30_000,
    });

    // A dirty buffer blocks the "changed on disk" auto-evaluation so the revision-1 state stays visible.
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await page.getByTestId("editor").click();
    await page.keyboard.type("x");
    await expect(page.getByTestId("dirty-marker")).toBeVisible();

    fs.appendFileSync(path.join(copy, projectFiles[0]), "\n");
    await expect(page.getByTestId("eval-stale")).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId("eval-stale")).toContainText("Nothing is current yet");
    await expect(page.getByTestId("eval-stale")).not.toContainText("previous evaluation");
    await expect(page.getByTestId("eval-state")).toContainText("revision 1");
    await expect(page.getByTestId("eval-state")).toContainText("not current");
    await expect(page.getByTestId("eval-state")).not.toContainText("stale");
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
