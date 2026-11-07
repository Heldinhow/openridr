// Pure helpers for the Solution tree: expansion keys, nested sources and Rider-style counts.
export const ROOT_KEY = "solution:root";
export const folderKey = (folder: string) => `solution:folder:${folder}`;
export const depsKey = (project: string) => `solution:deps:${project}`;
export type DependencyGroup = "frameworks" | "packages" | "projects";
export const depGroupKey = (project: string, group: DependencyGroup) =>
  `solution:deps:${project}:${group}`;
export const dirKey = (dir: string) => `solution:dir:${dir}`;

export const dirOf = (path: string) => (path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "");

/** True when a model path is absolute — how a project outside the workspace root is named. */
export const isAbsolutePath = (path: string) => path.startsWith("/") || /^[A-Za-z]:[\\/]/.test(path);
export const baseOf = (path: string) => path.split("/").pop() ?? path;
export const countLabel = (count: number, noun: string) => `${count} ${noun}${count === 1 ? "" : "s"}`;

/** Workspace-relative tab path: normalize separators and strip the workspace root from absolute paths. */
export function workspaceRelative(path: string, workspaceDir: string): string {
  const normalized = path.replace(/\\/g, "/");
  const base = workspaceDir.replace(/\\/g, "/").replace(/\/+$/, "");
  // A solution at the filesystem root has an empty workspaceDir; absolute paths are already
  // relative to `/`, so only the leading separator has to go.
  if (!base) return normalized.startsWith("/") ? normalized.slice(1) : normalized;
  if (normalized === base) return "";
  return normalized.startsWith(`${base}/`) ? normalized.slice(base.length + 1) : normalized;
}

export type SourceNode =
  | { kind: "dir"; name: string; path: string; children: SourceNode[] }
  | { kind: "file"; name: string; path: string };

/** Nests workspace-relative source paths under the project folder; folders first, A-Z. */
export function nestSources(projectPath: string, sources: string[]): SourceNode[] {
  const base = dirOf(projectPath);
  const top: SourceNode[] = [];
  for (const source of [...sources].sort((a, b) => a.localeCompare(b))) {
    const rel = base && source.startsWith(`${base}/`) ? source.slice(base.length + 1) : source;
    const parts = rel.split("/");
    let level = top;
    let dir = base;
    for (const part of parts.slice(0, -1)) {
      dir = dir ? `${dir}/${part}` : part;
      let node = level.find((item) => item.kind === "dir" && item.name === part) as
        | Extract<SourceNode, { kind: "dir" }>
        | undefined;
      if (!node) {
        node = { kind: "dir", name: part, path: dir, children: [] };
        level.push(node);
      }
      level = node.children;
    }
    level.push({ kind: "file", name: parts[parts.length - 1], path: source });
  }
  const order = (nodes: SourceNode[]) => {
    nodes.sort((a, b) => (a.kind === b.kind ? 0 : a.kind === "dir" ? -1 : 1));
    for (const node of nodes) if (node.kind === "dir") order(node.children);
  };
  order(top);
  return top;
}

/** Files under folders that are open; top-level files are always visible. */
export function visibleSourceFiles(
  nodes: SourceNode[],
  open: (key: string) => boolean,
  out: string[] = [],
): string[] {
  for (const node of nodes) {
    if (node.kind === "file") out.push(node.path);
    else if (open(dirKey(node.path))) visibleSourceFiles(node.children, open, out);
  }
  return out;
}
