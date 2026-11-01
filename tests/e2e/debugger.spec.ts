import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import { revisionOf, settleRestore } from "./automatic";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { openToolchain, reveal } from "./disclosures";

/**
 * Ticket 13: the approved netcoredbg release has no verified compatible adapter on this
 * macOS arm64 machine. These tests prove the honest blocked outcome, the read-only plan, the
 * session-scoped experimental flag and the host gates; nothing is downloaded or installed.
 */
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
import { e2eHome, host } from "./environment";
const screenshots = path.resolve(process.env.OPENRIDR_SCREENSHOT_DIR ?? "docs/evidence/screenshots");
const logs = path.resolve(process.env.OPENRIDR_EVIDENCE_LOG_DIR ?? "docs/evidence/logs");
const debugToolDir = path.join(e2eHome, "tools/netcoredbg");
const realAdapter = path.join(
  process.env.HOME ?? "",
  "Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092",
);

test.use({ viewport: { width: 1440, height: 900 } });
test.describe.configure({ timeout: 120_000 });

async function hostCall(request: APIRequestContext, method: string, params = {}) {
  const response = await request.post(`${host}/call`, { data: { method, params } });
  return response.json();
}

// The workspace is opened through the host and the page loaded afterwards, so the Toolchain cards can be
// inspected without the Trust prompt (which the form would raise) covering them.
async function openWorkspace(page: Page) {
  await page.request.post(`${host}/call`, { data: { method: "workspace.open", params: { path: fixture } } });
  await page.goto("/");
  await openToolchain(page);
  await reveal(page.getByTestId("sdk-debug-adapter"));
  await expect(page.getByTestId("sdk-debug-adapter")).toBeVisible();
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
  await expect(page.getByTestId("debug-install")).toHaveCount(0);
  await expect(page.getByTestId("debug-disabled-reason")).toContainText(
    "installation stays refused until you enable the experimental flag",
  );

  // Plan details carry every material field and the blocked reason.
  await reveal(page.getByTestId("debug-plan-open"));
  await page.getByTestId("debug-plan-open").click();
  await expect(page.getByTestId("debug-plan")).toBeVisible();
  await expect(page.getByTestId("debug-plan")).toHaveAttribute("open", "");
  await expect(page.getByTestId("debug-install")).toBeVisible();
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
  expect(
    await page.getByTestId("debug-install").evaluate((button) => getComputedStyle(button).color),
  ).not.toBe("rgb(84, 138, 247)");
  await page.screenshot({ path: path.join(screenshots, "13-debugger-plan.png"), fullPage: true });
  fs.writeFileSync(
    path.join(logs, "13-debugger-plan.json"),
    JSON.stringify({ platform: debug.platform, plan: debug.plan }, null, 2),
  );

  // No install is offered while the flag is off; the plan stays blocked.
  await expect(page.getByTestId("debug-install")).toBeDisabled();
  expect(fs.existsSync(debugToolDir)).toBe(false);

  // Flag off is a hard gate even with trust and a valid plan.
  expect((await hostCall(request, "trust.grant", { remember: false })).ok).toBe(true);
  await page.reload();
  await openToolchain(page);
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
  await expect(page.getByTestId("debug-plan")).toHaveAttribute("open", "");
  await expect(page.getByTestId("debug-install")).toBeVisible();
  await expect(page.getByTestId("debug-plan-asset")).toContainText(
    "netcoredbg-osx-arm64.zip",
  );
  await expect(page.getByTestId("debug-plan-integrity")).toContainText(
    "f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea",
  );
  await page.screenshot({ path: path.join(screenshots, "13-debugger-flag.png"), fullPage: true });

  // Analysis, Run and the editor stay available: the debugger limit does not block them.
  // Opening the workspace again resets trust and the session flag, so trust and the flag are set once more.
  await page.getByTestId("welcome-nav-projects").click();
  await page.getByTestId("workspace-path-input").fill(fixture);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  expect((await hostCall(request, "debug.flag", { enabled: true, confirm: true })).ok).toBe(true);
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
    await page.getByTestId("trust-prompt-trust").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
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
    // Opening again asks for trust (and resets the session flag), so trust and set the flag once more.
    await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-trust").click();
    expect((await hostCall(request, "debug.flag", { enabled: true, confirm: true })).ok).toBe(true);
    await page.getByTestId("workspace-tools-toggle").click();
    await expect(page.getByTestId("tree")).toBeVisible();
    await settleRestore(page);
    await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(2);
    await selectEvaluationTfm(page, "net8.0");
    await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(3);

    // No build yet: Debug is available; the host starts the implicit build on launch.
    await expect(page.getByTestId("toolbar-debug")).toBeEnabled();

    // Breakpoint in Program.cs through the real editor. The gutter marker column is a
    // separate column from the line numbers.
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await expect(page.getByTestId("editor")).toBeVisible();
    await page.getByTestId("editor").click();
    const breakpointLine = page.locator(".cm-line").nth(2);
    await breakpointLine.click();
    await expect(page.getByTestId("editor-shell")).toHaveAttribute("data-cursor", /^2:/);
    const lineBox = (await breakpointLine.boundingBox())!;
    const gutterBox = (await page.locator(".cm-debug-gutter").boundingBox())!;
    await page.mouse.click(gutterBox.x + 5, lineBox.y + lineBox.height / 2);
    await page.getByTestId("bottom-tab-debug").click();
    await expect(page.getByTestId("debug-breakpoints")).toContainText(
      "src/OrderDesk.Cli/Program.cs:3",
    );
    await expect(page.getByTestId("debug-breakpoints")).toContainText("pending");
    await expect(page.locator(".cm-lineNumbers")).toBeVisible();
    await expect(page.locator(".cm-debug-gutter")).toBeVisible();
    expect(await page.locator(".cm-debug-marker").count()).toBe(1);
    await expect(page.locator(".cm-line-breakpoint")).toHaveCount(1);
    const lineColor = (selector: string) => page.locator(selector).evaluate((line) => {
      const channels = getComputedStyle(line).backgroundColor.match(/[\d.]+/g)?.map(Number) ?? [];
      return channels.slice(0, 3);
    });
    const breakpointColor = await lineColor(".cm-line-breakpoint");
    expect(breakpointColor[0]).toBeGreaterThan(breakpointColor[1]);

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
    // Flags netcoredbg 3.2.0 answers to `initialize`; an unadvertised one must stay absent.
    await expect(page.getByTestId("dap-capabilities")).toContainText("supportsConditionalBreakpoints");
    await expect(page.getByTestId("dap-capabilities")).not.toContainText("supportsGotoTargetsRequest");
    await expect(page.getByTestId("dap-thread-list")).toContainText("Main Thread");
    await expect(page.getByTestId("debug-frames")).toContainText("Program.<Main>$()");
    await expect(page.getByTestId("debug-locals")).toContainText("order");
    const fullyVisibleInPanel = (element: Element) => {
      const panel = element.closest(".panel-body");
      if (!panel) return false;
      const content = element.getBoundingClientRect();
      const viewport = panel.getBoundingClientRect();
      return content.top >= viewport.top && content.bottom <= viewport.bottom;
    };
    await expect.poll(() => page.getByTestId("debug-frames").evaluate(fullyVisibleInPanel)).toBe(true);
    await expect.poll(() => page.getByTestId("debug-locals").evaluate(fullyVisibleInPanel)).toBe(true);
    await expect(page.getByTestId("debug-breakpoints")).toContainText("verified by the adapter");
    await expect(page.getByTestId("dap-log")).toContainText("event stopped · reason breakpoint");
    // The logs are tabs: the inactive one is hidden but keeps its text.
    await expect(page.getByTestId("dap-output")).toBeVisible();
    await page.getByTestId("debug-tab-output").click();
    await expect(page.getByTestId("dap-log")).toBeVisible();
    await expect(page.getByTestId("dap-output")).toBeHidden();
    await page.getByTestId("debug-tab-console").click();
    await expect(page.getByTestId("dap-output")).toBeVisible();
    await expect(page.locator(".cm-line-current")).toHaveCount(1);
    const currentColor = await lineColor(".cm-line-current");
    expect(currentColor[0]).toBeGreaterThan(currentColor[2]);
    expect(currentColor[1]).toBeGreaterThan(currentColor[2]);
    await page.screenshot({ path: path.join(screenshots, "14-dap-paused.png"), fullPage: true });

    // Step into the constructor and back out, then step over the current statement.
    await page.getByTestId("debug-step-into").click();
    await expect(page.getByTestId("debug-frames")).toContainText(/Order\.cs:[5-9]/, {
      timeout: 15_000,
    });
    // A frame with source opens in the editor at its line; its locals replace the list.
    await page.getByRole("button", { name: "Program.<Main>$()" }).click();
    await expect(page.getByTestId("breadcrumb")).toContainText("Program.cs");
    await expect(page.getByRole("button", { name: "Program.<Main>$()" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await expect(page.getByTestId("debug-locals")).toContainText("order");
    await page.getByTestId("debug-step-out").click();
    await expect(page.getByTestId("debug-frames")).toContainText("Program.cs:3", {
      timeout: 15_000,
    });
    await page.keyboard.press("F8");
    await expect(page.getByTestId("debug-frames")).toContainText("Program.cs:4", {
      timeout: 15_000,
    });
    await expect(page.getByTestId("debug-locals")).not.toContainText('order … null');

    // A nested variable expands on demand from the adapter; the fields are real, not placeholders.
    const orderToggle = page.getByRole("button", { name: "Expand order" });
    await orderToggle.click();
    await expect(page.getByTestId("debug-locals")).toContainText("Id", { timeout: 15_000 });
    await expect(page.getByTestId("debug-locals")).toContainText("A-104");
    await expect(orderToggle).toHaveAttribute("aria-expanded", "true");

    // Typed expressions run in the selected frame; an invalid one shows the adapter's message.
    await page.getByTestId("debug-evaluate-input").fill("order.Total");
    await page.getByTestId("debug-evaluate-run").click();
    await expect(page.getByTestId("debug-evaluate-result")).toContainText("42", { timeout: 15_000 });
    await page.getByTestId("debug-evaluate-input").fill("notAName");
    await page.getByTestId("debug-evaluate-run").click();
    await expect(page.getByTestId("debug-evaluate-result").locator(".warning")).toBeVisible({
      timeout: 15_000,
    });
    // A watch is kept and evaluated against the current pause.
    await page.getByTestId("debug-watch-input").fill("order.Id");
    await page.getByTestId("debug-watch-add").click();
    await expect(page.getByTestId("debug-watch-0")).toContainText("A-104", { timeout: 15_000 });
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

test("one click on Debug enables the flag, builds implicitly and stops at a breakpoint", async ({
  page,
  request,
}) => {
  const realAdapter = path.join(
    process.env.HOME ?? "",
    "Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092",
  );
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");

  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug1-"));
  const copy = path.join(base, "Order Desk one click");
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
    await page.getByTestId("trust-prompt-trust").click();
    await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });
    // Trust restores by itself; Debug never restores. Wait for the real assets, then let the
    // host build implicitly on the first Debug click.
    await page.getByTestId("workspace-tools-toggle").click();
    await settleRestore(page);
    await page.getByTestId("workspace-tools-toggle").click();
    await hostCall(request, "debug.setBreakpoints", {
      path: path.join(copy, "src/OrderDesk.Cli/Program.cs"),
      lines: [3],
    });

    // aria-disabled (blocked but clickable) fails Playwright's enabled check, so force it.
    await page.getByTestId("toolbar-debug").click({ force: true });
    // The Tauri webview has no window.confirm, so consent is an in-app dialog.
    await page.getByTestId("debug-consent-accept").click();
    await expect(page.getByTestId("dap-state")).toContainText(/Building|Preparing|Paused/, {
      timeout: 30_000,
    });
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 120_000 });
    await expect(page.getByTestId("debug-frames")).toContainText("Program.cs:3");
    await page.getByTestId("debug-stop").click();
  } finally {
    await hostCall(request, "debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

/**
 * Copies OrderDesk with a replaced Program.cs, installs the real adapter, opens the copy
 * trusted and builds it in Debug with a breakpoint on `breakpointLine` (if given).
 */
async function prepareDebugCopy(
  page: Page,
  request: APIRequestContext,
  base: string,
  source: string,
  breakpointLine: number | null,
) {
  const copy = path.join(base, "Order Desk debug");
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  const program = path.join(copy, "src/OrderDesk.Cli/Program.cs");
  fs.writeFileSync(program, source);
  const realAdapter = path.join(
    process.env.HOME ?? "",
    "Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092",
  );
  fs.rmSync(debugToolDir, { recursive: true, force: true });
  fs.mkdirSync(debugToolDir, { recursive: true });
  fs.cpSync(realAdapter, path.join(debugToolDir, "3.2.0-1092"), { recursive: true });

  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });

  const flagged = await hostCall(request, "debug.flag", { enabled: true, confirm: true });
  expect(flagged.ok, JSON.stringify(flagged.error)).toBe(true);
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

  if (breakpointLine !== null) {
    const breakpoint = await hostCall(request, "debug.setBreakpoints", {
      path: program,
      lines: [breakpointLine],
    });
    expect(breakpoint.ok, JSON.stringify(breakpoint.error)).toBe(true);
  }
  await page.getByTestId("bottom-tab-build").click();
  await page.getByTestId("build-action").click();
  await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
    timeout: 60_000,
  });
  return program;
}

