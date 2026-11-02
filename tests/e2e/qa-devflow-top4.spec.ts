import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import { revisionOf, settleRestore } from "./automatic";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { e2eHome, host } from "./environment";

/**
 * SPEC-DEVFLOW-TOP4.md, finding 1: the editor must scroll to the current debug line when it
 * changes and is off screen. A real netcoredbg session stops far below the fold with the
 * editor at the top; the marker only exists in the DOM when CodeMirror renders that line.
 */
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const debugToolDir = path.join(e2eHome, "tools/netcoredbg");
const realAdapter = path.join(
  process.env.HOME ?? "",
  "Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092",
);

test.use({ viewport: { width: 1440, height: 900 } });
test.describe.configure({ timeout: 180_000 });

async function hostCall(request: APIRequestContext, method: string, params = {}) {
  const response = await request.post(`${host}/call`, { data: { method, params } });
  return response.json();
}

async function selectEvaluationTfm(page: Page, value: string) {
  const summary = page.getByTestId("workspace-actions-menu").locator("summary");
  await summary.click();
  const tfm = page.getByTestId("eval-tfm");
  await expect(tfm).toBeVisible();
  await tfm.selectOption(value);
  await summary.click();
  await expect(tfm).toBeHidden();
  await expect(tfm).toHaveValue(value);
}

