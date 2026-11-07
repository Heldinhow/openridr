import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
import { settleDiscovery } from "./automatic";
import { host } from "./environment";
const screenshots = path.resolve(process.env.OPENRIDR_SCREENSHOT_DIR ?? "docs/evidence/screenshots");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(180_000);

const originalProgram = fs.readFileSync(
  path.join(fixture, "src/OrderDesk.Cli/Program.cs"),
  "utf8",
);

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-build-"));
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return { base, copy };
}

async function openTrustedEvaluated(page: Page, copy: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", {
    timeout: 30_000,
  });
}

// Trust restores and evaluates by itself (docs/decisions.md); wait for that to settle. The
// automatic test discovery also builds the test project, and a build started while it runs can
// collide on the same output file (#251).
async function restoreAndReload(page: Page) {
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
    timeout: 60_000,
  });
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });
  await settleDiscovery(page);
}

async function replaceProgram(page: Page, text: string) {
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await expect(page.getByTestId("editor")).toBeVisible();
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+A");
  await page.keyboard.insertText(text);
  await page.keyboard.press("Meta+s");
  await expect(page.getByTestId("app-log")).toContainText("Saved src/OrderDesk.Cli/Program.cs");
}

const buildLogLimit = 20_000;

/** The raw output plus the command and cwd behind it, read from the DOM after a failed wait. */
async function buildFailureDetail(page: Page) {
  const read = async (testId: string) => {
    try {
      // Short timeout: on the failure path a missing element must not eat the test budget.
      return (await page.getByTestId(testId).textContent({ timeout: 2_000 })) ?? "";
    } catch (error) {
      return `<unreadable: ${String(error)}>`;
    }
  };
  const log = await read("build-log");
  const clipped =
    log.length > buildLogLimit ? `${log.slice(0, buildLogLimit)}\n… clipped` : log;
  return [
    `build-command: ${await read("build-command")}`,
    `build-cwd: ${await read("build-cwd")}`,
    `build-log:\n${clipped}`,
  ].join("\n");
}

/**
 * Waits for the build state and, on failure, appends the raw dotnet output to the error: the
 * message alone ("Build failed · exit 1") is not diagnosable once the page is gone.
 */
