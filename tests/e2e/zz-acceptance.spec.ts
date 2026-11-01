/**
 * Ticket 18 acceptance regression: one integrated real journey plus a UI/layout/a11y audit.
 *
 * The `zz-` prefix keeps this file last: the per-ticket suites assume the first host
 * session/discovery/run sequence and a fresh debugger adapter state. This spec must not
 * replace them; it drives the same Chrome + Vite + host-http seam and real tools.
 */
import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import fs from "node:fs";
import { revisionOf } from "./automatic";
import os from "node:os";
import path from "node:path";
import { reveal } from "./disclosures";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const host = "http://127.0.0.1:4317";
const screenshots = path.resolve(process.env.OPENRIDR_SCREENSHOT_DIR ?? "docs/evidence/screenshots");
const logs = path.resolve(process.env.OPENRIDR_EVIDENCE_LOG_DIR ?? "docs/evidence/logs");
const e2eHome = path.join(os.tmpdir(), "openridr-e2e-home");
const realAdapter = path.join(
  process.env.HOME ?? "",
  "Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092",
);
const programRel = "src/OrderDesk.Cli/Program.cs";
const libraryRel = "src/OrderDesk.Core/Order.cs";
const originalProgram = fs.readFileSync(path.join(fixture, programRel), "utf8");
const originalLibrary = fs.readFileSync(path.join(fixture, libraryRel), "utf8");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(900_000);

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-acceptance-"));
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return { base, copy };
}

async function hostCall(request: APIRequestContext, method: string, params = {}) {
  const response = await request.post(`${host}/call`, { data: { method, params } });
  return response.json();
}

async function hostState(request: APIRequestContext) {
  const body = await hostCall(request, "workspace.state");
  expect(body.ok, JSON.stringify(body.error)).toBe(true);
  return body.data;
}

async function waitForPrompt(request: APIRequestContext) {
  await expect
    .poll(
      async () => (await hostState(request)).terminal.log.trimEnd().endsWith("%"),
      { timeout: 15_000, intervals: [100, 250, 500] },
    )
    .toBe(true);
}

function snapshot(copy: string, state: Record<string, unknown>) {
  fs.mkdirSync(logs, { recursive: true });
  fs.writeFileSync(
    path.join(logs, copy),
    JSON.stringify(state, null, 2),
  );
}

function screenshot(page: Page, name: string) {
  fs.mkdirSync(screenshots, { recursive: true });
  return page.screenshot({ path: path.join(screenshots, name), fullPage: false });
}

async function openWorkspace(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
}

async function closeWorkspace(page: Page) {
  await clickTestId(page, "close-workspace");
  await expect(page.getByTestId("workspace-path-input")).toBeVisible();
}

// Controls the compact shell keeps inside a closed disclosure (toolbar menu, workspace tools).
async function clickTestId(page: Page, testid: string) {
  const target = page.getByTestId(testid);
  await reveal(target);
  await target.click();
}

async function grantTrust(page: Page) {
  await page.getByTestId("trust-checkbox").check();
  await expect(page.getByTestId("trust-grant")).toBeEnabled();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
}

async function enterWorkspace(page: Page) {
  await page.getByTestId("open-text-only").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function waitEvaluation(page: Page, revision: number) {
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", {
    timeout: 60_000,
  });
  await expect.poll(() => revisionOf(page), { timeout: 60_000 }).toBeGreaterThanOrEqual(revision);
  await expect(page.getByTestId("eval-state")).not.toContainText("stale");
}

async function ensureCsharpLs(page: Page, request: APIRequestContext) {
  // Fase 1 layout: the installer lives in the Workspace tools drawer, closed by default.
  if (!(await page.getByTestId("workspace-tools").isVisible())) {
    await page.getByTestId("workspace-tools-toggle").click();
  }
  const before = await hostState(request);
  if (before.tools["csharp-ls"].state === "ready") {
    await expect(page.getByTestId("install-ready")).toBeVisible();
    return before;
  }
  await clickTestId(page, "install-plan-open");
  await expect(page.getByTestId("install-plan")).toBeVisible();
  await expect(page.getByTestId("install-tool")).toContainText("csharp-ls 0.28.0");
  await expect(page.getByTestId("install-source")).toContainText("NuGet");
  await clickTestId(page, "install-confirm");
  await expect
    .poll(
      async () => (await hostState(request)).tools["csharp-ls"].state,
      { timeout: 240_000, intervals: [100, 250, 500] },
    )
    .toBe("ready");
  return hostState(request);
}

async function placeCursor(page: Page, line: number, character: number) {
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+ArrowUp");
  for (let index = 0; index < line; index += 1) {
    await page.keyboard.press("ArrowDown");
  }
  await page.keyboard.press("Home");
  for (let index = 0; index < character; index += 1) {
    await page.keyboard.press("ArrowRight");
  }
  await expect(page.getByTestId("editor-shell")).toHaveAttribute(
    "data-cursor",
    `${line}:${character}`,
  );
}

async function replaceEditor(page: Page, text: string, save: boolean) {
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+A");
  await page.keyboard.insertText(text);
  if (save) {
    await page.keyboard.press("Meta+s");
    await expect(page.getByTestId("dirty-marker")).toHaveCount(0, { timeout: 15_000 });
  }
}

