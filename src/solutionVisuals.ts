// Pure mapping from paths/project kinds to the visual language of the Solution tree.
// Colors live in styles.css (`--icon-*`); this module only names the kind, never a hex value.
export type SymbolKind = "class" | "interface" | "enum" | "record" | "struct";
export type ProjectKind = "console" | "library" | "test" | "web" | "unknown";
export type FileKind =
  | "csharp"
  | "solution"
  | "project"
  | "props"
  | "json"
  | "xml"
  | "razor"
  | "resource"
  | "markdown"
  | "text"
  | "yaml"
  | "config"
  | "script"
  | "file";

const BY_EXTENSION: Record<string, FileKind> = {
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
  razor: "razor",
  cshtml: "razor",
  resx: "resource",
  resources: "resource",
  md: "markdown",
  markdown: "markdown",
  yml: "yaml",
  yaml: "yaml",
  editorconfig: "config",
  gitignore: "config",
  gitattributes: "config",
  gitmodules: "config",
  sh: "script",
  bash: "script",
  zsh: "script",
  ps1: "script",
  cmd: "script",
  bat: "script",
  txt: "text",
};

/** Type of a file row; only the extension is read, never the contents. */
export function fileKind(path: string): FileKind {
  const name = path.split("/").pop() ?? path;
  const lower = name.toLowerCase();
  if (lower.startsWith(".") && !lower.slice(1).includes(".")) {
    // Dotfiles: `.editorconfig` has an extension-like suffix; `.gitignore` and `.env` are names.
    return BY_EXTENSION[lower.slice(1)] ?? "config";
  }
  const dot = lower.lastIndexOf(".");
  if (dot < 0) return "file";
  const ext = lower.slice(dot + 1);
  return BY_EXTENSION[ext] ?? (ext.endsWith("proj") ? "project" : "file");
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
