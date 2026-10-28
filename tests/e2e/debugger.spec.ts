import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { reveal } from "./disclosures";

/**
 * Ticket 13: the approved netcoredbg release has no verified compatible adapter on this
 * macOS arm64 machine. These tests prove the honest blocked outcome, the read-only plan, the
 * session-scoped experimental flag and the host gates; nothing is downloaded or installed.
 */
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const host = "http://127.0.0.1:4317";
const screenshots = path.resolve(process.cwd(), "docs/evidence/screenshots");
const logs = path.resolve(process.cwd(), "docs/evidence/logs");
const e2eHome = path.join(os.tmpdir(), "openridr-e2e-home");
const debugToolDir = path.join(e2eHome, "tools/netcoredbg");

test.use({ viewport: { width: 1440, height: 900 } });
test.describe.configure({ timeout: 120_000 });

async function hostCall(request: APIRequestContext, method: string, params = {}) {
  const response = await request.post(`${host}/call`, { data: { method, params } });
  return response.json();
}

async function openWorkspace(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(fixture);
  await page.getByTestId("workspace-open").click();
  await reveal(page.getByTestId("sdk-debug-adapter"));
  await expect(page.getByTestId("sdk-debug-adapter")).toBeVisible();
}

test.beforeAll(() => {
  fs.mkdirSync(screenshots, { recursive: true });
  fs.mkdirSync(logs, { recursive: true });
});

test("macOS arm64 requires the experimental flag and shows the real asset", async ({
  page,
  request,
}) => {
  fs.rmSync(debugToolDir, { recursive: true, force: true });
  await openWorkspace(page);

  const planned = await hostCall(request, "debug.plan");
  expect(planned.ok).toBe(true);
  const debug = planned.data.debug;
  if (debug.platform !== "osx-arm64") {
    test.skip(true, "this blocked outcome is specific to macOS arm64");
  }

  // The card states the approved version, the official source, the real asset and the limit.
  await expect(page.getByTestId("debug-state")).toHaveText("Experimental · disabled");
  await expect(page.getByTestId("sdk-debug-adapter")).toContainText(
    "netcoredbg-osx-arm64.zip · available",
  );
  await expect(page.getByTestId("debug-source")).toContainText("approved version 3.2.0-1092");
  await expect(page.getByTestId("debug-source")).toContainText("Samsung/netcoredbg");
  await expect(page.getByTestId("debug-source")).toContainText("DAP proven by installation: no");
  await expect(page.getByTestId("debug-message")).toContainText(
    "The official 3.2.0-1092 osx-arm64 asset exists",
  );
  await expect(page.getByTestId("debug-message")).toContainText("community supported");
  await expect(page.getByTestId("debug-message")).toContainText(
    "installation stays refused until you enable the experimental flag",
  );
  await expect(page.getByTestId("debug-dap")).toContainText("DAP launch is implemented");
  await expect(page.getByTestId("debug-flag-note")).toContainText(
    "neither installs netcoredbg nor proves compatibility",
  );
  await expect(page.getByTestId("debug-install")).toBeDisabled();
  await expect(page.getByTestId("debug-disabled-reason")).toContainText(
    "installation stays refused until you enable the experimental flag",
  );

  // Plan details carry every material field and the blocked reason.
  await reveal(page.getByTestId("debug-plan-open"));
  await page.getByTestId("debug-plan-open").click();
  await expect(page.getByTestId("debug-plan")).toBeVisible();
  await page.getByTestId("debug-plan").locator("summary").click();
  await expect(page.getByTestId("debug-plan-tool")).toContainText("netcoredbg 3.2.0-1092");
  await expect(page.getByTestId("debug-plan-source")).toContainText(
    "https://github.com/Samsung/netcoredbg/releases/tag/3.2.0-1092",
  );
  await expect(page.getByTestId("debug-plan-asset")).toContainText(
    "netcoredbg-osx-arm64.zip",
  );
  await expect(page.getByTestId("debug-plan-asset")).toContainText(
    "f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea",
  );
  await expect(page.getByTestId("debug-plan-destination")).toContainText(
    "tools/netcoredbg/3.2.0-1092",
  );
  await expect(page.getByTestId("debug-plan-permissions")).toContainText("no elevation");
  await expect(page.getByTestId("debug-plan-integrity")).toContainText(
    "no code signature or notarization",
  );
  await expect(page.getByTestId("debug-plan-limits")).toContainText("progress is reported only as bytes");
  await expect(page.getByTestId("debug-plan-blocked")).toContainText(
    "installation stays refused until you enable the experimental flag",
  );
  await page.screenshot({ path: path.join(screenshots, "13-debugger-plan.png"), fullPage: true });
  fs.writeFileSync(
    path.join(logs, "13-debugger-plan.json"),
    JSON.stringify({ platform: debug.platform, plan: debug.plan }, null, 2),
  );

  // No install is offered while the flag is off; the plan stays blocked.
  await expect(page.getByTestId("debug-install")).toBeDisabled();
  expect(fs.existsSync(debugToolDir)).toBe(false);

  // Flag off is a hard gate even with trust and a valid plan.
  await page.getByTestId("trust-checkbox").check();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
  const beforeFlag = await hostCall(request, "debug.install", {
    planId: debug.plan.planId,
    confirm: true,
  });
  expect(beforeFlag.ok).toBe(false);
  expect(beforeFlag.error.code).toBe("experimental_required");
  expect(beforeFlag.error.message).toContain("Nothing was downloaded");
  expect(fs.existsSync(debugToolDir)).toBe(false);
  fs.writeFileSync(
    path.join(logs, "13-debugger-refused.json"),
    JSON.stringify({ error: beforeFlag.error }, null, 2),
  );

  // Explicit, separate experimental opt-in: acknowledged by checkbox, then enabled.
  await reveal(page.getByTestId("debug-flag-checkbox"));
  await page.getByTestId("debug-flag-checkbox").check();
  await expect(page.getByTestId("debug-flag-enable")).toBeEnabled();
  await page.getByTestId("debug-flag-enable").click();
  await expect(page.getByTestId("sdk-apple-silicon")).toContainText("experimental · enabled");
  await expect(page.getByTestId("debug-state")).toHaveText("Plan ready — nothing downloaded yet");
  await expect(page.getByTestId("debug-message")).toContainText(
    "available with explicit consent",
  );
  await expect(page.getByTestId("debug-flag-note")).toContainText(
    "neither installs netcoredbg nor proves compatibility",
  );
  // The replanned consent is for the official asset and is now installable; this test does not
  // download (the real install evidence comes from the host flow in the real app home).
  await expect(page.getByTestId("debug-install")).toBeEnabled();
  await expect(page.getByTestId("debug-plan-blocked")).toHaveCount(0);
  expect(fs.existsSync(debugToolDir)).toBe(false);
  await reveal(page.getByTestId("debug-plan-open"));
  await page.getByTestId("debug-plan-open").click();
  await page.getByTestId("debug-plan").locator("summary").click();
  await expect(page.getByTestId("debug-plan-asset")).toContainText(
    "netcoredbg-osx-arm64.zip",
  );
  await expect(page.getByTestId("debug-plan-integrity")).toContainText(
    "f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea",
  );
  await page.screenshot({ path: path.join(screenshots, "13-debugger-flag.png"), fullPage: true });

  // Analysis, Run and the editor stay available: the debugger limit does not block them.
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(page.getByTestId("toolbar-debug")).toBeDisabled();
  await expect(page.getByTestId("toolbar-debug")).toHaveAttribute(
    "title",
    /osx-arm64 asset is available with explicit consent/,
  );
  await expect(page.getByTestId("toolbar-run")).toBeVisible();
  await expect(page.getByTestId("lsp-panel")).toBeVisible();
});

