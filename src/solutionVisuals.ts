// Pure mapping from paths/project kinds to the visual language of the Solution tree.
// The drawings are the Material Icon Theme SVGs in `src/assets/`; this module only names the
// icon file, never a hex value and never a path into the bundle.
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

/**
 * The icon a file row draws, named after the Material Icon Theme SVG it comes from
 * (`src/assets/file-icons/<name>.svg`). Several file kinds share one icon (e.g. `.sln` and `.resx`),
 * and one kind can draw more than one icon (`.json` and `package.json`).
 * Provenance, commit and license: THIRD-PARTY-NOTICES.md.
 */
export type FileIconName =
  | "csharp"
  | "visualstudio"
  | "fsharp"
  | "xml"
  | "settings"
  | "json"
  | "xaml"
  | "razor"
  | "markdown"
  | "mdx"
  | "yaml"
  | "document"
  | "console"
  | "powershell"
  | "css"
  | "sass"
  | "less"
  | "html"
  | "image"
  | "svg"
  | "git"
  | "editorconfig"
  | "docker"
  | "license"
  | "readme"
  | "nodejs"
  | "tsconfig"
  | "nuget"
  | "tune"
  | "file";

/**
 * The folder drawing, named after the Material Icon Theme SVG it comes from
 * (`src/assets/folder-icons/<name>.svg` and `<name>-open.svg`).
 */
export type FolderIconName =
  | "folder"
  | "folder-src"
  | "folder-test"
  | "folder-docs"
  | "folder-config"
  | "folder-public"
  | "folder-dist"
  | "folder-github"
  | "folder-vscode"
  | "folder-scripts"
  | "folder-resource"
  | "folder-controller"
  | "folder-class"
  | "folder-views"
  | "folder-migrations";

/**
 * The drawing a kind gets when the theme has no answer for the whole name or the extension.
 * The theme resolves by name and by extension; this is the fallback layer under both.
 */
const KIND_ICON: Record<FileKind, FileIconName> = {
  csharp: "csharp",
  solution: "visualstudio",
  project: "visualstudio",
  dotsettings: "settings",
  props: "settings",
  json: "json",
  xml: "xml",
  razor: "razor",
  resource: "xml",
  markdown: "markdown",
  text: "document",
  yaml: "yaml",
  docker: "docker",
  git: "git",
  config: "settings",
  script: "console",
  powershell: "powershell",
  batch: "console",
  css: "css",
  html: "html",
  image: "image",
  license: "license",
  file: "file",
};

/**
 * Whole names where the theme draws something other than the kind's default. Written from the
 * theme's `fileNames` table at the pinned commit; a name the theme does not list is not here.
 */
const ICON_NAMES: Record<string, FileIconName> = {
  ".editorconfig": "editorconfig",
  ".env": "tune",
  "nuget.config": "nuget",
  "package.json": "nodejs",
  "tsconfig.json": "tsconfig",
  readme: "readme",
  "readme.md": "readme",
};

/**
 * Extensions where the theme draws something other than the kind's default. The longest matching
 * suffix wins, as in the theme: `sln.dotsettings` beats `dotsettings`, which beats nothing.
 */
const ICON_EXTENSIONS: Record<string, FileIconName> = {
  "sln.dotsettings.user": "settings",
  "sln.dotsettings": "settings",
  "dotsettings.user": "file",
  dotsettings: "file",
  fsproj: "fsharp",
  vbproj: "xml",
  targets: "xml",
  config: "settings",
  xaml: "xaml",
  resources: "file",
  mdx: "mdx",
  scss: "sass",
  less: "less",
  svg: "svg",
};

const ICON_EXTENSIONS_BY_LENGTH = Object.keys(ICON_EXTENSIONS).sort((a, b) => b.length - a.length);

/**
 * The folder names the theme maps to each shipped folder icon, lowercase and without the leading
 * dot (the theme registers both `.github` and `github`). Copied from the theme's `folderNames` at
 * the pinned commit, restricted to the plain names a .NET solution uses.
 */
const FOLDER_NAMES: Record<FolderIconName, string[]> = {
  folder: [],
  "folder-src": ["src", "srcs", "source", "sources", "code"],
  "folder-test": ["test", "tests", "testing", "spec", "specs", "snapshots", "testfiles"],
  "folder-docs": ["doc", "docs", "document", "documents", "documentation", "wiki", "note", "notes"],
  "folder-config": [
    "config", "configs", "configuration", "configurations", "conf", "confs", "cfg", "cfgs",
    "setting", "settings", "props", "properties", "option", "options", "pref", "prefs",
    "preference", "preferences", "meta-inf",
  ],
  "folder-public": [
    "public", "www", "wwwroot", "web", "website", "websites", "site", "browser", "browsers",
    "proxy",
  ],
  "folder-dist": [
    "bin", "build", "builds", "built", "compiled", "dist", "distribution", "out", "output",
    "outputs", "release",
  ],
  "folder-github": ["github"],
  "folder-vscode": ["vscode", "vscode-test"],
  "folder-scripts": ["script", "scripts", "scripting", "xtask"],
  "folder-resource": ["asset", "assets", "resource", "resources", "res", "static", "report", "reports"],
  "folder-controller": [
    "controller", "controllers", "controls", "service", "services", "provider", "providers",
    "handler", "handlers",
  ],
  "folder-class": ["class", "classes", "model", "models", "schema", "schemas"],
  "folder-views": ["view", "views", "page", "pages", "screen", "screens", "html", "public_html"],
  "folder-migrations": ["migration", "migrations"],
};

const FOLDER_BY_NAME = new Map<string, FolderIconName>();
for (const [icon, names] of Object.entries(FOLDER_NAMES) as [FolderIconName, string[]][]) {
  for (const name of names) FOLDER_BY_NAME.set(name, icon);
}

/**
 * Icon of a file row. The precedence is the theme's: whole name, then the longest matching
 * extension, then the kind's default — which is where `.sln` and `.csproj` both land on the
 * theme's single Visual Studio drawing.
 */
export function fileIcon(path: string): FileIconName {
  const name = path.split("/").pop() ?? path;
  const lower = name.toLowerCase();
  const byName = ICON_NAMES[lower];
  if (byName) return byName;
  for (const extension of ICON_EXTENSIONS_BY_LENGTH) {
    if (lower === extension || lower.endsWith(`.${extension}`)) return ICON_EXTENSIONS[extension];
  }
  return KIND_ICON[fileKind(path)];
}

/** Icon of a folder row; a name the theme does not know gets the generic folder. */
export function folderIcon(name: string): FolderIconName {
  return FOLDER_BY_NAME.get(name.toLowerCase().replace(/^\./, "")) ?? "folder";
}

/**
 * Whole names beat extensions (`Dockerfile`, `global.json`, `LICENSE`). The theme's `languageIds`
 * level has no equivalent here: OpenRidr has no language registry, so the extension carries it.
 */
const FILE_NAMES: Record<string, FileKind> = {
  dockerfile: "docker",
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

/**
 * Second layer of a project row: `kind · TFM · restore`, only the parts that exist. The kind is
 * text here because the glyph is `aria-hidden`; the row's accessible name carries it too.
 */
export function projectMeta(kind: ProjectKind, tfms: string[], restore: string | null): string {
  const parts: string[] = [];
  if (kind !== "unknown") parts.push(kind);
  if (tfms.length > 0) parts.push(tfms.join(", "));
  if (restore) parts.push(restore);
  return parts.join(" · ");
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
