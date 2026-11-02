import { expect, test, type Page } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { hostPort } from "./environment";

// The shared E2E host runs with OPENRIDR_SESSION_STORE=off so specs cannot restore each other's
// tabs. Session persistence needs its own host, like trust-prompt.spec.ts does for trust.
const port = hostPort + 3;
const fixture = path.resolve("fixtures/OrderDesk");
const program = "src/OrderDesk.Cli/Program.cs";
const order = "src/OrderDesk.Core/Order.cs";
let home: string;
let server: ChildProcess;

test.describe.configure({ mode: "serial", timeout: 180_000 });

test.beforeAll(async () => {
  home = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-session-home-"));
  server = spawn("cargo", ["run", "-p", "openridr-host-http", "--", "--port", String(port)], {
    env: { ...process.env, OPENRIDR_HOME: home, OPENRIDR_TRUST_STORE: "on" },
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
  fs.rmSync(home, { recursive: true, force: true });
});

async function useIsolatedHost(page: Page) {
  await page.route(/\/__host\/(call|health)$/, async (route) => {
    const target = `http://127.0.0.1:${port}/${route.request().url().split("/").pop()}`;
    await route.fetch({ url: target }).then((response) => route.fulfill({ response })).catch(() => {});
  });
}

function projectCopy() {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-editor-session-"));
  const copy = path.join(base, "Order Desk session");
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return { base, solution: path.join(copy, "OrderDesk.sln") };
}

async function hostCall(page: Page, method: string, params: object = {}) {
  return page.evaluate(async ({ method, params }) => {
    const response = await fetch("/__host/call", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ method, params }),
    });
    return response.json();
  }, { method, params });
}

async function openWorkspace(page: Page, solution: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  const dialog = page.getByTestId("trust-prompt-dialog");
  await dialog.or(page.getByTestId("tree")).first().waitFor();
  if (await dialog.isVisible()) await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await expect(page.getByTestId("tree")).toBeVisible();
}

test("d9: reopening a solution restores tabs, cursor and breakpoints", async ({ page }) => {
  const { base, solution } = projectCopy();
  try {
    await useIsolatedHost(page);
    await openWorkspace(page, solution);
    await page.getByTestId(`tree-file-${program}`).click();
    await expect(page.getByTestId(`tab-${program}`)).toBeVisible();
    await page.getByTestId(`tree-file-${order}`).click();
    await expect(page.getByTestId(`tab-${order}`)).toBeVisible();
    await page.getByTestId(`tab-${program}`).click();

    const line = page.locator(".cm-line").nth(2);
    await line.click();
    const lineBox = (await line.boundingBox())!;
    const gutterBox = (await page.locator(".cm-debug-gutter").boundingBox())!;
    await page.mouse.click(gutterBox.x + 5, lineBox.y + lineBox.height / 2);
    await expect(page.locator(".cm-line-breakpoint")).toHaveCount(1);

    // The store is the source of truth; wait for the debounced write before reopening.
    await expect
      .poll(async () => {
        const answer = await hostCall(page, "runConfigs.read", { solution });
        const tab = answer.data?.session?.tabs?.[0];
        return tab?.line === 2 && answer.data?.session?.breakpoints?.[program]?.[0] === 3;
      })
      .toBe(true);

    await hostCall(page, "workspace.close");
    await expect(page.getByTestId("workspace-path-input")).toBeVisible({ timeout: 15_000 });
    await openWorkspace(page, solution);

    await expect(page.getByTestId(`tab-${program}`)).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId("status-cursor")).toContainText("Ln 3");
    await expect(page.locator(".cm-line-breakpoint")).toHaveCount(1);

    // The restored target is one-shot: a tab round trip must not force the old cursor again.
    await page.locator(".cm-line").nth(3).click();
    await expect(page.getByTestId("status-cursor")).toContainText("Ln 4");
    await page.getByTestId(`tab-${order}`).click();
    await page.getByTestId(`tab-${program}`).click();
    await expect
      .poll(async () => {
        const answer = await hostCall(page, "runConfigs.read", { solution });
        return answer.data?.session?.tabs?.find((tab: { path: string }) => tab.path === program)?.line;
      })
      .toBe(0);
    await expect(page.getByTestId("status-cursor")).not.toContainText("Ln 3");

    const stored = await hostCall(page, "runConfigs.read", { solution });
    expect(stored.data.session.activePath).toBe(program);
    expect(stored.data.session.breakpoints[program]).toEqual([3]);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("d9: breakpoints saved in Trusted mode return when trust is granted after a Restricted open", async ({ page }) => {
  const { base, solution } = projectCopy();
  try {
    await useIsolatedHost(page);
    await openWorkspace(page, solution);
    await page.getByTestId(`tree-file-${program}`).click();
    const line = page.locator(".cm-line").nth(2);
    await line.click();
    const lineBox = (await line.boundingBox())!;
    const gutterBox = (await page.locator(".cm-debug-gutter").boundingBox())!;
    await page.mouse.click(gutterBox.x + 5, lineBox.y + lineBox.height / 2);
    await expect(page.locator(".cm-line-breakpoint")).toHaveCount(1);
    await expect
      .poll(async () => {
        const answer = await hostCall(page, "runConfigs.read", { solution });
        return answer.data?.session?.breakpoints?.[program]?.[0];
      })
      .toBe(3);

    // Opening in Restricted mode cannot apply breakpoints, so the store must keep them.
    await hostCall(page, "trust.revoke");
    await hostCall(page, "workspace.close");
    await expect(page.getByTestId("workspace-path-input")).toBeVisible({ timeout: 15_000 });
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(solution);
    await page.getByTestId("workspace-open").click();
    const dialog = page.getByTestId("trust-prompt-dialog");
    await dialog.or(page.getByTestId("tree")).first().waitFor();
    if (await dialog.isVisible()) await page.getByTestId("trust-prompt-restricted").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");
    await expect(page.getByTestId(`tab-${program}`)).toBeVisible({ timeout: 15_000 });
    await expect(page.locator(".cm-line-breakpoint")).toHaveCount(0);

    await page.getByTestId("status-mode").click();
    await page.getByTestId("trust-checkbox").check();
    await page.getByTestId("trust-grant").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    await expect(page.locator(".cm-line-breakpoint")).toHaveCount(1);

    const stored = await hostCall(page, "runConfigs.read", { solution });
    expect(stored.data.session.breakpoints[program]).toEqual([3]);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
