// Pure helpers for the Solution tree: expansion keys, nested sources and Rider-style counts.
export const ROOT_KEY = "solution:root";
export const folderKey = (folder: string) => `solution:folder:${folder}`;
export const depsKey = (project: string) => `solution:deps:${project}`;
export const dirKey = (dir: string) => `solution:dir:${dir}`;

export const dirOf = (path: string) => (path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "");
export const baseOf = (path: string) => path.split("/").pop() ?? path;
export const countLabel = (count: number, noun: string) => `${count} ${noun}${count === 1 ? "" : "s"}`;

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
