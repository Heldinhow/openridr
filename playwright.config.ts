import { defineConfig } from "@playwright/test";
import { e2eHome, frontend, host, hostPort } from "./tests/e2e/environment";

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: false,
  workers: 1,
  timeout: 30_000,
  expect: { timeout: 10_000 },
  reporter: [["list"]],
  use: {
    baseURL: frontend,
    // The app opens with the bottom tool window closed (Rider); the specs were written against it open.
    storageState: {
      cookies: [],
      origins: [{
        origin: new URL(frontend).origin,
        localStorage: [{
          name: "openridr.layout.v1",
          value: JSON.stringify({ bottom: { active: "build", size: 220, open: true } }),
        }],
      }],
    },
    // Chrome by default; set PLAYWRIGHT_CHANNEL=chromium where Chrome is not installed.
    channel: process.env.PLAYWRIGHT_CHANNEL || "chrome",
    launchOptions: {
      slowMo: Number(process.env.OPENRIDR_E2E_SLOW_MO ?? 0),
    },
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  webServer: [
    {
      command: `cargo run -p openridr-host-http -- --port ${hostPort}`,
      url: `${host}/health`,
      timeout: 300_000,
      reuseExistingServer: false,
      // Dev/test-only probe used by the process-boundary E2E. Never set in the desktop app.
      // OPENRIDR_TRUST_STORE=off: specs reopen the same fixture and expect Restricted mode each time.
      env: { OPENRIDR_ENABLE_PROBE: "1", OPENRIDR_HOME: e2eHome, OPENRIDR_TRUST_STORE: "off" },
    },
    {
      command: "bun run dev --config vite.e2e.config.ts",
      url: frontend,
      timeout: 120_000,
      reuseExistingServer: false,
      env: { VITE_HOST_URL: "/__host" },
    },
  ],
});
