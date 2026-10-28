import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const host = "http://127.0.0.1:4317";
const screenshots = path.resolve(process.cwd(), "docs/evidence/screenshots");
const e2eHome = path.join(os.tmpdir(), "openridr-e2e-home");

test.use({ viewport: { width: 1440, height: 900 } });
test.describe.configure({ timeout: 180_000 });

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-lsp-"));
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return { base, copy };
}

async function hostState(request: APIRequestContext) {
  const response = await request.post(`${host}/call`, {
    data: { method: "workspace.state", params: {} },
  });
  const body = await response.json();
  expect(body.ok, JSON.stringify(body.error)).toBe(true);
  return body.data;
}

async function hostCall(request: APIRequestContext, method: string, params = {}) {
  const response = await request.post(`${host}/call`, { data: { method, params } });
  return response.json();
}

async function openWorkspace(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
}

async function trustWorkspace(page: Page) {
  await page.getByTestId("trust-checkbox").check();
  await expect(page.getByTestId("trust-grant")).toBeEnabled();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
}

async function enterWorkspace(page: Page) {
  await page.getByTestId("open-text-only").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function waitForEvaluation(page: Page, revision?: number) {
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", {
    timeout: 30_000,
  });
  if (revision !== undefined) {
    await expect(page.getByTestId("eval-state")).toContainText(`revision ${revision}`);
  }
}

async function restoreAndReload(page: Page) {
  await page.getByTestId("restore-action").click();
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
    timeout: 90_000,
  });
  await expect(page.getByTestId("restore-state")).toContainText("exit 0");
  await page.getByTestId("restore-reload").click();
  await waitForEvaluation(page);
  await expect(page.getByTestId("eval-restore-banner")).toHaveCount(0);
}

async function waitToolState(
  request: APIRequestContext,
  state: string,
  seen?: Set<string>,
) {
  await expect
    .poll(
      async () => {
        const snapshot = await hostState(request);
        const tool = snapshot.tools["csharp-ls"];
        seen?.add(tool.state);
        return tool.state;
      },
      { timeout: 180_000, intervals: [25, 50, 100, 200] },
    )
    .toBe(state);
}

async function ensureTool(page: Page, request: APIRequestContext) {
  const snapshot = await hostState(request);
  if (snapshot.tools["csharp-ls"].state === "ready") return;
  await page.getByTestId("install-plan-open").click();
  await expect(page.getByTestId("install-plan")).toBeVisible();
  await page.getByTestId("install-confirm").click();
  await waitToolState(request, "ready");
}

