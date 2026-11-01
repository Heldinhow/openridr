import { expect, test, type Page } from "@playwright/test";
import { revisionOf, settleRestore } from "./automatic";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
import { host } from "./environment";
const screenshots = path.resolve(process.env.OPENRIDR_SCREENSHOT_DIR ?? "docs/evidence/screenshots");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(180_000);

const originalProgram = fs.readFileSync(
  path.join(fixture, "src/OrderDesk.Cli/Program.cs"),
  "utf8",
);

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-run-"));
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return { base, copy };
}

async function openTrustedEvaluated(page: Page, project: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(project);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", {
    timeout: 30_000,
  });
}

// Trust restores and evaluates by itself (docs/decisions.md); wait for that to settle, then pin the TFM.
async function restoreAndReload(page: Page) {
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
    timeout: 60_000,
  });
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });
  const text = (await page.getByTestId("eval-state").textContent()) ?? "";
  const before = Number(/revision (\d+)/.exec(text)?.[1] ?? 0);
  await page.getByTestId("workspace-actions-menu").locator("summary").click();
  await page.getByTestId("eval-tfm").selectOption("net8.0");
  await expect(page.getByTestId("eval-state")).toContainText(`revision ${before + 1}`, {
    timeout: 30_000,
  });
  await expect(page.getByTestId("eval-state")).not.toContainText("stale");
}

async function build(page: Page) {
  await page.getByTestId("build-action").click();
  await expect(page.getByTestId("build-state")).toContainText("Building", {
    timeout: 10_000,
  });
  await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
    timeout: 60_000,
  });
  await expect(page.getByTestId("build-state")).toContainText("exit 0");
}