test("achado 1: the editor scrolls to the current execution line when it is off screen", async ({
  page,
  request,
}) => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-top4-scroll-"));
  const copy = path.join(base, "Order Desk scroll");
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  const program = path.join(copy, "src/OrderDesk.Cli/Program.cs");
  const comments = Array.from({ length: 70 }, (_, index) => `// filler ${index + 1}`);
  const stopLine = comments.length + 2;
  fs.writeFileSync(
    program,
    ["var label = \"bottom\";", ...comments, "Console.WriteLine(label);", ""].join("\n"),
  );
  fs.rmSync(debugToolDir, { recursive: true, force: true });
  fs.mkdirSync(debugToolDir, { recursive: true });
  fs.cpSync(realAdapter, path.join(debugToolDir, "3.2.0-1092"), { recursive: true });

  try {
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-trust").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    await page.getByTestId("workspace-tools-toggle").click();
    await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });

    expect((await hostCall(request, "debug.flag", { enabled: true, confirm: true })).ok).toBe(true);
    const planned = await hostCall(request, "debug.plan");
    const installed = await hostCall(request, "debug.install", {
      planId: planned.data.debug.plan.planId,
      confirm: true,
    });
    expect(installed.data.debug.adapterState).toBe("ready");

    await settleRestore(page);
    await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(2);
    await selectEvaluationTfm(page, "net8.0");
    await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(3);

    const breakpoint = await hostCall(request, "debug.setBreakpoints", {
      path: program,
      lines: [stopLine],
    });
    expect(breakpoint.ok, JSON.stringify(breakpoint.error)).toBe(true);

    // Open the file in the editor and leave it at the top: the stop line is far below the fold.
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await expect(page.getByTestId("editor")).toBeVisible();
    const scroller = page.locator(".cm-scroller");
    await scroller.evaluate((element) => {
      element.scrollTop = 0;
    });
    expect(await scroller.evaluate((element) => element.scrollTop)).toBe(0);

    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("build-action").click();
    await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
      timeout: 60_000,
    });

    // Launch from the Debug panel so the editor keeps its scroll position and focus stays out of it.
    await page.getByTestId("bottom-tab-debug").click();
    await page.getByTestId("debug-launch").click();
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });

    await expect.poll(() => scroller.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
    const current = page.locator(".cm-line-current");
    await expect(current).toHaveCount(1);
    await expect(current).toContainText("Console.WriteLine");
    expect(
      await current.evaluate((line) => {
        const viewport = line.closest(".cm-scroller")!.getBoundingClientRect();
        const content = line.getBoundingClientRect();
        return content.top >= viewport.top && content.bottom <= viewport.bottom;
      }),
    ).toBe(true);
    // The automatic scroll never pulls focus from the Debug panel into the editor.
    expect(
      await page
        .getByTestId("editor-shell")
        .evaluate((shell) => shell.contains(document.activeElement)),
    ).toBe(false);
  } finally {
    await hostCall(request, "debug.stop");
    await hostCall(request, "workspace.close");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

/**
 * SPEC-DEVFLOW-TOP4.md, finding 2: with focus in the editor, ⌘U arms the tests chord and the
 * next letter runs the action. CodeMirror never sees the prefix (its Mod-u is undoSelection)
 * nor the letter, so the buffer stays untouched.
 */
test("achado 2: ⌘U chords run tests from the editor without typing into the buffer", async ({
  page,
  request,
}) => {
  test.setTimeout(240_000);
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-top4-chord-"));
  const copy = path.join(base, "Order Desk chord");
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  try {
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-trust").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    await page.getByTestId("workspace-tools-toggle").click();
    await expect(page.getByTestId("tree")).toBeVisible();
    await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });
    await settleRestore(page);
    await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(2);

    await page.getByTestId("bottom-tab-tests").click();
    await expect(page.getByTestId("tests-project")).toHaveValue(
      "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
    );
    await page.getByTestId("tests-tfm").selectOption("net8.0");
    await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(3);
    await page.getByTestId("tests-discover").click();
    await expect(page.getByTestId("tests-state")).toContainText("Discovered 4 tests", {
      timeout: 120_000,
    });

    await page.getByTestId("tree-file-tests/OrderDesk.Tests/OrderTests.cs").click();
    const editor = page.getByTestId("editor");
    await expect(editor).toContainText("Id_is_preserved");
    await editor.click();
    const before = await editor.innerText();

    await page.keyboard.press("Meta+u");
    await page.keyboard.press("l");

    // Neither ⌘U (undoSelection) nor the chord letter reached the buffer.
    await expect.poll(() => editor.innerText()).toBe(before);
    await expect(page.getByTestId("dirty-marker")).toHaveCount(0);
    await expect(page.getByTestId("tests-run-state")).toContainText("Run completed", {
      timeout: 120_000,
    });
  } finally {
    await hostCall(request, "workspace.close");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

/**
 * SPEC-DEVFLOW-TOP4.md, finding 3: the global handler runs in capture and consumes the event
 * when the app knows the shortcut, so ⇧⌘K and ⌃D never reach the CodeMirror default keymap
 * (deleteLine and deleteCharForward).
 */
test("achado 3: ⇧⌘K opens Push from the editor without deleting the current line", async ({
  page,
  request,
}) => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-top4-push-"));
  const copy = path.join(base, "Order Desk push");
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  const git = (args: string[]) =>
    execFileSync("git", args, {
      cwd: copy,
      encoding: "utf8",
      env: {
        ...process.env,
        HOME: base,
        GIT_CONFIG_NOSYSTEM: "1",
        GIT_TERMINAL_PROMPT: "0",
      },
    });
  git(["init", "-q", "-b", "main"]);
  git(["config", "user.name", "OpenRidr E2E"]);
  git(["config", "user.email", "e2e@openridr.invalid"]);
  git(["add", "."]);
  git(["commit", "-qm", "Initial commit"]);

  try {
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-trust").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    const editor = page.getByTestId("editor");
    await expect(editor).toContainText("Console.WriteLine");
    await editor.click();
    const before = await editor.innerText();

    await page.keyboard.press("Meta+Shift+K");
    await expect(page.getByTestId("push-dialog")).toBeVisible({ timeout: 30_000 });
    // CodeMirror's Shift+Meta+K would have deleted the line before the dialog opened.
    await expect.poll(() => editor.innerText()).toBe(before);
    await expect(page.getByTestId("dirty-marker")).toHaveCount(0);
    await page.getByTestId("push-cancel").click();
    await expect(page.getByTestId("push-dialog")).toHaveCount(0);
  } finally {
    await hostCall(request, "workspace.close");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("achado 3: ⌃D starts Debug from the editor without deleting a character", async ({
  page,
  request,
}) => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
  test.setTimeout(240_000);
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-top4-debug-"));
  const copy = path.join(base, "Order Desk debug");
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  const program = path.join(copy, "src/OrderDesk.Cli/Program.cs");
  fs.rmSync(debugToolDir, { recursive: true, force: true });
  fs.mkdirSync(debugToolDir, { recursive: true });
  fs.cpSync(realAdapter, path.join(debugToolDir, "3.2.0-1092"), { recursive: true });

  try {
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-trust").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    await page.getByTestId("workspace-tools-toggle").click();
    await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });

    expect((await hostCall(request, "debug.flag", { enabled: true, confirm: true })).ok).toBe(true);
    const planned = await hostCall(request, "debug.plan");
    const installed = await hostCall(request, "debug.install", {
      planId: planned.data.debug.plan.planId,
      confirm: true,
    });
    expect(installed.data.debug.adapterState).toBe("ready");

    await settleRestore(page);
    await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(2);
    await selectEvaluationTfm(page, "net8.0");
    await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(3);

    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    const editor = page.getByTestId("editor");
    await expect(editor).toContainText("Console.WriteLine");
    await page.getByTestId("bottom-tab-debug").click();
    await expect(page.getByTestId("dap-state")).toHaveText("Not started");

    const breakpoint = await hostCall(request, "debug.setBreakpoints", {
      path: program,
      lines: [3],
    });
    expect(breakpoint.ok, JSON.stringify(breakpoint.error)).toBe(true);

    await editor.click();
    // Debug inlays decorate the paused line; compare the document text without them.
    const bufferLines = () =>
      editor.evaluate((root) =>
        Array.from(root.querySelectorAll(".cm-line"), (line) => {
          const copy = line.cloneNode(true) as HTMLElement;
          copy.querySelectorAll('[data-testid="debug-inlay"]').forEach((node) => node.remove());
          return copy.textContent ?? "";
        }),
      );
    const before = await bufferLines();
    await page.keyboard.press("Control+d");

    // CodeMirror's Control+D (deleteCharForward) would have changed the buffer immediately.
    await expect
      .poll(async () => (await page.getByTestId("dap-state").textContent()) ?? "", {
        timeout: 60_000,
      })
      .toMatch(/Building|Preparing|Running|Paused/);
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });
    await expect.poll(bufferLines).toEqual(before);
    await expect(page.getByTestId("dirty-marker")).toHaveCount(0);
  } finally {
    await hostCall(request, "debug.stop");
    await hostCall(request, "workspace.close");
    fs.rmSync(base, { recursive: true, force: true });
  }
});
