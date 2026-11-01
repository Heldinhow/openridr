import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { randomUUID } from "node:crypto";

// Interrupting the automatic restore: Cancel restore while it runs, and Revoke trust while it runs.
// A Directory.Build.targets in the temp copy makes `dotnet restore` wait (a `sleep` before NuGet's
// _GenerateRestoreGraphProjectEntry), so the restore is deterministically still running when the clicks
// land; cancelling must stop the whole process group.
// Named `zy-` so it runs after specs that assert the host's first restore/discovery counters.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(120_000);

type ProcessSnapshot = { pid: number; started: string; command: string };
type DelayProcess = ProcessSnapshot & { token: string };

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

/** ps emits a double space for single-digit days; both reader and snapshot must collapse it. */
function normalizeSpaces(value: string) {
  return value.trim().replace(/\s+/g, " ");
}

/** Snapshot of a PID; start time plus the per-run token in argv identify the delayed leaf. */
function processSnapshot(pid: number): ProcessSnapshot | null {
  const result = spawnSync("ps", ["-p", String(pid), "-o", "lstart=,command="], {
    encoding: "utf8",
  });
  if (result.status !== 0) return null;
  const parts = result.stdout.trim().split(/\s+/);
  const started = normalizeSpaces(parts.slice(0, 5).join(" "));
  const command = parts.slice(5).join(" ");
  return started && command ? { pid, started, command } : null;
}

/** Records written by the delay script itself; no command-line matching is involved. */
function readOwnedProcesses(pidDir: string): DelayProcess[] {
  if (!fs.existsSync(pidDir)) return [];
  const records: DelayProcess[] = [];
  for (const name of fs.readdirSync(pidDir)) {
    try {
      const [pidText, startedText, token] = fs
        .readFileSync(path.join(pidDir, name), "utf8")
        .trim()
        .split("|");
      const pid = Number(pidText);
      const started = normalizeSpaces(startedText ?? "");
      const normalizedToken = (token ?? "").trim();
      if (Number.isInteger(pid) && pid > 0 && started && normalizedToken) {
        records.push({ pid, started, token: normalizedToken, command: "" });
      }
    } catch {
      // The delay script was still writing this record; the next poll reads it whole.
    }
  }
  return records;
}

function ownedAlive(record: DelayProcess) {
  const current = processSnapshot(record.pid);
  return (
    current !== null &&
    current.started === record.started &&
    current.command.includes(record.token)
  );
}

/**
 * PID-only cleanup for the leaf delay. Signalling an unrecorded process group would claim ownership of
 * siblings that were never observed; the host still interrupts the restore's own group.
 */
function stopOwnedProcesses(pidDir: string) {
  for (const record of readOwnedProcesses(pidDir)) {
    if (!ownedAlive(record)) continue;
    try {
      process.kill(record.pid, "SIGKILL");
    } catch {
      // The process already exited.
    }
  }
}

function sleepBinary() {
  const result = spawnSync("sh", ["-c", "command -v sleep"], { encoding: "utf8" });
  const value = result.stdout.trim();
  if (result.status !== 0 || !value) throw new Error("sleep executable not found");
  return value;
}

/** A unique sleep path keeps the per-run token observable in argv after exec. */
function createDelayBinary(base: string, token: string) {
  const link = path.join(base, `sleep-${token}`);
  fs.symlinkSync(sleepBinary(), link);
  return link;
}

function writeOwnedRecord(pidDir: string, snapshot: ProcessSnapshot, token: string) {
  fs.writeFileSync(
    path.join(pidDir, String(snapshot.pid)),
    `${snapshot.pid}|${snapshot.started}|${token}\n`,
  );
}

