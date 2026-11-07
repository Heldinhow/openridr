import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { hostPort } from "./environment";
import { startIsolatedHost, type IsolatedHost } from "./isolated-host";

// The old install-plan layout spec was removed with the plan UI. The setup gate is the
// replacement surface and keeps the same layout guarantee at both supported widths.
const port = hostPort + 7;
const fixture = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
let host: IsolatedHost;

test.describe.configure({ mode: "serial", timeout: 180_000 });

test.beforeAll(async () => {
  host = await startIsolatedHost({
    port,
    homePrefix: "openridr-gate-layout-",
    env: { OPENRIDR_ENABLE_PROBE: "1" },
    prepare: (home) => {
      const installer = path.join(home, "failed-installer");
      fs.writeFileSync(installer, "#!/bin/sh\nsleep 1\necho 'offline' >&2\nexit 1\n");
      fs.chmodSync(installer, 0o755);
      const bundled = path.join(home, "bundled");
      fs.mkdirSync(bundled, { recursive: true });
      const adapter = path.join(bundled, "netcoredbg");
      fs.writeFileSync(adapter, "#!/bin/sh\nprintf 'netcoredbg 3.2.0-1092\\n'\n");
      fs.chmodSync(adapter, 0o755);
      return { OPENRIDR_TOOLS_MUXER: installer, OPENRIDR_BUNDLED_NETCOREDBG: bundled };
    },
  });
});

test.afterAll(async () => {
  await host?.stop();
});

for (const viewport of [{ width: 1440, height: 900 }, { width: 1100, height: 700 }]) {
  test(`setup gate has no horizontal overflow at ${viewport.width}x${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await host.useFor(page);
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