async function expectBuildState(page: Page, text: string, timeout?: number) {
  try {
    await expect(page.getByTestId("build-state")).toContainText(text, { timeout });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`${message}\n\n${await buildFailureDetail(page)}`);
  }
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("a real build reports its raw log, fails on a compile error and navigates to it", async ({
  page,
}) => {
  const { base, copy } = tempCopy("Order Desk build");
  try {
    await openTrustedEvaluated(page, copy);
    await restoreAndReload(page);
    fs.mkdirSync(screenshots, { recursive: true });

    // The startup select offers only console projects; the library is never a candidate.
    const startup = page.getByTestId("build-startup");
    await expect(startup.locator("option")).toHaveCount(1);
    await expect(startup).toHaveValue("src/OrderDesk.Cli/OrderDesk.Cli.csproj");
    await expect(startup).not.toContainText("OrderDesk.Core");

    // The primary action names its scope: an incremental build of the startup project.
    const buildAction = page.getByTestId("build-action");
    await expect(buildAction).toContainText("Build startup");
    await expect(buildAction).toContainText(/Cmd\+F9|Ctrl\+F9/);
    await expect(buildAction).not.toContainText("Rebuild");

    await page.getByTestId("build-action").click();
    await expectBuildState(page, "Build succeeded", 60_000);
    await expectBuildState(page, "exit 0");
    await expect(page.getByTestId("build-command")).toContainText("dotnet build");
    await expect(page.getByTestId("build-command")).toContainText("--no-restore");
    await expect(page.getByTestId("build-command")).toContainText("-c Debug");
    await expect(page.getByTestId("build-command")).toHaveCSS("white-space", "pre");
    await expect(page.getByTestId("build-command")).toHaveCSS("user-select", "text");
    await expect(page.getByTestId("build-cwd")).toContainText("Order Desk build");
    await expect(page.getByTestId("build-sdk")).toContainText("8.0.408");
    await expect(page.getByTestId("build-project")).toContainText("OrderDesk.Cli");
    await expect(page.getByTestId("build-project")).toContainText("console");
    await expect(page.getByTestId("build-target-path")).toContainText(
      "src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll",
    );
    await expect(page.getByTestId("build-saved-note")).toContainText(
      "no unsaved buffers",
    );
    await expect(page.getByTestId("build-log")).toContainText("Build succeeded");
    await page.screenshot({ path: path.join(screenshots, "11-build-ok.png") });

    // A solution build does not change the startup button's scope: it still says Build startup
    // and points at Build Solution for the whole solution.
    await expect(buildAction).toContainText("Build startup");
    await expect(buildAction).not.toContainText("Rebuild");
    await page.getByTestId("build-solution").click();
    await expectBuildState(page, "Build succeeded", 60_000);
    await expect(buildAction).toContainText("Build startup");
    await expect(buildAction).not.toContainText("Rebuild");
    await expect(buildAction).toHaveAttribute("title", /Build Solution/);

    // A real compiler error, saved explicitly through the editor.
    await replaceProgram(page, `${originalProgram}\nnew Missing();\n`);
    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("build-action").click();
    await expectBuildState(page, "Build failed", 60_000);
    await expectBuildState(page, "exit 1");
    await expect(page.getByTestId("build-log")).toContainText("error CS");
    await expect(page.getByTestId("build-problems-count")).toContainText("1 build problem");
    await page.screenshot({ path: path.join(screenshots, "11-build-failed.png") });

    // The location span is host-made; the UI only turns it into a button. Clicking opens the
    // saved file at the exact position the log reported.
    const logLink = page.getByTestId("build-log").getByRole("button").first();
    await expect(logLink).toContainText("src/OrderDesk.Cli/Program.cs(");
    const linkPosition = await logLink.getAttribute("data-position");
    expect(linkPosition).toMatch(/^\d+:\d+$/);
    await logLink.click();
    await expect(page.getByTestId("editor-shell")).toHaveAttribute("data-cursor", linkPosition!);

    // Problems keeps Build · dotnet separate and navigates to the real position.
    await page.getByTestId("bottom-tab-problems").click();
    await expect(page.getByTestId("build-problems")).toBeVisible();
    const row = page.getByTestId("build-problem-row-0");
    await expect(row).toContainText("Build · dotnet");
    await expect(row).toContainText("unknown");
    await expect(row).toContainText("CS");
    const position = await row.getAttribute("data-position");
    expect(position).toMatch(/^\d+:\d+$/);
    await row.click();
    await expect(page.getByTestId("editor-shell")).toHaveAttribute("data-cursor", position!);
    await page.screenshot({ path: path.join(screenshots, "11-problems-build.png") });

    // Fixing the buffer makes the old build obsolete; it is never reported as fixed.
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(originalProgram);
    await page.keyboard.press("Meta+s");
    await expect(page.getByTestId("app-log")).toContainText("Saved src/OrderDesk.Cli/Program.cs");
    await page.getByTestId("bottom-tab-problems").click();
    await expect(page.getByTestId("build-problem-obsolete")).toBeVisible();
    await expect(page.getByTestId("build-problem-row-0")).toBeVisible();

    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("build-action").click();
    await expectBuildState(page, "Build succeeded", 60_000);
    await expectBuildState(page, "exit 0");
    await expect(page.getByTestId("build-problems-count")).toContainText("0 build problems");
    await page.screenshot({ path: path.join(screenshots, "11-build-fixed.png") });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("build waits for the automatic restore and asks before using unsaved buffers", async ({
  page,
}) => {
  const { base, copy } = tempCopy("Order Desk build pending");
  try {
    await openTrustedEvaluated(page, copy);
    await restoreAndReload(page);
    await expect(page.getByTestId("build-action")).toBeEnabled();

    // An unsaved buffer: the decision is explicit, and the build uses saved content only.
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(`${originalProgram}\n// unsaved only\n`);
    await expect(page.getByTestId("dirty-marker")).toBeVisible();
    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("build-action").click();
    await expect(page.getByTestId("build-dirty-dialog")).toBeVisible();
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({ path: path.join(screenshots, "11-build-dirty.png") });
    await page.getByTestId("build-saved-only").click();
    await expectBuildState(page, "Build succeeded", 60_000);
    await expect(page.getByTestId("build-saved-note")).toContainText("1 unsaved buffer excluded");
    await expect(page.getByTestId("build-log")).toContainText("Build succeeded");

    // Cmd+F9 asks again instead of saving silently.
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(`${originalProgram}\n// unsaved again\n`);
    await page.keyboard.press("Meta+F9");
    await expect(page.getByTestId("build-dirty-dialog")).toBeVisible();
    await page.getByTestId("build-decision-cancel").click();
    await expect(page.getByTestId("build-dirty-dialog")).toHaveCount(0);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
