import { expect, test, type Page } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { revisionOf, settleRestore } from "./automatic";
import { hostPort, seedVerifiedTools } from "./environment";

/**
 * Issue #47: hit count (counted by the host, because netcoredbg ignores `hitCondition`) and
 * watches. The shared E2E host ships the fake bundled adapter from `seedVerifiedTools`, so these
 * tests own a host whose bundled adapter is the operator-installed netcoredbg: the same DAP
 * session a user gets, without a download.
 */
const port = hostPort + 4;
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const realAdapter = path.join(
  process.env.HOME ?? "",
  "Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092",
);
let home: string;
let server: ChildProcess;

test.describe.configure({ mode: "serial", timeout: 180_000 });
test.use({ viewport: { width: 1440, height: 900 } });

test.beforeAll(async () => {
  home = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug-advanced-home-"));
  const bundled = seedVerifiedTools(home);
  // The seed writes a script that only prints a version; replace it with the real adapter.
  fs.rmSync(path.join(bundled, "netcoredbg"), { recursive: true, force: true });
  if (fs.existsSync(realAdapter)) {
    fs.cpSync(path.join(realAdapter, "netcoredbg"), path.join(bundled, "netcoredbg"), {
      recursive: true,
    });
  }
  server = spawn("cargo", ["run", "-p", "openridr-host-http", "--", "--port", String(port)], {
    env: {
      ...process.env,
      OPENRIDR_HOME: home,
      OPENRIDR_TRUST_STORE: "on",
      OPENRIDR_ENABLE_PROBE: "1",
      OPENRIDR_BUNDLED_NETCOREDBG: bundled,
    },
    stdio: "ignore",
  });
  await expect
    .poll(async () => fetch(`http://127.0.0.1:${port}/health`).then((r) => r.ok, () => false), {
      timeout: 300_000,
    })
    .toBe(true);
});

test.afterAll(() => {
  server?.kill();
  if (home) fs.rmSync(home, { recursive: true, force: true });
});

async function useIsolatedHost(page: Page) {
  await page.route(/\/__host\/(call|health)$/, async (route) => {
    const target = `http://127.0.0.1:${port}/${route.request().url().split("/").pop()}`;
    await route
      .fetch({ url: target })
      .then((response) => route.fulfill({ response }))
      .catch(() => {});
  });
}

async function hostCall(method: string, params: object = {}) {
  const response = await fetch(`http://127.0.0.1:${port}/call`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ method, params }),
  });
  return response.json();
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

const loopSource = [
  "var total = 0;",
  "for (var index = 0; index < 5; index++)",
  "{",
  "    total += index;",
  "}",
  "System.Console.WriteLine(total);",
  "",
].join("\n");

/** Copies the fixture, arms a breakpoint on line 4, opens it trusted and builds it. */
async function prepare(page: Page, base: string, source: string, breakpointLine = 4) {
  const copy = path.join(base, "Order Desk advanced");
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  const program = path.join(copy, "src/OrderDesk.Cli/Program.cs");
  fs.writeFileSync(program, source);

  await useIsolatedHost(page);
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  const dialog = page.getByTestId("trust-prompt-dialog");
  await dialog.or(page.getByTestId("tree")).first().waitFor();
  if (await dialog.isVisible()) await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });

  await settleRestore(page);
  await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(2);
  await selectEvaluationTfm(page, "net8.0");
  await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(3);

  const breakpoint = await hostCall("debug.setBreakpoints", { path: program, lines: [breakpointLine] });
  expect(breakpoint.ok, JSON.stringify(breakpoint.error)).toBe(true);
  await page.getByTestId("bottom-tab-build").click();
  await page.getByTestId("build-action").click();
  await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
    timeout: 60_000,
  });
}

async function newBase(name: string) {
  return fs.mkdtempSync(path.join(os.tmpdir(), `openridr-debug-advanced-${name}-`));
}

test.beforeEach(() => {
  test.skip(!fs.existsSync(realAdapter), "the real netcoredbg install is not present");
});

