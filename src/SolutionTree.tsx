import { useEffect, useMemo, useState, type MouseEvent, type ReactNode } from "react";
import { FileIcon } from "./FileIcon";
import { call, type EvaluationProject, type OutlineProject } from "./host/client";
import { Icon } from "./Icon";
import {
  ROOT_KEY, baseOf, countLabel, depsKey, dirKey, dirOf, folderKey, nestSources, type SourceNode,
} from "./solutionNodes";
import { TreeRow } from "./TreeRow";

export type TreeTarget =
  | { kind: "root" }
  | { kind: "project"; path: string; name: string }
  | { kind: "file"; path: string }
  | { kind: "dir"; path: string };

type Entry = { name: string; path: string; dir: boolean; hidden: boolean };

// One shape for the evaluated model and the text outline, so both render through the same rows.
type View = {
  name: string;
  path: string;
  kind: string;
  tfms: string[];
  folder: string | null;
  sources: string[];
  refs: string[];
  packages: string[];
  evaluated: boolean;
  failed: boolean;
  restore: EvaluationProject["restored"];
  error: string | null;
  note: string | null;
};

function toViews(evaluated: EvaluationProject[] | null, outline: OutlineProject[]): View[] {
  if (evaluated) {
    return evaluated.map((p) => ({
      name: p.name, path: p.path, kind: p.kind, tfms: p.tfms, folder: p.folder, sources: p.sources,
      refs: p.projectRefs.map((r) => r.name),
      packages: p.packageRefs.map((k) => (k.version ? `${k.name} ${k.version}` : `${k.name} (version unknown)`)),
      evaluated: true, failed: p.state === "failed", restore: p.restored, error: p.error, note: p.note,
    }));
  }
  return outline.map((p) => ({
    name: p.name, path: p.path, kind: p.kind, tfms: p.tfms, folder: null, sources: p.files,
    refs: p.references, packages: [], evaluated: false, failed: false, restore: "restored", error: null, note: null,
  }));
}

/** Every key that opens a node, for "Expand all" and the initial state (Dependencies stays closed). */
export function expandableKeys(evaluated: EvaluationProject[] | null, outline: OutlineProject[], deps = false) {
  const keys = new Set<string>([ROOT_KEY]);
  for (const view of toViews(evaluated, outline)) {
    keys.add(view.path);
    if (view.folder) keys.add(folderKey(view.folder));
    if (deps) keys.add(depsKey(view.path));
    for (const node of nestSources(view.path, view.sources)) collectDirs(node, keys);
  }
  return keys;
}

function collectDirs(node: SourceNode, keys: Set<string>) {
  if (node.kind !== "dir") return;
  keys.add(dirKey(node.path));
  node.children.forEach((child) => collectDirs(child, keys));
}

type Props = {
  workspaceName: string;
  evaluated: EvaluationProject[] | null;
  outline: OutlineProject[];
  expanded: Set<string>;
  onToggle: (key: string) => void;
  selectedProjectPath: string | null;
  onSelectProject: (path: string) => void;
  activePath: string | null;
  onOpenFile: (path: string) => void;
  restoreIncomplete: (projectPath: string) => boolean;
  showAll: boolean;
  filter: string;
  onMenu: (event: MouseEvent, target: TreeTarget) => void;
};

