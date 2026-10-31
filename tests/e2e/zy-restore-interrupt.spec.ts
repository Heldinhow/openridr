import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";

// Interrupting the automatic restore: Cancel restore while it runs, and Revoke trust while it runs.
// A Directory.Build.targets in the temp copy makes `dotnet restore` wait (a `sleep` before NuGet's
// _GenerateRestoreGraphProjectEntry), so the restore is deterministically still running when the clicks
// land; cancelling must stop the whole process group.
// Named `zy-` so it runs after specs that assert the host's first restore/discovery counters.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(120_000);

type DelayProcess = { pid: number; pgid: number; started: string };

/** Quote a path for /bin/sh without relying on the working directory. */
function shellQuote(value: string) {
  return `'${value.replace(/'/g, `'\\''`)}'`;
}

function xmlAttribute(value: string) {
  return value
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/** Snapshot of a PID; the start time distinguishes it from a later process that reuses the PID. */
function processSnapshot(pid: number): DelayProcess | null {
  const result = spawnSync("ps", ["-p", String(pid), "-o", "pgid=,lstart="], {
    encoding: "utf8",
  });
  if (result.status !== 0) return null;
  const [pgidText, ...startedParts] = result.stdout.trim().split(/\s+/);
  const pgid = Number(pgidText);
  const started = startedParts.join(" ").trim();
  return Number.isInteger(pgid) && started ? { pid, pgid, started } : null;
}

/** Records written by the delay script itself; no command-line matching is involved. */
function readOwnedProcesses(pidDir: string): DelayProcess[] {
  if (!fs.existsSync(pidDir)) return [];
  const records: DelayProcess[] = [];
  for (const name of fs.readdirSync(pidDir)) {
    try {
      const [pidText, pgidText, ...startedParts] = fs
        .readFileSync(path.join(pidDir, name), "utf8")
        .trim()
        .split("|");
      const pid = Number(pidText);
      const pgid = Number(pgidText);
      const started = startedParts.join("|").trim();
      if (Number.isInteger(pid) && pid > 0 && Number.isInteger(pgid) && started) {
        records.push({ pid, pgid, started });
      }
    } catch {
      // The delay script was still writing this record; the next poll reads it whole.
    }
  }
  return records;
}

function ownedAlive(record: DelayProcess) {
  const current = processSnapshot(record.pid);
  return current !== null && current.started === record.started;
}

/** Kill only PIDs whose recorded start time still matches; a reused or foreign PID is never touched. */
function stopOwnedProcesses(pidDir: string) {
  const ownGroup = processSnapshot(process.pid)?.pgid;
  for (const record of readOwnedProcesses(pidDir)) {
    const current = processSnapshot(record.pid);
    if (!current || current.started !== record.started) continue;
    if (current.pgid === record.pgid && current.pgid > 1 && current.pgid !== ownGroup) {
      try {
        process.kill(-current.pgid, "SIGKILL");
      } catch {
        // The group already exited.
      }
    }
    try {
      process.kill(record.pid, "SIGKILL");
    } catch {
      // The process already exited.
    }
  }
}

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-restore-int-"));
  const copy = path.join(base, name);
  const pidDir = path.join(base, "restore-delay-pids");
  fs.mkdirSync(pidDir);
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  const delayScript = path.join(copy, "restore-delay.sh");
  fs.writeFileSync(
    delayScript,
    [
      "#!/bin/sh",
      `pgid=$(ps -o pgid= -p "$$" | tr -d ' ')`,
      `started=$(ps -o lstart= -p "$$" | sed 's/^ *//;s/ *$//' )`,
      `printf '%s|%s|%s\\n' "$$" "$pgid" "$started" > ${shellQuote(pidDir)}/$$`,
      "exec sleep 4040",
      "",
    ].join("\n"),
  );
  fs.chmodSync(delayScript, 0o755);
  fs.writeFileSync(
    path.join(copy, "Directory.Build.targets"),
    `<Project><Target Name="SlowRestore" BeforeTargets="_GenerateRestoreGraphProjectEntry"><Exec Command="sh &quot;${xmlAttribute(delayScript)}&quot;" /></Target></Project>\n`,
  );
  return { base, copy, pidDir };
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

async function waitForOwnedDelay(pidDir: string) {
  await expect
    .poll(() => readOwnedProcesses(pidDir).some(ownedAlive), { timeout: 15_000 })
    .toBe(true);
}

async function waitForOwnedDelayGone(pidDir: string) {
  await expect
    .poll(() => readOwnedProcesses(pidDir).every((record) => !ownedAlive(record)), {
      timeout: 15_000,
    })
    .toBe(true);
}

test("Cancel restore during the automatic restore ends it as cancelled and keeps the editor", async ({ page }) => {
  const { base, copy, pidDir } = tempCopy("Order Desk restore cancel");
  try {
    await openAndTrust(page, copy);
    await waitForOwnedDelay(pidDir);
    await page.getByTestId("restore-cancel").click();
    await expect(page.getByTestId("restore-state")).toContainText("Restore cancelled", { timeout: 30_000 });
    await waitForOwnedDelayGone(pidDir);
    await expect(page.getByTestId("restore-cancel")).toHaveCount(0);
    await expect(page.getByTestId("tree")).toBeVisible();
    // A cancelled restore is not retried by itself (once per context) and leaves a working retry.
    await page.waitForTimeout(3_000);
    await expect(page.getByTestId("restore-state")).toContainText("Restore cancelled");
    await expect(page.getByTestId("restore-retry")).toBeVisible();
  } finally {
    stopOwnedProcesses(pidDir);
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("Revoke trust during the automatic restore stops it and nothing evaluates afterwards", async ({ page }) => {
  const { base, copy, pidDir } = tempCopy("Order Desk restore revoke");
  try {
    await openAndTrust(page, copy);
    await waitForOwnedDelay(pidDir);
    await page.getByTestId("mode-badge").click();
    await page.getByTestId("trust-revoke").click();
    await waitForOwnedDelayGone(pidDir);
    await expect(page.getByTestId("mode-badge")).toHaveText("Restricted mode");
    await expect(page.getByTestId("restore-cancel")).toHaveCount(0, { timeout: 30_000 });
    await expect(page.getByTestId("restore-state")).not.toContainText("Restoring…");
    await expect(page.getByTestId("restore-state")).not.toContainText("Restore succeeded");
    await page.waitForTimeout(3_000);
    await expect(page.getByTestId("eval-state")).not.toContainText("Evaluated");
  } finally {
    stopOwnedProcesses(pidDir);
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("ownership cleanup leaves a foreign sleep 4040 untouched", async () => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-owned-"));
  const pidDir = path.join(base, "pids");
  fs.mkdirSync(pidDir);
  const sentinel = spawn("sleep", ["4040"], { stdio: "ignore" });
  try {
    const before = sentinel.pid === undefined ? null : processSnapshot(sentinel.pid);
    expect(before).not.toBeNull();
    stopOwnedProcesses(pidDir);
    expect(sentinel.pid === undefined ? null : processSnapshot(sentinel.pid)).toEqual(before);
  } finally {
    if (sentinel.pid !== undefined) {
      try {
        process.kill(sentinel.pid, "SIGKILL");
      } catch {
        // The sentinel already exited.
      }
    }
    fs.rmSync(base, { recursive: true, force: true });
  }
});
