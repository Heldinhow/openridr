import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// The repository surface is the seam: GitHub renders these documents to a visitor, so the test
// reads the files as published instead of importing product code.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const document = (name: string) => readFileSync(resolve(root, name), "utf8");

const PRIVATE_REPORTING = "https://github.com/Heldinhow/openridr/security/advisories/new";
const EMAIL = /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/;

function relativeLinks(markdown: string): string[] {
  return [...markdown.matchAll(/\]\(([^)#\s]+)(?:#[^)]*)?\)/g)]
    .map((match) => match[1])
    .filter((target) => !/^[a-z]+:/i.test(target) && !target.startsWith("#"));
}

describe("CODE_OF_CONDUCT.md", () => {
  const conduct = document("CODE_OF_CONDUCT.md");

  test("is the Contributor Covenant 2.1, not a summary of it", () => {
    expect(conduct).toContain("Contributor Covenant Code of Conduct");
    expect(conduct).toContain("version 2.1");
    expect(conduct).toContain("## Enforcement Guidelines");
  });

  test("sends conduct reports to the active private channel", () => {
    expect(conduct).toContain(PRIVATE_REPORTING);
    expect(conduct.toLowerCase()).toContain("private vulnerability reporting");
    expect(conduct.toLowerCase()).toContain("is active");
  });

  test("does not claim the private channel is unavailable", () => {
    expect(conduct.toLowerCase()).not.toContain("not yet enabled");
  });

  test("does not publish a personal email address", () => {
    expect(conduct.match(EMAIL)).toBeNull();
  });
});

describe("SECURITY.md", () => {
  const security = document("SECURITY.md");

  test("states which versions receive fixes", () => {
    expect(security).toContain("## Supported Versions");
    expect(security).toContain("latest alpha");
    expect(security).toContain("Older alphas");
  });

  test("only claims the platform that was verified", () => {
    expect(security).toContain("Apple Silicon");
    expect(security).toContain("not verified");
  });

  test("sends vulnerability reports to the active private channel", () => {
    expect(security).toContain(PRIVATE_REPORTING);
    expect(security.toLowerCase()).toContain("private vulnerability reporting");
    expect(security.toLowerCase()).toContain("is active");
  });

  test("does not claim the private channel is unavailable", () => {
    expect(security.toLowerCase()).not.toContain("not yet enabled");
  });

  test("does not publish a personal email address", () => {
    expect(security.match(EMAIL)).toBeNull();
  });
});

describe("relative links", () => {
  for (const name of ["CODE_OF_CONDUCT.md", "SECURITY.md"]) {
    test(`${name} links resolve on disk`, () => {
      const missing = relativeLinks(document(name)).filter(
        (target) => !existsSync(resolve(root, target)),
      );
      expect(missing).toEqual([]);
    });
  }
});
