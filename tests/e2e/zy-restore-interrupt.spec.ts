import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

// Interrupting the automatic restore: Cancel restore while it runs, and Revoke trust while it runs.
// A Directory.Build.targets in the temp copy makes `dotnet restore` wait (a `sleep` before NuGet's `_GenerateRestoreGraphProjectEntry`), so the
// restore is deterministically still running when the clicks land; cancelling must stop the whole process group.
// Named `zy-` so it runs after specs that assert the host's first restore/discovery counters.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(120_000);

// The delaying `sleep 4040` is unique to this spec; it must exist while restoring and be gone after Cancel/Revoke.
const sleepAlive = () => spawnSync("pgrep", ["-f", "sleep 4040"]).status === 0;

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-restore-int-"));
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  fs.writeFileSync(
    path.join(copy, "Directory.Build.targets"),
    '<Project><Target Name="SlowRestore" BeforeTargets="_GenerateRestoreGraphProjectEntry"><Exec Command="sleep 4040" /></Target></Project>\n',
  );
  return { base, copy };
}

async function openAndTrust(page: Page, copy: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-checkbox").check();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("restore-state")).toContainText("Restoring…", { timeout: 30_000 });
}

test("Cancel restore during the automatic restore ends it as cancelled and keeps the editor", async ({ page }) => {
  const { base, copy } = tempCopy("Order Desk restore cancel");
  try {
    await openAndTrust(page, copy);
    await expect.poll(sleepAlive, { timeout: 15_000 }).toBe(true);
    await page.getByTestId("restore-cancel").click();
    await expect(page.getByTestId("restore-state")).toContainText("Restore cancelled", { timeout: 30_000 });
    await expect.poll(sleepAlive, { timeout: 15_000 }).toBe(false);
    await expect(page.getByTestId("restore-cancel")).toHaveCount(0);
    await expect(page.getByTestId("tree")).toBeVisible();
    // A cancelled restore is not retried by itself (once per context) and leaves a working retry.
    await page.waitForTimeout(3_000);
    await expect(page.getByTestId("restore-state")).toContainText("Restore cancelled");
    await expect(page.getByTestId("restore-retry")).toBeVisible();
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("Revoke trust during the automatic restore stops it and nothing evaluates afterwards", async ({ page }) => {
  const { base, copy } = tempCopy("Order Desk restore revoke");
  try {
    await openAndTrust(page, copy);
    await expect.poll(sleepAlive, { timeout: 15_000 }).toBe(true);
    await page.getByTestId("mode-badge").click();
    await page.getByTestId("trust-revoke").click();
    await expect.poll(sleepAlive, { timeout: 15_000 }).toBe(false);
    await expect(page.getByTestId("mode-badge")).toHaveText("Restricted mode");
    await expect(page.getByTestId("restore-cancel")).toHaveCount(0, { timeout: 30_000 });
    await expect(page.getByTestId("restore-state")).not.toContainText("Restoring…");
    await expect(page.getByTestId("restore-state")).not.toContainText("Restore succeeded");
    await page.waitForTimeout(3_000);
    await expect(page.getByTestId("eval-state")).not.toContainText("Evaluated");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
