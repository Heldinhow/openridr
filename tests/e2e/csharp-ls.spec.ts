import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { reveal } from "./disclosures";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
import { e2eHome, host } from "./environment";
const screenshots = path.resolve(process.cwd(), "docs/evidence/screenshots");
const logs = path.resolve(process.cwd(), "docs/evidence/logs");

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

/** SHA-256 of every C# source in the copy, to prove a code action never wrote disk. */
function sourceHashes(root: string) {
  const hashes: Record<string, string> = {};
  const walk = (dir: string) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      if (entry.name === "bin" || entry.name === "obj") continue;
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(full);
      } else if (entry.name.endsWith(".cs")) {
        hashes[path.relative(root, full)] = crypto
          .createHash("sha256")
          .update(fs.readFileSync(full))
          .digest("hex");
      }
    }
  };
  walk(path.join(root, "src"));
  walk(path.join(root, "tests"));
  return hashes;
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
  await page.getByTestId("workspace-tools-toggle").click();
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

test("disabled trust and install plan communicate state before any confirmation", async ({
  page,
}) => {
  await openWorkspace(page, fixture);
  await expect(page.getByTestId("trust-grant")).toBeDisabled();
  await expect(page.getByTestId("trust-grant-reason")).toBeVisible();
  await expect(page.getByTestId("trust-grant")).toHaveAttribute(
    "aria-describedby",
    "trust-grant-reason",
  );
  expect(
    await page.getByTestId("trust-grant").evaluate((button) => getComputedStyle(button).color),
  ).not.toBe("rgb(240, 160, 121)");

  await trustWorkspace(page);
  await enterWorkspace(page);
  await page.getByTestId("install-plan-open").click();
  const plan = page.getByTestId("install-plan");
  await expect(plan).toBeVisible();
  await expect(plan).toHaveAttribute("open", "");
  for (const field of [
    "install-tool",
    "install-source",
    "install-platform",
    "install-destination",
    "install-staging",
    "install-command",
    "install-cwd",
    "install-permissions",
    "install-integrity",
    "install-existing",
  ]) {
    await expect(page.getByTestId(field)).toBeVisible();
  }
  await expect(plan.getByTestId("install-confirm")).toBeVisible();
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
    const workspaceId = (await hostState(request)).workspace.id;
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
      // Other specs leave exited processes in the shared registry; cancel must create none
      // for this workspace.
      expect(
        (await hostState(request)).processes.filter(
          (process: { workspaceId: number }) => process.workspaceId === workspaceId,
        ),
      ).toEqual([]);
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

test("problems navigate real diagnostics by revision and keep stale items", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk problems");
  const program = path.join(copy, "src/OrderDesk.Cli/Program.cs");
  const clean = fs.readFileSync(program, "utf8");
  const broken = `${clean}\nvar report = new OrderReport();\n`;
  let stoppedPid: number | null = null;
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
    const startSession = (await hostState(request)).lsp.session as number;
    await expect(page.getByTestId("lsp-session")).toContainText(
      `session ${startSession}`,
    );
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await expect(page.getByTestId("editor")).toContainText("OrderDesk.Core");

    // The bottom Problems tab is reachable and activated by keyboard.
    await page.getByTestId("bottom-tab-problems").focus();
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("bottom-tab-problems")).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await expect(page.getByTestId("problems-empty")).toContainText(
      "No problems reported for src/OrderDesk.Cli/Program.cs",
      { timeout: 30_000 },
    );
    await expect(page.getByTestId("problems-empty")).toContainText(
      "solution is not claimed clean",
    );
    await expect(page.getByTestId("problems-coverage")).toContainText(
      "Analyzer coverage: not verified",
    );

    // Edit the real buffer: csharp-ls publishes a real CS0246 for the new revision.
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(broken);
    await expect(page.getByTestId("bottom-tab-problems")).toContainText(
      "Problems (1)",
      { timeout: 30_000 },
    );
    const brokenState = (await hostState(request)).diagnostics;
    const brokenSnapshot = brokenState.snapshots.find(
      (snapshot: { items: { code: string }[] }) =>
        snapshot.items.some((item) => item.code === "CS0246"),
    );
    expect(brokenSnapshot).toBeTruthy();
    const brokenItem = brokenSnapshot.items.find(
      (item: { code: string }) => item.code === "CS0246",
    );
    expect(brokenSnapshot.origin.label).toBe("LSP · csharp-ls");
    expect(brokenItem.category).toBe("unknown");
    expect(brokenItem.source).toBe("lsp");
    expect(brokenSnapshot.serverVersion).toBeNull();
    expect(brokenSnapshot.clientRevision).toBeGreaterThanOrEqual(2);
    expect(brokenSnapshot.state).toBe("current");
    expect(brokenItem.position).not.toBeNull();

    const row = page.getByTestId("problem-row-0");
    await expect(row).toContainText("CS0246");
    await expect(row).toContainText("LSP · csharp-ls");
    await expect(row).toContainText("unknown");
    await row.click();
    await expect(page.getByTestId("editor-shell")).toHaveAttribute(
      "data-cursor",
      `${brokenItem.position.line}:${brokenItem.position.character}`,
    );
    const cursorAtDiagnostic = await page
      .getByTestId("editor-shell")
      .getAttribute("data-cursor");
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({
      path: path.join(screenshots, "06-problems.png"),
      fullPage: false,
    });

    // Pause the real server to hold the waiting state: the old item stays visible, labelled
    // awaiting, and navigation is refused instead of moving the cursor.
    const beforePause = await hostState(request);
    stoppedPid = beforePause.lsp.pid as number;
    process.kill(stoppedPid, "SIGSTOP");
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(`${broken}\nvar extra = new AlsoMissing();\n`);
    await expect(page.getByTestId("problem-stale")).toBeVisible({ timeout: 5_000 });
    await expect(page.getByTestId("bottom-tab-problems")).toHaveText("Problems", {
      timeout: 5_000,
    });
    const awaitingRow = page.getByTestId("problem-row-0");
    await expect(awaitingRow).toContainText("Awaiting diagnostics");
    await expect(awaitingRow).toContainText("CS0246");
    const cursorBeforeAwaitingClick = await page
      .getByTestId("editor-shell")
      .getAttribute("data-cursor");
    await awaitingRow.click();
    await expect(page.getByTestId("problem-details-state")).toContainText(
      "Awaiting diagnostics",
    );
    await expect(page.getByTestId("problem-navigate")).toBeDisabled();
    await expect(page.getByTestId("editor-shell")).toHaveAttribute(
      "data-cursor",
      cursorBeforeAwaitingClick!,
    );
    await page.screenshot({
      path: path.join(screenshots, "06-problems-awaiting.png"),
      fullPage: false,
    });
    process.kill(stoppedPid, "SIGCONT");
    stoppedPid = null;

    // The real republication replaces the coverage for the new revision.
    await expect(page.getByTestId("bottom-tab-problems")).toContainText(
      "Problems (2)",
      { timeout: 30_000 },
    );
    const republished = (await hostState(request)).diagnostics;
    const republishedSnapshot = republished.snapshots.find(
      (snapshot: { state: string }) => snapshot.state === "current",
    );
    expect(republishedSnapshot.clientRevision).toBeGreaterThan(
      brokenSnapshot.clientRevision,
    );
    expect(republishedSnapshot.currentRevision).toBe(
      republishedSnapshot.clientRevision,
    );

    // Restart: items from the previous session stay identified as stale.
    await page.getByTestId("lsp-restart").click();
    await expect(page.getByTestId("lsp-session")).toContainText(
      `session ${startSession + 1}`,
      { timeout: 30_000 },
    );
    await expect(page.getByTestId("lsp-state")).toContainText("Ready", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("problem-stale")).toBeVisible({ timeout: 10_000 });
    const restarted = (await hostState(request)).diagnostics;
    const staleSnapshot = restarted.snapshots.find(
      (snapshot: { session: number }) => snapshot.session === startSession,
    );
    expect(staleSnapshot.state).toBe("stale");
    expect(staleSnapshot.items.length).toBeGreaterThan(0);
    const staleRow = page
      .locator('[data-testid^="problem-row-"][data-state="stale"]')
      .first();
    await expect(staleRow).toContainText("Stale");
    const cursorBeforeStaleClick = await page
      .getByTestId("editor-shell")
      .getAttribute("data-cursor");
    await staleRow.click();
    await expect(page.getByTestId("problem-details-state")).toContainText("Stale");
    await expect(page.getByTestId("problem-navigate")).toBeDisabled();
    await expect(page.getByTestId("editor-shell")).toHaveAttribute(
      "data-cursor",
      cursorBeforeStaleClick!,
    );

    // Fix the buffer. Only the current document/publication is claimed clean.
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(clean);
    await expect(page.getByTestId("problems-empty")).toContainText(
      "No problems reported for src/OrderDesk.Cli/Program.cs",
      { timeout: 30_000 },
    );
    await expect(page.getByTestId("problems-empty")).toContainText(
      "solution is not claimed clean",
    );
    await expect(page.getByTestId("problems-coverage")).toContainText(
      "No solution-wide clean state is claimed",
    );

    // Developer boundary: a real publication without a range must not offer navigation.
    const probe = await hostCall(request, "diagnostics.probe", {
      path: "diagnostics-probe/NoPosition.cs",
    });
    expect(probe.ok, JSON.stringify(probe.error)).toBe(true);
    const probedRow = page
      .locator('[data-testid^="problem-row-"]')
      .filter({ hasText: "PROBE0001" })
      .first();
    await expect(probedRow).toBeVisible({ timeout: 10_000 });
    const cursorBeforeProbeClick = await page
      .getByTestId("editor-shell")
      .getAttribute("data-cursor");
    await probedRow.click();
    await expect(page.getByTestId("problem-no-position")).toContainText(
      "No position was published",
    );
    await expect(page.getByTestId("problem-navigate")).toBeDisabled();
    await expect(page.getByTestId("editor-shell")).toHaveAttribute(
      "data-cursor",
      cursorBeforeProbeClick!,
    );

    // Revoke: analysis is unavailable, items keep their session as stale, editing works.
    await reveal(page.getByTestId("trust-revoke"));
    await page.getByTestId("trust-revoke").click();
    await expect(page.getByTestId("mode-restricted")).toBeVisible();
    await expect(page.getByTestId("problem-stale")).toContainText("Trust was revoked");
    await expect(
      page.locator('[data-testid^="problem-row-"]').first(),
    ).toHaveAttribute("data-state", "stale");
    const afterRevokeText = `${clean}// editing after revoke\n`;
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(afterRevokeText);
    await page.keyboard.press("Meta+s");
    await expect(page.getByTestId("app-log")).toContainText(
      "Saved src/OrderDesk.Cli/Program.cs",
    );
    expect(fs.readFileSync(program, "utf8")).toContain("editing after revoke");
  } finally {
    if (stoppedPid !== null) {
      try {
        process.kill(stoppedPid, "SIGCONT");
      } catch {
        // The process already exited.
      }
    }
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
    const startSession = (await hostState(request)).lsp.session as number;
    await expect(page.getByTestId("lsp-session")).toContainText(
      `session ${startSession}`,
    );

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
    await expect(page.getByTestId("lsp-log")).toContainText(`session ${startSession}`);

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
    await expect(page.getByTestId("lsp-session")).toContainText(
      `session ${startSession + 1}`,
      { timeout: 30_000 },
    );
    await expect(page.getByTestId("lsp-state")).toContainText("Ready", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("lsp-diagnostics")).toContainText("0 item", {
      timeout: 30_000,
    });

    await reveal(page.getByTestId("trust-revoke"));
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

test("completion and definition use the real session and discard stale answers", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk completion");
  const programPath = "src/OrderDesk.Cli/Program.cs";
  const libraryPath = "src/OrderDesk.Core/Order.cs";
  const completionText =
    'using OrderDesk.Core;\n\nvar order = new Order("A-104", 42m);\norder.\n';
  let stoppedPid: number | null = null;
  const resume = () => {
    if (stoppedPid !== null) {
      try {
        process.kill(stoppedPid, "SIGCONT");
      } catch {
        // The process already exited.
      }
      stoppedPid = null;
    }
  };
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await trustWorkspace(page);
    await enterWorkspace(page);
    await waitForEvaluation(page);
    await restoreAndReload(page);
    await ensureTool(page, request);

    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await expect(page.getByTestId("editor")).toContainText("OrderDesk.Core");

    // Before analysis starts, the shortcut explains the state instead of doing nothing.
    await page.getByTestId("editor").click();
    await page.keyboard.press("Control+Space");
    await expect(page.getByTestId("editor-lsp-status")).toHaveAttribute(
      "data-state",
      "unavailable",
    );
    await expect(page.getByTestId("editor-lsp-status")).toContainText(
      "Analysis is not running",
    );
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({
      path: path.join(screenshots, "08-completion-unavailable.png"),
      fullPage: false,
    });

    await page.getByTestId("lsp-start").click();
    await expect(page.getByTestId("lsp-state")).toContainText("Ready", {
      timeout: 60_000,
    });
    const session = (await hostState(request)).lsp.session as number;

    // The real session has to know the open document before it is asked anything.
    await expect
      .poll(
        async () => {
          const body = await hostCall(request, "lsp.completion", {
            path: programPath,
            line: 0,
            character: 0,
            revision: 0,
          });
          expect(body.ok, JSON.stringify(body.error)).toBe(true);
          return body.data.documentKnown;
        },
        { timeout: 30_000, intervals: [100, 250, 500] },
      )
      .toBe(true);

    // Put the cursor after a real member access and let the host discover the real list.
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+A");
    await page.keyboard.insertText(completionText);
    await expect
      .poll(
        async () => {
          const body = await hostCall(request, "lsp.completion", {
            path: programPath,
            line: 3,
            character: 6,
            revision: 1,
          });
          expect(body.ok, JSON.stringify(body.error)).toBe(true);
          return body.data.state === "ok" &&
            (body.data.items as { label: string }[]).some(
              (item) => item.label === "Id" || item.label === "Total",
            );
        },
        { timeout: 30_000, intervals: [100, 250, 500] },
      )
      .toBe(true);
    const discovered = await hostCall(request, "lsp.completion", {
      path: programPath,
      line: 3,
      character: 6,
      revision: 1,
    });
    expect(discovered.data.state).toBe("ok");
    expect(discovered.data.clientRevision).toBeGreaterThanOrEqual(2);
    expect(discovered.data.session).toBe(session);
    const chosen = (discovered.data.items as { label: string; kind: string | null }[]).find(
      (item) => item.label === "Id" || item.label === "Total",
    );
    expect(
      chosen,
      `real completion list: ${JSON.stringify(discovered.data.items.map((item: { label: string }) => item.label))}`,
    ).toBeTruthy();
    expect(chosen!.kind).toBe("property");

    await page.keyboard.press("Control+Space");
    await expect(page.locator(".cm-tooltip-autocomplete")).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.getByTestId("editor-lsp-status")).toHaveAttribute(
      "data-state",
      "ok",
    );
    await expect(page.getByTestId("editor-lsp-status")).toContainText("Completion:");
    await expect(page.locator(".cm-tooltip-autocomplete")).toContainText(chosen!.label);
    await page.screenshot({
      path: path.join(screenshots, "08-completion.png"),
      fullPage: false,
    });
    await page.keyboard.type(chosen!.label);
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("editor")).toContainText(`order.${chosen!.label}`);
    // The accepted edit is a buffer change; let the 250 ms didChange debounce reach the server
    // before the next request so its document revision is stable.
    await page.waitForTimeout(600);

    // Definition: ask the host for the real destination, then reach it with the editor keymap.
    const definitionCursor = { line: 2, character: 20 };
    const expected = await hostCall(request, "lsp.definition", {
      path: programPath,
      ...definitionCursor,
      revision: 1,
    });
    expect(expected.ok, JSON.stringify(expected.error)).toBe(true);
    expect(expected.data.state).toBe("ok");
    expect(expected.data.locations[0].path).toBe(libraryPath);
    const target = expected.data.locations[0] as {
      path: string;
      line: number;
      character: number;
    };

    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+ArrowUp");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Home");
    for (let index = 0; index < definitionCursor.character; index += 1) {
      await page.keyboard.press("ArrowRight");
    }
    await expect(page.getByTestId("editor-shell")).toHaveAttribute(
      "data-cursor",
      `${definitionCursor.line}:${definitionCursor.character}`,
    );
    await page.keyboard.press("Meta+b");
    await expect(
      page.getByTestId(`tab-${libraryPath}`),
    ).toHaveClass(/active/, { timeout: 20_000 });
    await expect(page.getByTestId("editor")).toContainText("public sealed class Order");
    await expect(page.getByTestId("editor-shell")).toHaveAttribute(
      "data-cursor",
      `${target.line}:${target.character}`,
    );
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({
      path: path.join(screenshots, "08-definition.png"),
      fullPage: false,
    });

    // A selection change while the answer is pending invalidates it: the paused server is
    // resumed after the cursor moved, so the late location must not open a file.
    await page.getByTestId(`tab-${programPath}`).click();
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+ArrowUp");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Home");
    for (let index = 0; index < 20; index += 1) {
      await page.keyboard.press("ArrowRight");
    }
    const paused = await hostState(request);
    stoppedPid = paused.lsp.pid as number;
    process.kill(stoppedPid, "SIGSTOP");
    await page.keyboard.press("Meta+b");
    await expect(page.getByTestId("editor-lsp-status")).toHaveAttribute(
      "data-state",
      "loading",
    );
    await page.keyboard.press("ArrowRight");
    resume();
    await expect(page.getByTestId("editor-lsp-status")).toHaveAttribute(
      "data-state",
      "stale",
      { timeout: 20_000 },
    );
    await expect(page.getByTestId("editor-lsp-status")).toContainText(
      "selection changed",
    );
    await expect(page.getByTestId(`tab-${programPath}`)).toHaveClass(/active/);

    // A wedged server is a timeout, not an endless wait, and the editor stays usable.
    const beforeTimeout = await hostState(request);
    stoppedPid = beforeTimeout.lsp.pid as number;
    process.kill(stoppedPid, "SIGSTOP");
    await page.keyboard.press("Control+Space");
    await expect(page.getByTestId("editor-lsp-status")).toHaveAttribute(
      "data-state",
      "timeout",
      { timeout: 20_000 },
    );
    await expect(page.getByTestId("editor-lsp-status")).toContainText(
      "did not answer within 5 seconds",
    );
    await page.keyboard.press("End");
    await page.keyboard.insertText("// editing is not blocked\n");
    await expect(page.getByTestId("editor")).toContainText(
      "editing is not blocked",
    );
    resume();

    // After revoke, the same shortcuts explain the missing trust instead of failing silently.
    await reveal(page.getByTestId("trust-revoke"));
    await page.getByTestId("trust-revoke").click();
    await expect(page.getByTestId("mode-restricted")).toBeVisible();
    await page.getByTestId("editor").click();
    await page.keyboard.press("Control+Space");
    await expect(page.getByTestId("editor-lsp-status")).toHaveAttribute(
      "data-state",
      "unavailable",
    );
    await expect(page.getByTestId("editor-lsp-status")).toContainText(
      "requires workspace trust",
    );
    await page.keyboard.press("Meta+b");
    await expect(page.getByTestId("editor-lsp-status")).toContainText(
      "requires workspace trust",
    );
    await expect(page.getByTestId(`tab-${programPath}`)).toHaveClass(/active/);
  } finally {
    resume();
    fs.rmSync(base, { recursive: true, force: true });
  }
});