async function run(page: Page) {
  await expect(page.getByTestId("run-action")).toBeEnabled();
  await page.getByTestId("bottom-tab-build").click();
  await page.getByTestId("run-action").click();
  await page.getByTestId("bottom-tab-run").click();
  await expect.poll(async () => (await page.getByTestId("run-state").textContent()) ?? "", {
    timeout: 15_000,
  }).toMatch(/Running|Run exited/);
  await page.getByTestId("bottom-tab-run").click();
  await expect(page.getByTestId("run-state")).toContainText("Run exited", {
    timeout: 30_000,
  });
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("a real run shows the saved build output and refuses a source saved after the build", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk run");
  try {
    const longArgument = `--e2e-long-argument=${"x".repeat(400)}`;
    const config = await request.post(`${host}/call`, {
      data: {
        method: "runConfigs.write",
        params: {
          solution: path.join(copy, "OrderDesk.sln"),
          activeId: "long-command",
          configs: [{
            id: "long-command",
            name: "Long command",
            project: "src/OrderDesk.Cli/OrderDesk.Cli.csproj",
            configuration: "Debug",
            targetFramework: "net8.0",
            args: [longArgument],
            workingDirectory: "src/OrderDesk.Cli",
            env: {},
            envFiles: [],
          }],
        },
      },
    });
    expect((await config.json()).ok).toBe(true);
    await openTrustedEvaluated(page, path.join(copy, "OrderDesk.sln"));
    await restoreAndReload(page);
    fs.mkdirSync(screenshots, { recursive: true });

    // The startup list is console-only: the library is never offered for run or build.
    const startup = page.getByTestId("build-startup");
    await expect(startup.locator("option")).toHaveCount(1);
    await expect(startup).toHaveValue("src/OrderDesk.Cli/OrderDesk.Cli.csproj");
    await expect(startup).not.toContainText("OrderDesk.Core");

    await build(page);
    const buildId = (await page.getByTestId("build-id").textContent()) ?? "";
    await run(page);
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-state")).toContainText("exit 0");
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-log")).toContainText("Order A-104:");
    await page.getByTestId("bottom-tab-run").click();
    await expect.poll(() => page.getByTestId("run-log").evaluate((element) => {
      const panel = element.closest(".panel-body");
      if (!panel) return false;
      const content = element.getBoundingClientRect();
      const viewport = panel.getBoundingClientRect();
      return content.top < viewport.bottom && content.bottom > viewport.top;
    })).toBe(true);
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-build-id")).toContainText(buildId.split("·")[0].trim());
    await page.getByTestId("bottom-tab-run").click();
    const command = page.getByTestId("run-command");
    await expect(command).toContainText("OrderDesk.Cli.dll");
    await expect(command).toContainText(longArgument);
    await expect(command).toHaveCSS("white-space", "pre");
    await expect(command).toHaveCSS("user-select", "text");
    await expect(command).toHaveCSS("overflow", "auto");
    expect(await command.evaluate((element) => element.scrollWidth > element.clientWidth)).toBe(true);
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-cwd")).toContainText("Order Desk run/src/OrderDesk.Cli");
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-identity")).toContainText("OrderDesk.Cli.dll");
    await page.getByTestId("bottom-tab-run").click();
    await page.getByTestId("run-log").scrollIntoViewIfNeeded();
    await page.screenshot({ path: path.join(screenshots, "12-run-log.png") });

    // An unsaved edit: the warning names the saved build and Run never saves silently.
    const edited = originalProgram.replace("A-104", "B-777");
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(edited);
    await expect(page.getByTestId("dirty-marker")).toBeVisible();
    await page.getByTestId("bottom-tab-build").click();
    await expect(page.getByTestId("run-dirty-warning")).toContainText(
      "Running the saved build; 1 unsaved buffer is not included",
    );
    await page.getByTestId("bottom-tab-run").click();
    const firstRun = (await page.getByTestId("run-identity").textContent()) ?? "";
    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("run-action").click();
    await expect(page.getByTestId("run-dirty-dialog")).toBeVisible();
    await page.getByTestId("run-dirty-dialog").scrollIntoViewIfNeeded();
    await page.screenshot({ path: path.join(screenshots, "12-run-dirty.png") });
    await page.getByTestId("run-saved-build").click();
    await page.getByTestId("bottom-tab-run").click();
    await expect.poll(async () => (await page.getByTestId("run-state").textContent()) ?? "", {
      timeout: 15_000,
    }).toMatch(/Running|Run exited/);
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-state")).toContainText("Run exited", {
      timeout: 30_000,
    });
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-identity")).not.toHaveText(firstRun);
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-log")).toContainText("Order A-104:");
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-log")).not.toContainText("Order B-777");

    // Saved after the build: Run is blocked until a rebuild, with the reason visible.
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+s");
    await expect(page.getByTestId("app-log")).toContainText("Saved src/OrderDesk.Cli/Program.cs");
    await page.getByTestId("bottom-tab-build").click();
    await expect(page.getByTestId("run-disabled-reason")).toContainText(
      "Sources were saved after this build",
    );
    await expect(page.getByTestId("run-action")).toBeDisabled();
    await page.keyboard.press("Control+R");
    await expect(page.getByTestId("app-log")).toContainText(
      "Run unavailable: Sources were saved after this build",
    );
    await page.screenshot({ path: path.join(screenshots, "12-run-saved-stale.png") });

    // The rebuild produces the new output; the run proves it used the new build.
    await build(page);
    await expect(page.getByTestId("run-action")).toBeEnabled();
    await run(page);
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-log")).toContainText("Order B-777:");
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-log")).not.toContainText("Order A-104:");

    // Ctrl+R uses the same gate and launches the saved build.
    await page.getByTestId("bottom-tab-run").click();
    const beforeShortcut = (await page.getByTestId("run-identity").textContent()) ?? "";
    await page.keyboard.press("Control+R");
    await page.getByTestId("bottom-tab-run").click();
    await expect.poll(async () => (await page.getByTestId("run-state").textContent()) ?? "", {
      timeout: 15_000,
    }).toMatch(/Running|Run exited/);
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-state")).toContainText("Run exited", {
      timeout: 30_000,
    });
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-identity")).not.toHaveText(beforeShortcut);
    await page.getByTestId("bottom-tab-run").click();
    await page.getByTestId("run-log").scrollIntoViewIfNeeded();
    await page.screenshot({ path: path.join(screenshots, "12-run-rebuilt.png") });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("stopping a real long-running program reports requested and observed termination", async ({
  page,
}) => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-run-long-"));
  const root = path.join(base, "Long Runner");
  fs.mkdirSync(root, { recursive: true });
  fs.writeFileSync(
    path.join(root, "global.json"),
    "{\"sdk\":{\"version\":\"8.0.408\",\"rollForward\":\"disable\"}}\n",
  );
  fs.writeFileSync(
    path.join(root, "LongRunner.csproj"),
    "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
  );
  fs.writeFileSync(
    path.join(root, "Program.cs"),
    "System.Console.WriteLine(\"long-running\");\nSystem.Threading.Thread.Sleep(60_000);\n",
  );
  try {
    await openTrustedEvaluated(page, path.join(root, "LongRunner.csproj"));
    await restoreAndReload(page);
    await build(page);
    fs.mkdirSync(screenshots, { recursive: true });

    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("run-action").click();
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-state")).toContainText("Running", {
      timeout: 15_000,
    });
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-log")).toContainText("long-running");

    await page.getByTestId("bottom-tab-run").click();
    await page.getByTestId("run-stop").click();
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-state")).toContainText("Run cancelled", {
      timeout: 15_000,
    });
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-termination")).toContainText(
      "Termination requested: SIGTERM",
    );
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-termination")).toContainText("Observed: exited");
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-exit")).toHaveText(/^exit (unknown|-?\d+)$/);
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-log")).toContainText("long-running");
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-stop")).toHaveCount(0);
    await page.getByTestId("bottom-tab-run").click();
    await page.getByTestId("run-termination").scrollIntoViewIfNeeded();
    await page.screenshot({ path: path.join(screenshots, "12-run-cancelled.png") });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("one click on Run on a prepared workspace neither restores nor re-evaluates", async ({ page }) => {
  const { base, copy } = tempCopy("Order Desk prepared");
  try {
    await openTrustedEvaluated(page, path.join(copy, "OrderDesk.sln"));
    await settleRestore(page);
    const revision = await revisionOf(page);
    const restoreCommand = await page.getByTestId("restore-command").textContent();
    await page.getByTestId("toolbar-run").click({ force: true });
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-log")).toContainText("Order A-104", { timeout: 120_000 });
    expect(await revisionOf(page)).toBe(revision);
    await expect(page.getByTestId("restore-command")).toHaveText(restoreCommand ?? "");
    await expect(page.getByTestId("restore-state")).not.toContainText("Restoring");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("one click on Run restores, builds and shows the program output", async ({ page }) => {
  const { base, copy } = tempCopy("Order Desk one click");
  try {
    await openTrustedEvaluated(page, path.join(copy, "OrderDesk.sln"));
    // aria-disabled (blocked but clickable) fails Playwright's enabled check, so force it.
    await page.getByTestId("toolbar-run").click({ force: true });
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-log")).toContainText("Order A-104", { timeout: 120_000 });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