export function SolutionTree(p: Props) {
  const views = useMemo(() => toViews(p.evaluated, p.outline), [p.evaluated, p.outline]);
  const filter = p.filter.trim().toLowerCase();
  const hit = (text: string) => text.toLowerCase().includes(filter);
  const [listing, setListing] = useState<Record<string, Entry[]>>({});

  // Show all files: load one directory level at a time; toggling the mode off drops the cache.
  async function ensureDir(dir: string) {
    if (listing[dir]) return;
    try {
      const result = await call<{ entries: Entry[] }>("workspace.tree", { dir });
      setListing((current) => ({ ...current, [dir]: result.entries }));
    } catch {
      setListing((current) => ({ ...current, [dir]: [] }));
    }
  }
  const dirsToLoad = p.showAll ? ["", ...views.map((view) => dirOf(view.path))] : [];
  useEffect(() => {
    if (!p.showAll) setListing({});
    else dirsToLoad.forEach((dir) => void ensureDir(dir));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [p.showAll, views.length]);

  const sourceSet = new Set(views.flatMap((view) => view.sources));
  const projectDirs = new Set(views.map((view) => dirOf(view.path)));
  const ancestors = new Set(views.flatMap((view) => {
    const out: string[] = [];
    for (let dir = dirOf(view.path); dir; dir = dirOf(dir)) out.push(dir);
    return out;
  }));
  const projectFiles = new Set(views.map((view) => view.path));
  const open = (key: string) => (filter ? true : p.expanded.has(key));

  function fileRow(path: string, level: number, noIndex = false) {
    return (
      <TreeRow
        key={path}
        level={level}
        icon={<FileIcon path={path} />}
        name={baseOf(path)}
        noIndex={noIndex}
        testId={`tree-file-${path}`}
        current={p.activePath === path}
        selected={p.activePath === path}
        title={path}
        onClick={() => p.onOpenFile(path)}
        onContextMenu={(event) => p.onMenu(event, { kind: "file", path })}
      />
    );
  }

  function sourceRows(nodes: SourceNode[], level: number): ReactNode[] {
    return nodes.flatMap((node) => {
      if (node.kind === "file") return !filter || hit(node.name) ? [fileRow(node.path, level)] : [];
      const children = sourceRows(node.children, level + 1);
      if (filter && children.length === 0) return [];
      return [
        <TreeRow
          key={node.path}
          level={level}
          open={open(dirKey(node.path))}
          icon={<Icon name="folder" />}
          name={node.name}
          testId={`tree-dir-${node.path}`}
          onClick={() => p.onToggle(dirKey(node.path))}
          onContextMenu={(event) => p.onMenu(event, { kind: "dir", path: node.path })}
        />,
        ...(open(dirKey(node.path)) ? children : []),
      ];
    });
  }

  // "Show all files": the real directory, including bin/obj/dotfiles marked "no index".
  function diskRows(dir: string, level: number, skip: (entry: Entry) => boolean): ReactNode[] {
    return (listing[dir] ?? []).filter((entry) => !skip(entry) && (!filter || hit(entry.name))).flatMap((entry) => {
      if (!entry.dir) return [fileRow(entry.path, level, entry.hidden || !sourceSet.has(entry.path))];
      const key = dirKey(entry.path);
      return [
        <TreeRow
          key={entry.path}
          level={level}
          open={open(key)}
          icon={<Icon name="folder" />}
          name={entry.name}
          noIndex={entry.hidden}
          testId={`tree-dir-${entry.path}`}
          onClick={() => {
            p.onToggle(key);
            void ensureDir(entry.path);
          }}
          onContextMenu={(event) => p.onMenu(event, { kind: "dir", path: entry.path })}
        />,
        ...(open(key) ? diskRows(entry.path, level + 1, () => false) : []),
      ];
    });
  }

  function projectRows(view: View, level: number) {
    const isOpen = open(view.path);
    const selected = p.selectedProjectPath === view.path;
    const sources = filter ? view.sources.filter((source) => hit(baseOf(source))) : view.sources;
    if (filter && sources.length === 0 && !hit(view.name)) return null;
    const hasDeps = view.refs.length > 0 || view.packages.length > 0;
    return (
      <div key={view.path}>
        <TreeRow
          level={level}
          open={isOpen}
          icon={<FileIcon path={view.path} project />}
          name={view.name}
          testId={`tree-project-${view.name}`}
          selected={selected}
          title={view.path}
          onClick={() => {
            p.onToggle(view.path);
            p.onSelectProject(view.path);
          }}
          onContextMenu={(event) => {
            p.onSelectProject(view.path);
            p.onMenu(event, { kind: "project", path: view.path, name: view.name });
          }}
          suffix={
            <>
              <span className="tag">· {view.kind}</span>
              {view.failed && <span className="tag error">error</span>}
              {view.tfms.map((tfm) => <span className="tag" key={tfm}>· {tfm}</span>)}
              {view.evaluated && view.restore !== "restored" && (
                <span className="tag warning">{view.restore === "unknown" ? "restore unknown" : "partial context"}</span>
              )}
              {p.restoreIncomplete(view.path) && <span className="tag warning">restore incomplete</span>}
            </>
          }
        />
        {isOpen && (
          <>
            {view.error && <div className="tree-note error" data-testid={`eval-project-error-${view.name}`}>Evaluation failed: {view.error}</div>}
            {view.note && <div className="tree-note">{view.note}</div>}
            {hasDeps && !filter && (
              <>
                <TreeRow
                  level={level + 1}
                  open={p.expanded.has(depsKey(view.path))}
                  icon={<Icon name="dependencies" />}
                  name="Dependencies"
                  testId={`tree-deps-${view.name}`}
                  onClick={() => p.onToggle(depsKey(view.path))}
                />
                {p.expanded.has(depsKey(view.path)) && (
                  <>
                    {view.refs.map((ref) => (
                      <TreeRow key={`ref:${ref}`} level={level + 2} icon={<Icon name="project" />} name={ref} suffix={<span className="tag">· project</span>} onClick={() => undefined} />
                    ))}
                    {view.packages.map((pkg) => (
                      <TreeRow key={`pkg:${pkg}`} level={level + 2} icon={<Icon name="package" />} name={pkg} suffix={<span className="tag">· package</span>} onClick={() => undefined} />
                    ))}
                  </>
                )}
              </>
            )}
            {p.showAll && !filter
              ? diskRows(dirOf(view.path), level + 1, (entry) => !entry.dir && projectFiles.has(entry.path))
              : sourceRows(nestSources(view.path, sources), level + 1)}
            {view.sources.length === 0 && !view.error && !p.showAll && (
              <div className="tree-note">{view.evaluated ? "No source files evaluated." : "No .cs files found (bin/obj hidden)."}</div>
            )}
          </>
        )}
      </div>
    );
  }

  const groups = new Map<string, View[]>();
  for (const view of views) groups.set(view.folder ?? "", [...(groups.get(view.folder ?? "") ?? []), view]);
  const rootOpen = open(ROOT_KEY);

  return (
    <div className="tree sol-tree">
      <TreeRow
        level={0}
        open={rootOpen}
        icon={<Icon name="solution" />}
        name={p.workspaceName.replace(/\.(slnx?|csproj)$/i, "")}
        testId="tree-root"
        suffix={<span className="tag tree-count">· {countLabel(views.length, "project")}</span>}
        onClick={() => p.onToggle(ROOT_KEY)}
        onContextMenu={(event) => p.onMenu(event, { kind: "root" })}
      />
      {rootOpen && (
        <>
          {Array.from(groups.entries()).map(([folder, items]) => {
            const rows = items.map((view) => projectRows(view, folder ? 2 : 1)).filter(Boolean);
            if (filter && rows.length === 0) return null;
            return (
              <div key={folder || "root"}>
                {folder && (
                  <TreeRow
                    level={1}
                    open={open(folderKey(folder))}
                    icon={<Icon name="solutionFolder" />}
                    name={folder}
                    testId={`tree-folder-${folder}`}
                    suffix={<span className="tag tree-count">· {countLabel(items.length, "project")}</span>}
                    onClick={() => p.onToggle(folderKey(folder))}
                  />
                )}
                {(!folder || open(folderKey(folder))) && rows}
              </div>
            );
          })}
          {p.showAll && !filter && diskRows("", 1, (entry) => (entry.dir && (ancestors.has(entry.path) || projectDirs.has(entry.path))) || /\.slnx?$/.test(entry.name) || projectFiles.has(entry.path))}
        </>
      )}
    </div>
  );
}
