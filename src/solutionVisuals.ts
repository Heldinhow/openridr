// Pure mapping from paths/project kinds to the visual language of the Solution tree.
// Colors live in styles.css (`--icon-*`); this module only names the kind, never a hex value.
export type SymbolKind = "class" | "interface" | "enum" | "record" | "struct";
export type ProjectKind = "console" | "library" | "test" | "web" | "unknown";
export type FileKind =
  | "csharp"
  | "solution"
  | "project"
  | "dotsettings"
  | "props"
  | "json"
  | "xml"
  | "razor"
  | "resource"
  | "markdown"
  | "text"
  | "yaml"
  | "docker"
  | "git"
  | "config"
  | "script"
  | "powershell"
  | "batch"
  | "css"
  | "html"
  | "image"
  | "license"
  | "file";

/** The glyph a row draws; several file kinds share one glyph (e.g. `.sln` and `.resx`). */
export type FileGlyph =
  | "csharp"
  | "doc"
  | "project"
  | "json"
  | "markdown"
  | "docker"
  | "yaml"
  | "text"
  | "git"
  | "xml"
  | "razor"
  | "resource"
  | "config"
  | "script"
  | "powershell"
  | "batch"
  | "css"
  | "html"
  | "image"
  | "license"
  | "file";

/**
 * Tint of a glyph: the Seti reference palette, which is a handful of families, not one colour per
 * type. `muted` is structure (no type information); the values live in styles.css.
 */
export type IconToken =
  | "seti-blue"
  | "seti-yellow"
  | "seti-purple"
  | "seti-green"
  | "seti-orange"
  | "git"
  | "muted";

/**
 * The derived table: one row per kind, written by hand from the Seti theme
 * (`vs-seti-icon-theme.json`, vendored font; see THIRD-PARTY-NOTICES.md). The theme JSON itself is
 * never loaded: the bundle carries only the font and this table.
 */
const FILE_TYPES: Record<FileKind, { glyph: FileGlyph; token: IconToken }> = {
  csharp: { glyph: "csharp", token: "seti-blue" },
  solution: { glyph: "doc", token: "muted" },
  project: { glyph: "project", token: "seti-orange" },
  dotsettings: { glyph: "text", token: "muted" },
  props: { glyph: "xml", token: "seti-orange" },
  json: { glyph: "json", token: "seti-yellow" },
  xml: { glyph: "xml", token: "seti-orange" },
  razor: { glyph: "razor", token: "seti-blue" },
  resource: { glyph: "resource", token: "muted" },
  markdown: { glyph: "markdown", token: "seti-blue" },
  text: { glyph: "text", token: "muted" },
  yaml: { glyph: "yaml", token: "seti-purple" },
  docker: { glyph: "docker", token: "seti-blue" },
  git: { glyph: "git", token: "git" },
  config: { glyph: "config", token: "muted" },
  script: { glyph: "script", token: "seti-green" },
  powershell: { glyph: "powershell", token: "seti-blue" },
  batch: { glyph: "batch", token: "seti-blue" },
  css: { glyph: "css", token: "seti-blue" },
  html: { glyph: "html", token: "seti-blue" },
  image: { glyph: "image", token: "seti-purple" },
  license: { glyph: "license", token: "seti-yellow" },
  file: { glyph: "file", token: "muted" },
};

export function fileGlyph(kind: FileKind): FileGlyph {
  return FILE_TYPES[kind].glyph;
}

/** Token that tints a kind's glyph; the value is `--icon-<token>` in styles.css. */
export function fileToken(kind: FileKind): IconToken {
  return FILE_TYPES[kind].token;
}

/**
 * Whole names beat extensions (`Dockerfile`, `global.json`, `LICENSE`). The theme's `languageIds`
 * level has no equivalent here: OpenRidr has no language registry, so the extension carries it.
 */
const FILE_NAMES: Record<string, FileKind> = {
  dockerfile: "docker",
  "docker-compose.yml": "docker",
  "docker-compose.yaml": "docker",
  "docker-compose.override.yml": "docker",
  "docker-compose.override.yaml": "docker",
  "compose.yml": "docker",
  "compose.yaml": "docker",
  ".dockerignore": "docker",
  ".gitignore": "git",
  ".gitattributes": "git",
  ".gitmodules": "git",
  ".editorconfig": "config",
  ".env": "config",
  "global.json": "json",
  "appsettings.json": "json",
  "package.json": "json",
  "tsconfig.json": "json",
  "nuget.config": "xml",
  license: "license",
  "license.md": "license",
  "license.txt": "license",
  copying: "license",
};

