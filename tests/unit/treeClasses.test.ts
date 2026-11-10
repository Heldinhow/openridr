import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { fileKind, fileToken, type FileKind } from "../../src/solutionVisuals";

// Every class the Solution tree renders has a rule behind it: no element carries a class that the
// stylesheets do not know about. This is the guard for "affordances that lie" — a class that only
// looks like styling is a bug, not a placeholder.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const read = (file: string) => readFileSync(resolve(root, file), "utf8");

const COMPONENTS = [
  "src/TreeRow.tsx",
  "src/SolutionTree.tsx",
  "src/FileIcon.tsx",
  "src/icons/FolderIcon.tsx",
  "src/icons/DependencyGlyph.tsx",
  "src/icons/RuntimeGlyph.tsx",
  "src/icons/fileGlyphs.tsx",
  "src/icons/ProjectIcon.tsx",
];

const STYLES = ["src/styles.css", "src/solution-icons.css", "src/rider-chrome.css"].map(read).join("\n");

/** Conditional class names that live inside template expressions, not in the literal. */
const CONDITIONAL = ["selected", "no-index", "open", "leaf", "active", "error", "warning"];

function classesOf(source: string): string[] {
  const found: string[] = [];
  for (const match of source.matchAll(/className=(?:"([^"]*)"|\{`([^`]*)`\})/g)) {
    const raw = (match[1] ?? match[2] ?? "").replace(/\$\{[^}]*\}/g, " ");
    for (const name of raw.split(/\s+/)) if (name) found.push(name);
  }
  return found;
}

function hasRule(name: string): boolean {
  return new RegExp(`\\.${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}(?![\\w-])`).test(STYLES);
}

describe("Solution tree classes", () => {
  test("every static class in the tree components has a CSS rule", () => {
    const missing: string[] = [];
    for (const file of COMPONENTS) {
      for (const name of classesOf(read(file))) {
        // `tint-${token}` and `project-${kind}` are expanded below, one rule per value.
        if (name.endsWith("-")) continue;
        if (!hasRule(name)) missing.push(`${file}: ${name}`);
      }
    }
    expect(missing).toEqual([]);
  });

  test("every tint and project-kind value has a rule", () => {
    const kinds: FileKind[] = [
      "csharp", "solution", "project", "dotsettings", "props", "json", "xml", "razor", "resource",
      "markdown", "text", "yaml", "docker", "git", "config", "script", "powershell", "batch", "css",
      "html", "image", "license", "file",
    ];
    for (const kind of kinds) {
      const tint = `tint-${fileToken(kind)}`;
      expect(`${tint}: ${hasRule(tint)}`).toBe(`${tint}: true`);
    }
    for (const kind of ["console", "library", "test", "web", "unknown"]) {
      const name = `project-${kind}`;
      expect(`${name}: ${hasRule(name)}`).toBe(`${name}: true`);
    }
    expect(fileKind("Program.cs")).toBe("csharp");
  });

  test("the conditional classes have rules too", () => {
    for (const name of CONDITIONAL) expect(`${name}: ${hasRule(name)}`).toBe(`${name}: true`);
  });

  test("the folder badge on a folder named config is gone", () => {
    // The `↗` sticker had no meaning; the folder name never changes the glyph.
    expect(read("src/TreeRow.tsx")).not.toContain("folder-config-badge");
    expect(STYLES).not.toContain("folder-config-badge");
  });
});