test("a hit count stops on the nth hit and lets the run continue", async ({ page }) => {
  const base = await newBase("hit");
  try {
    await prepare(page, base, loopSource);
    await page.getByTestId("bottom-tab-debug").click();
    const hit = page.locator('input[data-testid^="debug-breakpoint-hit-"]');
    await expect(hit).toHaveCount(1);
    await hit.fill("3");
    await page.locator('button[data-testid^="debug-breakpoint-hit-save-"]').click();
    await expect(hit).toHaveValue("3");

    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });
    // The third pass of the loop has index 2; the host counted the two earlier stops itself.
    await page.getByTestId("debug-evaluate-input").fill("index");
    await page.getByTestId("debug-evaluate-run").click();
    await expect(page.getByTestId("debug-evaluate-result")).toContainText("2");
    const log = page.getByTestId("dap-log");
    await expect(log).toContainText("hit count 1/3");
    await expect(log).toContainText("hit count 2/3");

    // Past the target the breakpoint no longer stops: the rest of the loop runs to the end.
    await page.getByTestId("debug-continue").click();
    await expect(page.getByTestId("dap-state")).toContainText("Terminated", { timeout: 60_000 });
    await expect(page.getByTestId("dap-output")).toContainText("10");
  } finally {
    await hostCall("debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a hit count larger than the hits lets the program finish", async ({ page }) => {
  const base = await newBase("hit-high");
  try {
    await prepare(page, base, loopSource);
    await page.getByTestId("bottom-tab-debug").click();
    const hit = page.locator('input[data-testid^="debug-breakpoint-hit-"]');
    await hit.fill("10");
    await page.locator('button[data-testid^="debug-breakpoint-hit-save-"]').click();
    await expect(hit).toHaveValue("10");

    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Terminated", { timeout: 60_000 });
    await expect(page.getByTestId("dap-state")).not.toContainText("Paused");
    await expect(page.getByTestId("dap-output")).toContainText("10");
  } finally {
    await hostCall("debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a condition and a hit count count only the condition's hits", async ({ page }) => {
  const base = await newBase("condition");
  try {
    await prepare(page, base, loopSource);
    await page.getByTestId("bottom-tab-debug").click();
    const condition = page.locator('input[data-testid^="debug-breakpoint-condition-"]');
    await condition.fill("index >= 1");
    await page.locator('button[data-testid^="debug-breakpoint-condition-save-"]').click();
    await expect(condition).toHaveValue("index >= 1");
    const hit = page.locator('input[data-testid^="debug-breakpoint-hit-"]');
    await hit.fill("2");
    await page.locator('button[data-testid^="debug-breakpoint-hit-save-"]').click();
    await expect(hit).toHaveValue("2");

    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });
    // Hits that pass the condition are index 1 (first) and index 2 (second).
    await page.getByTestId("debug-evaluate-input").fill("index");
    await page.getByTestId("debug-evaluate-run").click();
    await expect(page.getByTestId("debug-evaluate-result")).toContainText("2");
  } finally {
    await hostCall("debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("an invalid hit count is refused in the panel", async ({ page }) => {
  const base = await newBase("invalid");
  try {
    await prepare(page, base, loopSource);
    await page.getByTestId("bottom-tab-debug").click();
    const hit = page.locator('input[data-testid^="debug-breakpoint-hit-"]');
    await hit.fill("1.5");
    await page.locator('button[data-testid^="debug-breakpoint-hit-save-"]').click();
    await expect(page.getByTestId("app-log")).toContainText("positive whole number");
    await expect(hit).toHaveValue("1.5");
  } finally {
    await hostCall("debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

const threadedSource = [
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
].join("\n");

async function pausedThreads() {
  const state = await hostCall("debug.state");
  return state.data.dap as {
    threads: { id: number; name: string }[];
    selectedThreadId: number | null;
  };
}

test("a watch survives a restart with the same expressions", async ({ page }) => {
  const base = await newBase("watch-restart");
  try {
    await prepare(page, base, loopSource);
    await page.getByTestId("bottom-tab-debug").click();
    await page.getByTestId("debug-watch-input").fill("index");
    await page.getByTestId("debug-watch-add").click();
    await expect(page.getByTestId("debug-watch-0")).toContainText("index");

    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });
    await expect(page.getByTestId("debug-watch-0")).toContainText("0");
    const before = (await hostCall("debug.state")).data.dap.session as number;

    // Arm a hit count so the restarted session stops on a different iteration: a stale pause
    // from the old session would still show 0.
    const hit = page.locator('input[data-testid^="debug-breakpoint-hit-"]');
    await hit.fill("3");
    await page.locator('button[data-testid^="debug-breakpoint-hit-save-"]').click();
    await expect(hit).toHaveValue("3");

    await page.getByTestId("debug-restart").click();
    await expect
      .poll(async () => {
        const state = await hostCall("debug.state");
        return [state.data.dap.session as number, state.data.dap.state as string];
      }, { timeout: 90_000 })
      .toEqual([expect.any(Number), "paused"]);
    const after = (await hostCall("debug.state")).data.dap;
    expect(after.session as number).toBeGreaterThan(before);

    // The same single watch is still there and evaluates against the new pause (index 2).
    await expect(page.locator('[data-testid^="debug-watch-"]').filter({ hasText: "index" })).toHaveCount(1);
    await expect(page.getByTestId("debug-watch-0")).toContainText("index");
    await expect(page.getByTestId("debug-watch-0").locator(".warning")).toHaveCount(0);
    await expect(page.getByTestId("debug-watch-0")).toContainText("2");
  } finally {
    await hostCall("debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a watch follows the selected thread", async ({ page }) => {
  const base = await newBase("watch-thread");
  try {
    await prepare(page, base, threadedSource, 11);
    await page.getByTestId("bottom-tab-debug").click();
    await page.getByTestId("debug-watch-input").fill("label");
    await page.getByTestId("debug-watch-add").click();

    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("dap-state")).toContainText("Paused", { timeout: 60_000 });
    await expect(page.getByTestId("debug-watch-0")).toContainText("worker");

    const paused = await pausedThreads();
    const main = paused.threads.find((thread) => thread.id !== paused.selectedThreadId);
    expect(main, JSON.stringify(paused.threads)).toBeTruthy();
    await page.getByTestId(`debug-thread-${main!.id}`).click();
    await expect(page.getByTestId("debug-watch-0").locator(".warning")).toBeVisible({
      timeout: 30_000,
    });

    await page.getByTestId(`debug-thread-${paused.selectedThreadId}`).click();
    await expect(page.getByTestId("debug-watch-0")).toContainText("worker");
    await expect(page.getByTestId("debug-watch-0").locator(".warning")).toHaveCount(0);
  } finally {
    await hostCall("debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a condition that never holds lets the program finish", async ({ page }) => {
  const base = await newBase("condition-off");
  try {
    await prepare(page, base, loopSource);
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
    await hostCall("debug.stop");
    fs.rmSync(base, { recursive: true, force: true });
  }
});
