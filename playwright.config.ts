import { defineConfig } from "@playwright/test";

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
      env: { OPENRIDR_ENABLE_PROBE: "1" },
    },
    {
      command: "bun run dev",
      url: "http://localhost:1420",
      timeout: 120_000,
      reuseExistingServer: !process.env.CI,
    },
  ],
});