test("selects another thread and shows its own frames while paused", async ({ page, request }) => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug-threads-"));
  try {
    // A second thread blocks on a gate and stops at line 10, while Main waits in Join.
    await prepareDebugCopy(
      page,
      request,
      base,
      [
        "using System.Threading;",
        "var gate = new ManualResetEventSlim(false);",
        "var worker = new Thread(() => Worker(gate));",
        "worker.Start();",
        "gate.Set();",
        "worker.Join();",
        "static void Worker(ManualResetEventSlim gate)",
        "{",
        "    gate.Wait();",
        '    var label = "worker";',
        "    System.Console.WriteLine(label);",
        "}",
        "",
      ].join("\n"),
      10,
    );
    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });
    await page.getByTestId("bottom-tab-debug").click();

    const threads = page.locator('[data-testid^="debug-thread-"]');
    await expect.poll(() => threads.count(), { timeout: 15_000 }).toBeGreaterThanOrEqual(2);
    const pressed = page.locator('[data-testid^="debug-thread-"] button[aria-pressed="true"]');
    await expect(pressed).toHaveCount(1);
    // The thread that hit the breakpoint is selected and shows the worker frame.
    await expect(page.getByTestId("debug-frames")).toContainText("Worker");

    // Capture the other thread's id first: a filter on the pressed state would move after the click.
    const otherId = await threads
      .filter({ hasNot: page.locator('button[aria-pressed="true"]') })
      .first()
      .getAttribute("data-testid");
    const otherButton = page.getByTestId(otherId!).locator("button");
    await otherButton.click();
    await expect(otherButton).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByTestId("debug-frames")).not.toContainText("Worker", { timeout: 15_000 });
    await expect(page.getByTestId("debug-frames")).toContainText("Program.<Main>");
  } finally {
    await hostCall(request, "debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("pauses a running program and restarts the session with the same configuration", async ({
  page,
  request,
}) => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug-pause-"));
  try {
    // A loop with no breakpoint keeps the program running until Pause stops it.
    await prepareDebugCopy(
      page,
      request,
      base,
      [
        "using System.Threading;",
        "var counter = 0;",
        "while (true)",
        "{",
        "    counter++;",
        "    Thread.Sleep(10);",
        "}",
        "",
      ].join("\n"),
      null,
    );
    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Running", { timeout: 60_000 });
    await page.getByTestId("bottom-tab-debug").click();
    await page.getByTestId("debug-pause").click();
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 30_000 });
    await expect(page.getByTestId("debug-frames")).toContainText("Program.<Main>");
    const sessionBefore = await page.getByTestId("dap-session").textContent();

    await page.getByTestId("debug-restart").click();
    await expect(page.getByTestId("dap-session")).not.toHaveText(sessionBefore ?? "", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("dap-state")).toContainText("Running", { timeout: 60_000 });
    await expect(page.getByTestId("dap-assembly")).toContainText("OrderDesk.Cli.dll");
    await page.getByTestId("debug-pause").click();
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 30_000 });
  } finally {
    await hostCall(request, "debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

const loopWithBreakpoint = [
  "var total = 0;",
  "for (var index = 0; index < 5; index++)",
  "{",
  "    total += index;",
  "}",
  "System.Console.WriteLine(total);",
  "",
].join("\n");

test("a conditional breakpoint stops only when its condition is true", async ({ page, request }) => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug-condition-"));
  try {
    await prepareDebugCopy(page, request, base, loopWithBreakpoint, 4);
    await page.getByTestId("bottom-tab-debug").click();
    const condition = page.locator('input[data-testid^="debug-breakpoint-condition-"]');
    await expect(condition).toHaveCount(1);
    await condition.fill("index == 3");
    await page.locator('button[data-testid^="debug-breakpoint-condition-save-"]').click();
    await expect(condition).toHaveValue("index == 3");

    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });
    // Only the fourth pass stops, so index is 3 and total is still 0 + 1 + 2.
    await page.getByTestId("debug-evaluate-input").fill("index");
    await page.getByTestId("debug-evaluate-run").click();
    await expect(page.getByTestId("debug-evaluate-result")).toContainText("3");
  } finally {
    await hostCall(request, "debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a condition that never holds lets the program finish", async ({ page, request }) => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug-condition-off-"));
  try {
    await prepareDebugCopy(page, request, base, loopWithBreakpoint, 4);
    await page.getByTestId("bottom-tab-debug").click();
    const condition = page.locator('input[data-testid^="debug-breakpoint-condition-"]');
    await condition.fill("index > 10");
    await page.locator('button[data-testid^="debug-breakpoint-condition-save-"]').click();
    await expect(condition).toHaveValue("index > 10");

    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Terminated", { timeout: 60_000 });
    await expect(page.getByTestId("dap-state")).not.toContainText("Paused");
    await expect(page.getByTestId("dap-output")).toContainText("10");
  } finally {
    await hostCall(request, "debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("mute all keeps breakpoints out of the run", async ({ page, request }) => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug-mute-"));
  try {
    await prepareDebugCopy(page, request, base, loopWithBreakpoint, 4);
    await page.getByTestId("bottom-tab-debug").click();
    await page.getByTestId("debug-breakpoints-mute").click();
    await expect(page.getByTestId("debug-breakpoints-mute")).toHaveAttribute("aria-pressed", "true");

    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Terminated", { timeout: 60_000 });
    await expect(page.getByTestId("dap-output")).toContainText("10");
    await expect(page.getByTestId("debug-breakpoints")).toContainText("Program.cs:4");
  } finally {
    await hostCall(request, "debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("an unhandled exception stops at its origin by default", async ({ page, request }) => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug-unhandled-"));
  try {
    await prepareDebugCopy(
      page,
      request,
      base,
      [
        "var total = 0;",
        "total += 1;",
        'throw new System.InvalidOperationException("boom");',
        "",
      ].join("\n"),
      null,
    );
    await page.getByTestId("bottom-tab-debug").click();
    await expect(page.getByTestId("debug-exceptions-thrown")).not.toBeChecked();
    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });
    await expect(page.getByTestId("dap-log")).toContainText("reason exception");
  } finally {
    await hostCall(request, "debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("thrown exceptions stop only when the filter is on", async ({ page, request }) => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug-thrown-"));
  try {
    // The exception is caught, so only the thrown filter can stop here.
    await prepareDebugCopy(
      page,
      request,
      base,
      [
        "var caught = 0;",
        "try { throw new System.InvalidOperationException(\"boom\"); }",
        "catch (System.InvalidOperationException) { caught++; }",
        "System.Console.WriteLine(caught);",
        "",
      ].join("\n"),
      null,
    );
    await page.getByTestId("bottom-tab-debug").click();
    // Controlled checkbox: it shows the host state only after the answer, so wait for it.
    await page.getByTestId("debug-exceptions-thrown").click();
    await expect(page.getByTestId("debug-exceptions-thrown")).toBeChecked({ timeout: 15_000 });

    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });
    await expect(page.getByTestId("dap-log")).toContainText("reason exception");
  } finally {
    await hostCall(request, "debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("run to cursor stops on the chosen line without keeping a breakpoint", async ({ page, request }) => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug-run-to-"));
  try {
    await prepareDebugCopy(page, request, base, loopWithBreakpoint, 1);
    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });

    // Opening the top frame shows Program.cs in the editor, as the call stack does.
    await page.getByTestId("debug-frames").getByRole("button", { name: "Program.<Main>$()" }).click();
    // Line 6 is the WriteLine after the loop; no breakpoint is set there.
    await page.locator(".cm-line", { hasText: "WriteLine" }).click({ timeout: 10_000 });
    await page.keyboard.press("Alt+F9");
    await expect(page.locator(".cm-line-current")).toContainText("WriteLine", { timeout: 60_000 });
    await expect(page.getByTestId("dap-state")).toContainText("Paused");
    await expect(page.getByTestId("debug-breakpoints")).not.toContainText(":6");
  } finally {
    await hostCall(request, "debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});