async function stopTestChild(child: ChildProcess, expected: ProcessSnapshot | null) {
  if (child.exitCode !== null || child.signalCode !== null || child.pid === undefined) return;
  const current = processSnapshot(child.pid);
  if (
    !current ||
    (expected !== null &&
      (current.started !== expected.started || current.command !== expected.command))
  ) {
    return;
  }
  const closed = new Promise<void>((resolve) => child.once("close", () => resolve()));
  child.kill("SIGKILL");
  await closed;
}

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-restore-int-"));
  const copy = path.join(base, name);
  const pidDir = path.join(base, "restore-delay-pids");
  fs.mkdirSync(pidDir);
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  const token = randomUUID();
  const delayBinary = createDelayBinary(base, token);
  const delayScript = path.join(copy, "restore-delay.sh");
  fs.writeFileSync(
    delayScript,
    [
      "#!/bin/sh",
      `started=$(ps -o lstart= -p "$$" | sed 's/^ *//;s/ *$//' )`,
      `printf '%s|%s|%s\\n' "$$" "$started" ${shellQuote(token)} > ${shellQuote(pidDir)}/$$`,
      `exec ${shellQuote(delayBinary)} 4040`,
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
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("mode-badge")).toHaveText("Trusted mode");
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

test("ownership cleanup stops only the recorded delay and leaves a simultaneous foreign sleep untouched", async () => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-owned-"));
  const pidDir = path.join(base, "pids");
  fs.mkdirSync(pidDir);
  const token = randomUUID();
  const owned = spawn(createDelayBinary(base, token), ["4040"], { stdio: "ignore" });
  const foreign = spawn("sleep", ["4040"], { stdio: "ignore" });
  const ownedSnapshot = owned.pid === undefined ? null : processSnapshot(owned.pid);
  const foreignSnapshot = foreign.pid === undefined ? null : processSnapshot(foreign.pid);
  try {
    expect(ownedSnapshot).not.toBeNull();
    expect(foreignSnapshot).not.toBeNull();
    writeOwnedRecord(pidDir, ownedSnapshot!, token);
    stopOwnedProcesses(pidDir);
    await expect
      .poll(() => !ownedAlive({ ...ownedSnapshot!, token }), { timeout: 10_000 })
      .toBe(true);
    expect(foreign.pid === undefined ? null : processSnapshot(foreign.pid)).toEqual(foreignSnapshot);
  } finally {
    await stopTestChild(owned, ownedSnapshot);
    await stopTestChild(foreign, foreignSnapshot);
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("ownership cleanup ignores malformed and mismatched records", async () => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-owned-"));
  const pidDir = path.join(base, "pids");
  fs.mkdirSync(pidDir);
  const sentinel = spawn("sleep", ["4040"], { stdio: "ignore" });
  const snapshot = sentinel.pid === undefined ? null : processSnapshot(sentinel.pid);
  try {
    expect(snapshot).not.toBeNull();
    fs.writeFileSync(path.join(pidDir, "malformed"), "not-a-record\n");
    fs.writeFileSync(
      path.join(pidDir, "mismatched"),
      `${snapshot!.pid}|${snapshot!.started}|wrong-token\n`,
    );
    stopOwnedProcesses(pidDir);
    expect(sentinel.pid === undefined ? null : processSnapshot(sentinel.pid)).toEqual(snapshot);
  } finally {
    await stopTestChild(sentinel, snapshot);
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("empty ownership records leave a foreign sleep 4040 untouched", async () => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-owned-"));
  const pidDir = path.join(base, "pids");
  fs.mkdirSync(pidDir);
  const sentinel = spawn("sleep", ["4040"], { stdio: "ignore" });
  const snapshot = sentinel.pid === undefined ? null : processSnapshot(sentinel.pid);
  try {
    expect(snapshot).not.toBeNull();
    stopOwnedProcesses(pidDir);
    expect(sentinel.pid === undefined ? null : processSnapshot(sentinel.pid)).toEqual(snapshot);
  } finally {
    await stopTestChild(sentinel, snapshot);
    fs.rmSync(base, { recursive: true, force: true });
  }
});