/** The longest matching suffix wins: `docker-compose.override.yml` before `yml`. */
const FILE_EXTENSIONS: Record<string, FileKind> = {
  cs: "csharp",
  csx: "csharp",
  sln: "solution",
  slnx: "solution",
  csproj: "project",
  fsproj: "project",
  vbproj: "project",
  props: "props",
  targets: "props",
  json: "json",
  jsonc: "json",
  xml: "xml",
  config: "xml",
  xaml: "xml",
  razor: "razor",
  cshtml: "razor",
  resx: "resource",
  resources: "resource",
  md: "markdown",
  markdown: "markdown",
  mdx: "markdown",
  yml: "yaml",
  yaml: "yaml",
  "docker-compose.yml": "docker",
  "docker-compose.yaml": "docker",
  "docker-compose.override.yml": "docker",
  "docker-compose.override.yaml": "docker",
  txt: "text",
  sh: "script",
  bash: "script",
  zsh: "script",
  ps1: "powershell",
  psm1: "powershell",
  cmd: "batch",
  bat: "batch",
  css: "css",
  scss: "css",
  less: "css",
  html: "html",
  htm: "html",
  dotsettings: "dotsettings",
  "dotsettings.user": "dotsettings",
  gitattributes: "git",
  gitmodules: "git",
  png: "image",
  jpg: "image",
  jpeg: "image",
  gif: "image",
  webp: "image",
  bmp: "image",
  ico: "image",
  svg: "image",
};

/** Longest first, so the first suffix that matches is the most specific one. */
const EXTENSIONS_BY_LENGTH = Object.keys(FILE_EXTENSIONS).sort((a, b) => b.length - a.length);

/** Type of a file row; only the name is read, never the contents. */
export function fileKind(path: string): FileKind {
  const name = path.split("/").pop() ?? path;
  const lower = name.toLowerCase();
  const byName = FILE_NAMES[lower];
  if (byName) return byName;
  // `Dockerfile.dev` is a Dockerfile; the whole-name table cannot enumerate every suffix.
  if (lower === "dockerfile" || lower.startsWith("dockerfile.")) return "docker";
  for (const extension of EXTENSIONS_BY_LENGTH) {
    if (lower === extension || lower.endsWith(`.${extension}`)) return FILE_EXTENSIONS[extension];
  }
  return "file";
}

/** A project that MSBuild says is `Exe` but whose text declares the Web SDK is a web project. */
export function projectKindOf(
  evaluatedKind: string | null | undefined,
  textKind: string | null | undefined,
): ProjectKind {
  const evaluated = evaluatedKind?.toLowerCase() ?? "";
  if (evaluated === "test") return "test";
  if (evaluated === "web") return "web";
  if (textKind?.toLowerCase() === "web") return "web";
  if (evaluated === "console" || evaluated === "library") return evaluated;
  const text = textKind?.toLowerCase() ?? "";
  if (text === "test" || text === "console" || text === "library" || text === "web") return text;
  return "unknown";
}

/** Framework row label derived from the TFM only; no version is invented. */
export function frameworkName(tfm: string, kind: ProjectKind): string | null {
  const normalized = tfm.trim();
  if (!normalized) return null;
  if (/^net\d+\.\d+$/i.test(normalized)) {
    return `${kind === "web" ? "Microsoft.AspNetCore.App" : "Microsoft.NETCore.App"} (${normalized})`;
  }
  if (/^netstandard\d/i.test(normalized)) return `.NET Standard (${normalized})`;
  if (/^netcoreapp\d/i.test(normalized)) return `Microsoft.NETCore.App (${normalized})`;
  if (/^net\d{2,3}$/i.test(normalized)) return `.NET Framework (${normalized})`;
  return normalized;
}

export function frameworkRows(tfms: string[], kind: ProjectKind): string[] {
  return tfms
    .map((tfm) => frameworkName(tfm, kind))
    .filter((label): label is string => label !== null);
}

export type PackageRefLike = { name: string; version: string | null };

/** One row per name@version (plus TFM when a caller has one); packages never share a React key. */
export function packageKey(pkg: PackageRefLike, tfm?: string | null): string {
  return `${pkg.name}@${pkg.version ?? "unknown"}${tfm ? `:${tfm}` : ""}`;
}

export function dedupePackages(packages: PackageRefLike[]): PackageRefLike[] {
  const seen = new Set<string>();
  return packages.filter((pkg) => {
    const key = packageKey(pkg);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

const SYMBOL_KINDS: SymbolKind[] = ["class", "interface", "enum", "record", "struct"];

/** Guard for the host answer: only the five declared kinds reach the badge. */
export function symbolKindOf(value: unknown): SymbolKind | undefined {
  return typeof value === "string" && (SYMBOL_KINDS as string[]).includes(value)
    ? (value as SymbolKind)
    : undefined;
}
