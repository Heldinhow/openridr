import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { reveal } from "./disclosures";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const solution = path.join(fixture, "OrderDesk.sln");
const program = path.join(fixture, "src/OrderDesk.Cli/Program.cs");
const programTab = "src/OrderDesk.Cli/Program.cs";
const core = path.join(fixture, "src/OrderDesk.Core/Order.cs");
const coreTab = "src/OrderDesk.Core/Order.cs";
const originalProgram = fs.readFileSync(program, "utf8");
const originalCore = fs.readFileSync(core, "utf8");
import { host } from "./environment";
const screenshots = path.resolve(process.cwd(), "docs/evidence/screenshots");

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
  "processes.probe",
  "git.status",
  "git.log",
  "git.commit",
];

test.beforeEach(() => {
  fs.writeFileSync(program, originalProgram);
  fs.writeFileSync(core, originalCore);
});

test.afterEach(() => {
  fs.writeFileSync(program, originalProgram);
  fs.writeFileSync(core, originalCore);
});

async function openWorkspace(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
}

async function enterTextOnly(page: Page) {
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("workspace-tools-toggle").click();
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

async function dispatchWindowFocus(page: Page) {
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
}

async function beforeUnloadPrevented(page: Page) {
  return page.evaluate(
    () => !window.dispatchEvent(new Event("beforeunload", { cancelable: true })),
  );
}

test("opens the fixture in restricted mode with trust available", async ({ page }) => {
  await openWorkspace(page, solution);

  await expect(page.getByTestId("workspace-path")).toContainText(solution);
  await expect(page.getByTestId("mode-restricted")).toHaveText("Restricted mode");

  const trust = page.getByRole("checkbox", { name: "I trust this repository to execute code" });
  await expect(trust).not.toBeChecked();
  await expect(trust).toBeEnabled();

  const grant = page.getByRole("button", { name: "Trust and inspect project" });
  await expect(grant).toBeDisabled();
  await expect(page.getByTestId("trust-grant-reason")).toContainText(
    "Select the trust checkbox to enable this action.",
  );
  await expect(page.getByTestId("sdk-host")).toContainText("10.0.100");
  await expect(page.getByTestId("sdk-project")).toContainText("Not inspected yet");
  await expect(page.getByTestId("sdk-debug-adapter")).toContainText("netcoredbg");
  await expect(page.getByTestId("sdk-debug-adapter")).toContainText(
    "osx-arm64 asset exists",
  );
  await expect(page.getByTestId("sdk-debug-adapter")).toContainText(
    "installation stays refused until you enable the experimental flag",
  );

  await enterTextOnly(page);
  await expect(page.getByTestId("mode-badge")).toHaveText("Restricted mode");
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
  const bufferDialog = page.getByRole("dialog");
  await expect(bufferDialog).toBeVisible();
  await expect(bufferDialog).toContainText("unsaved changes");
  await expect(page.getByTestId("confirm-keep")).toHaveCSS("border-color", "rgb(84, 138, 247)");
  await expect(page.getByTestId("confirm-discard")).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  await page.getByTestId("confirm-keep").click();
  await expect(tab).toBeVisible();
  await expect(tab.getByTestId("dirty-marker")).toBeVisible();

  await tab.getByTestId("close-buffer").click();
  await page.getByTestId("confirm-discard").click();
  await expect(page.getByTestId("editor")).toHaveCount(0);
  expect(fs.readFileSync(program, "utf8")).toBe(originalProgram);

  await openProgram(page);
  await replaceEditorText(page, "// workspace buffer\n");
  await reveal(page.getByTestId("close-workspace"));
  await page.getByTestId("close-workspace").click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("Close workspace with unsaved changes?");
  await page.getByTestId("confirm-keep").click();
  await expect(page.getByTestId("tree")).toBeVisible();

  await reveal(page.getByTestId("close-workspace"));
  await page.getByTestId("close-workspace").click();
  await page.getByTestId("confirm-discard").click();
  await expect(page.getByTestId("workspace-path-input")).toBeVisible();
});

test("saves a modified buffer from the close dialog", async ({ page }) => {
  await openWorkspace(page, solution);
  await enterTextOnly(page);
  await openProgram(page);

  await replaceEditorText(page, "// saved from the close dialog\n");
  await expect(page.getByTestId("editor-revision")).toHaveText(" · Modified");

  await page.getByTestId(`tab-${programTab}`).getByTestId("close-buffer").click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("Close modified file?");
  await dialog.getByTestId("confirm-save").click();

  await expect(page.getByTestId("editor")).toHaveCount(0);
  expect(fs.readFileSync(program, "utf8")).toBe("// saved from the close dialog\n");

  await openProgram(page);
  await expect(page.getByTestId("editor-revision")).toHaveText("");
});

test("a save failure from the close dialog keeps the buffer and explains the error", async ({
  page,
}) => {
  await openWorkspace(page, solution);
  await enterTextOnly(page);
  await openProgram(page);

  await replaceEditorText(page, "// kept on failure\n");
  const directory = path.dirname(program);
  await page.getByTestId(`tab-${programTab}`).getByTestId("close-buffer").click();
  try {
    fs.chmodSync(directory, 0o555);
    await page.getByTestId("confirm-save").click();
    await expect(page.getByTestId("confirm-error")).toContainText("Save failed; buffer kept.");
    await expect(page.getByRole("dialog")).toBeVisible();
    await expect(page.getByTestId(`tab-${programTab}`)).toBeVisible();
    await expect(page.getByTestId(`tab-${programTab}`).getByTestId("dirty-marker")).toBeVisible();
    expect(fs.readFileSync(program, "utf8")).toBe(originalProgram);
  } finally {
    fs.chmodSync(directory, 0o755);
  }

  await page.getByTestId("confirm-keep").click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByTestId("editor")).toContainText("kept on failure");
});

