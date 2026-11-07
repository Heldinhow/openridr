import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path, { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { COMFORT_FLOOR_MIB, HARD_FLOOR_MIB, diskVerdict, parseAvailableMib } from "../e2e/disk-floor";

// The E2E harness refuses a run below the disk floor at the load of the config, before any server.
// The proof never fills a disk: a fake `df` first on the PATH answers with the MiB the case asks for,
// like tests/shell/doctor.test.sh does for the doctor.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const measured = "/tmp/openridr-disk-floor-measured";

describe("the floors", () => {
  test("come from the doctor, the check a session runs before anything else", () => {
    const script = readFileSync(
      resolve(root, ".agents/skills/verify-openridr/scripts/doctor.sh"),
      "utf8",
    );
    const floors = [...script.matchAll(/free_mib"?\s+-ge\s+(\d+)/g)].map((match) => Number(match[1]));
    expect(floors).toEqual([COMFORT_FLOOR_MIB, HARD_FLOOR_MIB]);
  });
});

describe("parseAvailableMib", () => {
  test("reads the Available column of df -Pk as whole MiB", () => {
    const output = [
      "Filesystem   1024-blocks      Used Available Capacity  Mounted on",
      "/dev/disk3s5   239331776 166157316  24601928    88%    /System/Volumes/Data",
    ].join("\n");
    expect(parseAvailableMib(output)).toBe(24025);
  });

  test("a measurement without a number is unknown, never zero", () => {
    expect(parseAvailableMib("")).toBeUndefined();
    expect(
      parseAvailableMib("Filesystem 1024-blocks Used Available Capacity Mounted on\n"),
    ).toBeUndefined();
    expect(parseAvailableMib("df: /nope: No such file or directory")).toBeUndefined();
    expect(
      parseAvailableMib(
        "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/disk9s1 1000 0 - 0% /",
      ),
    ).toBeUndefined();
  });

  test("a line that is not a df row is unknown, even with a number in it", () => {
    // A refusal on this line would break the fail-open promise: garbage must not read as 0 MiB.
    expect(parseAvailableMib("garbage\nfoo bar baz 0")).toBeUndefined();
    expect(
      parseAvailableMib("Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/disk9s1 1 0 4096 nope /"),
    ).toBeUndefined();
  });

  test("a number too large to be a disk is unknown, not a silent ok", () => {
    const huge = "9".repeat(400);
    expect(
      parseAvailableMib(
        `Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/disk9s1 1 0 ${huge} 0% /`,
      ),
    ).toBeUndefined();
  });
});

describe("diskVerdict", () => {
  const messageAt = (freeMib: number | undefined) => {
    const verdict = diskVerdict(freeMib, measured);
    return verdict.level === "ok" ? "" : verdict.message;
  };

  test("above the comfort floor writes nothing", () => {
    expect(diskVerdict(4000, measured).level).toBe("ok");
    expect(messageAt(4000)).toBe("");
  });

  test("between the floors is a note, not a refusal", () => {
    expect(diskVerdict(1500, measured).level).toBe("note");
    expect(messageAt(1500)).toContain(`disk 1500 MiB free on ${measured}`);
    expect(messageAt(1500)).toContain(`${COMFORT_FLOOR_MIB} MiB floor`);
  });

  test("below the hard floor refuses with the number, the floor and the directory", () => {
    expect(diskVerdict(900, measured).level).toBe("refuse");
    expect(messageAt(900)).toContain("E2E refused:");
    expect(messageAt(900)).toContain(`disk 900 MiB free on ${measured}`);
    expect(messageAt(900)).toContain(`${HARD_FLOOR_MIB} MiB hard floor`);
  });

  test("an unknown measurement is a note, never a refusal", () => {
    expect(diskVerdict(undefined, measured).level).toBe("note");
    expect(messageAt(undefined)).toContain(`disk free space on ${measured} unknown`);
  });
});

/**
 * Loads the real config in a fresh process with a fake `df` first on the PATH. The stub answers only
 * `df -Pk <the measured dir>` and fails anything else, so a change of volume or flags fails the case
 * instead of passing on a stub that answers everything.
 */
function loadConfig(freeMib: number): { code: number; stderr: string } {
  const scratch = mkdtempSync(path.join(os.tmpdir(), "openridr-disk-floor-"));
  try {
    const stub = path.join(scratch, "df");
    writeFileSync(
      stub,
      [
        "#!/bin/sh",
        'dir=${2%/}',
        'if [ "$#" -ne 2 ] || [ "$1" != -Pk ] || [ "$dir" != "${FAKE_DF_EXPECT%/}" ]; then',
        '  printf \'unexpected df call: %s\\n\' "$*" >&2',
        "  exit 1",
        "fi",
        "printf '%s\\n' 'Filesystem 1024-blocks Used Available Capacity Mounted on'",
        'printf \'%s\\n\' "/dev/disk9s1 100000000 0 $((FAKE_DF_MIB * 1024)) 0% /"',
        "",
      ].join("\n"),
    );
    chmodSync(stub, 0o755);
    const run = spawnSync(process.execPath, ["-e", 'await import("./playwright.config.ts")'], {
      cwd: root,
      encoding: "utf8",
      env: {
        ...process.env,
        PATH: `${scratch}:${process.env.PATH ?? ""}`,
        FAKE_DF_MIB: String(freeMib),
        FAKE_DF_EXPECT: os.tmpdir(),
      },
    });
    return { code: run.status ?? 1, stderr: run.stderr };
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

describe("the playwright config", () => {
  test("refuses the run below the hard floor, at load", () => {
    const run = loadConfig(900);
    expect(run.code).not.toBe(0);
    expect(run.stderr).toContain("E2E refused:");
    expect(run.stderr).toContain(`disk 900 MiB free on ${os.tmpdir()}`);
    expect(run.stderr).toContain(`${HARD_FLOOR_MIB} MiB hard floor`);
  });

  test("warns and keeps loading between the floors", () => {
    const run = loadConfig(1500);
    expect(run.code).toBe(0);
    expect(run.stderr).toContain(`note  disk 1500 MiB free on ${os.tmpdir()}`);
  });
});
