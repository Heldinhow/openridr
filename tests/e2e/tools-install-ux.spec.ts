import { expect, test, type Page } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { hostPort } from "./environment";

// The shared E2E host talks to NuGet.org. This spec starts its own host with the test-only
// installer seam: a script that writes observed bytes, fails once like an unreachable source,
// then succeeds on retry. Nothing touches the network or the real app home.
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
  printf '#!/bin/sh\\necho "csharp-ls, version 0.28.0 (probe)"\\n' > "$staging/csharp-ls"
  chmod +x "$staging/csharp-ls"
  exit 0
fi
touch "$OPENRIDR_HOME/probe-tools-failed"
echo "error: Unable to load the service index for source https://api.nuget.org/v3/index.json. The HTTP request timed out." >&2
exit 1
`,
  );
  fs.chmodSync(installer, 0o755);
  server = spawn("cargo", ["run", "-p", "openridr-host-http", "--", "--port", String(port)], {
    env: {
      ...process.env,
      OPENRIDR_HOME: home,
      OPENRIDR_TRUST_STORE: "on",
      OPENRIDR_ENABLE_PROBE: "1",
      OPENRIDR_TOOLS_MUXER: installer,
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

test("the csharp-ls install shows progress, a network error and a working retry", async ({
  page,
}) => {
  await useIsolatedHost(page);
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(fixture);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");

  // The first C# project without analysis offers the plan; review and confirmation are explicit.
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("install-panel")).toBeVisible();
  await page.getByTestId("install-plan-open").click();
  await expect(page.getByTestId("install-plan")).toBeVisible();
  await page.getByTestId("install-confirm").click();

  // Bytes are observed while the installer runs; no percentage is invented.
  await expect(page.getByTestId("install-state")).toHaveText("Downloading…");
  await expect(page.getByTestId("install-progress")).toContainText("downloaded", {
    timeout: 15_000,
  });
  await expect(page.getByTestId("install-progress")).toContainText("no percentage");

  // The failed attempt is never reported ready and names the network plus Try again.
  await expect(page.getByTestId("install-state")).toHaveText("Failed", { timeout: 30_000 });
  await expect(page.getByTestId("install-error")).toContainText("network");
  await expect(page.getByTestId("install-error")).toContainText("Try again");
  await expect(page.getByTestId("install-retry")).toBeEnabled();
  await expect(page.getByTestId("install-ready")).toHaveCount(0);

  // Retry re-enters the download without reopening the workspace, then reaches verified ready.
  await page.getByTestId("install-retry").click();
  await expect(page.getByTestId("install-progress")).toContainText("downloaded", {
    timeout: 15_000,
  });
  await expect(page.getByTestId("install-ready")).toBeVisible({ timeout: 30_000 });
  await expect(page.getByTestId("install-state")).toHaveText("Ready");
  await expect(page.getByTestId("install-error")).toHaveCount(0);
  await expect(page.getByTestId("install-retry")).toHaveCount(0);
});
