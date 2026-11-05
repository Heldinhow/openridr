import { expect, test, type Page } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { hostPort } from "./environment";

// A fresh app home proves the missing-analysis gate: nothing is installed here, and the plan the
// notice asks for is inert until the explicit confirmation in the tools panel.
const port = hostPort + 6;
const base = `http://127.0.0.1:${port}`;
const fixture = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
let home: string;
let server: ChildProcess;

test.describe.configure({ mode: "serial", timeout: 240_000 });

test.beforeAll(async () => {
  home = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-analysis-gate-"));
  server = spawn("cargo", ["run", "-p", "openridr-host-http", "--", "--port", String(port)], {
    env: { ...process.env, OPENRIDR_HOME: home, OPENRIDR_TRUST_STORE: "on" },
    stdio: "ignore",
  });
  await expect
    .poll(async () => fetch(`${base}/health`).then((response) => response.ok, () => false), {
      timeout: 300_000,
    })
    .toBe(true);
});

test.afterAll(() => {
  server?.kill();
  fs.rmSync(home, { recursive: true, force: true });
});

test.beforeEach(async ({ request }) => {
  await request.post(`${base}/call`, { data: { method: "workspace.close", params: {} } }).catch(() => {});
});

test.afterEach(async ({ request }) => {
  await request.post(`${base}/call`, { data: { method: "lsp.stop", params: {} } }).catch(() => {});
  await request.post(`${base}/call`, { data: { method: "workspace.close", params: {} } }).catch(() => {});
});

async function useIsolatedHost(page: Page) {
  await page.route(/\/__host\/(call|health)$/, async (route) => {
    const target = `${base}/${route.request().url().split("/").pop()}`;
    await route.fetch({ url: target }).then((response) => route.fulfill({ response })).catch(() => {});
  });
}

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
    await useIsolatedHost(page);
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

test("Find Usages, Rename and code actions share the actionable install gate", async ({
  page,
  request,
}) => {
  const copy = tempSolution();
  try {
    await useIsolatedHost(page);
    await openWorkspace(page, copy.solution);
    await trustAndEnter(page);
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await expect(page.getByTestId("editor")).toContainText("OrderDesk.Core");

    for (const [keys, label] of [
      ["Shift+F12", "Find Usages unavailable:"],
      ["Shift+F6", "Rename unavailable:"],
      ["Alt+Enter", "Code actions unavailable:"],
    ] as const) {
      await page.getByTestId("editor").click();
      await page.keyboard.press(keys);
      await expect(page.getByTestId("app-log")).toContainText(label);
      await expect(page.getByTestId("notice-action")).toHaveText("Install analysis");
    }

    // The action opens the tools panel and requests the exact plan; it never confirms or downloads.
    await page.getByTestId("editor").click();
    await page.keyboard.press("Shift+F12");
    await page.getByTestId("notice-action").click();
    await expect(page.getByTestId("workspace-tools")).toBeVisible();
    await expect(page.getByTestId("install-plan")).toBeVisible();
    await expect(page.getByTestId("install-state")).toHaveText("Plan ready — nothing downloaded yet");
    await expect(page.getByTestId("install-confirm")).toBeVisible();
    await expect(page.getByTestId("install-progress")).toHaveCount(0);

    const response = await request.post(`${base}/call`, {
      data: { method: "workspace.state", params: {} },
    });
    const tool = (await response.json()).data.tools["csharp-ls"];
    expect(tool.state).toBe("planned");
    expect(tool.processId).toBeNull();
    expect(tool.progress).toBeNull();
  } finally {
    fs.rmSync(copy.root, { recursive: true, force: true });
  }
});