test("host gates consent, trust and plan identity before any download", async ({ request }) => {
  fs.rmSync(debugToolDir, { recursive: true, force: true });
  const opened = await hostCall(request, "workspace.open", { path: fixture });
  expect(opened.ok, JSON.stringify(opened.error)).toBe(true);

  // Before trust: installation and retry are executable actions and are refused.
  const beforeTrust = await hostCall(request, "debug.install", {
    planId: "fnv1a64:0000000000000000",
    confirm: true,
  });
  expect(beforeTrust.ok).toBe(false);
  expect(beforeTrust.error.code).toBe("trust_required");
  const retryBeforeTrust = await hostCall(request, "debug.retry", {});
  expect(retryBeforeTrust.ok).toBe(false);
  expect(retryBeforeTrust.error.code).toBe("trust_required");

  // Planning and state are read-only and need no trust.
  const planned = await hostCall(request, "debug.plan");
  expect(planned.ok).toBe(true);
  expect(planned.data.debug.adapterState).toBe("experimental_disabled");

  await hostCall(request, "trust.grant", {});
  const planId = (await hostCall(request, "debug.plan")).data.debug.plan.planId;
  const noConsent = await hostCall(request, "debug.install", { planId, confirm: false });
  expect(noConsent.ok).toBe(false);
  expect(noConsent.error.code).toBe("confirmation_required");
  const stalePlan = await hostCall(request, "debug.install", {
    planId: "fnv1a64:0000000000000000",
    confirm: true,
  });
  expect(stalePlan.ok).toBe(false);
  expect(stalePlan.error.code).toBe("confirmation_required");
  expect(stalePlan.error.message).toContain("Nothing was downloaded");

  // The flag is not trust and never authorizes an install.
  const flagWithoutConfirm = await hostCall(request, "debug.flag", { enabled: true });
  expect(flagWithoutConfirm.ok).toBe(false);
  expect(flagWithoutConfirm.error.code).toBe("confirmation_required");
  const flag = await hostCall(request, "debug.flag", { enabled: true, confirm: true });
  expect(flag.ok).toBe(true);
  expect(flag.data.debug.appleSiliconExperimental).toBe(true);
  expect(flag.data.debug.installable).toBe(true);
  expect(flag.data.debug.plan.blockedReason).toBeNull();
  expect(flag.data.debug.plan.asset.name).toBe("netcoredbg-osx-arm64.zip");

  expect(fs.existsSync(debugToolDir)).toBe(false);
  fs.writeFileSync(
    path.join(logs, "13-debugger-gates.json"),
    JSON.stringify(
      {
        beforeTrust: beforeTrust.error,
        retryBeforeTrust: retryBeforeTrust.error,
        noConsent: noConsent.error,
        stalePlan: stalePlan.error,
        flagWithoutConfirm: flagWithoutConfirm.error,
        debug: flag.data.debug,
        toolDirExists: fs.existsSync(debugToolDir),
      },
      null,
      2,
    ),
  );
});

