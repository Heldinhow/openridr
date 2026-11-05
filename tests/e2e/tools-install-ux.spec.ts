import { expect, test, type Page } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { hostPort } from "./environment";

// A fresh app home proves the mandatory setup: the first attempt fails like an unreachable
// source, retry installs and verifies, and the trust prompt appears only after setup is done.
const port = hostPort + 5;
const fixture = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
let home: string;
let server: ChildProcess;

test.describe.configure({ mode: "serial", timeout: 240_000 });

test.beforeAll(async () => {
  home = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-tools-ux-home-"));
  const installer = path.join(home, "probe-installer");
  fs.writeFileSync(
    installer,
    `#!/bin/sh
staging="$OPENRIDR_HOME/tools/csharp-ls/0.28.staging"
mkdir -p "$staging"
dd if=/dev/zero of="$staging/partial.bin" bs=1024 count=512 2>/dev/null
sleep 2
if [ -f "$OPENRIDR_HOME/probe-tools-failed" ]; then
  printf '#!/bin/sh\\nprintf "csharp-ls, version 0.28.0 (probe)\\\\n"\\n' > "$staging/csharp-ls"
  chmod +x "$staging/csharp-ls"
  exit 0
fi
touch "$OPENRIDR_HOME/probe-tools-failed"
echo "error: Unable to load the service index for source https://api.nuget.org/v3/index.json. The HTTP request timed out." >&2
exit 1
`,
  );
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
      OPENRIDR_TRUST_STORE: "on",
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

async function openWorkspace(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(fixture);
  await page.getByTestId("workspace-open").click();
}

test("setup blocks the workspace, reports the failure and retries without a plan", async ({
  page,
}) => {
  await useIsolatedHost(page);
  await openWorkspace(page);

  // Setup runs before trust: no trust prompt and no workspace while the tool is not ready.
  await expect(page.getByTestId("setup-gate")).toBeVisible();
  await expect(page.getByTestId("trust-prompt-dialog")).toHaveCount(0);
  await expect(page.getByTestId("tree")).toHaveCount(0);
  await expect(page.getByTestId("setup-gate-progress")).toContainText("Downloaded", {
    timeout: 15_000,
  });
  await expect(page.getByTestId("setup-gate-progress")).toContainText("no percentage");

  // The failed attempt is never reported ready and names the network plus Retry setup.
  await expect(page.getByTestId("setup-gate-state")).toHaveText("Setup failed", {
    timeout: 30_000,
  });
  await expect(page.getByTestId("setup-gate-error")).toContainText("network");
  await expect(page.getByTestId("setup-gate-retry")).toBeEnabled();

  // Retry re-enters the install without reopening the workspace, then reaches verified ready.
  await page.getByTestId("setup-gate-retry").click();
  await expect(page.getByTestId("setup-gate-progress")).toContainText("Downloaded", {
    timeout: 15_000,
  });
  // Ready unlocks trust immediately; the gate unmounts instead of waiting to show "Ready".
  await expect(page.getByTestId("trust-prompt-dialog")).toBeVisible({ timeout: 90_000 });
  await expect(page.getByTestId("setup-gate")).toHaveCount(0);
});

test("a second opening reuses the verified tools and never shows the gate", async ({ page }) => {
  await useIsolatedHost(page);
  await openWorkspace(page);
  // The host verified the installation in the previous test; the trust prompt is the next step.
  await page.getByTestId("trust-prompt-dialog").waitFor({ timeout: 15_000 });
  await expect(page.getByTestId("setup-gate")).toHaveCount(0);
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("install-state")).toHaveText("Ready");
  await expect(page.getByTestId("install-plan-open")).toHaveCount(0);
  await expect(page.getByTestId("install-confirm")).toHaveCount(0);
  await expect(page.getByTestId("install-reinstall")).toBeVisible();
});
