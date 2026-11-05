import { useEffect, useMemo, useState, type MouseEvent, type ReactNode } from "react";
import { FileIcon } from "./FileIcon";
import { call, type EvaluatedPackageRef, type EvaluationProject, type OutlineProject } from "./host/client";
import { Icon } from "./Icon";
import { ProjectIcon } from "./icons/ProjectIcon";
import {
  ROOT_KEY, baseOf, countLabel, depGroupKey, depsKey, dirKey, dirOf, folderKey, nestSources,
  type DependencyGroup, type SourceNode,
} from "./solutionNodes";
import { fileKind, frameworkRows, projectKindOf, symbolKindOf, type ProjectKind, type SymbolKind } from "./solutionVisuals";
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
  /** Kind read from the project text, used for `web` when MSBuild reports `Exe`. */
  textKind: string | null;
  tfms: string[];
  folder: string | null;
  sources: string[];
  refs: string[];
  packages: EvaluatedPackageRef[];
  evaluated: boolean;
  failed: boolean;
  restore: EvaluationProject["restored"];
  error: string | null;
  note: string | null;
};

function toViews(evaluated: EvaluationProject[] | null, outline: OutlineProject[]): View[] {
  const outlineByPath = new Map(outline.map((project) => [project.path, project]));
  if (evaluated) {
    return evaluated.map((p) => ({
      // A failed evaluation has no sources; fall back to the text outline plus the project file itself.
      sources: p.state === "failed" && p.sources.length === 0
        ? [...(outlineByPath.get(p.path)?.files ?? []), p.path]
        : p.sources,
      name: p.name, path: p.path, kind: p.kind, textKind: outlineByPath.get(p.path)?.kind ?? null,
      tfms: p.tfms, folder: p.folder,
      refs: p.projectRefs.map((r) => r.name),
      packages: p.packageRefs,
      evaluated: true, failed: p.state === "failed", restore: p.restored, error: p.error, note: p.note,
    }));
  }
  return outline.map((p) => ({
    name: p.name, path: p.path, kind: p.kind, textKind: p.kind, tfms: p.tfms, folder: null, sources: p.files,
    refs: p.references, packages: [], evaluated: false, failed: false, restore: "restored", error: null, note: null,
  }));
}

