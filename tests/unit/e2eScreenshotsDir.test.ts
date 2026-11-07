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
// `"docs/evidence/screenshots"` and `path.join("docs", "evidence", "screenshots")` are the same
// path, so the scan compares letters only: separators, quotes and commas disappear.
const VERSIONED_LETTERS = "docsevidencescreenshots";
const lettersOf = (source: string) => source.toLowerCase().replace(/[^a-z]/g, "");
const namesVersionedDir = (source: string) => lettersOf(source).includes(VERSIONED_LETTERS);

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

describe("the versioned-dir guard", () => {
  test("rejects the literal path and the segmented path.join form", () => {
    expect(namesVersionedDir('path.resolve("docs/evidence/screenshots")')).toBe(true);
    expect(namesVersionedDir('path.join("docs", "evidence", "screenshots")')).toBe(true);
    expect(namesVersionedDir('path.join(screenshotsDir, "11-build-ok.png")')).toBe(false);
  });
});

describe("E2E specs", () => {
  test("none names the versioned screenshots directory", () => {
    const offenders = readdirSync(e2eDir, { recursive: true })
      .map(String)
      .filter((name) => name.endsWith(".spec.ts"))
      .filter((name) => namesVersionedDir(readFileSync(path.join(e2eDir, name), "utf8")));
    expect(offenders).toEqual([]);
  });
});