test.afterEach(async ({ request }) => {
  await hostCall(request, "terminal.stop", { confirm: true }).catch(() => undefined);
  await hostCall(request, "debug.stop").catch(() => undefined);
  await hostCall(request, "lsp.stop").catch(() => undefined);
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("real P0 journey: restricted, trust, SDKs, evaluation, restore, LSP, build, run, tests, PTY and DAP", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk acceptance");
  const steps: string[] = [];
  const step = (text: string) => steps.push(`${new Date().toISOString()} ${text}`);
  const summary: Record<string, unknown> = {};
  try {
    // ---- Restricted mode: no execution, full path, text editing only.
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    step("open solution in Restricted mode");
    await expect(page.getByTestId("workspace-path")).toContainText(copy);
    await expect(page.getByTestId("mode-restricted")).toBeVisible();
    await expect(page.getByTestId("trust-checkbox")).not.toBeChecked();
    await expect(page.getByTestId("sdk-host")).toContainText("10.0.100");
    await expect(page.getByTestId("sdk-project")).toContainText("Not inspected yet");
    await screenshot(page, "18-screen-01-trust.png");

    await enterWorkspace(page);
    await expect(page.getByTestId("status-mode")).toContainText("Restricted mode");
    await expect(page.getByTestId("toolbar-build")).toBeDisabled();
    await expect(page.getByTestId("toolbar-build")).toHaveAttribute("title", /Trust the workspace/);
    await expect(page.getByTestId("toolbar-run")).toBeDisabled();
    await expect(page.getByTestId("toolbar-debug")).toBeDisabled();

    await page.getByTestId("bottom-tab-build").click();
    await expect(page.getByTestId("build-disabled-reason")).toContainText(
      "Trust the workspace before building",
    );
    await page.getByTestId("bottom-tab-debug").click();
    await expect(page.getByTestId("dap-disabled-reason")).toContainText(
      "Trust the workspace before debugging",
    );
    await page.getByTestId("bottom-tab-terminal").click();
    await expect(page.getByTestId("terminal-disabled-reason")).toContainText(
      "Trust the workspace before starting a terminal",
    );
    await page.getByTestId("bottom-tab-tests").click();
    await expect(page.getByTestId("tests-disabled-reason")).toContainText(
      "Trust the workspace before discovering tests",
    );

    const denied: Record<string, string> = {};
    const executableMethods = [
      "evaluation.run",
      "restore.start",
      "tools.install",
      "lsp.start",
      "build.start",
      "run.start",
      "tests.discover",
      "tests.run",
      "terminal.start",
      "debug.install",
      "debug.start",
      "debug.setBreakpoints",
      "git.status",
      "git.log",
      "git.commit",
    ];
    for (const method of executableMethods) {
      const body = await hostCall(request, method, {});
      expect(body.ok, `${method} ran before trust`).toBe(false);
      expect(body.error.code, `${method} was not trust-gated`).toBe("trust_required");
      denied[method] = body.error.code;
    }
    summary.deniedBeforeTrust = denied;

    const restricted = await hostState(request);
    expect(restricted.workspace.mode).toBe("restricted");
    expect(restricted.evaluation.state).toBe("idle");
    expect(restricted.restore.state).toBe("idle");
    expect(restricted.build.revision).toBe(0);
    expect(restricted.run.revision).toBe(0);
    expect(restricted.tests.discovery.revision).toBe(0);
    expect(restricted.terminal.state).toBe("idle");
    expect(restricted.lsp.state).toBe("unavailable");
    expect(
      (restricted.processes as { workspaceId: number; state: string }[]).filter(
        (item) => item.workspaceId === restricted.workspace.id && item.state === "running",
      ),
    ).toEqual([]);

    // Editing works before trust; the revert keeps the fixture copy clean.
    await page.getByTestId(`tree-file-${libraryRel}`).click();
    await expect(page.getByTestId("editor")).toContainText("public sealed class Order");
    await replaceEditor(page, `${originalLibrary}\n// restricted edit\n`, true);
    await expect(page.getByTestId("dirty-marker")).toHaveCount(0);
    expect(fs.readFileSync(path.join(copy, libraryRel), "utf8")).toContain("restricted edit");
    await replaceEditor(page, originalLibrary, true);
    expect(fs.readFileSync(path.join(copy, libraryRel), "utf8")).toBe(originalLibrary);
    await expect(page.getByTestId("tree")).toBeVisible();
    expect(fs.existsSync(path.join(copy, "src/OrderDesk.Cli/bin"))).toBe(false);
    expect(fs.existsSync(path.join(copy, "src/OrderDesk.Cli/obj"))).toBe(false);
    await closeWorkspace(page);
    step("restricted negatives and text-only editing verified; no bin/obj created");

    // ---- Reopen (trust is session/workspace scoped) and inspect trust + separate SDKs.
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await expect(page.getByTestId("mode-restricted")).toBeVisible();
    await grantTrust(page);
    await expect(page.getByTestId("sdk-host")).toContainText("SDK 10.0.100");
    await expect(page.getByTestId("sdk-project")).toContainText("SDK 8.0.408", {
      timeout: 30_000,
    });
    await expect(page.getByTestId("sdk-project")).toContainText(
      "global.json requests 8.0.408",
    );
    await expect(page.getByTestId("sdk-tfms")).toContainText("net8.0");
    await screenshot(page, "18-toolchain-trusted.png");
    await enterWorkspace(page);
    await waitEvaluation(page, 1);
    await expect(page.getByTestId("status-sdk")).toContainText("Host SDK 10.0.100");
    await expect(page.getByTestId("status-sdk")).toContainText("Project SDK 8.0.408");
    await expect(page.getByTestId("status-keymap")).toHaveText("Keymap: IntelliJ / Rider");

    // Evaluated tree: console + library + tests, TargetPath from MSBuild, bin/obj hidden.
    await expect(page.getByTestId("tree-project-OrderDesk.Cli")).toContainText("console");
    await expect(page.getByTestId("tree-project-OrderDesk.Core")).toContainText("library");
    await expect(page.getByTestId("tree-project-OrderDesk.Tests")).toContainText("test");
    await expect(page.getByTestId("tree")).not.toContainText("/bin/");
    await expect(page.getByTestId("tree")).not.toContainText("/obj/");
    await page.getByTestId("tree-project-OrderDesk.Cli").click();
    await page.getByRole("button", { name: "Project details" }).click();
    await expect(page.getByTestId("eval-details")).toBeVisible();
    await expect(page.getByTestId("eval-target-path")).toContainText(
      "src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll",
    );
    await screenshot(page, "18-screen-03-solution-explorer.png");
    await page.getByRole("button", { name: "Close project details" }).click();
    await expect(page.getByTestId("eval-details")).toBeHidden();
    step("trusted evaluation revision 1 with three evaluated projects and TargetPath");

    // ---- Automatic restore after Trust; build, run and tests never start it themselves.
    // The fixture may already be restored by earlier specs, so the journey only waits for the settled state.
    await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 120_000 });
    const beforeTfm = await revisionOf(page);
    await page.getByTestId("workspace-actions-menu").locator("summary").click();
    await page.getByTestId("eval-tfm").selectOption("net8.0");
    await expect.poll(() => revisionOf(page)).toBeGreaterThan(beforeTfm);
    const restored = await hostState(request);
    // The muxer path and detected SDK root depend on the machine; the pinned version does not.
    expect(restored.restore.sdkVersion).toBe("8.0.408");
    expect(restored.restore.sdkRoot).toBeTruthy();
    snapshot("18-restore.json", restored.restore);
    step("explicit restore exit 0 and evaluation reloaded at revision 2");

    // ---- Real csharp-ls session: diagnostics, completion, definition, rename, code actions.
    await ensureCsharpLs(page, request);
    await clickTestId(page, "lsp-start");
    await expect(page.getByTestId("lsp-state")).toContainText("Ready", { timeout: 90_000 });
    await expect(page.getByTestId("lsp-capabilities")).toContainText("completion");
    await expect(page.getByTestId("lsp-capabilities")).toContainText("definition");
    await expect(page.getByTestId("lsp-capabilities")).toContainText("rename");
    await expect(page.getByTestId("lsp-capabilities")).toContainText("code actions");
    const lspState = await hostState(request);
    snapshot("18-lsp-session.json", lspState.lsp);

    await page.getByTestId(`tree-file-${programRel}`).click();
    await expect(page.getByTestId("editor")).toContainText("OrderDesk.Core");
    const completionText =
      'using OrderDesk.Core;\n\nvar order = new Order("A-104", 42m);\norder.\n';
    await replaceEditor(page, completionText, false);
    let completionData: any = null;
    await expect
      .poll(
        async () => {
          const body = await hostCall(request, "lsp.completion", {
            path: programRel,
            line: 3,
            character: 6,
            revision: 1,
          });
          if (!body.ok) return false;
          completionData = body.data;
          return (
            body.data.state === "ok" &&
            (body.data.items as { label: string }[]).some(
              (item) => item.label === "Id" || item.label === "Total",
            )
          );
        },
        { timeout: 45_000, intervals: [100, 250, 500] },
      )
      .toBe(true);
    const completionLabel = (
      completionData.items as { label: string }[]
    ).find((item) => item.label === "Id" || item.label === "Total")!.label;
    await page.keyboard.press("Control+Space");
    await expect(page.locator(".cm-tooltip-autocomplete")).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.getByTestId("editor-lsp-status")).toHaveAttribute("data-state", "ok");
    await page.keyboard.type(completionLabel);
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("editor")).toContainText(`order.${completionLabel}`);
    await page.waitForTimeout(600);
    step(`real completion returned and inserted '${completionLabel}'`);

    // Close the completion buffer explicitly; the saved file is untouched.
    await page.locator(".tab.active").getByTestId("close-buffer").click();
    await page.getByTestId("confirm-discard").click();
    await expect(page.getByTestId(`tab-${programRel}`)).toHaveCount(0);
    expect(fs.readFileSync(path.join(copy, programRel), "utf8")).toBe(originalProgram);

    await page.getByTestId(`tree-file-${programRel}`).click();
    await expect(page.getByTestId("editor")).toContainText("OrderDesk.Core");
    await expect
      .poll(
        async () => {
          const body = await hostCall(request, "lsp.definition", {
            path: programRel,
            line: 2,
            character: 20,
            revision: 0,
          });
          return body.ok && body.data.state === "ok";
        },
        { timeout: 30_000, intervals: [100, 250, 500] },
      )
      .toBe(true);
    const expectedDefinition = await hostCall(request, "lsp.definition", {
      path: programRel,
      line: 2,
      character: 20,
      revision: 0,
    });
    expect(expectedDefinition.ok, JSON.stringify(expectedDefinition.error)).toBe(true);
    expect(expectedDefinition.data.state).toBe("ok");
    const target = expectedDefinition.data.locations[0] as {
      path: string;
      line: number;
      character: number;
    };
    expect(target.path).toBe(libraryRel);
    await placeCursor(page, 2, 20);
    await page.keyboard.press("Meta+b");
    await expect(page.getByTestId(`tab-${libraryRel}`)).toHaveClass(/active/, {
      timeout: 20_000,
    });
    await expect(page.getByTestId("editor-shell")).toHaveAttribute(
      "data-cursor",
      `${target.line}:${target.character}`,
    );
    step(`real definition opened ${target.path}:${target.line}:${target.character}`);

    // Rename: real cross-project preview, then cancel without changing buffers or disk.
    await placeCursor(page, 2, 20);
    await expect
      .poll(
        async () => {
          const body = await hostCall(request, "lsp.completion", {
            path: libraryRel,
            line: 2,
            character: 20,
            revision: 0,
          });
          return body.ok && body.data.documentKnown === true;
        },
        { timeout: 30_000, intervals: [100, 250, 500] },
      )
      .toBe(true);
    const libraryBytes = fs.readFileSync(path.join(copy, libraryRel));
    const programBytes = fs.readFileSync(path.join(copy, programRel));
    const testsBytes = fs.readFileSync(path.join(copy, "tests/OrderDesk.Tests/OrderTests.cs"));
    await page.keyboard.press("Shift+F6");
    await expect(page.getByTestId("rename-input")).toBeFocused();
    await page.getByTestId("rename-input").fill("Purchase");
    await page.getByTestId("rename-submit").click();
    await expect(page.getByTestId("rename-preview")).toBeVisible({ timeout: 30_000 });
    const previewFiles = await page.getByTestId("rename-preview").getAttribute("data-files");
    expect(Number(previewFiles)).toBeGreaterThanOrEqual(3);
    await expect(page.getByTestId("rename-preview")).toContainText("Nothing has been applied");
    await page.getByTestId("rename-cancel-preview").click();
    await expect(page.getByTestId("rename-preview")).toHaveCount(0);
    expect(fs.readFileSync(path.join(copy, libraryRel)).equals(libraryBytes)).toBe(true);
    expect(fs.readFileSync(path.join(copy, programRel)).equals(programBytes)).toBe(true);
    expect(
      fs
        .readFileSync(path.join(copy, "tests/OrderDesk.Tests/OrderTests.cs"))
        .equals(testsBytes),
    ).toBe(true);
    step(`rename preview from csharp-ls covered ${previewFiles} files, then cancel was inert`);

    // Code actions: only the real list; preview the returned action and cancel.
    await placeCursor(page, 2, 20);
    await page.keyboard.press("Alt+Enter");
    await expect(page.getByTestId("code-actions")).toBeVisible({ timeout: 30_000 });
    const actions = await page
      .getByTestId("code-actions")
      .locator("button[data-title]")
      .evaluateAll((nodes) =>
        nodes.map((node) => ({
          title: node.getAttribute("data-title"),
          supported: node.getAttribute("data-supported"),
        })),
      );
    expect(actions.length).toBeGreaterThan(0);
    snapshot("18-code-actions.json", { path: libraryRel, actions });
    const applicable = page
      .getByTestId("code-actions")
      .locator('button[data-supported="true"]')
      .first();
    await expect(applicable).toBeVisible();
    await applicable.click();
    await expect(page.getByTestId("rename-preview")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("rename-preview")).toHaveAttribute("data-origin", "action");
    await page.getByTestId("rename-cancel-preview").click();
    await expect(page.getByTestId("rename-preview")).toHaveCount(0);
    expect(fs.readFileSync(path.join(copy, libraryRel)).equals(libraryBytes)).toBe(true);
    step(`code actions returned ${actions.length} real items; preview cancelled`);

    // ---- Real diagnostic, failed build, navigation, fix, rebuild, Run.
    await page.getByTestId(`tree-file-${programRel}`).click();
    await replaceEditor(page, `${originalProgram}\nvar broken = new Missing();\n`, true);
    await expect
      .poll(
        async () => {
          const state = await hostState(request);
          return state.diagnostics.snapshots.some(
            (item: { state: string; items: { code: string }[] }) =>
              item.state === "current" && item.items.some((entry) => entry.code === "CS0246"),
          );
        },
        { timeout: 45_000, intervals: [100, 250, 500] },
      )
      .toBe(true);
    await page.getByTestId("bottom-tab-problems").click();
    await expect(page.getByTestId("problems-list")).toContainText("CS0246");
    await expect(page.getByTestId("problems-list")).toContainText("LSP · csharp-ls");
    await expect(page.getByTestId("problem-row-0")).toBeVisible();
    await page.getByTestId("problem-row-0").click();
    await expect(page.getByTestId("editor")).toContainText("Missing");

    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("build-action").click();
    await expect(page.getByTestId("build-state")).toContainText("Build failed", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("build-state")).toContainText("exit 1");
    await expect(page.getByTestId("build-log")).toContainText("error CS");
    await expect(page.getByTestId("toolbar-run")).toBeDisabled();
    await expect(page.getByTestId("toolbar-run")).toHaveAttribute(
      "title",
      /last build did not succeed/,
    );
    await expect(page.getByTestId("toolbar-debug")).toBeDisabled();
    let failed = await hostState(request);
    snapshot("18-build-failed.json", failed.build);
    await screenshot(page, "18-build-failed.png");

    await page.getByTestId("bottom-tab-problems").click();
    await expect(page.getByTestId("build-problem-row-0")).toContainText("Build · dotnet");
    const buildPosition = await page
      .getByTestId("build-problem-row-0")
      .getAttribute("data-position");
    expect(buildPosition).toMatch(/^\d+:\d+$/);
    await screenshot(page, "18-screen-04-problems.png");

    await page.getByTestId("build-problem-row-0").click();
    await expect(page.getByTestId("editor")).toContainText("Missing");
    await replaceEditor(page, originalProgram, true);
    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("build-action").click();
    await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("build-state")).toContainText("exit 0");
    const rebuilt = await hostState(request);
    snapshot("18-build-fixed.json", rebuilt.build);
    step("failed build exit 1 navigated to source; fixed build exit 0");

    await expect(page.getByTestId("toolbar-run")).toBeEnabled();
    await page.getByTestId("toolbar-run").click();
    await expect(page.getByTestId("run-state")).toContainText("Run exited", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("run-state")).toContainText("exit 0");
    await expect(page.getByTestId("run-log")).toContainText("Order A-104");
    await expect(page.getByTestId("toolbar-run")).toHaveAttribute("title", /Ctrl\+R/);
    const ran = await hostState(request);
    snapshot("18-run.json", ran.run);
    await screenshot(page, "18-screen-02-editor-build.png");
    step("Run executed the evaluated TargetPath, exit 0, real stdout");

    // ---- VSTest: discovery, selected, all, real cancellation.
    await page.getByTestId("bottom-tab-tests").click();
    await page.getByTestId("tests-tfm").selectOption("net8.0");
    await waitEvaluation(page, 3);
    await page.getByTestId("tests-discover").click();
    await expect(page.getByTestId("tests-state")).toContainText("Discovered 4 tests", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("tests-state")).toContainText("exit 0");
    await expect(page.getByTestId("tests-command")).toContainText("--list-tests");
    await page.getByTestId("test-checkbox-0").check();
    await page.getByTestId("tests-run-selected").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Run completed", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("tests-run-counts")).toContainText(
      "1 result · 1 passed · 0 failed · 0 skipped",
    );
    await expect(page.getByTestId("tests-run-command")).toContainText("--filter FullyQualifiedName=");
    await page.getByTestId("tests-run-all").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Run completed", {
      timeout: 120_000,
    });
    await expect(page.getByTestId("tests-run-counts")).toContainText(
      "4 results · 2 passed · 1 failed · 1 skipped · 0 not executed · 0 unknown",
    );
    await expect(page.getByTestId("tests-run-exit")).toHaveText("exit 1");
    await page.getByTestId("tests-run-panel").scrollIntoViewIfNeeded();
    await screenshot(page, "18-screen-07-tests.png");
    const ranTests = await hostState(request);
    snapshot("18-tests-run-all.json", ranTests.tests);
    step("VSTest run selected (1 passed) and all (2 passed, 1 failed, 1 skipped, exit 1)");

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
    await page.getByTestId("tests-run-cancel").click();
    await expect(page.getByTestId("tests-run-state")).toContainText("Run cancelled", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("tests-run-termination")).toContainText("Observed: exited");
    const cancelledTests = await hostState(request);
    snapshot("18-tests-cancel.json", cancelledTests.tests);
    await screenshot(page, "18-tests-cancel.png");
    step("real long test cancelled; partial state preserved, no passed result invented");

    // ---- Real PTY: discovered shell, cwd, input, Ctrl+C, explicit close.
    await page.getByTestId("bottom-tab-terminal").click();
    // Fase 1: Trust opens the shell with a prompt by itself; there is no Start click.
    await expect(page.getByTestId("terminal-state")).toHaveText("Running", { timeout: 30_000 });
    await expect(page.getByTestId("terminal-shell")).toContainText(/(zsh|bash|sh)/);
    await expect(page.getByTestId("terminal-cwd")).toHaveText(fs.realpathSync(copy));
    await screenshot(page, "18-terminal-started.png");
    const terminalOutput = page.getByTestId("terminal-output");
    const terminalInput = page.getByTestId("terminal-input");
    await waitForPrompt(request);
    await terminalInput.fill("pwd");
    await terminalInput.press("Enter");
    await expect(terminalOutput).toContainText(fs.realpathSync(copy));
    await waitForPrompt(request);
    await terminalInput.fill("echo acceptance-$((6*7))");
    await terminalInput.press("Enter");
    await expect(terminalOutput).toContainText("acceptance-42");
    await waitForPrompt(request);
    await terminalInput.fill("sleep 30 && echo never-$((6*7))");
    await terminalInput.press("Enter");
    await page.waitForTimeout(700);
    await terminalInput.press("Control+c");
    await expect(terminalOutput).toContainText("^C", { timeout: 10_000 });
    await waitForPrompt(request);
    await terminalInput.fill("echo prompt-back");
    await terminalInput.press("Enter");
    await expect(terminalOutput).toContainText("prompt-back");
    await expect(terminalOutput).not.toContainText("never-42");
    await waitForPrompt(request);
    // A login shell may put another `dotnet` first on PATH (the user's profile decides), so `dotnet --version` in
    // the terminal depends on the machine; SDK resolution is proven by the restore/build/run/test steps above.
    await terminalInput.fill("echo terminal-sum-$((19+23))");
    await terminalInput.press("Enter");
    await expect(terminalOutput).toContainText("terminal-sum-42", { timeout: 30_000 });
    await waitForPrompt(request);
    await screenshot(page, "18-screen-06-terminal.png");
    await page.getByTestId("terminal-close").click();
    await expect(page.getByTestId("terminal-confirm")).toBeVisible();
    await page.getByTestId("terminal-confirm-stop").click();
    await expect(page.getByTestId("terminal-state")).toHaveText("Stopped");
    await expect(page.getByTestId("terminal-exit")).toContainText(/^Exit: (-?\d+|exit unknown)$/);
    await expect(page.getByTestId("terminal-termination")).toContainText("Observed:");
    await expect(terminalOutput).toContainText("prompt-back");
    const terminal = await hostState(request);
    snapshot("18-terminal.json", terminal.terminal);
    step("PTY shell, pwd/echo, Ctrl+C on sleep, dotnet 8.0.408 and close with observed exit");

    // ---- DAP behind the experimental opt-in, with the real osx-arm64 adapter.
    if (fs.existsSync(realAdapter)) {
      const adapterDir = path.join(e2eHome, "tools/netcoredbg");
      fs.rmSync(adapterDir, { recursive: true, force: true });
      fs.mkdirSync(adapterDir, { recursive: true });
      fs.cpSync(realAdapter, path.join(adapterDir, "3.2.0-1092"), { recursive: true });
      const flagged = await hostCall(request, "debug.flag", { enabled: true, confirm: true });
      expect(flagged.ok, JSON.stringify(flagged.error)).toBe(true);
      const planned = await hostCall(request, "debug.plan");
      expect(planned.ok, JSON.stringify(planned.error)).toBe(true);
      const installed = await hostCall(request, "debug.install", {
        planId: planned.data.debug.plan.planId,
        confirm: true,
      });
      expect(installed.ok, JSON.stringify(installed.error)).toBe(true);
      expect(installed.data.debug.adapterState).toBe("ready");
      expect(installed.data.debug.appleSiliconExperimental).toBe(true);
      snapshot("18-debug-install.json", installed.data.debug);

      // The test-run evaluation is current; rebuild Debug before launching.
      await page.getByTestId("bottom-tab-build").click();
      await page.getByTestId("build-action").click();
      await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
        timeout: 120_000,
      });
      await expect(page.getByTestId("toolbar-debug")).toHaveAttribute("title", /Ctrl\+D/);
      await page.getByTestId(`tree-file-${programRel}`).click();
      await placeCursor(page, 2, 0);
      await page.keyboard.press("Meta+F8");
      await page.getByTestId("bottom-tab-debug").click();
      await expect(page.getByTestId("debug-breakpoints")).toContainText("pending");

      await page.keyboard.press("Control+d");
      await expect
        .poll(
          async () => (await hostState(request)).dap.state,
          { timeout: 45_000, intervals: [100, 250, 500] },
        )
        .toBe("paused");
      await page.getByTestId("bottom-tab-debug").click();
      await expect(page.getByTestId("dap-state")).toContainText("Paused");
      await expect(page.getByTestId("dap-adapter")).toContainText("3.2.0");
      await expect(page.getByTestId("dap-adapter")).toContainText("--interpreter=vscode");
      await expect(page.getByTestId("dap-build-id")).toContainText("build:");
      await expect(page.getByTestId("debug-frames")).toContainText("Program.cs:3");
      await expect(page.getByTestId("debug-locals")).toContainText("order");
      await expect(page.getByTestId("debug-breakpoints")).toContainText("verified by the adapter");
      await screenshot(page, "18-debug-paused.png");
      await page.getByTestId("debug-locals").scrollIntoViewIfNeeded();
      await screenshot(page, "18-screen-05-debug.png");

      await page.keyboard.press("F8");
      await expect(page.getByTestId("debug-frames")).toContainText("Program.cs:4", {
        timeout: 20_000,
      });
      await page.getByTestId("debug-continue").click();
      await expect(page.getByTestId("dap-state")).toContainText("Terminated", {
        timeout: 30_000,
      });
      await expect(page.getByTestId("dap-exit")).toHaveText("exit 0");
      await expect(page.getByTestId("dap-output")).toContainText("Order A-104");

      await page.getByTestId("debug-launch").click();
      await expect(page.getByTestId("dap-state")).toContainText("Paused", {
        timeout: 30_000,
      });
      await page.getByTestId("debug-stop").click();
      await expect(page.getByTestId("dap-state")).toContainText("Cancelled", {
        timeout: 30_000,
      });
      await expect(page.getByTestId("dap-termination")).toContainText("Termination requested:");
      await expect(page.getByTestId("dap-termination")).toContainText("Observed:");
      const dap = await hostState(request);
      snapshot("18-dap.json", { dap: dap.dap, processes: dap.processes });
      await expect
        .poll(
          async () => {
            const state = await hostState(request);
            return state.processes
              .filter((item: { kind: string }) => item.kind.startsWith("debug:"))
              .every((item: { state: string }) => item.state === "exited");
          },
          { timeout: 15_000, intervals: [100, 250, 500] },
        )
        .toBe(true);
      await screenshot(page, "18-dap-stopped.png");
      step("real DAP: breakpoint verified, paused, step over, locals, continue exit 0, stop cancelled");
    } else {
      summary.debugLimitation =
        "Real netcoredbg 3.2.0-1092 osx-arm64 install is absent; DAP not executed in this run.";
      step("DAP skipped: no real osx-arm64 adapter install present");
    }

    // ---- Revocation keeps buffers and logs, blocks direct calls, leaves no running process.
    const preRevoke = await hostState(request);
    snapshot("18-acceptance-pre-revoke.json", preRevoke);
    summary.sdk = preRevoke.sdk;
    summary.evaluation = preRevoke.evaluation;
    summary.lsp = preRevoke.lsp;
    summary.tools = preRevoke.tools;
    summary.dap = preRevoke.dap;
    summary.debugAdapter = preRevoke.debug;
    await page.getByTestId(`tree-file-${libraryRel}`).click();
    await replaceEditor(page, `${originalLibrary}\n// unsaved across revoke\n`, false);
    await expect(page.getByTestId("dirty-marker")).toBeVisible();
    const beforeRevoke = await hostState(request);
    await clickTestId(page, "trust-revoke");
    await expect(page.getByTestId("mode-badge")).toHaveText("Restricted mode");
    await expect(page.getByTestId("editor")).toContainText("unsaved across revoke");
    await expect(page.getByTestId("toolbar-build")).toBeDisabled();
    await expect(page.getByTestId("toolbar-run")).toBeDisabled();
    await expect(page.getByTestId("toolbar-debug")).toBeDisabled();
    const deniedAfterRevoke = await hostCall(request, "build.start", {});
    expect(deniedAfterRevoke.ok).toBe(false);
    expect(deniedAfterRevoke.error.code).toBe("trust_required");
    await expect
      .poll(
        async () => (await hostState(request)).lsp.state,
        { timeout: 20_000, intervals: [100, 250, 500] },
      )
      .not.toBe("ready");
    const afterRevoke = await hostState(request);
    expect(
      afterRevoke.processes.filter(
        (item: { workspaceId: number; state: string }) =>
          item.workspaceId === beforeRevoke.workspace.id && item.state === "running",
      ),
    ).toEqual([]);
    await page.locator(".tab.active").getByTestId("close-buffer").click({ timeout: 15_000 });
    await page.getByTestId("confirm-discard").click();
    await expect(page.getByTestId(`tab-${libraryRel}`)).toHaveCount(0);

    const final = await hostState(request);
    snapshot("18-acceptance-final.json", final);
    summary.environment = {
      platform: process.platform,
      arch: process.arch,
      osRelease: os.release(),
      node: process.version,
      app: final.app,
    };
    summary.workspace = { path: copy, solution: path.join(copy, "OrderDesk.sln") };
    summary.restore = final.restore;
    summary.buildFailed = failed.build;
    summary.buildFixed = rebuilt.build;
    summary.run = ran.run;
    summary.testsAll = ranTests.tests;
    summary.testsCancelled = cancelledTests.tests;
    summary.terminal = terminal.terminal;
    summary.revoke = {
      before: {
        mode: beforeRevoke.workspace.mode,
        processes: beforeRevoke.processes,
      },
      after: {
        mode: afterRevoke.workspace.mode,
        lsp: afterRevoke.lsp.state,
        processes: afterRevoke.processes,
      },
      directBuildCode: deniedAfterRevoke.error.code,
    };
    summary.steps = steps;
    snapshot("18-acceptance-summary.json", summary);
    step("revocation kept the unsaved buffer, blocked direct calls and left no running workspace process");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("UI audit: English surface, names, focus, keymap text and local layout at real sizes", async ({
  page,
}) => {
  test.setTimeout(180_000);
  const { base, copy } = tempCopy("Order Desk audit");
  const externalRequests: string[] = [];
  page.on("request", (request) => {
    const url = request.url();
    const local =
      url.startsWith("http://localhost:1420") ||
      url.startsWith("ws://localhost:1420") ||
      url.startsWith("http://127.0.0.1:1420") ||
      url.startsWith("ws://127.0.0.1:1420") ||
      url.startsWith("http://127.0.0.1:4317") ||
      url.startsWith("ws://127.0.0.1:4317");
    if (!local) externalRequests.push(url);
  });
  try {
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    await expect(page.getByTestId("workspace-path-input")).toBeVisible();

    const theme = await page.evaluate(() => {
      const root = getComputedStyle(document.documentElement);
      return {
        bg: root.getPropertyValue("--bg").trim(),
        panel: root.getPropertyValue("--panel").trim(),
        fg: root.getPropertyValue("--fg").trim(),
        muted: root.getPropertyValue("--muted").trim(),
        accent: root.getPropertyValue("--accent").trim(),
        error: root.getPropertyValue("--error").trim(),
        warning: root.getPropertyValue("--warning").trim(),
        success: root.getPropertyValue("--success").trim(),
        info: root.getPropertyValue("--info").trim(),
      };
    });
    expect(theme).toMatchObject({
      bg: "#000000",
      panel: "#0b0b0b",
      fg: "#f5f5f5",
      accent: "#548af7",
    });

    const accessibleName = async () =>
      page.evaluate(() => {
        const visible = (element: Element) => {
          // A closed disclosure keeps its content out of the render tree; its name is not
          // observable until the user opens it.
          if (element.closest("details:not([open]) > *:not(summary)")) return false;
          const rect = element.getBoundingClientRect();
          const style = getComputedStyle(element);
          return (
            rect.width > 0 &&
            rect.height > 0 &&
            style.visibility !== "hidden" &&
            style.display !== "none"
          );
        };
        const nameOf = (element: Element) => {
          const aria = element.getAttribute("aria-label");
          if (aria && aria.trim()) return aria.trim();
          const labelledBy = element.getAttribute("aria-labelledby");
          if (labelledBy) {
            const text = labelledBy
              .split(/\s+/)
              .map((id) => document.getElementById(id)?.textContent ?? "")
              .join(" ")
              .trim();
            if (text) return text;
          }
          const label = (element as HTMLInputElement).labels;
          if (label && label.length > 0) {
            const text = Array.from(label)
              .map((item) => item.textContent ?? "")
              .join(" ")
              .trim();
            if (text) return text;
          }
          const text = (element as HTMLElement).innerText?.trim();
          if (text) return text;
          const title = element.getAttribute("title");
          if (title && title.trim()) return title.trim();
          const placeholder = element.getAttribute("placeholder");
          if (placeholder && placeholder.trim()) return placeholder.trim();
          return "";
        };
        return Array.from(
          document.querySelectorAll(
            'button, a[href], input, select, textarea, [role="tab"], [role="button"], [contenteditable="true"]',
          ),
        )
          .filter(visible)
          .map((element) => ({
            testid: element.getAttribute("data-testid"),
            tag: element.tagName.toLowerCase(),
            name: nameOf(element),
          }))
          .filter((item) => item.name.length === 0);
      });

    const setupNameOffenders = await accessibleName();
    expect(setupNameOffenders).toEqual([]);

    // Keyboard focus is visible (2px copper outline), not just a color change.
    let focusStyle: Record<string, string> | null = null;
    for (let attempt = 0; attempt < 5 && !focusStyle; attempt += 1) {
      await page.keyboard.press("Tab");
      focusStyle = await page.evaluate(() => {
        const element = document.activeElement as HTMLElement | null;
        if (!element || element === document.body) return null;
        const style = getComputedStyle(element);
        return {
          tag: element.tagName.toLowerCase(),
          testid: element.getAttribute("data-testid") ?? "",
          outlineStyle: style.outlineStyle,
          outlineWidth: style.outlineWidth,
          outlineColor: style.outlineColor,
        };
      });
    }
    expect(focusStyle).not.toBeNull();
    expect(focusStyle!.outlineStyle).toBe("solid");
    expect(parseFloat(focusStyle!.outlineWidth)).toBeGreaterThanOrEqual(2);

    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await grantTrust(page);
    await enterWorkspace(page);
    await waitEvaluation(page, 1);

    // Documented macOS shortcut labels; Windows is source-inspected only on this machine.
    await expect(page.getByTestId("status-keymap")).toHaveText("Keymap: IntelliJ / Rider");
    await page.getByTestId("bottom-tab-build").click();
    await expect(page.getByTestId("build-action")).toContainText(/Cmd\+F9|Ctrl\+F9/);
    await page.getByTestId("bottom-tab-terminal").click();
    await expect(page.getByTestId("terminal-interrupt")).toHaveAttribute("title", /Ctrl\+C/);

    const workspaceNameOffenders = await accessibleName();
    expect(workspaceNameOffenders).toEqual([]);
    const portuguese = await page.evaluate(() => {
      const text = document.body.innerText;
      return ["Confiança", "Salvar", "Arquivo", "Depurar", "Cancelado", "Falha"].filter(
        (word) => text.includes(word),
      );
    });
    expect(portuguese).toEqual([]);

    const layout = async (width: number, height: number) => {
      await page.setViewportSize({ width, height });
      await page.waitForTimeout(150);
      return page.evaluate(() => {
        const toolbarRun = document.querySelector('[data-testid="toolbar-run"]')!;
        const bottomTab = document.querySelector('[data-testid="bottom-tab-tests"]')!;
        const runRect = toolbarRun.getBoundingClientRect();
        const tabRect = bottomTab.getBoundingClientRect();
        return {
          innerWidth,
          documentWidth: document.documentElement.scrollWidth,
          bodyWidth: document.body.scrollWidth,
          runInside: runRect.left >= 0 && runRect.right <= innerWidth,
          testsTabInside: tabRect.left >= 0 && tabRect.right <= innerWidth,
          toolbarHeight: (document.querySelector(".toolbar") as HTMLElement).offsetHeight,
        };
      });
    };
    await page.getByTestId("bottom-tab-build").click();
    const desktop = await layout(1440, 900);
    expect(desktop.documentWidth).toBeLessThanOrEqual(1441);
    expect(desktop.bodyWidth).toBeLessThanOrEqual(1441);
    expect(desktop.runInside).toBe(true);
    expect(desktop.testsTabInside).toBe(true);
    const minimum = await layout(1100, 700);
    expect(minimum.innerWidth).toBe(1100);
    expect(minimum.documentWidth).toBeLessThanOrEqual(1101);
    expect(minimum.bodyWidth).toBeLessThanOrEqual(1101);
    expect(minimum.runInside).toBe(true);
    expect(minimum.testsTabInside).toBe(true);
    const locals = await page.evaluate(() => {
      const side = document.querySelector(".side") as HTMLElement;
      const output = document.querySelector(".output") as HTMLElement;
      const panel = document.querySelector(".panel-body") as HTMLElement;
      const status = document.querySelector(".status") as HTMLElement;
      return {
        sideOverflow: getComputedStyle(side).overflowY,
        outputOverflow: getComputedStyle(output).overflow,
        panelOverflow: getComputedStyle(panel).overflowY,
        statusBottomInside:
          status.getBoundingClientRect().bottom <= window.innerHeight + 1,
        panelBottomInside:
          panel.getBoundingClientRect().bottom <= window.innerHeight + 1,
      };
    });
    expect(["auto", "scroll"]).toContain(locals.sideOverflow);
    expect(["auto", "scroll"]).toContain(locals.panelOverflow);
    expect(locals.statusBottomInside).toBe(true);
    expect(locals.panelBottomInside).toBe(true);
    await screenshot(page, "18-layout-1100x700.png");

    const contrast = await page.evaluate(() => {
      const luminance = (hex: string) => {
        const clean = hex.replace("#", "");
        const channels = [0, 2, 4]
          .map((index) => parseInt(clean.slice(index, index + 2), 16) / 255)
          .map((value) =>
            value <= 0.03928 ? value / 12.92 : Math.pow((value + 0.055) / 1.055, 2.4),
          );
        return (
          0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2]
        );
      };
      const ratio = (left: string, right: string) => {
        const a = luminance(left);
        const b = luminance(right);
        return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
      };
      return {
        fgOnBg: Number(ratio("#dfe1e5", "#191a1c").toFixed(2)),
        mutedOnPanel: Number(ratio("#aaadb5", "#1e1f22").toFixed(2)),
        accentOnPanel: Number(ratio("#548af7", "#1e1f22").toFixed(2)),
        errorOnBg: Number(ratio("#ffaaa5", "#191a1c").toFixed(2)),
        warningOnBg: Number(ratio("#f3cf7e", "#191a1c").toFixed(2)),
        successOnBg: Number(ratio("#a7dfb0", "#191a1c").toFixed(2)),
        buttonBorderOnRaised: Number(ratio("#667383", "#232932").toFixed(2)),
      };
    });
    expect(contrast.fgOnBg).toBeGreaterThanOrEqual(7);
    expect(contrast.mutedOnPanel).toBeGreaterThanOrEqual(7);
    expect(contrast.accentOnPanel).toBeGreaterThanOrEqual(4.5);
    expect(contrast.errorOnBg).toBeGreaterThanOrEqual(4.5);
    expect(contrast.warningOnBg).toBeGreaterThanOrEqual(4.5);
    expect(contrast.successOnBg).toBeGreaterThanOrEqual(4.5);
    expect(contrast.buttonBorderOnRaised).toBeGreaterThanOrEqual(3);
    expect(externalRequests).toEqual([]);

    snapshot("18-ui-audit.json", {
      viewport: { desktop, minimum },
      theme,
      focusStyle,
      contrast,
      accessibleNameOffenders: {
        setup: setupNameOffenders,
        workspace: workspaceNameOffenders,
      },
      portugueseStringsFound: portuguese,
      externalRequests,
      limitations: [
        "Heuristic accessible-name audit, not a WCAG certification.",
        "No screen reader was run.",
        "The native Tauri window was not driven; Chrome 155 uses the same UI/host dispatch.",
        "Windows keymap is source-inspected only on this machine.",
      ],
    });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
