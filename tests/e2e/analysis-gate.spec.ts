import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { seedVerifiedTools } from "./environment";
import { startIsolatedHost, type IsolatedHost } from "./isolated-host";

// A fresh app home proves the missing-analysis gate: nothing is installed here, and the plan the
// notice asks for is inert until the explicit confirmation in the tools panel.
const fixture = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
let host: IsolatedHost;

test.describe.configure({ mode: "serial", timeout: 240_000 });

test.beforeAll(async () => {
  host = await startIsolatedHost({
    homePrefix: "openridr-analysis-gate-",
    env: { OPENRIDR_TRUST_STORE: "on" },
    prepare: (home) => ({ OPENRIDR_BUNDLED_NETCOREDBG: seedVerifiedTools(home) }),
  });
});

test.afterAll(async () => {
  await host?.stop();
});

test.beforeEach(async ({ request }) => {
  await request.post(`${host.base}/call`, { data: { method: "workspace.close", params: {} } }).catch(() => {});
});

test.afterEach(async ({ request }) => {
  await request.post(`${host.base}/call`, { data: { method: "lsp.stop", params: {} } }).catch(() => {});
  await request.post(`${host.base}/call`, { data: { method: "workspace.close", params: {} } }).catch(() => {});
});

async function openWorkspace(page: Page, solution: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
}

function tempSolution() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-analysis-gate-"));
  const copy = path.join(root, "OrderDesk");
  fs.cpSync(path.dirname(fixture), copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return { root, solution: path.join(copy, "OrderDesk.sln") };
}

async function trustAndEnter(page: Page) {
  const prompt = page.getByTestId("trust-prompt-dialog");
  await prompt.or(page.getByTestId("tree")).first().waitFor();
  if (await prompt.isVisible()) await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

test("Restricted mode keeps the trust gate and offers no analysis install", async ({ page }) => {
  const copy = tempSolution();
  try {
    await host.useFor(page);
    await openWorkspace(page, copy.solution);
    await page.getByTestId("trust-prompt-restricted").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();

    await page.getByTestId("editor").click();
    await page.keyboard.press("Shift+F12");
    await expect(page.getByTestId("app-log")).toContainText(
      "Find Usages unavailable: Trust the workspace before finding usages",
    );
    await expect(page.getByTestId("notice-action")).toHaveCount(0);
  } finally {
    fs.rmSync(copy.root, { recursive: true, force: true });
  }
});

test("the tool drawer is diagnosis, not an install gate", async ({ page }) => {
  const copy = tempSolution();
  try {
    await host.useFor(page);
    await openWorkspace(page, copy.solution);
    await trustAndEnter(page);
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    // Setup completed before the trust prompt; the workspace never opened half-installed.
    await expect(page.getByTestId("setup-gate")).toHaveCount(0);
    await page.getByTestId("workspace-tools-toggle").click();
    await expect(page.getByTestId("install-panel")).toBeVisible();
    await expect(page.getByTestId("install-state")).toHaveText("Ready");
    await expect(page.getByTestId("install-plan-open")).toHaveCount(0);
    await expect(page.getByTestId("install-confirm")).toHaveCount(0);
    await expect(page.getByTestId("install-reinstall")).toBeVisible();

    // Restricted mode keeps the trust gate; it never offers an analysis install instead.
    await page.getByTestId("status-mode").click();
    await page.getByTestId("trust-revoke").click();
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await page.getByTestId("editor").click();
    await page.keyboard.press("Shift+F12");
    await expect(page.getByTestId("app-log")).toContainText(
      "Find Usages unavailable: Trust the workspace before finding usages",
    );
    await expect(page.getByTestId("notice-action")).toHaveCount(0);
  } finally {
    fs.rmSync(copy.root, { recursive: true, force: true });
  }
});
