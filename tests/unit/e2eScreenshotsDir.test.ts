import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import os from "node:os";
import path, { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { screenshotsDirFor } from "../e2e/environment";

// The E2E screenshots dir is a harness contract: a spec that fails must not rewrite versioned
// evidence, so the default lives outside the repository and only the override reaches it.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const e2eDir = resolve(root, "tests/e2e");
const VERSIONED = "docs/evidence/screenshots";

describe("screenshotsDirFor", () => {
  test("defaults outside the repository, keyed by the UI port", () => {
    expect(screenshotsDirFor(1420, undefined)).toBe(
      path.join(os.tmpdir(), "openridr-e2e-screenshots-1420"),
    );
    expect(screenshotsDirFor(1531, undefined)).toBe(
      path.join(os.tmpdir(), "openridr-e2e-screenshots-1531"),
    );
    expect(screenshotsDirFor(1420, undefined)).not.toBe(screenshotsDirFor(1531, undefined));
    expect(screenshotsDirFor(1420, undefined).startsWith(root + path.sep)).toBe(false);
  });

  test("the override wins, and a relative one is resolved against the current directory", () => {
    expect(screenshotsDirFor(1531, "/tmp/openridr-e2e-shots-mine")).toBe(
      "/tmp/openridr-e2e-shots-mine",
    );
    expect(screenshotsDirFor(1531, "shots")).toBe(path.resolve("shots"));
  });
});

describe("E2E specs", () => {
  test("none names the versioned screenshots directory", () => {
    const offenders = readdirSync(e2eDir)
      .filter((name) => name.endsWith(".spec.ts"))
      .filter((name) => readFileSync(path.join(e2eDir, name), "utf8").includes(VERSIONED));
    expect(offenders).toEqual([]);
  });
});