/** Every key that opens a node, for "Expand all" and the initial state (Dependencies stays closed). */
export function expandableKeys(evaluated: EvaluationProject[] | null, outline: OutlineProject[], deps = false) {
  const keys = new Set<string>([ROOT_KEY]);
  for (const view of toViews(evaluated, outline)) {
    keys.add(view.path);
    if (view.folder) keys.add(folderKey(view.folder));
    if (deps) {
      const kind = projectKindOf(view.kind, view.textKind);
      const frameworks = frameworkRows(view.tfms, kind);
      const hasDeps = frameworks.length > 0 || view.packages.length > 0 || view.refs.length > 0;
      if (hasDeps) {
        keys.add(depsKey(view.path));
        if (frameworks.length > 0) keys.add(depGroupKey(view.path, "frameworks"));
        if (view.packages.length > 0) keys.add(depGroupKey(view.path, "packages"));
        if (view.refs.length > 0) keys.add(depGroupKey(view.path, "projects"));
      }
    }
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
  const [symbolKinds, setSymbolKinds] = useState<Record<string, SymbolKind>>({});

  // One read-only scan per set of evaluated sources; a missing/failed answer keeps the C# glyph.
  const sourceSignature = useMemo(
    () => Array.from(new Set(views.flatMap((view) => view.sources)))
      .filter((path) => path.toLowerCase().endsWith(".cs"))
      .sort()
      .join("\n"),
    [views],
  );
  useEffect(() => {
    const paths = sourceSignature ? sourceSignature.split("\n") : [];
    if (paths.length === 0) {
      setSymbolKinds({});
      return;
    }
    let alive = true;
    void call<{ kinds: Record<string, string> }>("workspace.symbolKinds", { paths })
      .then((result) => {
        if (!alive) return;
        const next: Record<string, SymbolKind> = {};
        for (const [path, kind] of Object.entries(result.kinds ?? {})) {
          const parsed = symbolKindOf(kind);
          if (parsed) next[path] = parsed;
        }
        setSymbolKinds(next);
      })
      .catch(() => {
        if (alive) setSymbolKinds({});
      });
    return () => {
      alive = false;
    };
  }, [sourceSignature]);

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
    const owner = views.find((view) => view.path === path);
    const projectKind = owner ? projectKindOf(owner.kind, owner.textKind) : undefined;
    const kind = fileKind(path);
    return (
      <TreeRow
        key={path}
        level={level}
        icon={<FileIcon path={path} projectKind={projectKind} symbolKind={symbolKinds[path]} />}
        name={baseOf(path)}
        noIndex={noIndex}
        testId={`tree-file-${path}`}
        fileKind={kind}
        symbolKind={symbolKinds[path]}
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
          icon={<Icon name="folder" className="tint-folder" />}
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
          icon={<Icon name="folder" className="tint-folder" />}
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
    const kind = projectKindOf(view.kind, view.textKind);
    const frameworks = frameworkRows(view.tfms, kind);
    const hasDeps = view.refs.length > 0 || view.packages.length > 0 || frameworks.length > 0;
    const groupRow = (group: DependencyGroup, name: string, icon: ReactNode, items: ReactNode[]) => {
      if (items.length === 0) return null;
      const key = depGroupKey(view.path, group);
      return (
        <>
          <TreeRow
            level={level + 2}
            open={p.expanded.has(key)}
            icon={icon}
            name={name}
            testId={`tree-deps-group-${view.name}-${group}`}
            onClick={() => p.onToggle(key)}
          />
          {p.expanded.has(key) && items}
        </>
      );
    };
    return (
      <div key={view.path}>
        <TreeRow
          level={level}
          open={isOpen}
          icon={<ProjectIcon kind={kind} />}
          name={view.name}
          testId={`tree-project-${view.name}`}
          selected={selected}
          title={view.path}
          projectKind={kind}
          projectTfms={view.tfms.join(" ")}
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
              {view.failed && <span className="tag error">error</span>}
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
                  icon={<Icon name="dependencies" className="tint-dependencies" />}
                  name="Dependencies"
                  testId={`tree-deps-${view.name}`}
                  onClick={() => p.onToggle(depsKey(view.path))}
                />
                {p.expanded.has(depsKey(view.path)) && (
                  <>
                    {groupRow(
                      "frameworks",
                      "Frameworks",
                      <Icon name="layers" className="tint-framework" />,
                      frameworks.map((framework) => (
                        <TreeRow
                          key={`fw:${framework}`}
                          level={level + 3}
                          icon={<Icon name="layers" className="tint-framework" />}
                          name={framework}
                          testId={`tree-framework-${view.name}-${framework}`}
                          onClick={() => undefined}
                        />
                      )),
                    )}
                    {groupRow(
                      "packages",
                      "Packages",
                      <Icon name="package" className="tint-package" />,
                      view.packages.map((pkg) => (
                        <TreeRow
                          key={`pkg:${pkg.name}`}
                          level={level + 3}
                          icon={<Icon name="package" className="tint-package" />}
                          name={pkg.name}
                          testId={`tree-package-${view.name}-${pkg.name}`}
                          suffix={
                            <span className="tag">
                              {pkg.version ? ` ${pkg.version}` : " (version unknown)"}
                            </span>
                          }
                          onClick={() => undefined}
                        />
                      )),
                    )}
                    {groupRow(
                      "projects",
                      "Projects",
                      <Icon name="solutionFolder" className="tint-solution-folder" />,
                      view.refs.map((ref) => {
                        const target = views.find((candidate) => candidate.name === ref);
                        const refKind: ProjectKind = target
                          ? projectKindOf(target.kind, target.textKind)
                          : "unknown";
                        return (
                          <TreeRow
                            key={`ref:${ref}`}
                            level={level + 3}
                            icon={<ProjectIcon kind={refKind} />}
                            name={ref}
                            testId={`tree-ref-${view.name}-${ref}`}
                            onClick={() => undefined}
                          />
                        );
                      }),
                    )}
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
        icon={<Icon name="solution" className="tint-solution" />}
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
                    icon={<Icon name="solutionFolder" className="tint-solution-folder" />}
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
