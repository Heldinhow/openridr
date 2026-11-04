import { expect, test, type Page } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { hostPort } from "./environment";

// The shared E2E host talks to the real release hosts. This spec starts its own host with the
// test-only probe seam: a loopback asset URL that always answers HTTP 500, so the download
// failure and the retry button are deterministic and never touch the network.
const port = hostPort + 4;
const fixture = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
let home: string;
let server: ChildProcess;
let assets: http.Server;
let requests = 0;

test.describe.configure({ mode: "serial", timeout: 240_000 });

test.beforeAll(async () => {
  home = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-debug-ux-home-"));
  assets = http.createServer((_request, response) => {
    requests += 1;
    response.writeHead(500, { "content-length": "0" });
    response.end();
  });
  await new Promise<void>((resolve) => assets.listen(0, "127.0.0.1", resolve));
  const address = assets.address();
  if (address === null || typeof address === "string") throw new Error("no asset port");
  server = spawn("cargo", ["run", "-p", "openridr-host-http", "--", "--port", String(port)], {
    env: {
      ...process.env,
      OPENRIDR_HOME: home,
      OPENRIDR_TRUST_STORE: "on",
      OPENRIDR_ENABLE_PROBE: "1",
      // Forced platform avoids the macOS arm64 experimental gate; nothing is extracted because
      // every request fails before the body.
      OPENRIDR_DEBUG_PLATFORM: "windows/x86_64",
      OPENRIDR_DEBUG_ASSET_URL: `http://127.0.0.1:${address.port}/netcoredbg.zip`,
      OPENRIDR_DEBUG_ASSET_SHA256: "0".repeat(64),
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
  assets?.close();
  fs.rmSync(home, { recursive: true, force: true });
});

async function useIsolatedHost(page: Page) {
  await page.route(/\/__host\/(call|health)$/, async (route) => {
    const target = `http://127.0.0.1:${port}/${route.request().url().split("/").pop()}`;
    await route.fetch({ url: target }).then((response) => route.fulfill({ response })).catch(() => {});
  });
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

test("a failed download shows the error and a working retry in the Debug tab", async ({
  page,
}) => {
  await useIsolatedHost(page);
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(fixture);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");

  const planned = await hostCall(page, "debug.plan");
  expect(planned.ok, JSON.stringify(planned.error)).toBe(true);
  const planId = planned.data.debug.plan.planId as string;
  const started = await hostCall(page, "debug.install", { planId, confirm: true });
  expect(started.ok, JSON.stringify(started.error)).toBe(true);
  await expect
    .poll(async () => {
      const state = await hostCall(page, "debug.state");
      return state.data.debug.adapterState as string;
    }, { timeout: 30_000 })
    .toBe("failed");

  await page.getByTestId("bottom-tab-debug").click();
  await expect(page.getByTestId("debug-install-status")).toBeVisible();
  await expect(page.getByTestId("debug-install-error")).toContainText("HTTP 500");
  await expect(page.getByTestId("debug-install-retry")).toBeEnabled();
  expect(requests).toBe(1);

  // The retry runs the already confirmed plan again; the same failure is surfaced, not hidden.
  await page.getByTestId("debug-install-retry").click();
  await expect.poll(() => requests, { timeout: 30_000 }).toBe(2);
  await expect
    .poll(async () => {
      const state = await hostCall(page, "debug.state");
      return state.data.debug.adapterState as string;
    }, { timeout: 30_000 })
    .toBe("failed");
  await expect(page.getByTestId("debug-install-error")).toContainText("HTTP 500");
  await expect(page.getByTestId("debug-tab")).toBeVisible();
});
