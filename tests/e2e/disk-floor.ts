import { execFileSync } from "node:child_process";
import os from "node:os";

/**
 * Both floors have one source: the doctor (`.agents/skills/verify-openridr/scripts/doctor.sh`), the
 * check a session runs before anything else. A test in `tests/unit` compares the script with these
 * two numbers, so changing one without the other fails. 3072 MiB is the comfort floor (up to 4
 * parallel sessions, ~200 MiB of E2E app home each plus caches); under 1024 MiB a run dies with
 * ENOSPC mid-copy.
 */
export const COMFORT_FLOOR_MIB = 3072;
export const HARD_FLOOR_MIB = 1024;

const FLOOR_CONTEXT = "up to 4 parallel sessions, ~200 MiB E2E home each + caches";

/** Available column of `df -Pk` (1024-blocks) as whole MiB. No number is unknown, never 0. */
export function parseAvailableMib(output: string): number | undefined {
  const available = output.split("\n")[1]?.trim().split(/\s+/)[3];
  if (available === undefined || !/^\d+$/.test(available)) return undefined;
  return Math.floor(Number(available) / 1024);
}

export type DiskVerdict =
  | { level: "ok" }
  | { level: "note"; message: string }
  | { level: "refuse"; message: string };

export function diskVerdict(freeMib: number | undefined, dir: string): DiskVerdict {
  if (freeMib === undefined) {
    return {
      level: "note",
      message: `disk free space on ${dir} unknown: df gave no number (floor ${COMFORT_FLOOR_MIB} MiB: ${FLOOR_CONTEXT})`,
    };
  }
  if (freeMib >= COMFORT_FLOOR_MIB) return { level: "ok" };
  if (freeMib >= HARD_FLOOR_MIB) {
    return {
      level: "note",
      message: `disk ${freeMib} MiB free on ${dir} (under the ${COMFORT_FLOOR_MIB} MiB floor: ${FLOOR_CONTEXT}; run one spec at a time)`,
    };
  }
  return {
    level: "refuse",
    message: `E2E refused: disk ${freeMib} MiB free on ${dir} (under the ${HARD_FLOOR_MIB} MiB hard floor: ${FLOOR_CONTEXT})`,
  };
}

/**
 * Measures the volume of `dir` (always the temp dir, where the E2E app home and captures live) and
 * refuses the run below the hard floor. Importing this module measures nothing; only this call does.
 * A `df` that fails or answers no number is a note, not a refusal: `df` is not an E2E requirement.
 */
export function guardDiskFloor(dir: string = os.tmpdir()): void {
  let output = "";
  try {
    output = execFileSync("df", ["-Pk", dir], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
  } catch {
    // Unknown, not zero: the run continues with a note (fail-open, like the doctor).
  }
  const verdict = diskVerdict(parseAvailableMib(output), dir);
  if (verdict.level === "refuse") throw new Error(verdict.message);
  if (verdict.level === "note") console.warn(`note  ${verdict.message}`);
}
