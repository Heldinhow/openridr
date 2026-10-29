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
    // Chrome by default; set PLAYWRIGHT_CHANNEL=chromium where Chrome is not installed.
    channel: process.env.PLAYWRIGHT_CHANNEL || "chrome",
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
      env: { OPENRIDR_ENABLE_PROBE: "1", OPENRIDR_HOME: e2eHome },
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
