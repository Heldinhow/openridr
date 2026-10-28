import { defineConfig } from "@playwright/test";
import os from "node:os";
import path from "node:path";

// Keep tool installation and the composite host root out of the real app home during E2E.
const e2eHome = path.join(os.tmpdir(), "openridr-e2e-home");

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: false,
  workers: 1,
  timeout: 30_000,
  expect: { timeout: 10_000 },
  reporter: [["list"]],
  use: {
    baseURL: "http://localhost:1420",
    channel: "chrome",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  webServer: [
    {
      command: "cargo run -p openridr-host-http -- --port 4317",
      url: "http://127.0.0.1:4317/health",
      timeout: 300_000,
      reuseExistingServer: !process.env.CI,
      // Dev/test-only probe used by the process-boundary E2E. Never set in the desktop app.
      env: { OPENRIDR_ENABLE_PROBE: "1", OPENRIDR_HOME: e2eHome },
    },
    {
      command: "bun run dev",
      url: "http://localhost:1420",
      timeout: 120_000,
      reuseExistingServer: !process.env.CI,
    },
  ],
});
