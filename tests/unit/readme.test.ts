import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// A visitor reads the README on GitHub, so the seam is the published document: these tests read the
// committed file instead of importing product code.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const readme = readFileSync(resolve(root, "README.md"), "utf8");

const REPO = "https://github.com/Heldinhow/openridr";
const EMAIL = /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/;

const linkTargets = [...readme.matchAll(/\]\(([^)#\s]+)(?:#[^)]*)?\)/g)].map((match) => match[1]);
const relativeTargets = linkTargets.filter(
  (target) => !/^[a-z]+:/i.test(target) && !target.startsWith("#"),
);
const localImages = [...readme.matchAll(/!\[[^\]]*\]\(([^)\s]+)\)/g)]
  .map((match) => match[1])
  .filter((source) => !/^[a-z]+:/i.test(source));

describe("README.md", () => {
  test("answers what OpenRidr is, whom it serves and how to get it", () => {
    const headings = [
      "## Status",
      "## Download",
      "## Features",
      "## Build from source",
      "## Contributing",
      "## Community",
      "## License",
    ];
    expect(headings.filter((heading) => !readme.includes(heading))).toEqual([]);
    expect(readme).toContain("C#");
    expect(readme).toContain(".NET");
  });

  test("claims only the alpha and the platform that was verified", () => {
    expect(readme.toLowerCase()).toContain("alpha");
    expect(readme).toContain("macOS");
    expect(readme).toContain("Apple Silicon");
    expect(readme.toLowerCase()).toContain("not verified");
  });

  test("sends visitors to the release list, because releases are prereleases", () => {
    expect(readme).toContain(`${REPO}/releases`);
    expect(readme).not.toContain(`${REPO}/releases/latest`);
  });

  test("shows a committed OpenRidr capture, never third-party material", () => {
    expect(localImages.length).toBeGreaterThan(0);
    for (const source of localImages) {
      expect(source.startsWith("docs/evidence/")).toBe(true);
      expect(source).not.toMatch(/rider|jetbrains/i);
      const path = resolve(root, source);
      expect(existsSync(path)).toBe(true);
      expect(statSync(path).size).toBeGreaterThan(0);
    }
  });

  test("every relative link resolves on disk", () => {
    expect(relativeTargets.length).toBeGreaterThan(0);
    expect(relativeTargets.filter((target) => !existsSync(resolve(root, target)))).toEqual([]);
  });

  test("does not publish a personal path or email address", () => {
    expect(readme).not.toContain("/Users/");
    expect(readme.match(EMAIL)).toBeNull();
  });

  test("names the license and links its text", () => {
    expect(readme).toContain("MIT");
    expect(readme).toContain("](LICENSE)");
  });
});