test("saves every modified buffer from the workspace close dialog", async ({ page }) => {
  await openWorkspace(page, solution);
  await enterTextOnly(page);
  await openProgram(page);

  await replaceEditorText(page, "// save all program\n");
  await page.getByTestId(`tree-file-${coreTab}`).click();
  await expect(page.getByTestId("editor")).toContainText("namespace");
  await replaceEditorText(page, "// save all core\n");
  await expect(page.getByTestId("status-dirty")).toContainText("2 modified buffers");

  await reveal(page.getByTestId("close-workspace"));
  await page.getByTestId("close-workspace").click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("Close workspace with unsaved changes?");
  await expect(dialog).toContainText("2 modified buffers");
  await dialog.getByTestId("confirm-save").click();

  await expect(page.getByTestId("workspace-path-input")).toBeVisible();
  expect(fs.readFileSync(program, "utf8")).toBe("// save all program\n");
  expect(fs.readFileSync(core, "utf8")).toBe("// save all core\n");
});

test("compares buffer and disk and revalidates a second disk change before overwriting", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openWorkspace(page, solution);
  await enterTextOnly(page);
  await openProgram(page);

  await replaceEditorText(page, "// buffer version\n");
  fs.writeFileSync(program, "// first disk version\n");
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+s");

  const banner = page.getByTestId("conflict-banner");
  await expect(banner).toBeVisible();
  await page.getByTestId("conflict-compare").locator("summary").click();
  await expect(page.getByTestId("conflict-compare-buffer")).toHaveText("// buffer version\n");
  await expect(page.getByTestId("conflict-compare-disk")).toHaveText(
    "// first disk version\n",
  );
  await expect(page.getByTestId("conflict-buffer-hash")).toHaveText(/fnv1a64:/);
  await expect(page.getByTestId("conflict-disk-hash")).toHaveText(/fnv1a64:/);
  await expect(page.getByTestId("conflict-compare-diff")).toHaveText(
    "1 line removed · 1 line added",
  );
  await expect(page.getByTestId("editor-revision")).toHaveText(" · Modified");
  fs.mkdirSync(screenshots, { recursive: true });
  await page.getByTestId("conflict-compare").scrollIntoViewIfNeeded();
  await page.screenshot({
    path: path.join(screenshots, "07-conflict-compare.png"),
    fullPage: false,
  });

  // The disk changes again while the conflict is pending: the write must conflict again,
  // show the newer disk version, and leave both buffer and disk intact.
  fs.writeFileSync(program, "// second disk version\n");
  await page.getByTestId("conflict-overwrite").click();
  await expect(banner).toBeVisible();
  await expect(page.getByTestId("conflict-compare-disk")).toHaveText(
    "// second disk version\n",
  );
  expect(fs.readFileSync(program, "utf8")).toBe("// second disk version\n");
  await expect(page.getByTestId("editor")).toContainText("buffer version");
  await expect(page.getByTestId("editor-revision")).toHaveText(" · Modified");

  await page.getByTestId("conflict-overwrite").click();
  await expect(banner).toHaveCount(0);
  expect(fs.readFileSync(program, "utf8")).toBe("// buffer version\n");
  await expect(page.getByTestId("editor-revision")).toHaveText("");

  // The stored hash now belongs to the overwritten buffer: the next save needs no reload.
  await replaceEditorText(page, "// saved after overwrite\n");
  await page.keyboard.press("Meta+s");
  await expect(page.getByTestId("app-log")).toContainText(`Saved ${programTab}`);
  expect(fs.readFileSync(program, "utf8")).toBe("// saved after overwrite\n");
});

