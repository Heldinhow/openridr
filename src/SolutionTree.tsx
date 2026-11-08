import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { FileIcon } from "./FileIcon";
import type { GitStatus } from "./git";
import { call, type EvaluatedPackageRef, type EvaluationProject, type OutlineProject } from "./host/client";
import { Icon } from "./Icon";
import { ProjectIcon } from "./icons/ProjectIcon";
import { SYMBOL_LABEL } from "./icons/SymbolMark";
import { createListingCache, type ListingEntry } from "./solutionListing";
import { treeGitStatusMap, mergeIgnored, treeQueryId, type TreeGitStatus } from "./solutionGitStatus";
import {
  ROOT_KEY, baseOf, countLabel, depGroupKey, depsKey, dirKey, dirOf, folderKey, isAbsolutePath,
  nestSources, visibleSourceFiles, type DependencyGroup, type SourceNode,
} from "./solutionNodes";
import {
  dedupePackages, fileKind, frameworkRows, packageKey, projectKindOf, symbolKindOf,
  type ProjectKind, type SymbolKind,
} from "./solutionVisuals";
import { askTreeQuery } from "./treeQueryRetry";
import { TreeRow, type MenuPosition } from "./TreeRow";

export type TreeTarget =
  | { kind: "root" }
  /** A .sln folder: it groups projects, so its commands are the solution's. */
  | { kind: "solution-folder"; name: string }
  | { kind: "project"; path: string; name: string }
  | { kind: "file"; path: string }
  | { kind: "dir"; path: string };

/** Accessible description parts: the row's glyph-only information, read after the name. */
const describe = (...parts: Array<string | null | undefined>) =>
  parts.filter(Boolean).join(". ");
const SYMLINK_DESCRIPTION = "Symbolic link — not followed";
const GIT_STATUS_LABEL: Record<TreeGitStatus, string> = {
  modified: "Modified",
  added: "Added",
  untracked: "Untracked",
  ignored: "Ignored",
  conflicted: "Conflicted",
};

type Entry = ListingEntry;
type SymbolCacheEntry = { kind: SymbolKind | null; modified: number };
type SymbolEntry = { path: string; kind: string | null; modified: number };

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
      packages: dedupePackages(p.packageRefs),
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
  /** Bumps when the model is re-evaluated; the symbol scan must not outlive a fresh model. */
  evaluationRevision: number;
  showAll: boolean;
  filter: string;
  /** Snapshot the app already keeps; the tree never polls Git on its own. */
  gitStatus: GitStatus | null;
  /** Root of the open workspace, so repository paths can be mapped to tree paths. */
  workspaceDir: string;
  /** Bumped on every applied Git snapshot; ignore rules and links can change without the paths. */
  gitRevision: number;
  onMenu: (position: MenuPosition, target: TreeTarget, row: HTMLElement) => void;
};

