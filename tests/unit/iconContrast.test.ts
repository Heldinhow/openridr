import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { fileIcon, folderIcon, type FileIconName, type FolderIconName } from "../../src/solutionVisuals";

// The drawings are third-party files shipped unmodified, so the palette is data: this reads the
// SVGs and the shipped CSS, never a copy of either. A fill that fails the floor on the resting row
// surface fails here, before the app is opened.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const styles = readFileSync(resolve(root, "src/styles.css"), "utf8");
const fileDir = resolve(root, "public/file-icons");
const folderDir = resolve(root, "public/folder-icons");

type Rgb = [number, number, number];

/** First declaration wins: the dark `:root` comes before the `@supports` light override. */
function token(css: string, name: string): string {
  const match = new RegExp(`--${name}:\\s*([^;]+);`).exec(css);
  if (!match) throw new Error(`token --${name} not found`);
  return match[1].trim();
}

function hex(value: string): Rgb {
  const match = /^#([0-9a-f]{6})$/i.exec(value);
  if (!match) throw new Error(`not a hex colour: ${value}`);
  const digits = match[1];
  return [0, 2, 4].map((i) => Number.parseInt(digits.slice(i, i + 2), 16)) as Rgb;
}

function channel(value: number): number {
  const v = value / 255;
  return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}

function luminance([r, g, b]: Rgb): number {
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrast(a: Rgb, b: Rgb): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/** Every fill the SVG paints with, lowercased; the theme never uses `currentColor`. */
function fillsOf(svg: string): string[] {
  return [...new Set([...svg.matchAll(/fill="(#[0-9a-fA-F]{6})"/g)].map((match) => match[1].toLowerCase()))];
}

const FILE_ICONS: FileIconName[] = [
  "csharp", "visualstudio", "fsharp", "xml", "settings", "json", "xaml", "razor", "markdown",
  "mdx", "yaml", "document", "console", "powershell", "css", "sass", "less", "html", "image",
  "svg", "git", "editorconfig", "docker", "license", "readme", "nodejs", "tsconfig", "nuget",
  "tune", "file",
];

const FOLDER_ICONS: FolderIconName[] = [
  "folder", "folder-src", "folder-test", "folder-docs", "folder-config", "folder-public",
  "folder-dist", "folder-github", "folder-vscode", "folder-scripts", "folder-resource",
  "folder-controller", "folder-class", "folder-views", "folder-migrations",
];

const PANEL = hex(token(styles, "panel"));

describe("Solution Explorer icon assets", () => {
  test("every icon the resolver can name has a shipped SVG, and nothing is shipped unused", () => {
    const files = readdirSync(fileDir).sort();
    expect(files).toEqual(FILE_ICONS.map((name) => `${name}.svg`).sort());
    const folders = readdirSync(folderDir).sort();
    expect(folders).toEqual(
      FOLDER_ICONS.flatMap((name) => [`${name}.svg`, `${name}-open.svg`]).sort(),
    );
  });

  test("the resolver only ever returns a name that is shipped", () => {
    for (const path of ["Program.cs", "OrderDesk.sln", "nuget.config", "README.md", "tool.exe"]) {
      expect(FILE_ICONS).toContain(fileIcon(path));
    }
    for (const name of ["src", "obj", ".github", "Whatever"]) {
      expect(FOLDER_ICONS).toContain(folderIcon(name));
    }
  });

  test("every fill clears 3:1 on the resting row surface", () => {
    const failures: string[] = [];
    for (const [dir, names] of [
      [fileDir, FILE_ICONS.map((name) => `${name}.svg`)],
      [folderDir, FOLDER_ICONS.flatMap((name) => [`${name}.svg`, `${name}-open.svg`])],
    ] as [string, string[]][]) {
      for (const name of names) {
        for (const fill of fillsOf(readFileSync(resolve(dir, name), "utf8"))) {
          const ratio = contrast(hex(fill), PANEL);
          if (ratio < 3) failures.push(`${name} ${fill}: ${ratio.toFixed(2)}:1`);
        }
      }
    }
    expect(failures).toEqual([]);
  });

  test("the theme's own palette is kept, including the fills that dip on the focused row", () => {
    // The focused row is #343434 and the theme is not adjusted for it: these are the four file
    // drawings and the three folder bodies that measure under 3:1 there. The value is the theme's,
    // and the filename next to the glyph carries the information the colour reinforces.
    const focused = hex(token(styles, "select-row-focus"));
    const below = (dir: string, name: string) =>
      Math.min(...fillsOf(readFileSync(resolve(dir, name), "utf8")).map((fill) => contrast(hex(fill), focused)));
    for (const name of ["visualstudio.svg", "css.svg", "less.svg", "tsconfig.svg"]) {
      expect(`${name}: ${below(fileDir, name) < 3}`).toBe(`${name}: true`);
    }
    for (const name of ["folder-github.svg", "folder-scripts.svg"]) {
      expect(`${name}: ${below(folderDir, name) < 3}`).toBe(`${name}: true`);
    }
    // Everything else clears the focused row too.
    for (const name of ["csharp.svg", "json.svg", "markdown.svg", "nuget.svg", "file.svg"]) {
      expect(`${name}: ${below(fileDir, name) >= 3}`).toBe(`${name}: true`);
    }
  });

  test("no Seti asset or token survived", () => {
    expect(styles).not.toContain("seti");
    expect(styles).not.toContain("--icon-");
    expect(readdirSync(resolve(root, "public"))).not.toContain("seti.woff");
  });
});
