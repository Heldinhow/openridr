import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const solution = path.join(fixture, "OrderDesk.sln");
const program = path.join(fixture, "src/OrderDesk.Cli/Program.cs");
const programTab = "src/OrderDesk.Cli/Program.cs";
const originalProgram = fs.readFileSync(program, "utf8");
const host = "http://127.0.0.1:4317";

const executableMethods = [
  "evaluation.run",
  "restore.start",
  "lsp.start",
  "tools.install",
  "terminal.start",
  "build.start",
  "run.start",
  "debug.start",
  "tests.discover",
  "tests.run",
];

test.beforeEach(() => {
  fs.writeFileSync(program, originalProgram);
});

test.afterEach(() => {
  fs.writeFileSync(program, originalProgram);
});

async function openWorkspace(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
}

async function enterTextOnly(page: Page) {
  await page.getByTestId("open-text-only").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function openProgram(page: Page) {
  await page.getByTestId(`tree-file-${programTab}`).click();
  await expect(page.getByTestId("editor")).toBeVisible();
}

async function replaceEditorText(page: Page, text: string) {
  const editor = page.getByTestId("editor");
  await editor.click();
  await page.keyboard.press("Meta+A");
  await page.keyboard.insertText(text);
}

test("opens the fixture in restricted mode with trust disabled", async ({ page }) => {
  await openWorkspace(page, solution);

  await expect(page.getByTestId("workspace-path")).toContainText(solution);
  await expect(page.getByTestId("mode-restricted")).toHaveText("Restricted mode");

  const trust = page.getByRole("checkbox", { name: "I trust this repository to execute code" });
  await expect(trust).not.toBeChecked();
  await expect(trust).toBeDisabled();

  const grant = page.getByRole("button", { name: "Trust and inspect project" });
  await expect(grant).toBeDisabled();
  await expect(page.getByText(/Trust enforcement is added by ticket 02/)).toBeVisible();
  await expect(page.getByText(/Blocked in Restricted mode/)).toBeVisible();

  await enterTextOnly(page);
  await expect(page.getByTestId("tree")).toContainText("Text outline · not evaluated");
  await expect(page.getByTestId("tree-project-OrderDesk.Cli")).toContainText("console");
  await expect(page.getByTestId("tree-project-OrderDesk.Cli")).toContainText("net8.0");
  await expect(page.getByTestId("tree-project-OrderDesk.Core")).toContainText("library");
  await expect(page.getByTestId("tree-project-OrderDesk.Tests")).toContainText("test");
  await expect(page.getByTestId(`tree-file-${programTab}`)).toBeVisible();
  await expect(page.getByTestId("status-keymap")).toHaveText("Keymap: IntelliJ / Rider");
  await expect(page.getByTestId("status-mode")).toContainText("Restricted mode");
  await expect(page.getByTestId("bottom-tab-build")).toBeVisible();
  await expect(page.getByTestId("bottom-tab-problems")).toBeVisible();
  await expect(page.getByTestId("bottom-tab-debug")).toBeVisible();
  await expect(page.getByTestId("bottom-tab-terminal")).toBeVisible();
  await expect(page.getByTestId("bottom-tab-tests")).toBeVisible();
  await page.getByTestId("bottom-tab-problems").click();
  await expect(page.getByTestId("problems-empty")).toBeVisible();
});

test("opens a file, edits, saves with Cmd+S and reopens the saved text", async ({ page }) => {
  await openWorkspace(page, solution);
  await enterTextOnly(page);
  await openProgram(page);
  await expect(page.getByTestId("editor")).toContainText("OrderDesk.Core");

  const edited = "// saved by playwright\nusing OrderDesk.Core;\n\nConsole.WriteLine(\"e2e\");\n";
  await replaceEditorText(page, edited);

  const tab = page.getByTestId(`tab-${programTab}`);
  await expect(tab.getByTestId("dirty-marker")).toBeVisible();
  await expect(page.getByTestId("status-dirty")).toContainText("1 modified buffer");

  await page.keyboard.press("Meta+s");
  await expect(page.getByTestId("app-log")).toContainText(`Saved ${programTab}`);
  await expect(tab.getByTestId("dirty-marker")).toHaveCount(0);
  expect(fs.readFileSync(program, "utf8")).toBe(edited);

  await tab.getByTestId("close-buffer").click();
  await expect(page.getByTestId("editor")).toHaveCount(0);
  await openProgram(page);
  await expect(page.getByTestId("editor")).toContainText("saved by playwright");
});

test("read and write failures keep the editor usable and preserve edits", async ({ page }) => {
  await openWorkspace(page, solution);
  await enterTextOnly(page);

  fs.rmSync(program);
  try {
    await page.getByTestId(`tree-file-${programTab}`).click();
    await expect(page.getByTestId("app-log")).toContainText("Could not open");
    await expect(page.getByTestId("editor")).toHaveCount(0);
  } finally {
    fs.writeFileSync(program, originalProgram);
  }

  await openProgram(page);
  await replaceEditorText(page, "// unsaved on write failure\n");
  const directory = path.dirname(program);
  try {
    fs.chmodSync(directory, 0o555);
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+s");
    await expect(page.getByTestId("app-log")).toContainText("Save failed; buffer kept.");
    await expect(page.getByTestId(`tab-${programTab}`).getByTestId("dirty-marker")).toBeVisible();
    expect(fs.readFileSync(program, "utf8")).toBe(originalProgram);
  } finally {
    fs.chmodSync(directory, 0o755);
  }
});

test("detects a disk conflict and never overwrites without an explicit choice", async ({
  page,
}) => {
  await openWorkspace(page, solution);
  await enterTextOnly(page);
  await openProgram(page);

  await replaceEditorText(page, "// buffer version\n");
  const diskVersion = "// disk version\n";
  fs.writeFileSync(program, diskVersion);

  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+s");

  const banner = page.getByTestId("conflict-banner");
  await expect(banner).toBeVisible();
  await expect(banner).toContainText("Your buffer is preserved");
  await expect(page.getByTestId("conflict-reload")).toHaveText(
    "Reload from disk (discard my edits)",
  );
  await expect(page.getByTestId("conflict-overwrite")).toHaveText("Overwrite disk");
  await expect(page.getByTestId("editor")).toContainText("buffer version");
  expect(fs.readFileSync(program, "utf8")).toBe(diskVersion);

  await page.getByTestId("conflict-reload").click();
  await expect(banner).toHaveCount(0);
  await expect(page.getByTestId("editor")).toContainText("disk version");

  await replaceEditorText(page, "// overwrite from buffer\n");
  fs.writeFileSync(program, "// second disk version\n");
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+s");
  await expect(banner).toBeVisible();
  await page.getByTestId("conflict-overwrite").click();
  await expect(banner).toHaveCount(0);
  expect(fs.readFileSync(program, "utf8")).toBe("// overwrite from buffer\n");
});

test("closing a modified buffer or workspace requires an explicit choice", async ({ page }) => {
  await openWorkspace(page, solution);
  await enterTextOnly(page);
  await openProgram(page);

  await replaceEditorText(page, "// unsaved buffer\n");
  const tab = page.getByTestId(`tab-${programTab}`);

  await tab.getByTestId("close-buffer").click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(page.getByRole("dialog")).toContainText("unsaved changes");
  await page.getByTestId("confirm-keep").click();
  await expect(tab).toBeVisible();
  await expect(tab.getByTestId("dirty-marker")).toBeVisible();

  await tab.getByTestId("close-buffer").click();
  await page.getByTestId("confirm-discard").click();
  await expect(page.getByTestId("editor")).toHaveCount(0);
  expect(fs.readFileSync(program, "utf8")).toBe(originalProgram);

  await openProgram(page);
  await replaceEditorText(page, "// workspace buffer\n");
  await page.getByTestId("close-workspace").click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("Close workspace with unsaved changes?");
  await page.getByTestId("confirm-keep").click();
  await expect(page.getByTestId("tree")).toBeVisible();

  await page.getByTestId("close-workspace").click();
  await page.getByTestId("confirm-discard").click();
  await expect(page.getByTestId("workspace-path-input")).toBeVisible();
});

test("ambiguous and empty directories ask the user instead of guessing", async ({ page }) => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-e2e-"));
  const ambiguous = path.join(base, "Order Desk ; & ambiguous");
  fs.mkdirSync(ambiguous);
  for (const name of ["Alpha", "Beta"]) {
    fs.writeFileSync(
      path.join(ambiguous, `${name}.csproj`),
      '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n',
    );
    fs.writeFileSync(path.join(ambiguous, `${name}.cs`), `// ${name}\n`);
  }

  try {
    await openWorkspace(page, ambiguous);
    await expect(page.getByTestId("workspace-error")).toContainText(
      "Multiple solutions or projects",
    );
    await expect(page.getByTestId("workspace-candidates")).toBeVisible();
    await expect(page.getByTestId("workspace-candidate-0")).toContainText("Alpha.csproj");
    await expect(page.getByTestId("workspace-candidate-1")).toContainText("Beta.csproj");

    await page.getByTestId("workspace-candidate-1").click();
    await expect(page.getByTestId("workspace-path")).toContainText("Beta.csproj");
    await enterTextOnly(page);
    await expect(page.getByTestId("tree-project-Beta")).toBeVisible();
    await page.getByTestId("tree-file-Beta.cs").click();
    await expect(page.getByTestId("editor")).toContainText("Beta");

    await replaceEditorText(page, "// edited without a solution\n");
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+s");
    await expect(page.getByTestId("app-log")).toContainText("Saved Beta.cs");
    expect(fs.readFileSync(path.join(ambiguous, "Beta.cs"), "utf8")).toBe(
      "// edited without a solution\n",
    );

    const empty = path.join(base, "empty workspace");
    fs.mkdirSync(empty);
    await openWorkspace(page, empty);
    await expect(page.getByTestId("workspace-error")).toContainText(
      "No .sln or .csproj found",
    );
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("direct host calls fail before trust and reject paths outside the workspace", async ({
  request,
}) => {
  const health = await request.get(`${host}/health`);
  expect(health.ok()).toBeTruthy();
  expect((await health.json()).service).toBe("openridr-host-http");

  for (const method of executableMethods) {
    const response = await request.post(`${host}/call`, {
      data: { method, params: {} },
    });
    const body = await response.json();
    expect(body.ok, method).toBe(false);
    expect(body.error.code, method).toBe("trust_required");
  }

  const opened = await request.post(`${host}/call`, {
    data: { method: "workspace.open", params: { path: solution } },
  });
  const openedBody = await opened.json();
  expect(openedBody.ok).toBe(true);
  expect(openedBody.data.workspace.mode).toBe("restricted");
  expect(openedBody.data.trust.granted).toBe(false);

  const outside = await request.post(`${host}/call`, {
    data: { method: "file.read", params: { path: "/etc/hosts" } },
  });
  expect((await outside.json()).error.code).toBe("outside_workspace");

  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});