export function SolutionTree(p: Props) {
  const views = useMemo(() => toViews(p.evaluated, p.outline), [p.evaluated, p.outline]);
  const filter = p.filter.trim().toLowerCase();
  const hit = (text: string) => text.toLowerCase().includes(filter);
  const [listing, setListing] = useState<Record<string, Entry[]>>({});
  // The cache is a mutable object, so `ensureDir` never sees a stale `listing` closure; a reload
  // drops every level and a late answer from the previous epoch is discarded by the cache itself.
  const listingCache = useRef<ReturnType<typeof createListingCache> | null>(null);
  if (!listingCache.current) {
    listingCache.current = createListingCache(async (dir) => {
      const result = await call<{ entries: Entry[] }>("workspace.tree", { dir });
      return result.entries ?? [];
    });
  }
  const [symbolKinds, setSymbolKinds] = useState<Record<string, SymbolKind>>({});
  const symbolKindsRef = useRef<Record<string, SymbolKind>>({});
  const symbolCache = useRef(new Map<string, SymbolCacheEntry>());
  const [symlinks, setSymlinks] = useState<Set<string>>(() => new Set());
  const symlinkRef = useRef<Set<string>>(new Set());
  const [ignoredPaths, setIgnoredPaths] = useState<string[]>([]);
  const sourceSet = useMemo(() => new Set(views.flatMap((view) => view.sources)), [views]);
  const projectDirs = useMemo(() => new Set(views.map((view) => dirOf(view.path))), [views]);
  const ancestors = useMemo(
    () => new Set(views.flatMap((view) => {
      const out: string[] = [];
      for (let dir = dirOf(view.path); dir; dir = dirOf(dir)) out.push(dir);
      return out;
    })),
    [views],
  );
  const projectFiles = useMemo(() => new Set(views.map((view) => view.path)), [views]);
  const gitStatusByPath = useMemo(
    () => mergeIgnored(treeGitStatusMap(p.gitStatus, p.workspaceDir), ignoredPaths),
    [p.gitStatus, p.workspaceDir, ignoredPaths],
  );
  const allSourcesKey = useMemo(
    () => Array.from(sourceSet).filter((path) => path.toLowerCase().endsWith(".cs")).sort().join("\n"),
    [sourceSet],
  );

  // Show all files: load one directory level at a time through the cache.
  async function ensureDir(dir: string, force = false) {
    await listingCache.current!.ensure(dir, force);
    setListing(listingCache.current!.snapshot());
  }

  // A new Git snapshot or workspace can change a path's type (link↔file) without changing the
  // directory, so the levels the user already has open are listed again; the cache drops the
  // previous epoch, so a late answer never repaints the tree. Expansion lives in `expanded` and is
  // preserved; only levels the user opened are re-read (no recursive scan, no watcher).
  useEffect(() => {
    const open = Object.keys(listing);
    listingCache.current!.reload();
    setListing(listingCache.current!.snapshot());
    if (!p.showAll) return;
    const dirs = new Set<string>(["", ...views.map((view) => dirOf(view.path))]);
    for (const dir of open) dirs.add(dir);
    for (const dir of dirs) void ensureDir(dir, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [p.showAll, p.gitRevision, p.workspaceDir, views.length]);

  // Only files the user can see (open project + open folders) are asked for; filter mode keeps the
  // badges already known and never starts a scan per keystroke.
  const visibleSignature = useMemo(() => {
    if (filter) return "";
    const visible = new Set<string>();
    if (p.showAll) {
      const walk = (dir: string) => {
        for (const entry of listing[dir] ?? []) {
          if (entry.dir) {
            const projectRoot = dir === "" && (projectDirs.has(entry.path) || ancestors.has(entry.path));
            if (!projectRoot && p.expanded.has(dirKey(entry.path))) walk(entry.path);
          } else if (entry.path.toLowerCase().endsWith(".cs") && sourceSet.has(entry.path)) {
            visible.add(entry.path);
          }
        }
      };
      for (const view of views) {
        if (p.expanded.has(view.path)) walk(dirOf(view.path));
      }
      if (p.expanded.has(ROOT_KEY)) walk("");
    } else {
      for (const view of views) {
        if (!p.expanded.has(view.path)) continue;
        for (const path of visibleSourceFiles(
          nestSources(view.path, view.sources),
          (key) => p.expanded.has(key),
        )) {
          if (path.toLowerCase().endsWith(".cs")) visible.add(path);
        }
      }
    }
    return Array.from(visible).sort().join("\n");
  }, [filter, p.expanded, p.showAll, views, listing, sourceSet, projectDirs, ancestors]);

  // Same walk as `visibleSignature`, without the `.cs` filter: the ignore rules apply to every
  // file row the tree is showing. While a filter is active the rows are not the expanded tree, so
  // the ignored marker is dropped until the filter is cleared (the Git status itself stays).
  const visibleFilePaths = useMemo(() => {
    if (filter) return [];
    const visible = new Set<string>();
    if (p.showAll) {
      const walk = (dir: string) => {
        for (const entry of listing[dir] ?? []) {
          if (entry.dir) {
            const projectRoot = dir === "" && (projectDirs.has(entry.path) || ancestors.has(entry.path));
            if (!projectRoot && p.expanded.has(dirKey(entry.path))) walk(entry.path);
          } else {
            visible.add(entry.path);
          }
        }
      };
      for (const view of views) {
        if (p.expanded.has(view.path)) walk(dirOf(view.path));
      }
      if (p.expanded.has(ROOT_KEY)) walk("");
    } else {
      for (const view of views) {
        if (!p.expanded.has(view.path)) continue;
        for (const path of visibleSourceFiles(
          nestSources(view.path, view.sources),
          (key) => p.expanded.has(key),
        )) {
          visible.add(path);
        }
      }
    }
    return Array.from(visible).sort();
  }, [filter, p.expanded, p.showAll, views, listing, projectDirs, ancestors]);

  // `.gitignore` answers for the visible rows only, in batches, and only when the Git snapshot is
  // available (trusted workspace): Restricted mode never calls Git. The request carries the Git
  // revision, so a commit/checkout that changes `.gitignore` re-asks even when the paths are the
  // same; the array is the request (no newline-joined signature, which would corrupt valid names).
  const ignoredRequest = useMemo(() => {
    if (!p.gitStatus || p.gitStatus.state !== "ok" || !p.gitStatus.root) return null;
    return {
      id: treeQueryId(`ignored:${p.workspaceDir}`, p.gitRevision, visibleFilePaths),
      paths: visibleFilePaths,
    };
  }, [p.gitStatus, p.workspaceDir, p.gitRevision, visibleFilePaths]);

  // The effect follows the request id, not the object: the snapshot poll replaces `views` with an
  // equal model, and a new object for the same id would cancel a retry that is still waiting.
  const ignoredRequestRef = useRef(ignoredRequest);
  ignoredRequestRef.current = ignoredRequest;
  useEffect(() => {
    // The previous answer belongs to an older snapshot: drop it before asking again, and let the
    // cleanup flag discard a response that arrives after this request was replaced.
    setIgnoredPaths((current) => (current.length === 0 ? current : []));
    const request = ignoredRequestRef.current;
    if (!request || request.paths.length === 0) return;
    const paths = request.paths;
    let alive = true;
    void (async () => {
      const ignored: string[] = [];
      for (let start = 0; start < paths.length; start += 128) {
        const result = await askTreeQuery(
          () => call<{ ignored: string[] }>("git.ignored", { paths: paths.slice(start, start + 128) }),
          () => !alive,
        );
        if (!alive) return;
        // A batch that was given up leaves its rows unmarked; the batches after it still get a turn.
        if (!result) continue;
        for (const path of Array.isArray(result.ignored) ? result.ignored : []) {
          if (typeof path === "string") ignored.push(path);
        }
      }
      if (!alive) return;
      setIgnoredPaths(ignored);
    })();
    return () => {
      alive = false;
    };
  }, [ignoredRequest?.id]);
  // Batches of 128 keep the host's 256-file cap out of reach; `known` makes unchanged files a stat.
  useEffect(() => {
    if (!visibleSignature) return;
    const visible = visibleSignature.split("\n");
    let alive = true;
    void (async () => {
      const updates: Array<[string, SymbolCacheEntry]> = [];
      for (let start = 0; start < visible.length; start += 128) {
        const batch = visible.slice(start, start + 128);
        const known: Record<string, number> = {};
        for (const path of batch) {
          const cached = symbolCache.current.get(path);
          if (cached) known[path] = cached.modified;
        }
        const result = await askTreeQuery(
          () => call<{ entries: SymbolEntry[] }>("workspace.symbolKinds", { paths: batch, known }),
          () => !alive,
        );
        if (!alive) return;
        // A batch that was given up keeps the badges already shown; the next batch still runs.
        if (!result) continue;
        for (const entry of Array.isArray(result.entries) ? result.entries : []) {
          if (!entry || typeof entry.path !== "string") continue;
          updates.push([
            entry.path,
            { kind: symbolKindOf(entry.kind) ?? null, modified: entry.modified },
          ]);
        }
      }
      if (!alive || updates.length === 0) return;
      const next = { ...symbolKindsRef.current };
      for (const [path, entry] of updates) {
        symbolCache.current.set(path, entry);
        if (entry.kind) next[path] = entry.kind;
        else delete next[path];
      }
      symbolKindsRef.current = next;
      setSymbolKinds(next);
    })();
    return () => {
      alive = false;
    };
  }, [visibleSignature, p.evaluationRevision]);

  // Renamed/deleted sources must not keep a badge forever.
  useEffect(() => {
    const allowed = new Set(allSourcesKey ? allSourcesKey.split("\n") : []);
    for (const path of Array.from(symbolCache.current.keys())) {
      if (!allowed.has(path)) symbolCache.current.delete(path);
    }
    const next = Object.fromEntries(
      Object.entries(symbolKindsRef.current).filter(([path]) => allowed.has(path)),
    );
    if (Object.keys(next).length !== Object.keys(symbolKindsRef.current).length) {
      symbolKindsRef.current = next;
      setSymbolKinds(next);
    }
  }, [allSourcesKey]);

  // Symlinks of the evaluated rows (sources and their folders). "Show all files" already gets the
  // flag from the listing, so the batch only runs in the evaluated mode. The workspace and the Git
  // revision are part of the request: the same paths in another workspace, or after a checkout that
  // swapped a link for a file, must be asked again; the array carries names verbatim.
  const symlinkRequest = useMemo(() => {
    if (p.showAll) return null;
    const paths = new Set<string>();
    for (const view of views) {
      for (const source of view.sources) {
        paths.add(source);
        for (let dir = dirOf(source); dir; dir = dirOf(dir)) paths.add(dir);
      }
    }
    const list = Array.from(paths).sort();
    return { id: treeQueryId(`symlinks:${p.workspaceDir}`, p.gitRevision, list), paths: list };
  }, [p.showAll, views, p.workspaceDir, p.gitRevision]);

  // Same as the ignored query: the poll replaces the model with an equal one, and the effect must
  // follow the request id so a retry in flight is not cancelled by an unrelated render.
  const symlinkRequestRef = useRef(symlinkRequest);
  symlinkRequestRef.current = symlinkRequest;
  useEffect(() => {
    // A marker from the previous workspace/revision must not survive while the new answer is in
    // flight, and a late response is dropped by the cleanup flag.
    symlinkRef.current = new Set();
    setSymlinks(symlinkRef.current);
    const request = symlinkRequestRef.current;
    if (!request || request.paths.length === 0) return;
    const candidates = request.paths;
    let alive = true;
    void (async () => {
      const links = new Set<string>();
      for (let start = 0; start < candidates.length; start += 128) {
        const result = await askTreeQuery(
          () => call<{ entries: Array<{ path: string; symlink: boolean }> }>(
            "workspace.symlinks",
            { paths: candidates.slice(start, start + 128) },
          ),
          () => !alive,
        );
        if (!alive) return;
        if (!result) continue;
        for (const entry of Array.isArray(result.entries) ? result.entries : []) {
          if (entry && typeof entry.path === "string" && entry.symlink) links.add(entry.path);
        }
      }
      if (!alive) return;
      symlinkRef.current = links;
      setSymlinks(links);
    })();
    return () => {
      alive = false;
    };
  }, [symlinkRequest?.id]);

  const open = (key: string) => (filter ? true : p.expanded.has(key));

  function rowMenu(target: TreeTarget) {
    return (position: MenuPosition, row: HTMLElement) => p.onMenu(position, target, row);
  }

  function fileRow(path: string, level: number, noIndex = false, symlink = false) {
    const kind = fileKind(path);
    const symbol = symbolKinds[path];
    const git = gitStatusByPath.get(path);
    return (
      <TreeRow
        key={path}
        level={level}
        icon={<FileIcon path={path} symbolKind={symbol} />}
        name={baseOf(path)}
        description={describe(
          symbol ? `C# ${SYMBOL_LABEL[symbol]}` : null,
          symlink ? SYMLINK_DESCRIPTION : null,
          git ? `Git status: ${GIT_STATUS_LABEL[git.status]}` : null,
        ) || undefined}
        noIndex={noIndex}
        symlink={symlink}
        gitStatus={git?.status}
        gitLabel={git?.label}
        testId={`tree-file-${path}`}
        fileKind={kind}
        symbolKind={symbol}
        current={p.activePath === path}
        selected={p.activePath === path}
        title={path}
        onClick={() => p.onOpenFile(path)}
        onMenu={rowMenu({ kind: "file", path })}
      />
    );
  }

  function sourceRows(nodes: SourceNode[], level: number): ReactNode[] {
    return nodes.flatMap((node) => {
      if (node.kind === "file") {
        return !filter || hit(node.name)
          ? [fileRow(node.path, level, false, symlinks.has(node.path))]
          : [];
      }
      const link = symlinks.has(node.path);
      const children = link ? [] : sourceRows(node.children, level + 1);
      if (filter && children.length === 0) return [];
      return [
        <TreeRow
          key={node.path}
          level={level}
          open={link ? undefined : open(dirKey(node.path))}
          symlink={link}
          description={link ? SYMLINK_DESCRIPTION : undefined}
          name={node.name}
          testId={`tree-dir-${node.path}`}
          onClick={() => {
            if (!link) p.onToggle(dirKey(node.path));
          }}
          onMenu={rowMenu({ kind: "dir", path: node.path })}
        />,
        ...(link || !open(dirKey(node.path)) ? [] : children),
      ];
    });
  }

  // "Show all files": the real directory, including bin/obj/dotfiles marked "no index".
  // A symlink is listed as a link object: the listing never stats the target to decide whether it
  // points at a directory, so the row is a leaf with the link glyph and no expansion.
  function diskRows(dir: string, level: number, skip: (entry: Entry) => boolean): ReactNode[] {
    return (listing[dir] ?? []).filter((entry) => !skip(entry) && (!filter || hit(entry.name))).flatMap((entry) => {
      if (entry.symlink) {
        return [
          <TreeRow
            key={entry.path}
            level={level}
            symlink
            description={SYMLINK_DESCRIPTION}
            name={entry.name}
            noIndex={entry.hidden}
            testId={`tree-file-${entry.path}`}
            onClick={() => undefined}
            onMenu={rowMenu({ kind: "file", path: entry.path })}
          />,
        ];
      }
      if (!entry.dir) {
        return [fileRow(entry.path, level, entry.hidden || !sourceSet.has(entry.path))];
      }
      const key = dirKey(entry.path);
      return [
        <TreeRow
          key={entry.path}
          level={level}
          open={open(key)}
          name={entry.name}
          noIndex={entry.hidden}
          testId={`tree-dir-${entry.path}`}
          onClick={() => {
            p.onToggle(key);
            void ensureDir(entry.path);
          }}
          onMenu={rowMenu({ kind: "dir", path: entry.path })}
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
    // Dependencies, its groups and the leaves name the project they belong to: their menu is the
    // project menu, so no row of the tree is left without one.
    const projectTarget: TreeTarget = { kind: "project", path: view.path, name: view.name };
    // A dependency row acts on its owner project, and opening its menu selects that project so
    // the menu, the details inspector and the build/dev actions never name two different owners.
    const projectMenu = (position: MenuPosition, row: HTMLElement) => {
      p.onSelectProject(view.path);
      p.onMenu(position, projectTarget, row);
    };
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
            onMenu={projectMenu}
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
          onMenu={(position, row) => {
            p.onSelectProject(view.path);
            p.onMenu(position, projectTarget, row);
          }}
          suffix={
            <>
              {isAbsolutePath(view.path) && (
                <span
                  className="tag"
                  data-testid={`tree-project-outside-${view.name}`}
                  title={`${view.path} is outside the solution directory`}
                >
                  outside
                </span>
              )}
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
                  onMenu={projectMenu}
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
                          onMenu={projectMenu}
                        />
                      )),
                    )}
                    {groupRow(
                      "packages",
                      "Packages",
                      <Icon name="package" className="tint-package" />,
                      view.packages.map((pkg) => (
                        <TreeRow
                          key={`pkg:${packageKey(pkg)}`}
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
                          onMenu={projectMenu}
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
                            onMenu={projectMenu}
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
        onMenu={rowMenu({ kind: "root" })}
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
                    name={folder}
                    testId={`tree-folder-${folder}`}
                    suffix={<span className="tag tree-count">· {countLabel(items.length, "project")}</span>}
                    onClick={() => p.onToggle(folderKey(folder))}
                    onMenu={rowMenu({ kind: "solution-folder", name: folder })}
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
