import { describe, expect, test } from "bun:test";
import { execFileSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { port } from "../e2e/environment";

// The E2E harness picks its own ports, and the browser refuses a fixed list of them
// (`net::ERR_UNSAFE_PORT`). The refusal has to happen at load, before any server starts.
const environment = resolve(dirname(fileURLToPath(import.meta.url)), "../e2e/environment.ts");

/** Loads the environment module in a fresh process, the way config and globalSetup do. */
function loadEnvironment(env: Record<string, string>): string {
  try {
    execFileSync(process.execPath, [environment], {
      env: { ...process.env, ...env },
      encoding: "utf8",
      stdio: "pipe",
    });
    return "";
  } catch (error) {
    return String((error as { stderr?: string }).stderr ?? "");
  }
}

describe("port", () => {
  test("rejects the ports Chrome blocks", () => {
    for (const value of ["1719", "1720", "1723"]) {
      expect(() => port(value, 1420)).toThrow(`Invalid E2E port: ${value}`);
      expect(() => port(value, 1420)).toThrow(/Chrome/);
    }
  });

  test("accepts the ports Chrome allows", () => {
    for (const value of ["1721", "1722", "1725"]) {
      expect(port(value, 1420)).toBe(Number(value));
    }
  });

  test("keeps the numeric validation", () => {
    for (const value of ["", "0", "65536", "-1", "1.5", "1420;echo"]) {
      expect(() => port(value, 1420)).toThrow("Invalid E2E port");
    }
    expect(port(undefined, 1420)).toBe(1420);
  });
});

describe("the environment module", () => {
  test("refuses a blocked UI port", () => {
    const stderr = loadEnvironment({ OPENRIDR_E2E_UI_PORT: "1720" });
    expect(stderr).toContain("Invalid E2E port: 1720");
    expect(stderr).toContain("Chrome");
  });

  test("refuses a blocked host port", () => {
    const stderr = loadEnvironment({ OPENRIDR_E2E_HOST_PORT: "1720" });
    expect(stderr).toContain("Invalid E2E port: 1720");
    expect(stderr).toContain("Chrome");
  });

  test("refuses a pair whose derived host range touches a blocked port", () => {
    // 1716 + 3 is 1719: a spec that starts its own host would reach a port Chrome blocks.
    const stderr = loadEnvironment({ OPENRIDR_E2E_HOST_PORT: "1716" });
    expect(stderr).toContain("Invalid E2E port: 1719");
    expect(stderr).toContain("1420/1716");
  });

  test("keeps refusing an equal pair", () => {
    const stderr = loadEnvironment({
      OPENRIDR_E2E_UI_PORT: "1531",
      OPENRIDR_E2E_HOST_PORT: "1531",
    });
    expect(stderr).toContain("E2E UI and host ports must differ");
  });
});
