import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const solution = path.join(fixture, "OrderDesk.sln");
import { host, screenshotsDir as screenshots } from "./environment";

test.use({ viewport: { width: 1440, height: 900 } });

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-restore-"));
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

function assets(copy: string, project: string) {
  return path.join(copy, project, "obj", "project.assets.json");
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("trust restores and evaluates by itself, streams a real log and reloads the model", async ({
  page,
}) => {
  const { base, copy } = tempCopy("Order Desk restore");
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await trustWorkspace(page);
    await enterWorkspace(page);

    // Trust starts the restore; no click is needed.
    await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("restore-state")).toContainText("exit 0");
    await expect(page.getByTestId("restore-command")).toContainText("dotnet restore");
    await expect(page.getByTestId("restore-command")).toContainText("-nologo -v:minimal");
    await expect(page.getByTestId("restore-command")).toContainText("OrderDesk.sln");
    await expect(page.getByTestId("restore-log")).toContainText(/Restored|up-to-date/);
    await expect(page.getByTestId("restore-scope")).toContainText("Solution");
    await expect(page.getByTestId("restore-projects")).toContainText(
      "3 projects report restored assets",
    );
    for (const project of ["src/OrderDesk.Cli", "src/OrderDesk.Core", "tests/OrderDesk.Tests"]) {
      expect(fs.existsSync(assets(copy, project)), `${project} was not restored`).toBe(true);
    }
    await expect(page.getByTestId("restore-incomplete-OrderDesk.Tests")).toHaveCount(0);

    // The successful restore is followed by an evaluation on its own: the model is current.
    await expect(page.getByTestId("eval-target-path")).toContainText(
      "src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll",
      { timeout: 60_000 },
    );
    await expect(page.getByTestId("eval-stale")).toHaveCount(0);
    await expect(page.getByTestId("eval-restore")).toHaveText("Restored");
    await expect(page.getByTestId("eval-restore-banner")).toHaveCount(0);

    await page.getByText("Restore log · stdout/stderr").click();
    await expect(page.getByTestId("restore-log")).toContainText("Restored");
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({
      path: path.join(screenshots, "04-restore.png"),
      fullPage: false,
    });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("restricted mode never restores by itself", async ({ page }) => {
  const { base, copy } = tempCopy("Order Desk restricted");
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await enterWorkspace(page);
    await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");
    // Negative check: give a restore that would have started time to appear.
    await page.waitForTimeout(2_000);
    await expect(page.getByTestId("restore-state")).toContainText("Restore not requested");
    for (const project of ["src/OrderDesk.Cli", "src/OrderDesk.Core", "tests/OrderDesk.Tests"]) {
      expect(fs.existsSync(assets(copy, project)), `${project} was restored in Restricted`).toBe(false);
    }
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a failed restore keeps its log, the editor and a working retry", async ({ page }) => {
  const { base, copy } = tempCopy("Order Desk restore failure");
  const testsProject = path.join(copy, "tests/OrderDesk.Tests/OrderDesk.Tests.csproj");
  try {
    // A package version that does not exist, resolved without network sources, fails fast.
    fs.writeFileSync(
      path.join(copy, "NuGet.config"),
      "<configuration><packageSources><clear /></packageSources></configuration>\n",
    );
    const original = fs.readFileSync(testsProject, "utf8");
    fs.writeFileSync(testsProject, original.replace('Version="2.9.2"', 'Version="9.9.999"'));

    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await trustWorkspace(page);
    await enterWorkspace(page);

    await expect(page.getByTestId("restore-state")).toContainText("Restore failed", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("restore-state")).toContainText("exit 1");
    await expect(page.getByTestId("status-restore")).toContainText("Restore failed");
    await expect(page.getByTestId("restore-log")).toContainText("NU1100");
    await expect(page.getByTestId("restore-log")).toContainText("Failed to restore");
    await expect(page.getByTestId("restore-incomplete-OrderDesk.Tests")).toContainText(
      "not restored",
    );
    await expect(page.getByTestId("restore-retry")).toBeVisible();
    await page.getByTestId("bottom-tab-build").click();
    await expect(page.getByTestId("build-disabled-reason")).toContainText(
      "The evaluated project model is stale; re-evaluate before building.",
    );
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({
      path: path.join(screenshots, "04-restore-failure.png"),
      fullPage: false,
    });

    // Editing stays available during and after a failed restore.
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await expect(page.getByTestId("editor")).toContainText("OrderDesk.Core");
    const before = fs.readFileSync(path.join(copy, "src/OrderDesk.Cli/Program.cs"), "utf8");
    const edited = `${before}// editor survived the failed restore\n`;
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(edited);
    await page.keyboard.press("Meta+s");
    await expect(page.getByTestId("app-log")).toContainText(
      "Saved src/OrderDesk.Cli/Program.cs",
    );
    expect(fs.readFileSync(path.join(copy, "src/OrderDesk.Cli/Program.cs"), "utf8")).toContain(
      "editor survived the failed restore",
    );

    // Fix the copy and retry from the status bar, which needs no trip to the Build tab.
    fs.writeFileSync(testsProject, original);
    await page.getByTestId("status-restore-retry").click();
    await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("restore-state")).toContainText("exit 0");
    await expect(page.getByTestId("restore-incomplete-OrderDesk.Tests")).toHaveCount(0);
    expect(fs.existsSync(assets(copy, "tests/OrderDesk.Tests"))).toBe(true);
    await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a project file changed on disk re-evaluates a Trusted workspace by itself", async ({ page }) => {
  const { base, copy } = tempCopy("Order Desk project change");
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await trustWorkspace(page);
    await enterWorkspace(page);
    await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });
    await expect(page.getByTestId("eval-stale")).toHaveCount(0);

    const project = path.join(copy, "src/OrderDesk.Cli/OrderDesk.Cli.csproj");
    fs.appendFileSync(project, "\n<!-- changed on disk -->\n");
    await expect(page.getByTestId("eval-stale")).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId("eval-stale")).toHaveCount(0, { timeout: 60_000 });
    // Only the evaluation repeats; the assets are now older than the project, so restore is still the user's call.
    await expect(page.getByTestId("eval-restore")).toContainText("Not restored");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a direct restore call is denied before trust", async ({ request }: { request: APIRequestContext }) => {
  const opened = await request.post(`${host}/call`, {
    data: { method: "workspace.open", params: { path: solution } },
  });
  const openedBody = await opened.json();
  expect(openedBody.ok).toBe(true);
  expect(openedBody.data.restore.state).toBe("idle");
  expect(openedBody.data.restore.revision).toBe(0);

  for (const method of ["restore.start", "restore.retry"]) {
    const denied = await request.post(`${host}/call`, { data: { method, params: {} } });
    const body = await denied.json();
    expect(body.ok).toBe(false);
    expect(body.error.code).toBe("trust_required");
  }
  const state = await request.post(`${host}/call`, {
    data: { method: "workspace.state", params: {} },
  });
  const body = await state.json();
  expect(body.data.restore.revision).toBe(0);
  const workspaceId = openedBody.data.workspace.id;
  const restoreProcesses = (body.data.processes as { workspaceId: number }[]).filter(
    (process) => process.workspaceId === workspaceId,
  );
  expect(restoreProcesses).toEqual([]);
});
