import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { fileKind, fileToken, type FileKind } from "../../src/solutionVisuals";

// The palette is data: this reads the shipped CSS, not a copy of it. A token that fails the floor
// on any row surface fails here, before the app is opened.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const styles = readFileSync(resolve(root, "src/styles.css"), "utf8");
const treeStyles = readFileSync(resolve(root, "src/solution-icons.css"), "utf8");

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

function rgba(value: string): { rgb: Rgb; alpha: number } {
  const match = /^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+)\s*)?\)$/.exec(value);
  if (!match) throw new Error(`not an rgb colour: ${value}`);
  return {
    rgb: [Number(match[1]), Number(match[2]), Number(match[3])],
    alpha: match[4] === undefined ? 1 : Number(match[4]),
  };
}

function over(foreground: Rgb, background: Rgb, alpha: number): Rgb {
  return [0, 1, 2].map((i) => Math.round(alpha * foreground[i] + (1 - alpha) * background[i])) as Rgb;
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

const panel = hex(token(styles, "panel"));
const hover = over(rgba(token(styles, "accent-surface")).rgb, panel, rgba(token(styles, "accent-surface")).alpha);
const noIndex = over(rgba(/\.sol-row\.no-index\s*\{\s*background:\s*([^;]+);/.exec(treeStyles)![1]).rgb, panel, rgba(/\.sol-row\.no-index\s*\{\s*background:\s*([^;]+);/.exec(treeStyles)![1]).alpha);
const noIndexHover = over(rgba(/\.sol-row\.no-index:hover\s*\{\s*background:\s*([^;]+);/.exec(treeStyles)![1]).rgb, panel, rgba(/\.sol-row\.no-index:hover\s*\{\s*background:\s*([^;]+);/.exec(treeStyles)![1]).alpha);

/** Every surface a tree row can paint behind a glyph. */
const SURFACES: Record<string, Rgb> = {
  panel,
  hover,
  raised: hex(token(styles, "raised")),
  select: hex(token(styles, "select")),
  "select-row-focus": hex(token(styles, "select-row-focus")),
  "no-index": noIndex,
  "no-index hover": noIndexHover,
};

/** The coloured `--icon-*` tokens of the dark theme, in declaration order. */
const iconTokens = [...styles.matchAll(/--(icon-[a-z-]+):\s*(#[0-9a-f]{6});/gi)].map((match) => match[1]);

const KINDS: FileKind[] = [
  "csharp", "solution", "project", "dotsettings", "props", "json", "xml", "razor", "resource",
  "markdown", "text", "yaml", "docker", "git", "config", "script", "powershell", "batch", "css",
  "html", "image", "license", "file",
];

describe("Solution Explorer palette", () => {
  test("the tree uses at most ten coloured tokens", () => {
    // The ceiling is the point of the palette: colour only where it disambiguates.
    expect(iconTokens.length <= 10).toBe(true);
  });

  test("every token a kind names exists in the CSS", () => {
    for (const kind of KINDS) {
      const name = fileToken(kind);
      if (name === "muted") {
        expect(token(styles, "muted")).not.toBeUndefined();
      } else {
        expect(iconTokens).toContain(`icon-${name}`);
      }
    }
  });

  test("every coloured token clears 3:1 on every row surface", () => {
    const failures: string[] = [];
    for (const name of iconTokens) {
      const colour = hex(token(styles, name));
      for (const [surface, background] of Object.entries(SURFACES)) {
        const ratio = contrast(colour, background);
        if (ratio < 3) failures.push(`--${name} on ${surface}: ${ratio.toFixed(2)}:1`);
      }
    }
    expect(failures).toEqual([]);
  });

  test("--icon-git is raised above the Seti reference value", () => {
    // Seti's `_git` #41535b measures 1.55:1 on the focused row; the shipped value has to pass.
    const git = hex(token(styles, "icon-git"));
    expect(git).not.toEqual([0x41, 0x53, 0x5b]);
    expect(contrast(git, SURFACES["select-row-focus"]) >= 3).toBe(true);
  });

  test("structure carries no type colour", () => {
    // Folders, Dependencies, Frameworks and Packages are structure: --muted, never an --icon-*.
    for (const name of iconTokens) {
      expect(["icon-solution", "icon-solution-folder", "icon-folder", "icon-dependencies", "icon-package", "icon-framework"]).not.toContain(name);
    }
    expect(fileToken(fileKind("App.sln"))).toBe("muted");
    expect(fileToken(fileKind("tool.exe"))).toBe("muted");
  });
});