/**
 * Ticket 14: a real DAP session with the installed netcoredbg, driven through the UI. The
 * E2E home receives a copy of the operator-installed adapter (the ticket-13 install flow's
 * verified artifact) so no download happens here.
 */
test("debugs a real build with breakpoints, frames, locals, steps, continue and stop", async ({
  page,
  request,
}) => {
  const realAdapter = path.join(
    process.env.HOME ?? "",
    "Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092",
  );
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");

  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug-"));
  const copy = path.join(base, "Order Desk debug");
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  fs.rmSync(debugToolDir, { recursive: true, force: true });
  fs.mkdirSync(debugToolDir, { recursive: true });
  fs.cpSync(realAdapter, path.join(debugToolDir, "3.2.0-1092"), { recursive: true });

  try {
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-checkbox").check();
    await page.getByTestId("trust-grant").click();
    await expect(page.getByTestId("mode-trusted")).toBeVisible();
    await page.getByTestId("open-text-only").click();
    await page.getByTestId("workspace-tools-toggle").click();
    await expect(page.getByTestId("tree")).toBeVisible();
    await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });

    // Session-scoped opt-in and the real verified adapter: installing again reuses it.
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
    expect(installed.data.debug.reused).toBe(true);
    expect(installed.data.debug.appleSiliconExperimental).toBe(true);

    await page.reload();
    const afterReload = await hostCall(request, "workspace.state");
    expect(afterReload.data.debug.appleSiliconExperimental).toBe(true);
    await expect(page.getByTestId("mode-trusted")).toBeVisible({ timeout: 20_000 });
    if (!(await page.getByTestId("tree").isVisible())) {
      await page.getByTestId("open-text-only").click();
      await page.getByTestId("workspace-tools-toggle").click();
    }
    await expect(page.getByTestId("tree")).toBeVisible();
    await page.getByTestId("restore-action").click();
    await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
      timeout: 60_000,
    });
    await page.getByTestId("restore-reload").click();
    await expect(page.getByTestId("eval-state")).toContainText("revision 2", { timeout: 30_000 });

    // The Debug launch stays blocked until a successful Debug build exists.
    await expect(page.getByTestId("toolbar-debug")).toBeDisabled();
    await expect(page.getByTestId("toolbar-debug")).toHaveAttribute(
      "title",
      /Build the startup project in Debug first/,
    );

    // Breakpoint in Program.cs through the real editor. The gutter marker column is a
    // separate column from the line numbers.
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await expect(page.getByTestId("editor")).toBeVisible();
    await page.getByTestId("editor").click();
    await page.locator(".cm-line").nth(2).click();
    await expect(page.getByTestId("editor-shell")).toHaveAttribute("data-cursor", /^2:/);
    await page.keyboard.press("Meta+F8");
    await page.getByTestId("bottom-tab-debug").click();
    await expect(page.getByTestId("debug-breakpoints")).toContainText(
      "src/OrderDesk.Cli/Program.cs:3",
    );
    await expect(page.getByTestId("debug-breakpoints")).toContainText("pending");
    await expect(page.locator(".cm-lineNumbers")).toBeVisible();
    await expect(page.locator(".cm-debug-gutter")).toBeVisible();
    expect(await page.locator(".cm-debug-marker").count()).toBe(1);

    // A mouse click on the marker column removes the same breakpoint; the shortcut adds it back.
    await page.locator(".cm-debug-marker").first().click();
    await expect(page.getByTestId("debug-breakpoints")).toContainText("No breakpoints");
    await page.locator(".cm-line").nth(2).click();
    await page.keyboard.press("Meta+F8");
    await expect(page.getByTestId("debug-breakpoints")).toContainText(
      "src/OrderDesk.Cli/Program.cs:3",
    );
    await expect(page.locator(".cm-debug-marker")).toHaveCount(1);

    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("build-action").click();
    await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("toolbar-debug")).toBeEnabled();

    // Launch with Ctrl+D; the real adapter must hit the breakpoint on line 3.
    await page.keyboard.press("Control+d");
    const pauseDeadline = Date.now() + 30_000;
    let pausedState: { dap: { state: string; phase: string; log: string } } | null = null;
    while (Date.now() < pauseDeadline) {
      const polled = await hostCall(request, "workspace.state");
      pausedState = polled.data;
      if (pausedState?.dap.state === "paused") break;
      await new Promise((resolve) => setTimeout(resolve, 200));
    }
    expect(pausedState?.dap.state, pausedState?.dap.log).toBe("paused");
    await page.getByTestId("bottom-tab-debug").click();
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 30_000 });
    await expect(page.getByTestId("dap-session")).toContainText("1");
    await expect(page.getByTestId("dap-build-id")).toContainText("build:");
    await expect(page.getByTestId("dap-assembly")).toContainText("OrderDesk.Cli.dll");
    await expect(page.getByTestId("dap-adapter")).toContainText("3.2.0");
    await expect(page.getByTestId("dap-adapter")).toContainText("--interpreter=vscode");
    await expect(page.getByTestId("dap-thread-list")).toContainText("Main Thread");
    await expect(page.getByTestId("debug-frames")).toContainText("Program.<Main>$()");
    await expect(page.getByTestId("debug-locals")).toContainText("order");
    await expect(page.getByTestId("debug-breakpoints")).toContainText("verified by the adapter");
    await expect(page.getByTestId("dap-log")).toContainText("event stopped · reason breakpoint");
    await page.screenshot({ path: path.join(screenshots, "14-dap-paused.png"), fullPage: true });

    // F8 steps over line 3; the frame and locals must move to line 4.
    await page.keyboard.press("F8");
    await expect(page.getByTestId("debug-frames")).toContainText("Program.cs:4", {
      timeout: 15_000,
    });
    await expect(page.getByTestId("debug-locals")).not.toContainText('order … null');
    await page.screenshot({ path: path.join(screenshots, "14-dap-stepped.png"), fullPage: true });

    // Continue runs to the real exit; the exit code is observed, not assumed.
    await page.getByTestId("debug-continue").click();
    await expect(page.getByTestId("dap-state")).toContainText("Terminated", { timeout: 30_000 });
    await expect(page.getByTestId("dap-exit")).toHaveText("exit 0");
    await expect(page.getByTestId("dap-output")).toContainText("Order A-104");
    await expect(page.getByTestId("dap-log")).toContainText("event exited · exit 0");

    // Second session from the Debug tab itself; stop while paused, with requested vs
    // observed termination.
    await page.getByTestId("debug-launch").click();
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 30_000 });
    await page.getByTestId("debug-stop").click();
    await expect(page.getByTestId("dap-state")).toContainText("Cancelled", { timeout: 30_000 });
    await expect(page.getByTestId("dap-termination")).toContainText("Termination requested:");
    await expect(page.getByTestId("dap-termination")).toContainText("Observed: exited");
    await expect(page.getByTestId("dap-log")).toContainText("Stopped by the user");
    await page.screenshot({ path: path.join(screenshots, "14-dap-stopped.png"), fullPage: true });

    let state = await hostCall(request, "workspace.state");
    const deadline = Date.now() + 5000;
    while (
      Date.now() < deadline &&
      state.data.processes.some(
        (process: { kind: string; state: string }) =>
          process.kind.startsWith("debug:") && process.state !== "exited",
      )
    ) {
      await new Promise((resolve) => setTimeout(resolve, 100));
      state = await hostCall(request, "workspace.state");
    }
    const debugProcesses = state.data.processes.filter((process: { kind: string }) =>
      process.kind.startsWith("debug:"),
    );
    expect(debugProcesses.length).toBeGreaterThan(0);
    for (const process of debugProcesses) {
      expect(process.state).toBe("exited");
    }
    fs.writeFileSync(
      path.join(logs, "14-e2e-debug-session.json"),
      JSON.stringify(
        {
          debug: state.data.dap,
          processes: debugProcesses,
        },
        null,
        2,
      ),
    );
  } finally {
    await page.request.post(`${host}/call`, {
      data: { method: "workspace.close", params: {} },
    });
    fs.rmSync(base, { recursive: true, force: true });
  }
});
