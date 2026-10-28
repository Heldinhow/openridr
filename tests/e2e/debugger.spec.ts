import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

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
  await expect(page.getByTestId("sdk-debug-adapter")).toBeVisible();
}

test.beforeAll(() => {
  fs.mkdirSync(screenshots, { recursive: true });
  fs.mkdirSync(logs, { recursive: true });
});

test("macOS arm64 blocks the debug adapter honestly and the flag installs nothing", async ({
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
    "netcoredbg-osx-arm64.zip · unverified",
  );
  await expect(page.getByTestId("debug-source")).toContainText("approved version 3.2.0-1092");
  await expect(page.getByTestId("debug-source")).toContainText("Samsung/netcoredbg");
  await expect(page.getByTestId("debug-source")).toContainText("DAP proven: no");
  await expect(page.getByTestId("debug-message")).toContainText(
    "Unavailable on macOS arm64: no verified compatible netcoredbg 3.2.0-1092 adapter",
  );
  await expect(page.getByTestId("debug-message")).toContainText("community supported");
  await expect(page.getByTestId("debug-dap")).toContainText("not implemented in this build");
  await expect(page.getByTestId("debug-flag-note")).toContainText(
    "neither installs netcoredbg nor proves compatibility",
  );
  await expect(page.getByTestId("debug-install")).toBeDisabled();
  await expect(page.getByTestId("debug-disabled-reason")).toContainText(
    "no verified compatible netcoredbg 3.2.0-1092 adapter",
  );

  // Plan details carry every material field and the blocked reason.
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
    "no verified compatible netcoredbg 3.2.0-1092 adapter",
  );
  await page.screenshot({ path: path.join(screenshots, "13-debugger-plan.png"), fullPage: true });
  fs.writeFileSync(
    path.join(logs, "13-debugger-plan.json"),
    JSON.stringify({ platform: debug.platform, plan: debug.plan }, null, 2),
  );

  // No install/cancel/retry is offered for a platform with no installable asset: there is
  // nothing to download, so the blocked state is the honest end of the flow.
  await expect(page.getByTestId("debug-install")).toBeDisabled();
  await expect(page.getByTestId("debug-cancel")).toHaveCount(0);
  expect(fs.existsSync(debugToolDir)).toBe(false);

  // Explicit, separate experimental opt-in: acknowledged by checkbox, then enabled.
  await page.getByTestId("debug-flag-checkbox").check();
  await expect(page.getByTestId("debug-flag-enable")).toBeEnabled();
  await page.getByTestId("debug-flag-enable").click();
  await expect(page.getByTestId("sdk-apple-silicon")).toContainText("experimental · enabled");
  await expect(page.getByTestId("debug-state")).toHaveText("Unavailable");
  await expect(page.getByTestId("debug-message")).toContainText(
    "neither installs the adapter nor proves compatibility",
  );
  expect(fs.existsSync(debugToolDir)).toBe(false);
  await page.screenshot({ path: path.join(screenshots, "13-debugger-flag.png"), fullPage: true });

  // A direct host call with trust and a valid plan is still refused before any fetch.
  await page.getByTestId("trust-checkbox").check();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
  const replan = await hostCall(request, "debug.plan");
  const planId = replan.data.debug.plan.planId;
  const refused = await hostCall(request, "debug.install", { planId, confirm: true });
  expect(refused.ok).toBe(false);
  expect(refused.error.code).toBe("unsupported_platform");
  expect(refused.error.message).toContain("Nothing was downloaded");
  expect(fs.existsSync(debugToolDir)).toBe(false);
  fs.writeFileSync(
    path.join(logs, "13-debugger-refused.json"),
    JSON.stringify(
      { planId, error: refused.error, debug: (await hostCall(request, "debug.state")).data.debug },
      null,
      2,
    ),
  );

  // Analysis, Run and the editor stay available: the debugger limit does not block them.
  await page.getByTestId("open-text-only").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(page.getByTestId("toolbar-debug")).toBeDisabled();
  await expect(page.getByTestId("toolbar-debug")).toHaveAttribute(
    "title",
    /Unavailable on macOS arm64/,
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
  expect(flag.data.debug.installable).toBe(false);

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