const renameFiles = {
  library: "src/OrderDesk.Core/Order.cs",
  program: "src/OrderDesk.Cli/Program.cs",
  tests: "tests/OrderDesk.Tests/OrderTests.cs",
};

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

/** Column 0 even on indented lines; `placeCursor` stops at the first non-space. */
async function placeCursorAtColumnZero(page: Page, line: number) {
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+ArrowUp");
  for (let index = 0; index < line; index += 1) {
    await page.keyboard.press("ArrowDown");
  }
  await page.keyboard.press("Home");
  await page.keyboard.press("Home");
  await expect(page.getByTestId("editor-shell")).toHaveAttribute(
    "data-cursor",
    `${line}:0`,
  );
}

async function startRenameSession(
  page: Page,
  request: APIRequestContext,
  copy: string,
) {
  await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
  await trustWorkspace(page);
  await enterWorkspace(page);
  await waitForEvaluation(page);
  await restoreAndReload(page);
  await ensureTool(page, request);
  await page.getByTestId(`tree-file-${renameFiles.library}`).click();
  await expect(page.getByTestId("editor")).toContainText("public sealed class Order");
  // Before analysis starts the visible control explains the missing session.
  await expect(page.getByTestId("rename-open")).toBeDisabled();
  await expect(page.getByTestId("rename-open")).toHaveAttribute(
    "title",
    /Start analysis/,
  );
  await expect(page.getByTestId("code-actions-open")).toBeDisabled();
  await expect(page.getByTestId("code-actions-open")).toHaveAttribute(
    "title",
    /Start analysis/,
  );
  await page.getByTestId("lsp-start").click();
  await expect(page.getByTestId("lsp-state")).toContainText("Ready", {
    timeout: 60_000,
  });
  await expect(page.getByTestId("lsp-capabilities")).toContainText("rename");
  await expect(page.getByTestId("lsp-capabilities")).toContainText("code actions");
  await expect
    .poll(
      async () => {
        const body = await hostCall(request, "lsp.completion", {
          path: renameFiles.library,
          line: 2,
          character: 20,
          revision: 0,
        });
        expect(body.ok, JSON.stringify(body.error)).toBe(true);
        return body.data.documentKnown;
      },
      { timeout: 30_000, intervals: [100, 250, 500] },
    )
    .toBe(true);
  await expect(page.getByTestId("rename-open")).toBeEnabled();
  await expect(page.getByTestId("code-actions-open")).toBeEnabled();
}

