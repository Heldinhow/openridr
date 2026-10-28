import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const host = "http://127.0.0.1:4317";
const screenshots = path.resolve(process.cwd(), "docs/evidence/screenshots");

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
  await page.getByTestId("trust-checkbox").check();
  await expect(page.getByTestId("trust-grant")).toBeEnabled();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
  await page.getByTestId("open-text-only").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", {
    timeout: 30_000,
  });
}

async function restoreAndReload(page: Page) {
  await page.getByTestId("restore-action").click();
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
    timeout: 60_000,
  });
  await page.getByTestId("restore-reload").click();
  await expect(page.getByTestId("eval-state")).toContainText("revision 2", {
    timeout: 30_000,
  });
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

    await page.getByTestId("build-action").click();
    await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("build-state")).toContainText("exit 0");
    await expect(page.getByTestId("build-command")).toContainText("dotnet build");
    await expect(page.getByTestId("build-command")).toContainText("--no-restore");
    await expect(page.getByTestId("build-command")).toContainText("-c Debug");
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

    // A real compiler error, saved explicitly through the editor.
    await replaceProgram(page, `${originalProgram}\nnew Missing();\n`);
    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("build-action").click();
    await expect(page.getByTestId("build-state")).toContainText("Build failed", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("build-state")).toContainText("exit 1");
    await expect(page.getByTestId("build-log")).toContainText("error CS");
    await expect(page.getByTestId("build-problems-count")).toContainText("1 build problem");
    await page.screenshot({ path: path.join(screenshots, "11-build-failed.png") });

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
    await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("build-state")).toContainText("exit 0");
    await expect(page.getByTestId("build-problems-count")).toContainText("0 build problems");
    await page.screenshot({ path: path.join(screenshots, "11-build-fixed.png") });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("build waits for the explicit restore and asks before using unsaved buffers", async ({
  page,
}) => {
  const { base, copy } = tempCopy("Order Desk build pending");
  try {
    await openTrustedEvaluated(page, copy);

    // Restore pending: the Build button is blocked with the explicit restore action.
    await expect(page.getByTestId("build-disabled-reason")).toContainText(
      "Dependencies are not restored",
    );
    await expect(page.getByTestId("build-action")).toBeDisabled();
    await expect(page.getByTestId("build-restore-required")).toBeVisible();
    await page.getByTestId("build-restore-action").click();
    await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
      timeout: 60_000,
    });
    await page.getByTestId("restore-reload").click();
    await expect(page.getByTestId("eval-state")).toContainText("revision 2", {
      timeout: 30_000,
    });
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
    await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
      timeout: 60_000,
    });
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