test("detects external changes on focus and tab activation without replacing the buffer", async ({
  page,
}) => {
  await openWorkspace(page, solution);
  await enterTextOnly(page);
  await openProgram(page);
  await expect(page.getByTestId("editor-revision")).toHaveText("");

  fs.writeFileSync(program, "// focus disk version\n");
  await dispatchWindowFocus(page);
  const banner = page.getByTestId("conflict-banner");
  await expect(banner).toBeVisible();
  await expect(page.getByTestId("editor")).toContainText("OrderDesk.Core");
  await expect(page.getByTestId("editor")).not.toContainText("focus disk version");
  await expect(page.getByTestId("editor-revision")).toHaveText(" · Modified");
  await page.getByTestId("conflict-compare").locator("summary").click();
  await expect(page.getByTestId("conflict-compare-disk")).toHaveText(
    "// focus disk version\n",
  );

  await page.getByTestId("conflict-reload").click();
  await expect(page.getByTestId("editor")).toContainText("focus disk version");
  await expect(banner).toHaveCount(0);
  await expect(page.getByTestId("editor-revision")).toHaveText("");
  expect(fs.readFileSync(program, "utf8")).toBe("// focus disk version\n");

  // The reloaded hash is the disk hash: a new save succeeds without another conflict.
  await replaceEditorText(page, "// after reload\n");
  await page.keyboard.press("Meta+s");
  await expect(page.getByTestId("app-log")).toContainText(`Saved ${programTab}`);
  expect(fs.readFileSync(program, "utf8")).toBe("// after reload\n");

  // Switching back to a tab re-reads that file.
  await page.getByTestId(`tree-file-${coreTab}`).click();
  await expect(page.getByTestId(`tab-${coreTab}`)).toBeVisible();
  fs.writeFileSync(program, "// tab switch disk version\n");
  await page.getByTestId(`tab-${programTab}`).click();
  await expect(banner).toBeVisible();
  await expect(page.getByTestId("editor")).toContainText("after reload");
  await page.getByTestId("conflict-compare").locator("summary").click();
  await expect(page.getByTestId("conflict-compare-disk")).toHaveText(
    "// tab switch disk version\n",
  );
  await page.getByTestId("conflict-compare").locator("summary").click();

  // A failed re-read reports and keeps the buffer editable.
  fs.rmSync(program);
  await dispatchWindowFocus(page);
  await expect(page.getByTestId("app-log")).toContainText("Could not re-read");
  await expect(page.getByTestId("editor")).toContainText("after reload");
  await replaceEditorText(page, "// still editable\n");
  await expect(page.getByTestId(`tab-${programTab}`).getByTestId("dirty-marker")).toBeVisible();
});

test("prevents beforeunload only while a buffer is dirty", async ({ page }) => {
  await openWorkspace(page, solution);
  await enterTextOnly(page);
  await openProgram(page);

  expect(await beforeUnloadPrevented(page)).toBe(false);
  await replaceEditorText(page, "// unsaved unload\n");
  expect(await beforeUnloadPrevented(page)).toBe(true);

  await page.keyboard.press("Meta+s");
  await expect(page.getByTestId("app-log")).toContainText(`Saved ${programTab}`);
  expect(await beforeUnloadPrevented(page)).toBe(false);
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
