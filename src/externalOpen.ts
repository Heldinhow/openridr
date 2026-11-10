import { workspaceRelative } from "./solutionNodes";

/**
 * Text formats the editor can show. The Finder/CLI "open with" hands over a path; a code file
 * inside the open workspace opens as a tab instead of being refused as a workspace (#373).
 * Anything else keeps the workspace error, so a binary is never read as text.
 */
const CODE_EXTENSIONS = new Set([
  "cs",
  "csx",
  "razor",
  "cshtml",
  "json",
  "xml",
  "props",
  "targets",
  "config",
  "resx",
  "md",
  "txt",
  "yml",
  "yaml",
  "sh",
  "ps1",
  "js",
  "mjs",
  "cjs",
  "jsx",
  "ts",
  "tsx",
  "css",
  "html",
  "htm",
  "editorconfig",
]);

/**
 * The workspace-relative path when `path` is a code file inside `workspaceDir`, else null.
 * A path outside the workspace (or one that only shares the prefix) is never made relative:
 * `workspaceRelative` returns it unchanged and the shape check below rejects it.
 */
export function workspaceFileTarget(path: string, workspaceDir: string): string | null {
  const relative = workspaceRelative(path, workspaceDir);
  if (!relative) return null;
  if (relative.startsWith("/") || relative.startsWith("../") || relative.includes("/../")) return null;
  if (/^[A-Za-z]:/.test(relative)) return null;
  const name = relative.split("/").pop() ?? "";
  const dot = name.lastIndexOf(".");
  if (dot <= 0) return null;
  return CODE_EXTENSIONS.has(name.slice(dot + 1).toLowerCase()) ? relative : null;
}
