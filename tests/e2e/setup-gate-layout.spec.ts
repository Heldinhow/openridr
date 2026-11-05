import { expect, test, type Page } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { hostPort } from "./environment";

// The old install-plan layout spec was removed with the plan UI. The setup gate is the
// replacement surface and keeps the same layout guarantee at both supported widths.
const port = hostPort + 7;
const fixture = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
let home: string;
let server: ChildProcess;

test.describe.configure({ mode: "serial", timeout: 180_000 });

test.beforeAll(async () => {
  home = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-gate-layout-"));
  const installer = path.join(home, "failed-installer");
  fs.writeFileSync(installer, "#!/bin/sh\nsleep 1\necho 'offline' >&2\nexit 1\n");
  fs.chmodSync(installer, 0o755);
  const bundled = path.join(home, "bundled");
  fs.mkdirSync(bundled, { recursive: true });
  const adapter = path.join(bundled, "netcoredbg");
  fs.writeFileSync(adapter, "#!/bin/sh\nprintf 'netcoredbg 3.2.0-1092\\n'\n");
  fs.chmodSync(adapter, 0o755);
  server = spawn("cargo", ["run", "-p", "openridr-host-http", "--", "--port", String(port)], {
    env: {
      ...process.env,
      OPENRIDR_HOME: home,
      OPENRIDR_ENABLE_PROBE: "1",
      OPENRIDR_TOOLS_MUXER: installer,
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
  fs.rmSync(home, { recursive: true, force: true });
});

async function useIsolatedHost(page: Page) {
  await page.route(/\/__host\/(call|health)$/, async (route) => {
    const target = `http://127.0.0.1:${port}/${route.request().url().split("/").pop()}`;
    await route.fetch({ url: target }).then((response) => route.fulfill({ response })).catch(() => {});
  });
}

for (const viewport of [{ width: 1440, height: 900 }, { width: 1100, height: 700 }]) {
  test(`setup gate has no horizontal overflow at ${viewport.width}x${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await useIsolatedHost(page);
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(fixture);
    await page.getByTestId("workspace-open").click();
    await expect(page.getByTestId("setup-gate")).toBeVisible();
    await expect(page.getByTestId("setup-gate-error")).toBeVisible({ timeout: 30_000 });
    const body = page.getByTestId("setup-gate");
    const metrics = await body.evaluate((element) => ({
      scrollWidth: element.scrollWidth,
      clientWidth: element.clientWidth,
    }));
    expect(metrics.scrollWidth).toBeLessThanOrEqual(metrics.clientWidth + 1);
    await expect(page.getByTestId("setup-gate-retry")).toBeVisible();
    await expect(page.getByTestId("setup-gate-cancel")).toBeVisible();
    await expect(page.getByTestId("setup-gate-runtime-link")).toHaveCount(0);
    await page.getByTestId("setup-gate-cancel").click();
    await expect(page.getByTestId("welcome-projects")).toBeVisible();
  });
}
