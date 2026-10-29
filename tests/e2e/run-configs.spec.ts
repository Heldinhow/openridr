import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const screenshots = path.resolve(process.cwd(), "docs/evidence/screenshots");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(240_000);

const program = `using OrderDesk.Core;

var order = new Order("A-104", 42m);
Console.WriteLine($"Order {order.Id}: {order.Total:C}");
Console.WriteLine($"args=[{string.Join("|", args)}]");
Console.WriteLine($"env={Environment.GetEnvironmentVariable("OPENRIDR_F4")}");
Console.WriteLine($"override={Environment.GetEnvironmentVariable("OPENRIDR_OVERRIDE")}");
Console.WriteLine($"cwd={Environment.CurrentDirectory}");
`;

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-configs-"));
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  fs.writeFileSync(path.join(copy, "src/OrderDesk.Cli/Program.cs"), program);
  fs.writeFileSync(
    path.join(copy, "run.env"),
    "# values used by the saved configuration\nOPENRIDR_F4=from-file\nOPENRIDR_OVERRIDE=file\n",
  );
  fs.writeFileSync(path.join(copy, "bad.env"), "NOT A LINE\n");
  return { base, copy };
}

async function openTrustedEvaluated(page: Page, solution: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-checkbox").check();
  await expect(page.getByTestId("trust-grant")).toBeEnabled();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("workspace-tools-toggle").click();
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

async function build(page: Page) {
  await page.getByTestId("bottom-tab-build").click();
  await page.getByTestId("build-action").click();
  await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
    timeout: 60_000,
  });
  await expect(page.getByTestId("build-state")).toContainText("exit 0");
}

async function openConfigDialog(page: Page) {
  await page.getByTestId("toolbar-run-config").click();
  await page.getByTestId("run-config-edit").click();
  await expect(page.getByTestId("run-config-dialog")).toBeVisible();
}