test.afterEach(async ({ request }) => {
  await hostCall(request, "lsp.stop").catch(() => {});
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("install plan is exact, cancel is inert, and the real install reaches verifying", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk tools");
  try {
    const initial = await hostState(request);
    const appHome = initial.app.appHome as string;
    const isolated = appHome === e2eHome;
    if (isolated) {
      // This E2E host owns this app home; start from an absent tool.
      fs.rmSync(path.join(appHome, "tools/csharp-ls"), { recursive: true, force: true });
    } else {
      test.info().annotations.push({
        type: "warning",
        description: `E2E host uses ${appHome}; an existing verified tool is reused instead of a fresh install.`,
      });
    }

    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await trustWorkspace(page);
    await enterWorkspace(page);

    await page.getByTestId("install-plan-open").click();
    await expect(page.getByTestId("install-plan")).toBeVisible();
    await expect(page.getByTestId("install-tool")).toContainText("csharp-ls 0.28.0");
    await expect(page.getByTestId("install-source")).toContainText("NuGet.org");
    await expect(page.getByTestId("install-source")).toContainText(
      "https://api.nuget.org/v3/index.json",
    );
    await expect(page.getByTestId("install-platform")).toContainText("macos");
    await expect(page.getByTestId("install-platform")).toContainText("aarch64");
    await expect(page.getByTestId("install-destination")).toContainText(
      "/tools/csharp-ls/0.28",
    );
    await expect(page.getByTestId("install-command")).toContainText(
      "tool install csharp-ls --version 0.28.0 --tool-path",
    );
    await expect(page.getByTestId("install-command")).toContainText("0.28.staging");
    await expect(page.getByTestId("install-permissions")).toContainText(
      "no global tool install",
    );
    await expect(page.getByTestId("install-integrity")).toContainText(
      "Skipping NuGet package signature verification",
    );

    const plan = (await hostState(request)).tools["csharp-ls"].plan;
    expect(plan.argv).toHaveLength(8);
    expect(plan.argv.at(-1)).toContain("0.28.staging");

    const readyBefore = (await hostState(request)).tools["csharp-ls"].state === "ready";
    if (!isolated && readyBefore) {
      // A host reusing the real app home already has the verified tool; prove reuse only.
      await expect(page.getByTestId("install-state")).toContainText("Ready");
      await page.getByTestId("install-confirm").click();
      await expect(page.getByTestId("install-log")).toContainText("reused", {
        timeout: 20_000,
      });
      return;
    }

    if (isolated) {
      await page.getByTestId("install-cancel").click();
      await expect(page.getByTestId("install-state")).toContainText("Cancelled");
      expect(fs.existsSync(plan.destination), "cancel created a tool directory").toBe(false);
      expect(fs.existsSync(plan.staging), "cancel created a staging directory").toBe(false);
      expect((await hostState(request)).processes).toEqual([]);
      await page.getByTestId("install-plan-open").click();
      await expect(page.getByTestId("install-plan")).toBeVisible();
    }

    const seen = new Set<string>();
    await page.getByTestId("install-confirm").click();
    await waitToolState(request, "ready", seen);
    expect([...seen], `states observed: ${[...seen].join(", ")}`).toContain("verifying");
    expect(seen).toContain("downloading");
    expect(seen).toContain("installing");

    await expect(page.getByTestId("install-state")).toContainText("Ready");
    await expect(page.getByTestId("install-ready")).toContainText("Executable and version verified");
    await expect(page.getByTestId("install-sha256")).toContainText(/[0-9a-f]{64}/);
    await expect(page.getByTestId("install-verified")).toContainText("csharp-ls, 0.28.0");
    expect(fs.existsSync(path.join(plan.destination, "csharp-ls"))).toBe(true);
    expect(fs.existsSync(plan.staging)).toBe(false);

    await page.getByText("Install log · stdout/stderr").click();
    await expect(page.getByTestId("install-log")).toContainText(
      "Tool 'csharp-ls' (version '0.28.0') was successfully installed",
    );
    await expect(page.getByTestId("install-log")).toContainText(
      "Skipping NuGet package signature verification",
    );
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({
      path: path.join(screenshots, "05-csharp-ls-install.png"),
      fullPage: false,
    });

    // A valid installation is reused, never overwritten.
    await page.getByTestId("install-plan-open").click();
    await expect(page.getByTestId("install-state")).toContainText("Ready");
    await page.getByTestId("install-confirm").click();
    await expect(page.getByTestId("install-log")).toContainText("reused", {
      timeout: 20_000,
    });
    const afterReuse = (await hostState(request)).tools["csharp-ls"];
    expect(afterReuse.reused).toBe(true);
    expect(afterReuse.sha256).toBe((await hostState(request)).tools["csharp-ls"].sha256);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("analysis loads, republishes diagnostics, survives a crash and stops on revoke", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk analysis");
  const program = path.join(copy, "src/OrderDesk.Cli/Program.cs");
  const clean = fs.readFileSync(program, "utf8");
  fs.writeFileSync(program, `${clean}\nvar report = new OrderReport();\n`);
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await trustWorkspace(page);
    await enterWorkspace(page);
    await waitForEvaluation(page);
    await restoreAndReload(page);
    await ensureTool(page, request);

    await page.getByTestId("lsp-start").click();
    await expect(page.getByTestId("lsp-state")).toContainText("Ready", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("lsp-capabilities")).toContainText("completion");
    await expect(page.getByTestId("lsp-capabilities")).toContainText("definition");
    await expect(page.getByTestId("lsp-capabilities")).toContainText("rename");
    await expect(page.getByTestId("lsp-capabilities")).toContainText("code actions");
    await expect(page.getByTestId("lsp-hostroot")).toContainText("hostroot");
    await expect(page.getByTestId("lsp-session")).toContainText("session 1");

    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await expect(page.getByTestId("editor")).toContainText("OrderDesk.Core");
    await expect(page.getByTestId("lsp-diagnostics")).toContainText("1 item", {
      timeout: 30_000,
    });
    const broken = (await hostState(request)).diagnostics.snapshots;
    expect(broken.length).toBeGreaterThan(0);
    expect(broken[0].serverVersion).toBeNull();
    expect(broken[0].clientRevision).toBe(1);
    expect(broken[0].items[0].code).toBe("CS0246");

    // Full-text didChange republishes for the edited revision.
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(clean);
    await expect(page.getByTestId("lsp-diagnostics")).toContainText("0 item", {
      timeout: 30_000,
    });
    const fixed = (await hostState(request)).diagnostics.snapshots[0];
    expect(fixed.clientRevision).toBe(2);
    expect(fixed.items).toEqual([]);

    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({
      path: path.join(screenshots, "05-csharp-ls.png"),
      fullPage: false,
    });

    // Kill the real server: failure must preserve the editor and its log.
    const beforeCrash = await hostState(request);
    const processId = beforeCrash.lsp.processId as number;
    await hostCall(request, "processes.terminate", { id: processId });
    await expect(page.getByTestId("lsp-state")).toContainText("Failed", {
      timeout: 30_000,
    });
    await expect(page.getByTestId("lsp-error")).toContainText("exited");
    await page.getByText("LSP log · server/stdout/stderr").click();
    await expect(page.getByTestId("lsp-log")).toContainText("session 1");

    const afterCrashText = `${clean}\n// editor survived the analysis failure\n`;
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(afterCrashText);
    await page.keyboard.press("Meta+s");
    await expect(page.getByTestId("app-log")).toContainText(
      "Saved src/OrderDesk.Cli/Program.cs",
    );
    expect(fs.readFileSync(program, "utf8")).toContain("editor survived the analysis failure");

    await page.getByTestId("lsp-restart").click();
    await expect(page.getByTestId("lsp-session")).toContainText("session 2", {
      timeout: 30_000,
    });
    await expect(page.getByTestId("lsp-state")).toContainText("Ready", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("lsp-diagnostics")).toContainText("0 item", {
      timeout: 30_000,
    });

    await page.getByTestId("trust-revoke").click();
    await expect(page.getByTestId("lsp-state")).toContainText("Stopped", {
      timeout: 30_000,
    });
    await expect(page.getByTestId("mode-restricted")).toBeVisible();
    const stopped = await hostState(request);
    const lspProcess = stopped.processes.find(
      (process: { id: number }) => process.id === stopped.lsp.processId,
    );
    expect(lspProcess?.state).toBe("exited");
    expect(stopped.lsp.exit).toBe(lspProcess?.exit);
    await expect(page.getByTestId("lsp-start")).toBeDisabled();
    await expect(page.getByTestId("lsp-disabled-reason")).toContainText("Trust");

    const afterRevokeText = `${afterCrashText}// editing after revoke\n`;
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(afterRevokeText);
    await page.keyboard.press("Meta+s");
    await expect(page.getByTestId("app-log")).toContainText(
      "Saved src/OrderDesk.Cli/Program.cs",
    );
    expect(fs.readFileSync(program, "utf8")).toContain("editing after revoke");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