test("rename previews real cross-project edits, cancels inert and applies to buffers only", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk rename");
  const library = path.join(copy, renameFiles.library);
  const program = path.join(copy, renameFiles.program);
  const tests = path.join(copy, renameFiles.tests);
  const before = [library, program, tests].map((file) => fs.readFileSync(file));
  try {
    await startRenameSession(page, request, copy);
    await placeCursor(page, 2, 20);
    await page.keyboard.press("Shift+F6");
    await expect(page.getByTestId("rename-input")).toBeFocused();
    await expect(page.getByTestId("rename-source")).toContainText(
      `${renameFiles.library} · 3:21`,
    );
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({
      path: path.join(screenshots, "09-rename-prompt.png"),
      fullPage: false,
    });
    await page.getByTestId("rename-input").fill("Purchase");
    await page.getByTestId("rename-submit").click();
    await expect(page.getByTestId("rename-preview")).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByTestId("rename-preview")).toHaveAttribute("data-files", "3");
    for (const file of Object.values(renameFiles)) {
      await expect(page.getByTestId(`rename-file-${file}`)).toBeVisible();
      await expect(page.getByTestId(`rename-index-${file}`)).toBeVisible();
    }
    await expect(page.getByTestId(`rename-index-${renameFiles.tests}`)).toContainText(
      "not opened here",
    );
    await expect(page.getByTestId(`rename-file-${renameFiles.tests}`)).toContainText(
      "not opened here",
    );
    await expect(page.getByTestId(`rename-diff-${renameFiles.program}`)).toContainText(
      "Purchase",
    );
    await expect(page.getByTestId(`rename-diff-${renameFiles.program}`)).toContainText(
      "new Order",
    );
    await expect(
      page.getByTestId(`rename-count-${renameFiles.library}`),
    ).toContainText("2 edit");
    await page.screenshot({
      path: path.join(screenshots, "09-rename-preview.png"),
      fullPage: false,
    });
    await page.getByTestId("rename-preview").evaluate((element) => {
      element.scrollTop = element.scrollHeight;
    });
    await page.screenshot({
      path: path.join(screenshots, "09-rename-preview-files.png"),
      fullPage: false,
    });

    // Cancel changes no buffer and no disk byte.
    const revisionBefore = await page.getByTestId("editor-revision").textContent();
    await page.getByTestId("rename-cancel-preview").click();
    await expect(page.getByTestId("rename-preview")).toHaveCount(0);
    await expect(page.getByTestId("editor-revision")).toHaveText(revisionBefore!);
    await expect(page.getByTestId(`tab-${renameFiles.tests}`)).toHaveCount(0);
    await expect(page.getByTestId("editor")).toContainText("public sealed class Order");
    [library, program, tests].forEach((file, index) => {
      expect(
        fs.readFileSync(file).equals(before[index]),
        `${file} changed when the preview was cancelled`,
      ).toBe(true);
    });

    // Apply through the visible control; every buffer becomes modified, disk stays untouched.
    await page.getByTestId("rename-open").click();
    await expect(page.getByTestId("rename-input")).toBeVisible();
    await page.getByTestId("rename-input").fill("Purchase");
    await page.getByTestId("rename-submit").click();
    await expect(page.getByTestId("rename-preview")).toBeVisible({
      timeout: 30_000,
    });
    await page.getByTestId("rename-apply").click();
    await expect(page.getByTestId("rename-preview")).toHaveCount(0, {
      timeout: 20_000,
    });
    await expect(page.getByTestId("editor")).toContainText(
      "public sealed class Purchase",
    );
    await expect(page.getByTestId("editor")).toContainText("public Purchase(");
    await expect(page.getByTestId("editor-revision")).toContainText("modified");
    await expect(page.getByTestId("app-log")).toContainText(
      "Nothing was written to disk",
    );
    await expect(page.getByTestId(`tab-${renameFiles.program}`)).toBeVisible();
    await expect(page.getByTestId(`tab-${renameFiles.tests}`)).toBeVisible();
    await expect(
      page.getByTestId(`tab-${renameFiles.tests}`).getByTestId("dirty-marker"),
    ).toBeVisible();
    [library, program, tests].forEach((file, index) => {
      expect(
        fs.readFileSync(file).equals(before[index]),
        `${file} was written by Apply`,
      ).toBe(true);
    });
    await page.screenshot({
      path: path.join(screenshots, "09-rename-applied.png"),
      fullPage: false,
    });

    await page.getByTestId(`tab-${renameFiles.program}`).click();
    await expect(page.getByTestId("editor")).toContainText("new Purchase");
    await expect(page.getByTestId("editor")).toContainText("order.Id");
    await page.getByTestId(`tab-${renameFiles.library}`).click();
    await page.getByTestId("save-file").click();
    await expect(page.getByTestId("app-log")).toContainText(
      `Saved ${renameFiles.library}`,
    );
    expect(fs.readFileSync(library, "utf8")).toContain("public sealed class Purchase");
    // Save remained explicit: only the saved file reached disk.
    expect(fs.readFileSync(program, "utf8")).toContain("new Order");
    expect(fs.readFileSync(tests, "utf8")).toContain("new Order");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a stale rename preview is rejected as a whole and late answers are discarded", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk rename stale");
  const library = path.join(copy, renameFiles.library);
  const program = path.join(copy, renameFiles.program);
  const tests = path.join(copy, renameFiles.tests);
  const before = [library, program, tests].map((file) => fs.readFileSync(file));
  let stoppedPid: number | null = null;
  const resume = () => {
    if (stoppedPid !== null) {
      try {
        process.kill(stoppedPid, "SIGCONT");
      } catch {
        // The process already exited.
      }
      stoppedPid = null;
    }
  };
  try {
    await startRenameSession(page, request, copy);
    await placeCursor(page, 2, 20);
    await page.keyboard.press("Shift+F6");
    await page.getByTestId("rename-input").fill("Purchase");
    await page.getByTestId("rename-submit").click();
    await expect(page.getByTestId("rename-preview")).toBeVisible({
      timeout: 30_000,
    });

    // Change the unopened tests file on disk after the preview: the whole set is rejected.
    const external = `${before[2].toString("utf8")}// external edit during preview\n`;
    fs.writeFileSync(tests, external);
    await page.getByTestId("rename-apply").click();
    await expect(page.getByTestId("rename-stale")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("rename-stale")).toContainText("changed on disk");
    await expect(page.getByTestId("rename-preview")).toBeVisible();
    await expect(page.getByTestId(`tab-${renameFiles.tests}`)).toHaveCount(0);
    await expect(page.getByTestId("editor")).toContainText("public sealed class Order");
    expect(fs.readFileSync(library).equals(before[0])).toBe(true);
    expect(fs.readFileSync(program).equals(before[1])).toBe(true);
    expect(fs.readFileSync(tests, "utf8")).toBe(external);
    await page.screenshot({
      path: path.join(screenshots, "09-rename-stale.png"),
      fullPage: false,
    });
    await page.getByTestId("rename-cancel-preview").click();
    fs.writeFileSync(tests, before[2]);

    // Change the source buffer after the preview: apply rejects everything.
    await page.getByTestId("rename-open").click();
    await page.getByTestId("rename-input").fill("Purchase");
    await page.getByTestId("rename-submit").click();
    await expect(page.getByTestId("rename-preview")).toBeVisible({
      timeout: 30_000,
    });
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+End");
    await page.keyboard.insertText("// buffer edit during preview\n");
    await expect(page.getByTestId("editor")).toContainText("buffer edit during preview");
    await page.getByTestId("rename-apply").click();
    await expect(page.getByTestId("rename-stale")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("rename-stale")).toContainText(
      "changed after the preview",
    );
    await expect(page.getByTestId("rename-preview")).toBeVisible();
    await expect(page.getByTestId(`tab-${renameFiles.program}`)).toHaveCount(0);
    await expect(page.getByTestId("editor")).toContainText("public sealed class Order");
    expect(fs.readFileSync(library).equals(before[0])).toBe(true);
    expect(fs.readFileSync(program).equals(before[1])).toBe(true);
    expect(fs.readFileSync(tests).equals(before[2])).toBe(true);
    await page.getByTestId("rename-cancel-preview").click();

    // A cancelled request whose answer arrives late never opens a preview.
    await placeCursor(page, 2, 20);
    const paused = await hostState(request);
    stoppedPid = paused.lsp.pid as number;
    process.kill(stoppedPid, "SIGSTOP");
    await page.getByTestId("rename-open").click();
    await page.getByTestId("rename-input").fill("Late");
    await page.getByTestId("rename-submit").click();
    await expect(page.getByTestId("rename-submit")).toContainText(
      "Waiting for csharp-ls",
      { timeout: 5_000 },
    );
    await page.getByTestId("rename-cancel").click();
    await expect(page.getByTestId("editor-lsp-status")).not.toContainText("Rename ");
    resume();
    await page.waitForTimeout(1_500);
    await expect(page.getByTestId("rename-preview")).toHaveCount(0);
    await expect(page.getByTestId("rename-input")).toHaveCount(0);
    await expect(page.getByTestId("editor")).toContainText("public sealed class Order");
    expect(fs.readFileSync(library).equals(before[0])).toBe(true);

    // Revoking trust after the preview blocks the pending set.
    await placeCursor(page, 2, 20);
    await page.getByTestId("rename-open").click();
    await page.getByTestId("rename-input").fill("Purchase");
    await page.getByTestId("rename-submit").click();
    await expect(page.getByTestId("rename-preview")).toBeVisible({
      timeout: 30_000,
    });
    await reveal(page.getByTestId("trust-revoke"));
    await page.getByTestId("trust-revoke").click();
    await expect(page.getByTestId("mode-restricted")).toBeVisible();
    await page.getByTestId("rename-apply").click();
    await expect(page.getByTestId("rename-stale")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("rename-stale")).toContainText(
      "Analysis is not running",
    );
    await expect(page.getByTestId("rename-preview")).toBeVisible();
    await expect(page.getByTestId(`tab-${renameFiles.program}`)).toHaveCount(0);
    await expect(page.getByTestId(`tab-${renameFiles.tests}`)).toHaveCount(0);
    await expect(page.getByTestId("editor")).toContainText("public sealed class Order");
    [library, program, tests].forEach((file, index) => {
      expect(fs.readFileSync(file).equals(before[index])).toBe(true);
    });
  } finally {
    resume();
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("code actions are listed from csharp-ls and preview/apply without writing disk", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk actions");
  const library = path.join(copy, renameFiles.library);
  const beforeDisk = sourceHashes(copy);
  try {
    await startRenameSession(page, request, copy);
    await placeCursor(page, 2, 20);
    await page.keyboard.press("Alt+Enter");
    await expect(page.getByTestId("code-actions")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("code-actions-summary")).toContainText(
      "from csharp-ls",
    );
    const panel = page.getByTestId("code-actions");
    const editorPane = page.getByRole("region", { name: "Editor" });
    const panelBox = (await panel.boundingBox())!;
    const paneBox = (await editorPane.boundingBox())!;
    expect(panelBox.x).toBeGreaterThanOrEqual(paneBox.x);
    expect(panelBox.y).toBeGreaterThanOrEqual(paneBox.y);
    expect(panelBox.x + panelBox.width).toBeLessThanOrEqual(paneBox.x + paneBox.width);
    expect(panelBox.y + panelBox.height).toBeLessThanOrEqual(paneBox.y + paneBox.height);
    const discovered = await panel
      .locator("button[data-title]")
      .evaluateAll((nodes) =>
        nodes.map((node) => ({
          title: node.getAttribute("data-title"),
          supported: node.getAttribute("data-supported"),
          disabled: node.getAttribute("data-disabled"),
          detail: node.parentElement?.textContent ?? "",
        })),
      );
    fs.mkdirSync(logs, { recursive: true });
    fs.writeFileSync(
      path.join(logs, "10-csharp-ls-actions.json"),
      JSON.stringify(
        {
          fixture: "OrderDesk",
          path: renameFiles.library,
          line: 2,
          character: 20,
          session: await panel.getAttribute("data-session"),
          revision: await panel.getAttribute("data-revision"),
          actions: discovered,
        },
        null,
        2,
      ),
    );
    const actionState = await hostState(request);
    fs.writeFileSync(path.join(logs, "10-lsp.log"), actionState.lsp.log);
    expect(discovered.length).toBeGreaterThan(0);
    expect(
      discovered.some(
        (action) =>
          action.title === "Convert to positional record" &&
          action.supported === "true",
      ),
      JSON.stringify(discovered),
    ).toBe(true);
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({
      path: path.join(screenshots, "10-code-actions-list.png"),
      fullPage: false,
    });

    await page.setViewportSize({ width: 1100, height: 700 });
    await expect(panel).toBeVisible();
    await expect(page.getByTestId("rename-open")).toHaveAccessibleName(/Rename symbol/);
    await expect(page.getByTestId("code-actions-open")).toHaveAccessibleName(/Code actions/);
    await expect(page.getByTestId("save-file")).toHaveAccessibleName(/Save/);
    for (const testId of ["code-actions-title", "code-actions-close"]) {
      const box = (await page.getByTestId(testId).boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.y).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(1100);
      expect(box.y + box.height).toBeLessThanOrEqual(700);
    }
    await page.setViewportSize({ width: 1440, height: 900 });

    await panel
      .getByRole("button", { name: "Convert to positional record", exact: true })
      .click();
    await expect(page.getByTestId("rename-preview")).toBeVisible();
    await expect(page.getByTestId("rename-preview")).toHaveAttribute(
      "data-origin",
      "action",
    );
    await expect(page.getByTestId("rename-preview")).toHaveAttribute("data-files", "1");
    await expect(page.getByTestId("action-preview-detail")).toContainText(
      "Kind: not provided by csharp-ls",
    );
    await expect(
      page.getByTestId(`rename-diff-${renameFiles.library}`),
    ).toContainText("record Order(string Id, decimal Total);");
    await page.screenshot({
      path: path.join(screenshots, "10-code-actions-preview.png"),
      fullPage: false,
    });
    await page.getByTestId("rename-cancel-preview").click();
    await expect(page.getByTestId("rename-preview")).toHaveCount(0);
    await expect(page.getByTestId("editor")).toContainText("public sealed class Order");
    await expect(page.getByTestId("editor-revision")).toContainText("revision 0");
    expect(sourceHashes(copy)).toEqual(beforeDisk);

    // A position csharp-ls answers with an empty array keeps editing possible and never
    // fabricates a quick fix.
    await placeCursorAtColumnZero(page, 5);
    await page.keyboard.press("Alt+Enter");
    await expect(page.getByTestId("code-actions-empty")).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByTestId("code-actions-empty")).toContainText(
      "No code actions",
    );
    await page.screenshot({
      path: path.join(screenshots, "10-code-actions-empty.png"),
      fullPage: false,
    });
    await page.getByTestId("code-actions-close").click();
    await expect(page.getByTestId("code-actions")).toHaveCount(0);

    // Apply the real action to buffers only.
    await placeCursor(page, 2, 20);
    await page.keyboard.press("Alt+Enter");
    await page
      .getByTestId("code-actions")
      .getByRole("button", { name: "Convert to positional record", exact: true })
      .click();
    await expect(page.getByTestId("rename-preview")).toBeVisible();
    await page.getByTestId("rename-apply").click();
    await expect(page.getByTestId("rename-preview")).toHaveCount(0, {
      timeout: 20_000,
    });
    await expect(page.getByTestId("editor")).toContainText(
      "record Order(string Id, decimal Total);",
    );
    await expect(
      page.getByTestId(`tab-${renameFiles.library}`).getByTestId("dirty-marker"),
    ).toBeVisible();
    await expect(page.getByTestId("editor-revision")).toContainText("revision 1");
    await expect(page.getByTestId("app-log")).toContainText(
      "diagnostics are not marked fixed",
    );
    expect(sourceHashes(copy)).toEqual(beforeDisk);
    expect(fs.readFileSync(library, "utf8")).toContain("public sealed class Order");
    await page.screenshot({
      path: path.join(screenshots, "10-code-actions-applied.png"),
      fullPage: false,
    });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("code action lists and previews are invalidated before any write", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk actions stale");
  const library = path.join(copy, renameFiles.library);
  const originalDisk = fs.readFileSync(library);
  const beforeDisk = sourceHashes(copy);
  let stoppedPid: number | null = null;
  const resume = () => {
    if (stoppedPid !== null) {
      try {
        process.kill(stoppedPid, "SIGCONT");
      } catch {
        // The process already exited.
      }
      stoppedPid = null;
    }
  };
  try {
    await startRenameSession(page, request, copy);
    await placeCursor(page, 2, 20);
    await page.keyboard.press("Alt+Enter");
    await expect(page.getByTestId("code-actions")).toBeVisible({ timeout: 30_000 });

    // Editing the source buffer discards the pending list.
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+End");
    await page.keyboard.insertText("// buffer edit during actions\n");
    await expect(page.getByTestId("code-actions")).toHaveCount(0);

    // A chosen preview also closes on a buffer edit; nothing is applied.
    await placeCursor(page, 2, 20);
    await page.keyboard.press("Alt+Enter");
    await page
      .getByTestId("code-actions")
      .getByRole("button", { name: "Convert to positional record", exact: true })
      .click();
    await expect(page.getByTestId("rename-preview")).toBeVisible();
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+End");
    await page.keyboard.insertText("// second buffer edit\n");
    await expect(page.getByTestId("rename-preview")).toHaveCount(0);
    await expect(page.getByTestId("app-log")).toContainText(
      "Code action preview discarded",
    );
    await expect(page.getByTestId("editor")).not.toContainText(
      "record Order(string Id, decimal Total);",
    );

    // A changed disk file rejects the whole set and keeps the preview.
    await placeCursor(page, 2, 20);
    await page.keyboard.press("Alt+Enter");
    await page
      .getByTestId("code-actions")
      .getByRole("button", { name: "Convert to positional record", exact: true })
      .click();
    await expect(page.getByTestId("rename-preview")).toBeVisible();
    fs.appendFileSync(library, "// external edit during action preview\n");
    await page.getByTestId("rename-apply").click();
    await expect(page.getByTestId("rename-stale")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("rename-stale")).toContainText("changed on disk");
    await expect(page.getByTestId("rename-preview")).toBeVisible();
    await expect(page.getByTestId("editor")).not.toContainText(
      "record Order(string Id, decimal Total);",
    );
    await page.screenshot({
      path: path.join(screenshots, "10-code-actions-stale.png"),
      fullPage: false,
    });
    await page.getByTestId("rename-cancel-preview").click();
    fs.writeFileSync(library, originalDisk);
    await expect(page.getByTestId("rename-preview")).toHaveCount(0);

    // A late answer whose cursor moved is discarded.
    const paused = await hostState(request);
    stoppedPid = paused.lsp.pid as number;
    process.kill(stoppedPid, "SIGSTOP");
    await placeCursor(page, 2, 20);
    await page.keyboard.press("Alt+Enter");
    await page.waitForTimeout(500);
    await page.keyboard.press("ArrowDown");
    resume();
    await page.waitForTimeout(2_000);
    await expect(page.getByTestId("code-actions")).toHaveCount(0);
    await expect(page.getByTestId("app-log")).toContainText("Code actions discarded");

    // Revoking trust discards a preview before it can be applied.
    await placeCursor(page, 2, 20);
    await page.keyboard.press("Alt+Enter");
    await page
      .getByTestId("code-actions")
      .getByRole("button", { name: "Convert to positional record", exact: true })
      .click();
    await expect(page.getByTestId("rename-preview")).toBeVisible();
    await reveal(page.getByTestId("trust-revoke"));
    await page.getByTestId("trust-revoke").click();
    await expect(page.getByTestId("mode-restricted")).toBeVisible();
    await expect(page.getByTestId("rename-preview")).toHaveCount(0);
    await expect(page.getByTestId("editor")).not.toContainText(
      "record Order(string Id, decimal Total);",
    );
    expect(sourceHashes(copy)).toEqual(beforeDisk);
  } finally {
    resume();
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("code actions forward the current diagnostic and list the real quick fix", async ({
  page,
  request,
}) => {
  const { base, copy } = tempCopy("Order Desk actions diagnostic");
  try {
    await startRenameSession(page, request, copy);
    await page.getByTestId(`tree-file-${renameFiles.program}`).click();
    await expect(page.getByTestId("editor")).toContainText("OrderDesk.Core");
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+End");
    await page.keyboard.insertText("\nvar broken = new MissingType();\n");
    await expect(page.getByTestId("editor")).toContainText("MissingType");
    await expect
      .poll(
        async () => {
          const state = await hostState(request);
          return state.diagnostics.snapshots.some(
            (snapshot: { state: string; items: { code: string }[] }) =>
              snapshot.state === "current" &&
              snapshot.items.some((item) => item.code === "CS0246"),
          );
        },
        { timeout: 30_000, intervals: [100, 250, 500] },
      )
      .toBe(true);

    // The cursor is just after the inserted line; move inside `MissingType` without
    // assuming a line number after the edit.
    for (let index = 0; index < 4; index += 1) {
      await page.keyboard.press("ArrowLeft");
    }
    await page.keyboard.press("Alt+Enter");
    await expect(page.getByTestId("code-actions")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("code-actions-summary")).toContainText(
      "1 current diagnostic",
    );
    await expect(
      page
        .getByTestId("code-actions")
        .getByRole("button", { name: /Generate type 'MissingType'/ })
        .first(),
    ).toBeVisible();
    const diagnosticState = await hostState(request);
    const cursor = (await page.getByTestId("editor-shell").getAttribute("data-cursor")) ??
      "0:0";
    const [cursorLine, cursorCharacter] = cursor.split(":").map(Number);
    fs.mkdirSync(logs, { recursive: true });
    fs.writeFileSync(path.join(logs, "10-lsp-diagnostic.log"), diagnosticState.lsp.log);
    fs.writeFileSync(
      path.join(logs, "10-code-actions-diagnostic-actions.json"),
      JSON.stringify(
        {
          path: renameFiles.program,
          line: cursorLine,
          character: cursorCharacter,
          diagnosticsUsed: 1,
          actions: await page
            .getByTestId("code-actions")
            .locator("button[data-title]")
            .evaluateAll((nodes) =>
              nodes.map((node) => ({
                title: node.getAttribute("data-title"),
                supported: node.getAttribute("data-supported"),
                reason: node.parentElement?.textContent ?? "",
              })),
            ),
        },
        null,
        2,
      ),
    );
    fs.mkdirSync(screenshots, { recursive: true });
    await page.screenshot({
      path: path.join(screenshots, "10-code-actions-diagnostic.png"),
      fullPage: false,
    });
    await page.getByTestId("code-actions-close").click();
    await expect(page.getByTestId("code-actions")).toHaveCount(0);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