test("saved run configurations drive build and run, persist and validate their fields", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk configurations");
  const solution = path.join(copy, "OrderDesk.sln");
  try {
    await openTrustedEvaluated(page, solution);
    await restoreAndReload(page);

    // The default configuration comes from the evaluation and is read only.
    await expect(page.getByTestId("run-config-active")).toHaveText("Run OrderDesk.Cli");
    await openConfigDialog(page);
    await expect(page.getByTestId("run-config-default-note")).toBeVisible();
    await expect(page.getByTestId("run-config-name")).toBeDisabled();
    await expect(page.getByTestId("run-config-name")).toHaveValue("Run OrderDesk.Cli");
    await expect(page.getByTestId("run-config-configuration")).toBeDisabled();
    await expect
      .poll(() =>
        page
          .getByTestId("run-config-name")
          .evaluate((field) => getComputedStyle(field).cursor),
      )
      .toBe("not-allowed");

    // Editing the default means duplicating it into a named configuration.
    await page.getByTestId("run-config-duplicate").click();
    await expect(page.getByTestId("run-config-name")).toBeEnabled();
    await page.getByTestId("run-config-name").fill("F4 arguments");
    await page.getByTestId("run-config-args").fill("--first\ntwo words");
    await page.getByTestId("run-config-cwd").fill("src");
    await page.getByTestId("run-config-env-add").click();
    await page.getByTestId("run-config-env-key-0").fill("OPENRIDR_F4");
    await page.getByTestId("run-config-env-value-0").fill("explicit");
    await page.getByTestId("run-config-env-file-add").click();
    await page.getByTestId("run-config-env-file-0").fill("run.env");
    await page.getByTestId("run-config-save").click();
    await expect(page.getByTestId("run-config-dialog")).toHaveCount(0);
    await expect(page.getByTestId("run-config-active")).toHaveText("F4 arguments");

    // Build and Run use the active configuration through the existing flow.
    await build(page);
    await page.getByTestId("toolbar-run").click();
    await expect(page.getByTestId("run-state")).toContainText("Run exited", {
      timeout: 30_000,
    });
    await expect(page.getByTestId("run-state")).toContainText("exit 0");
    await expect(page.getByTestId("run-log")).toContainText("args=[--first|two words]");
    await expect(page.getByTestId("run-log")).toContainText("env=explicit");
    await expect(page.getByTestId("run-log")).toContainText("override=file");
    await expect(page.getByTestId("run-cwd")).toContainText(`${copy}/src`);
    fs.mkdirSync(screenshots, { recursive: true });
    await page.getByTestId("run-log").scrollIntoViewIfNeeded();
    await page.screenshot({ path: path.join(screenshots, "15-run-config-output.png") });

    // TFM/Configuration changes in the bar write the active configuration and re-evaluate.
    await page.getByTestId("eval-configuration").selectOption("Release");
    await expect(page.getByTestId("eval-configuration")).toHaveValue("Release");
    await expect(page.getByTestId("eval-state")).toContainText("revision 3", {
      timeout: 30_000,
    });
    await openConfigDialog(page);
    await expect(page.getByTestId("run-config-configuration")).toHaveValue("Release");
    await page.getByTestId("run-config-cancel").click();
    await page.getByTestId("eval-configuration").selectOption("Debug");
    await expect(page.getByTestId("eval-configuration")).toHaveValue("Debug");
    await expect(page.getByTestId("eval-state")).toContainText("revision 4", {
      timeout: 30_000,
    });

    // A .env line that is not KEY=VALUE keeps the dialog open with its field and reason.
    await openConfigDialog(page);
    await page.getByTestId("run-config-new").click();
    await page.getByTestId("run-config-name").fill("Invalid env");
    await page.getByTestId("run-config-env-file-add").click();
    await page.getByTestId("run-config-env-file-0").fill("bad.env");
    await page.getByTestId("run-config-save").click();
    await expect(page.getByTestId("run-config-dialog")).toBeVisible();
    await expect(page.getByTestId("run-config-error-envFiles")).toContainText("line 1");
    await expect(page.getByTestId("run-config-error")).toContainText(
      "cannot run until these fields are fixed",
    );
    await page.screenshot({ path: path.join(screenshots, "15-run-config-invalid.png") });
    await page.getByTestId("run-config-remove").click();
    await expect(page.getByTestId("run-config-active")).toHaveText("Run OrderDesk.Cli");
    await page.getByTestId("run-config-cancel").click();
    await page.getByTestId("toolbar-run-config").click();
    await page.getByRole("button", { name: "F4 arguments" }).click();
    await expect(page.getByTestId("run-config-active")).toHaveText("F4 arguments");

    // The saved configuration survives a reload and reopening the solution.
    await page.reload();
    await openTrustedEvaluated(page, solution);
    await expect(page.getByTestId("run-config-active")).toHaveText("F4 arguments", {
      timeout: 30_000,
    });
    await page.getByTestId("toolbar-run-config").click();
    await expect(page.getByTestId("run-config-option-default")).toBeVisible();
    await page.getByTestId("run-config-edit").click();
    await expect(page.getByTestId("run-config-name")).toHaveValue("F4 arguments");
    await expect(page.getByTestId("run-config-args")).toHaveValue("--first\ntwo words");
    await expect(page.getByTestId("run-config-cwd")).toHaveValue("src");
    await expect(page.getByTestId("run-config-env-key-0")).toHaveValue("OPENRIDR_F4");
    await expect(page.getByTestId("run-config-env-value-0")).toHaveValue("explicit");
    await expect(page.getByTestId("run-config-env-file-0")).toHaveValue("run.env");

    // Switching back to the read-only default never reuses the named configuration.
    await page.getByTestId("run-config-item-default").click();
    await expect(page.getByTestId("run-config-default-note")).toBeVisible();
    await expect(page.getByTestId("run-config-name")).toBeDisabled();
    await page.getByTestId("run-config-cancel").click();
    await page.getByTestId("toolbar-run-config").click();
    await page.getByTestId("run-config-option-default").click();
    await expect(page.getByTestId("run-config-active")).toHaveText("Run OrderDesk.Cli");

    // Remove the named configuration; the active id falls back to the default.
    await openConfigDialog(page);
    await page
      .getByTestId("run-config-dialog")
      .getByRole("button", { name: "F4 arguments" })
      .click();
    await page.getByTestId("run-config-remove").click();
    await expect(page.getByTestId("run-config-active")).toHaveText("Run OrderDesk.Cli");
    await page.getByTestId("run-config-cancel").click();
    const stored = await request.post(`${host}/call`, {
      data: { method: "runConfigs.read", params: { solution } },
    });
    const body = await stored.json();
    expect(body.ok, JSON.stringify(body.error)).toBe(true);
    expect(fs.existsSync(body.data.path)).toBe(true);
    expect(fs.existsSync(path.join(copy, "run-configs"))).toBe(false);
  } finally {
    await request.post(`${host}/call`, {
      data: { method: "workspace.close", params: {} },
    });
    fs.rmSync(base, { recursive: true, force: true });
  }
});
