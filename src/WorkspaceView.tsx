import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type FormEvent } from "react";
import {
  call,
  candidatesOf,
  HostCallError,
  messageOf,
  type Candidate,
  type CodeAction,
  type CodeActionsResponse,
  type DapEvaluation,
  type DocumentSymbolResponse,
  type EvaluationProject,
  type ReferencesResponse,
  type RenameResponse,
  type RunConfig,
  type LaunchProfile,
  type SessionState,
  type Snapshot,
  type OutlineProject,
} from "./host/client";
import { CodeEditor, type ChangedLine } from "./CodeEditor";
import { EditorContextMenu, type ContextMenuItem } from "./EditorContextMenu";
import { SolutionToolbar } from "./SolutionToolbar";
import { SolutionTree, expandableKeys, type TreeTarget } from "./SolutionTree";
import type { MenuPosition } from "./TreeRow";
import { ROOT_KEY, baseOf, dirKey, dirOf, folderKey, workspaceRelative } from "./solutionNodes";
import { fileMenuItems, projectMenuItems, solutionFolderMenuItems, solutionMenuItems } from "./solutionMenu";
import { FindUsagesPanel } from "./FindUsagesPanel";
import {
  gutterActionAt,
  gutterActionsFor,
  lineSnippet,
  type GutterAction,
} from "./codeNavigation";
import { ProblemsPanel } from "./ProblemsPanel";
import { RunPanel } from "./RunPanel";
import { BuildPanel, isBuildObsolete } from "./BuildPanel";
import {
  TestsPanel,
  failedTestIds,
  rerunFailedReason,
  testDebugUnavailable,
  testRunBlockedReason,
} from "./TestsPanel";
import { TestsQuickList } from "./TestsQuickList";
import { ProcessPanel } from "./ProcessPanel";
import { collectDiagnostics, problemCounts, samePath } from "./diagnosticMarks";
import { partialWorkspaceStatus, evaluationStatusView } from "./evaluationStatus";
import { DebugAdapterPanel, LspPanel, ToolInstaller, lspStartBlocker, lspStateLabel } from "./ToolchainPanels";
import { DebugPanel } from "./DebugPanel";
import { TerminalPanel } from "./TerminalPanel";
import { FileIcon } from "./FileIcon";
import { Icon } from "./Icon";
import { Splitter } from "./Splitter";
import { BottomToolWindow } from "./shell/BottomToolWindow";
import { MidText } from "./shell/MidText";
import { StatusBar } from "./shell/StatusBar";
import { ToolWindowBand } from "./shell/ToolWindowBand";
import { FindAction, type SearchTab } from "./actions/FindAction";
import { FindInFiles } from "./FindInFiles";
import { RecentFilesDialog } from "./RecentFilesDialog";
import { KeymapDialog } from "./actions/KeymapDialog";
import { SettingsDialog } from "./SettingsDialog";
import {
  DEFAULT_RUN_CONFIG_ID,
  RunConfigDialog,
  RunConfigWidget,
  deriveDefaultRunConfig,
  type RunConfigView,
} from "./runConfigs";
import {
  BranchWidget,
  CommitPanel,
  GitLogPanel,
  gitBlockedReason,
  gitPushBlockedReason,
  gitUpdateBlockedReason,
  type GitDiff as GitDiffView,
  type GitIntent,
  type GitStatus,
} from "./git";
import { NuGetPanel } from "./NuGetPanel";
import { CommitDiff } from "./CommitDiff";
import { useExecutionFlow } from "./useExecutionFlow";
import {
  buildDisabledReason as buildGate,
  debugDisabledReason as debugGate,
  runAvailabilityReason as runGate,
  type GateContext,
} from "./executionGates";
import { useEvaluationFlow } from "./useEvaluationFlow";
import { useRunConfigs } from "./useRunConfigs";
import {
  chordPrefixFor,
  chordTarget,
  defaultSettings,
  matches,
  normalizeSettings,
  type AppSettings,
  type Keymap,
} from "./actions/keymap";
import {
  firstActionFor,
  requireAction,
  resolveActions,
  type Action,
  type ResolvedAction,
} from "./actions/registry";
import {
  loadShellLayout,
  saveShellLayout,
  toolWindows,
  toolWindowsIn,
  type BottomToolId,
  type ShellLayout,
  type ToolWindowId,
  type ToolWindowSlot,
} from "./shell/toolWindows";

type Tab = {
  path: string;
  content: string;
  hash: string;
  revision: number;
  /** Text at the last read/write; null when the disk text is unknown. Dirty = content differs. */
  savedContent: string | null;
  conflict: null | { content: string; hash: string };
};

/** Breakpoint lines per source path, sorted, in the shape the session store keeps. */
function breakpointsByPath(breakpoints: Snapshot["dap"]["breakpoints"]) {
  const groups: Record<string, number[]> = {};
  for (const breakpoint of breakpoints) (groups[breakpoint.path] ??= []).push(breakpoint.line);
  for (const lines of Object.values(groups)) lines.sort((left, right) => left - right);
  return groups;
}

type DocumentSymbolsState = {
  path: string;
  revision: number;
  session: number;
  contextId: number;
  workspaceId: number;
  response: DocumentSymbolResponse;
};

type FindUsagesState = {
  response: ReferencesResponse;
  snippets: Record<string, string | null>;
};

type ContextMenuState = {
  x: number;
  y: number;
  line: number;
  character: number;
};

type RenamePreview = {
  kind: "rename";
  response: RenameResponse;
  sourcePath: string;
  sourceRevision: number;
  session: number;
  contextId: number;
  workspaceId: number;
  /** UI revision of every tab that was open when the preview was built. */
  tabRevisions: Record<string, number>;
  stale: string | null;
  applying: boolean;
};

type ActionPreview = {
  kind: "action";
  action: CodeAction;
  response: CodeActionsResponse;
  sourcePath: string;
  sourceRevision: number;
  session: number;
  contextId: number;
  workspaceId: number;
  tabRevisions: Record<string, number>;
  stale: string | null;
  applying: boolean;
};

type EditPreview = RenamePreview | ActionPreview;

type ActionList = {
  response: CodeActionsResponse;
  sourcePath: string;
  sourceRevision: number;
  line: number;
  character: number;
  session: number;
  contextId: number;
  workspaceId: number;
  tabRevisions: Record<string, number>;
  stale: string | null;
};

type Confirmation =
  | { kind: "tab"; path: string }
  | { kind: "workspace" }
  | { kind: "window" }
  | { kind: "quit" };

// One prompt serves create and rename; `path` is the folder for create and the current path for
// rename. The host remains the confinement authority; this only rejects names that carry a path.
type NamePrompt = {
  kind: "create" | "rename";
  path: string;
  value: string;
  error: string | null;
  busy: boolean;
};

type DeletePrompt = { path: string; directory: boolean; error: string | null; busy: boolean };

function nameProblem(name: string): string | null {
  if (!name) return "Enter a name.";
  if (name === "." || name === "..") return "That is not a file or folder name.";
  if (name.includes("/") || name.includes("\\")) return "Use a name without a path.";
  return null;
}

type NavEntry = { path: string; line: number; character: number };

type NoticeAction = { label: string; onClick: () => void };
type Notice = { tone: "info" | "error"; text: string; action?: NoticeAction };
type PendingAnalysisAction = {
  kind: "findUsages" | "rename" | "codeActions";
  position: { line: number; character: number };
};
type AutoDiscoveryError = { boundary: string; message: string };

type Props = {
  snapshot: Snapshot;
  initialFile: string | null;
  recentSolutions: string[];
  // Path from Finder or the Dock; each request is a new object, so the same path can repeat.
  externalOpen: { path: string } | null;
  // Cmd+Q or the app menu Quit; increments per request and is answered with the unsaved-changes dialog.
  quitRequest: number;
  onOpenRecent: (path: string) => Promise<void>;
  onSnapshot: (snapshot: Snapshot) => void;
  onClosed: (snapshot: Snapshot) => void;
};

const bottomTools = toolWindowsIn("bottom");

const NO_CHANGED_LINES: ChangedLine[] = [];

// "@@ -a,b +c,d @@" hunks -> new-file lines; hunks that also delete lines are "mod", pure additions "add".
function changedLinesFromDiff(diff: string): ChangedLine[] {
  const out: ChangedLine[] = [];
  let line = 0;
  let hunk = false;
  const pending: number[] = [];
  let deletes = 0;
  const flush = () => {
    const kind = deletes > 0 ? "mod" : "add";
    for (const n of pending) out.push([n, kind]);
    pending.length = 0;
    deletes = 0;
  };
  for (const text of diff.split("\n")) {
    const header = /^@@ -\d+(?:,\d+)? \+(\d+)/.exec(text);
    if (header) { flush(); line = Number(header[1]); hunk = true; continue; }
    if (!hunk) continue;
    if (text.startsWith("+")) { pending.push(line++); }
    else if (text.startsWith("-")) { deletes++; }
    else if (text.startsWith("\\")) { continue; }
    else { flush(); line++; }
  }
  flush();
  return out;
}

function baseName(path: string) {
  return path.split("/").pop() ?? path;
}

function isDirty(tab: Tab) {
  return tab.content !== tab.savedContent || tab.conflict !== null;
}

function lineDiffCount(buffer: string, disk: string) {
  const left = buffer.split("\n");
  const right = disk.split("\n");
  // shortcut: buffers over 1200 lines show only hashes, not a line diff; upgrade to a streaming diff if files grow.
  if (left.length > 1200 || right.length > 1200) return null;
  let previous = new Array<number>(right.length + 1).fill(0);
  for (let i = 1; i <= left.length; i += 1) {
    const current = new Array<number>(right.length + 1).fill(0);
    for (let j = 1; j <= right.length; j += 1) {
      current[j] =
        left[i - 1] === right[j - 1]
          ? previous[j - 1] + 1
          : Math.max(previous[j], current[j - 1]);
    }
    previous = current;
  }
  const common = previous[right.length];
  return { added: right.length - common, removed: left.length - common };
}

function countLabel(count: number, word: string) {
  return `${count} ${word}${count === 1 ? "" : "s"}`;
}

function previewId(preview: EditPreview) {
  return preview.kind === "rename" ? preview.response.renameId : preview.action.actionId;
}

function previewDocuments(preview: EditPreview) {
  return preview.kind === "rename" ? preview.response.documents : preview.action.documents;
}

function previewEditCount(preview: EditPreview) {
  return preview.kind === "rename" ? preview.response.editCount : preview.action.editCount;
}

function previewTitle(preview: EditPreview) {
  return preview.kind === "rename"
    ? `Rename to “${preview.response.newName}”`
    : preview.action.title;
}

function previewKindLabel(preview: EditPreview) {
  return preview.kind === "rename" ? "rename" : "code action";
}

function projectFor(path: string, projects: OutlineProject[]) {
  return projects.find((project) => {
    const dir = project.path.replace(/\/[^/]+$/, "");
    return path.startsWith(`${dir}/`);
  });
}

function evaluatedProjectFor(path: string, projects: EvaluationProject[]) {
  return projects.find((project) => {
    const dir = project.path.replace(/\/[^/]+$/, "");
    return path === project.path || path.startsWith(`${dir}/`);
  });
}

function restoreStateLabel(restore: Snapshot["restore"]) {
  const exit = restore.exit === null ? "exit unknown" : `exit ${restore.exit}`;
  switch (restore.state) {
    case "running":
      return "Restoring…";
    case "ok":
      return `Restore succeeded · ${exit}`;
    case "failed":
      return `Restore failed · ${exit}`;
    case "cancelled":
      return `Restore cancelled · ${exit}`;
    default:
      return "Restore not requested";
  }
}

export function WorkspaceView({ snapshot, initialFile, recentSolutions, externalOpen, quitRequest, onOpenRecent, onSnapshot, onClosed }: Props) {
  const workspace = snapshot.workspace!;
  const projects = snapshot.outline?.projects ?? [];
  const trusted = snapshot.trust.granted;
  const projectSdk = snapshot.sdk.project;
  const evaluation = snapshot.evaluation;
  // "Previous evaluation" is only true once a second revision exists; a stale first model has nothing before it.
  const hasEarlierEvaluation = evaluation.revision > 1;
  const emptySolution = workspace.kind === "solution" && projects.length === 0;
  const evaluatedProjects = evaluation.projects;
  const showsEvaluatedTree =
    evaluation.state !== "idle" && evaluatedProjects.length > 0;
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activePath, setActivePath] = useState<string | null>(null);
  const [message, setMessage] = useState<Notice | null>(null);
  const [trustError, setTrustError] = useState<string | null>(null);
  // Ambiguous external open with the workspace mounted: the user picks the solution from these buttons.
  const [externalCandidates, setExternalCandidates] = useState<{ path: string; candidates: Candidate[] } | null>(null);
  const [confirm, setConfirm] = useState<Confirmation | null>(null);
  const [confirmError, setConfirmError] = useState<string | null>(null);
  const [namePrompt, setNamePrompt] = useState<NamePrompt | null>(null);
  const [deletePrompt, setDeletePrompt] = useState<DeletePrompt | null>(null);
  const [layout, setLayout] = useState<ShellLayout>(loadShellLayout);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [findActionOpen, setFindActionOpen] = useState(false);
  const [searchTab, setSearchTab] = useState<SearchTab | "last">("last");
  const [searchSeq, setSearchSeq] = useState(0);
  const [findInFiles, setFindInFiles] = useState<{ scope: string; nonce: number } | null>(null);
  const [recentFiles, setRecentFiles] = useState<string[]>([]);
  const [recentFilesOpen, setRecentFilesOpen] = useState(false);
  const [keymapOpen, setKeymapOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const settingsBaseline = useRef<AppSettings | null>(null);
  const [quickListOpen, setQuickListOpen] = useState(false);
  const [testProjectPath, setTestProjectPath] = useState("");
  const [testSelected, setTestSelected] = useState<Set<string>>(new Set());
  const [testAutoDiscoveryError, setTestAutoDiscoveryError] =
    useState<AutoDiscoveryError | null>(null);
  const testDiscoveryAutoStarted = useRef<string | null>(null);
  const [gitStatus, setGitStatus] = useState<GitStatus | null>(null);
  const [gitStatusError, setGitStatusError] = useState<string | null>(null);
  const [gitIntent, setGitIntent] = useState<GitIntent | null>(null);
  const [commitFocusPath, setCommitFocusPath] = useState<string | null>(null);
  // A refresh that outlives a workspace or trust change must not paint another project's status,
  // and one Git run at a time is enough: a call made while another is in flight queues one rerun.
  const gitIdentity = useRef(`${workspace.id}:${trusted}`);
  gitIdentity.current = `${workspace.id}:${trusted}`;
  const gitRefresh = useRef({ inFlight: false, queued: false });
  const refreshGit = useCallback(async () => {
    if (!trusted) {
      setGitStatus(null);
      setGitStatusError(null);
      return;
    }
    const state = gitRefresh.current;
    if (state.inFlight) {
      state.queued = true;
      return;
    }
    state.inFlight = true;
    const identity = `${workspace.id}:${trusted}`;
    try {
      do {
        state.queued = false;
        try {
          const next = await call<GitStatus>("git.status");
          if (gitIdentity.current !== identity) return;
          setGitStatus(next);
          setGitStatusError(null);
        } catch (error) {
          if (gitIdentity.current !== identity) return;
          setGitStatusError(messageOf(error));
        }
      } while (state.queued);
    } finally {
      state.inFlight = false;
    }
  }, [trusted, workspace.id]);
  useEffect(() => {
    // Refresh on workspace/trust changes; tool windows refresh on open and after operations.
    void refreshGit();
  }, [refreshGit, workspace.id]);
  useEffect(() => {
    // A commit or checkout run in a terminal is invisible to the app; coming back to the window is
    // the cheap moment to look again. Debounced, and `refreshGit` dedupes the calls itself.
    let timer: ReturnType<typeof setTimeout> | undefined;
    const onFocus = () => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => void refreshGit(), 400);
    };
    window.addEventListener("focus", onFocus);
    return () => {
      window.removeEventListener("focus", onFocus);
      if (timer) clearTimeout(timer);
    };
  }, [refreshGit]);
  function openGitIntent(kind: GitIntent["kind"]) {
    setGitIntent((current) => ({ kind, nonce: (current?.nonce ?? 0) + 1 }));
  }
  const [activeSlot, setActiveSlot] = useState<ToolWindowSlot>("bottom");
  const bottomTab = layout.bottom.active as BottomToolId;
  const bottomOpen = layout.bottom.open;
  const leftOpen = layout.left.open;
  const sidebarWidth = layout.left.size;
  const outputHeight = layout.bottom.size;
  const panelBodyRef = useRef<HTMLDivElement>(null);
  // Toolbar dropdowns are <details>: close them on an outside click.
  useEffect(() => {
    const close = (event: PointerEvent) => {
      document.querySelectorAll<HTMLDetailsElement>("details.toolbar-menu[open]").forEach((menu) => {
        if (!menu.contains(event.target as Node)) menu.open = false;
      });
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, []);
  const [viewportWidth, setViewportWidth] = useState(window.innerWidth);
  const [viewportHeight, setViewportHeight] = useState(window.innerHeight);
  const [projectDetailsOpen, setProjectDetailsOpen] = useState(false);
  const [editorTarget, setEditorTarget] = useState<{
    path: string;
    line: number;
    character: number;
    nonce: number;
  } | null>(null);
  useEffect(() => {
    if (!editorTarget) return;
    // Parent effects run after the child editor's, so the target is applied before it is cleared;
    // a remount (switching tabs) must not force the old cursor and focus again.
    const timer = window.setTimeout(() => setEditorTarget(null), 0);
    return () => window.clearTimeout(timer);
  }, [editorTarget]);
  // 84 = toolbar 52 + splitter 4 + status 28; the panel never squeezes the editor below 180 px.
  const outputMax = Math.max(
    120,
    Math.min(500, Math.floor(viewportHeight * 0.42), viewportHeight - 84 - 180),
  );
  const visibleOutputHeight = Math.min(outputHeight, outputMax);
  // 34 rail + sidebar + 4 splitter + editor + 34 rail: a narrow window narrows the tree, not the editor.
  const effectiveSidebarWidth = Math.min(sidebarWidth, Math.max(160, viewportWidth - 392));
  useEffect(() => {
    const resize = () => {
      setViewportWidth(window.innerWidth);
      setViewportHeight(window.innerHeight);
    };
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);
  useEffect(() => {
    saveShellLayout(layout);
  }, [layout]);
  useEffect(() => {
    let active = true;
    call<{ settings: unknown }>("settings.read")
      .then((result) => {
        if (!active) return;
        setSettings(normalizeSettings(result.settings));
      })
      .catch((error) => {
        // Defaults keep the app usable; the Keymap dialog shows the write failure if any.
        if (active) setMessage({ tone: "error", text: `Settings could not be read: ${messageOf(error)}` });
      });
    return () => {
      active = false;
    };
  }, []);

  function setBottomTab(tab: BottomToolId) {
    setActiveSlot("bottom");
    setLayout((current) => ({ ...current, bottom: { ...current.bottom, active: tab, open: true } }));
  }

  function setSidebarWidth(value: number) {
    setLayout((current) => ({ ...current, left: { ...current.left, size: value } }));
  }

  function setOutputHeight(value: number) {
    setLayout((current) => ({ ...current, bottom: { ...current.bottom, size: value } }));
  }

  function toggleToolWindow(slot: ToolWindowSlot, id: ToolWindowId) {
    setActiveSlot(slot);
    setLayout((current) => {
      const state = current[slot];
      const next = state.active === id
        ? { ...state, open: !state.open }
        : { ...state, active: id, open: true };
      return slot === "left" ? { ...current, left: next } : { ...current, bottom: next };
    });
  }

  const [editedAt, setEditedAt] = useState<Record<string, number>>({});
  const [savedAt, setSavedAt] = useState<Record<string, number>>({});
  const navigationSeq = useRef(0);
  const navHistory = useRef<{ entries: NavEntry[]; index: number }>({ entries: [], index: -1 });
  const [navEnabled, setNavEnabled] = useState({ back: false, forward: false });
  const [gotoLineRequest, setGotoLineRequest] = useState(0);
  const tabsRef = useRef(tabs);
  tabsRef.current = tabs;
  const activePathRef = useRef(activePath);
  activePathRef.current = activePath;
  const unsavedRef = useRef(false);
  const handledQuitRequest = useRef(0);
  const writing = useRef(new Set<string>());
  const diskEpoch = useRef(new Map<string, number>());
  const checkingDisk = useRef(new Set<string>());
  const [selectedProjectPath, setSelectedProjectPath] = useState<string | null>(null);
  const [startupPath, setStartupPath] = useState<string | null>(null);
  const seededRunConfigsFor = useRef<number | null>(null);
  const { runConfigStore, runConfigsLoaded, commitRunConfigs, persistStartupProject, persistSession, selectLaunchProfile } = useRunConfigs({ workspacePath: workspace.path, setMessage });
  useEffect(() => {
    if (!runConfigsLoaded || !runConfigStore || seededRunConfigsFor.current === workspace.id) return;
    seededRunConfigsFor.current = workspace.id;
    setStartupPath(runConfigStore.startupProject ?? null);
    // Only the store load for this workspace seeds the choice; later commits are user actions.
  }, [runConfigsLoaded, runConfigStore, workspace.id]);
  const [sessionReady, setSessionReady] = useState(false);
  const [breakpointsApplied, setBreakpointsApplied] = useState(false);
  const restoredSessionFor = useRef<number | null>(null);
  const tabCursors = useRef<Record<string, { line: number; character: number }>>({});
  const savedBreakpoints = useRef<Record<string, number[]>>({});
  useEffect(() => {
    if (!runConfigsLoaded || !runConfigStore || restoredSessionFor.current === workspace.id) return;
    restoredSessionFor.current = workspace.id;
    void restoreSession(runConfigStore.session);
  }, [runConfigsLoaded, runConfigStore, workspace.id]);
  const [runConfigOpen, setRunConfigOpen] = useState(false);
  const [runConfigInitialId, setRunConfigInitialId] = useState(DEFAULT_RUN_CONFIG_ID);
  const [toolsOpen, setToolsOpen] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(() => expandableKeys(null, projects));
  const seededTreeExpansion = useRef(0);
  const [showAllFiles, setShowAllFiles] = useState(false);
  const [treeSearchOpen, setTreeSearchOpen] = useState(false);
  const [treeFilter, setTreeFilter] = useState("");
  const [treeMenu, setTreeMenu] = useState<{ x: number; y: number; target: TreeTarget } | null>(null);
  // The menu hands the focus back to the row that opened it: WKWebView does not move the focus on a
  // right-click, so the focused element at open time may be an older row.
  const treeMenuReturnFocus = useRef<HTMLElement | null>(null);
  const [editPreview, setEditPreview] = useState<EditPreview | null>(null);
  const editPreviewRef = useRef<EditPreview | null>(null);
  editPreviewRef.current = editPreview;
  const editPreviewApplySeq = useRef(0);
  const [actionList, setActionList] = useState<ActionList | null>(null);
  const [documentSymbols, setDocumentSymbols] = useState<DocumentSymbolsState | null>(null);
  const [findUsages, setFindUsages] = useState<FindUsagesState | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [gitDiffView, setGitDiffView] = useState<GitDiffView | null>(null);
  const [commitDiffPath, setCommitDiffPath] = useState<string | null>(null);
  const [commitDiffActive, setCommitDiffActive] = useState(false);
  const [renameRequest, setRenameRequest] = useState(0);
  const [definitionRequest, setDefinitionRequest] = useState(0);
  const renameSeq = useRef(0);
  const actionSeq = useRef(0);
  const findUsagesSeq = useRef(0);
  const cursorRef = useRef({ line: 0, character: 0 });
  const [cursor, setCursor] = useState({ line: 0, character: 0 });
  const sessionPayload = useRef<SessionState | null>(null);
  useEffect(() => {
    // Never write before the restore finished: the first empty render must not erase the store.
    if (!sessionReady) return;
    const live = breakpointsByPath(snapshot.dap.breakpoints);
    const applied = trusted && breakpointsApplied;
    const session: SessionState = {
      tabs: tabs.map((tab) => ({
        path: tab.path,
        ...(tabCursors.current[tab.path] ?? { line: 0, character: 0 }),
      })),
      activePath,
      // Before the saved breakpoints are applied the live list is still empty; writing it would
      // erase the store. Restricted mode also keeps the ones already stored.
      breakpoints: applied ? live : savedBreakpoints.current,
    };
    sessionPayload.current = session;
    if (applied) savedBreakpoints.current = live;
    const timer = window.setTimeout(() => {
      persistSession(session).catch((error) =>
        setMessage({ tone: "error", text: `Editor session could not be saved: ${messageOf(error)}` }),
      );
    }, 400);
    return () => window.clearTimeout(timer);
  }, [
    sessionReady,
    trusted,
    breakpointsApplied,
    tabs,
    activePath,
    cursor,
    snapshot.dap.breakpoints,
    workspace.id,
  ]);
  useEffect(() => {
    // The 400 ms debounce must not drop the last state when the workspace closes or switches.
    // A direct call avoids setting state on an unmounted component.
    return () => {
      const session = sessionPayload.current;
      if (!session) return;
      void call("session.write", { solution: workspace.path, session }).catch(() => {});
    };
  }, [workspace.path]);
  useEffect(() => {
    // A Restricted open cannot restore breakpoints; apply them once trust is granted instead of
    // letting the autosave overwrite the store with the still-empty live list.
    if (!sessionReady || !trusted || breakpointsApplied) return;
    let cancelled = false;
    void (async () => {
      let restored: Snapshot | null = null;
      for (const [path, lines] of Object.entries(savedBreakpoints.current)) {
        if (lines.length === 0) continue;
        try {
          restored = await call<Snapshot>("debug.setBreakpoints", { path, lines });
        } catch {
          // One missing source keeps the other breakpoints; the next start clears its own list.
        }
      }
      if (cancelled) return;
      if (restored) onSnapshot(restored);
      setBreakpointsApplied(true);
    })();
    return () => {
      cancelled = true;
    };
  }, [sessionReady, trusted, breakpointsApplied, workspace.id]);
  const editPreviewHeading = useRef<HTMLHeadingElement | null>(null);
  const actionListHeading = useRef<HTMLHeadingElement | null>(null);

  const active = tabs.find((tab) => tab.path === activePath) ?? null;
  useEffect(() => {
    if (!activePath) return;
    setRecentFiles((current) =>
      [activePath, ...current.filter((path) => path !== activePath)].slice(0, 20),
    );
  }, [activePath]);
  const previewDocs = editPreview ? previewDocuments(editPreview) : [];
  const activeProject = active ? projectFor(active.path, projects) : undefined;
  const activeEvaluationProject = active
    ? evaluatedProjectFor(active.path, evaluatedProjects)
    : undefined;
  const selectedProject =
    evaluatedProjects.find((project) => project.path === selectedProjectPath) ?? null;
  const dirtyCount = tabs.filter(isDirty).length;
  unsavedRef.current = dirtyCount > 0;
  const activeConflictDiff = useMemo(
    () =>
      active?.conflict
        ? lineDiffCount(active.content, active.conflict.content)
        : null,
    [active?.path, active?.content, active?.conflict?.content, active?.conflict?.hash],
  );
  const showsProcesses = snapshot.processes.some(
    (process) => process.workspaceId === workspace.id,
  );
  // A process that needs a decision opens the Tools window (right side) instead of a strip over the editor.
  const awaitingProcessIds = snapshot.processes
    .filter((process) => process.workspaceId === workspace.id && process.awaitingDecision)
    .map((process) => process.id)
    .join(",");
  useEffect(() => {
    if (awaitingProcessIds) setToolsOpen(true);
  }, [awaitingProcessIds]);
  const evaluationAvailable = trusted && projectSdk.state === "resolved";
  const restore = snapshot.restore;
  const restoring = restore.state === "running";
  const lsp = snapshot.lsp;
  const lspActive = ["loading", "ready", "partial", "awaiting_diagnostics"].includes(
    lsp.state,
  );
  const lspActiveRef = useRef(false);
  lspActiveRef.current = lspActive;
  const lspViewRef = useRef(lsp);
  lspViewRef.current = lsp;
  const contextIdRef = useRef(snapshot.contextId);
  contextIdRef.current = snapshot.contextId;
  const workspaceIdRef = useRef(workspace.id);
  workspaceIdRef.current = workspace.id;
  const analysisAutoStarted = useRef<string | null>(null);
  const pendingAnalysisAction = useRef<PendingAnalysisAction | null>(null);
  async function startAnalysis(automatic = false) {
    try {
      onSnapshot(await call<Snapshot>("lsp.start"));
    } catch (error) {
      // The host has the last word on preconditions (e.g. stale restore assets); an automatic
      // attempt stays quiet and leaves the visible Start button, a click reports the reason.
      if (!automatic) setMessage({ tone: "error", text: `Analysis could not start: ${messageOf(error)}` });
    }
  }
  useEffect(() => {
    // The host stops analysis itself on restore/project changes (its reason says what to do);
    // start again once per such boundary. A user stop or a failure is never undone behind their back.
    const stoppedByHost = lsp.state === "stopped" && lsp.stoppedReason !== "Stopped by the user.";
    if ((lsp.state !== "unavailable" && !stoppedByHost) || lspStartBlocker(snapshot) !== null) return;
    const boundary = `${workspace.id}:${snapshot.contextId}:${restore.revision}`;
    if (analysisAutoStarted.current === boundary) return;
    analysisAutoStarted.current = boundary;
    void startAnalysis(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [snapshot, workspace.id]);
  useEffect(() => {
    // One blocked analysis action waits for the next active session and runs once against the
    // current render, so stale gates from the blocked render cannot re-arm it.
    if (!lspActive || pendingAnalysisAction.current === null) return;
    const pending = pendingAnalysisAction.current;
    pendingAnalysisAction.current = null;
    if (pending.kind === "findUsages") void requestFindUsages(pending.position);
    else if (pending.kind === "rename") requestRename(pending.position);
    else void requestCodeActions(pending.position);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lspActive]);
  const toolState = snapshot.tools["csharp-ls"].state;
  const analysisStatus = !trusted
    ? null
    : lspActive || lsp.state === "starting"
      ? { text: `Analysis: ${lspStateLabel(lsp.state)}`, action: null }
      : toolState !== "ready"
        ? ["downloading", "installing", "verifying"].includes(toolState)
          ? {
              text: snapshot.tools["csharp-ls"].bundled
                ? "Analysis: verifying the bundled tool…"
                : "Analysis: installing…",
              action: null,
            }
          : {
              text: "Analysis: not installed",
              action: { label: "Open tool diagnostics", disabledReason: null, onClick: () => setToolsOpen(true) },
            }
        : {
            text: `Analysis: ${lsp.state === "unavailable" ? "not running" : lspStateLabel(lsp.state).toLowerCase()}`,
            action: { label: "Start analysis", disabledReason: lspStartBlocker(snapshot), onClick: () => void startAnalysis() },
          };
  const renameDisabledReason = !active
    ? "Open a file to rename a symbol."
    : !trusted
      ? "Trust the workspace before renaming; no analysis runs in Restricted mode."
      : !lspActive
        ? "Start analysis before renaming a symbol."
        : !lsp.capabilities?.rename
          ? "csharp-ls did not advertise rename for this session."
          : active.conflict
            ? "Resolve the disk conflict before renaming this file."
            : editPreview
              ? "Finish or cancel the current rename preview first."
              : null;
  const codeActionsDisabledReason = !active
    ? "Open a file to request code actions."
    : !trusted
      ? "Trust the workspace before requesting code actions; no analysis runs in Restricted mode."
      : !lspActive
        ? "Start analysis before requesting code actions."
        : !lsp.capabilities?.codeActions
          ? "csharp-ls did not advertise code actions for this session."
          : active.conflict
            ? "Resolve the disk conflict before requesting code actions for this file."
            : editPreview
              ? "Finish or cancel the current preview first."
              : null;
  const findUsagesDisabledReason = !active
    ? "Open a file to find usages."
    : !trusted
      ? "Trust the workspace before finding usages; no analysis runs in Restricted mode."
      : !lspActive
        ? "Start analysis before finding usages."
        : !lsp.capabilities?.references
          ? "csharp-ls did not advertise references for this session."
          : active.conflict
            ? "Resolve the disk conflict before finding usages in this file."
            : editPreview
              ? "Finish or cancel the current preview first."
              : null;
  // One notice shape for Find Usages, Rename and code actions. The gate only offers a way
  // forward when analysis is what blocks the action; trust and capability blocks stay plain.
  function analysisNotice(label: string, reason: string, pending: PendingAnalysisAction): Notice {
    let action: NoticeAction | undefined;
    if (trusted && active && !lspActive) {
      if (toolState !== "ready") {
        action = {
          label: "Open tool diagnostics",
          onClick: () => setToolsOpen(true),
        };
      } else if (lspStartBlocker(snapshot) === null) {
        action = { label: "Start analysis", onClick: () => void startAnalysis() };
      }
      if (action) pendingAnalysisAction.current = pending;
    }
    return { tone: "error", text: `${label} unavailable: ${reason}`, action };
  }
  const goToDefinitionDisabledReason = !active
    ? "Open a file to go to a declaration."
    : !trusted
      ? "Trust the workspace before going to a declaration; no analysis runs in Restricted mode."
      : !lspActive
        ? "Start analysis before going to a declaration."
        : !lsp.capabilities?.definition
          ? "csharp-ls did not advertise definitions for this session."
          : active.conflict
            ? "Resolve the disk conflict before navigating from this file."
            : null;
  const lspSent = useRef<{ session: number; paths: Set<string> }>({
    session: 0,
    paths: new Set(),
  });
  const lspChangeTimer = useRef<number | null>(null);
  const lspPending = useRef<Map<string, string>>(new Map());
  const restoreDisabledReason = !trusted
    ? "Trust the workspace before restoring; nothing runs in Restricted mode."
    : projectSdk.state !== "resolved"
      ? "Resolve the project SDK before restoring."
      : restoring
        ? "A restore is already running."
        : null;
  const restoreAvailable = restoreDisabledReason === null;
  const restoreIncompleteAfterRun =
    restore.revision > 0 && restore.projects.some((project) => project.state !== "restored");
  const restoreIncomplete =
    restore.state !== "running" &&
    (restore.revision > 0
      ? restoreIncompleteAfterRun
      : showsEvaluatedTree && evaluatedProjects.some((project) => project.restored !== "restored"));
  const restoreProjectState = new Map(
    restore.projects.map((project) => [project.path, project.state]),
  );
  const baseConfigurations =
    evaluation.configurations.length > 0 ? evaluation.configurations : ["Debug", "Release"];
  const configurations = baseConfigurations.includes(evaluation.context.configuration)
    ? baseConfigurations
    : [evaluation.context.configuration, ...baseConfigurations];
  const tfmOptions =
    evaluation.context.targetFramework &&
    !evaluation.availableTfms.includes(evaluation.context.targetFramework)
      ? [evaluation.context.targetFramework, ...evaluation.availableTfms]
      : evaluation.availableTfms;
  const projectGroups = (() => {
    const grouped = new Map<string, EvaluationProject[]>();
    for (const project of evaluatedProjects) {
      const key = project.folder ?? "";
      grouped.set(key, [...(grouped.get(key) ?? []), project]);
    }
    return Array.from(grouped.entries());
  })();
  const build = snapshot.build;
  const consoleCandidates = evaluatedProjects.filter(
    (project) => project.kind === "console" && project.state === "ok" && project.targetPath,
  );
  // Several consoles are not an ambiguity: the saved choice wins, else the first candidate.
  const defaultStartupProject =
    consoleCandidates.find((project) => project.path === startupPath) ??
    consoleCandidates[0] ??
    null;
  const activeNamedConfig =
    runConfigStore && runConfigStore.activeId !== DEFAULT_RUN_CONFIG_ID
      ? runConfigStore.configs.find((config) => config.id === runConfigStore.activeId) ?? null
      : null;
  const { evaluating, evaluateContext, runEvaluation, startRestore, cancelRestore, selectRunConfig, commitRunConfigDialog } = useEvaluationFlow({
    snapshot, trusted, restoreAvailable, evaluationAvailable, runConfigsLoaded, activeNamedConfig, runConfigStore,
    hasUnsavedBuffers: dirtyCount > 0,
    commitRunConfigs, onSnapshot, setMessage,
  });
  const evaluationBusy = evaluating || evaluation.state === "running";
  const startupProject = activeNamedConfig
    ? consoleCandidates.find((project) => project.path === activeNamedConfig.project) ?? null
    : defaultStartupProject;
  const [launchProfileState, setLaunchProfileState] = useState<{ project: string; profiles: LaunchProfile[]; selected: string | null } | null>(null);
  const launchProfileRequest = useRef(0);
  const profileProject = defaultStartupProject?.path ?? null;
  async function refreshLaunchProfiles() {
    const request = ++launchProfileRequest.current;
    if (!profileProject) { setLaunchProfileState(null); return; }
    try {
      const result = await call<{ profiles: LaunchProfile[]; selected: string | null }>("runConfigs.launchProfiles", { solution: workspace.path, project: profileProject });
      if (request !== launchProfileRequest.current) return;
      const supported = result.profiles.filter((profile) => profile.commandName === "Project");
      const selected = supported.some((profile) => profile.name === result.selected) ? result.selected : supported[0]?.name ?? null;
      setLaunchProfileState({ project: profileProject, profiles: result.profiles, selected });
      setMessage((previous) => previous?.text.startsWith("Launch settings could not be read:") ? null : previous);
    } catch (error) {
      if (request !== launchProfileRequest.current) return;
      setLaunchProfileState({ project: profileProject, profiles: [], selected: null });
      setMessage({ tone: "error", text: `Launch settings could not be read: ${messageOf(error)}` });
    }
  }
  useEffect(() => {
    void refreshLaunchProfiles();
    return () => { launchProfileRequest.current += 1; };
  }, [workspace.path, profileProject, evaluation.revision]);
  const profileState = launchProfileState?.project === profileProject ? launchProfileState : null;
  const baseDefaultRunConfig = deriveDefaultRunConfig(defaultStartupProject, evaluation);
  const defaultRunConfig = baseDefaultRunConfig && profileState?.selected
    ? { ...baseDefaultRunConfig, name: `${defaultStartupProject?.name} · ${profileState.selected}`, launchProfile: profileState.selected }
    : baseDefaultRunConfig;
  const activeRunConfig: RunConfigView | null = activeNamedConfig ?? defaultRunConfig;
  const activeRunConfigIssues = activeRunConfig
    ? (runConfigStore?.issues ?? []).filter(
        (issue) => issue.configId === activeRunConfig.id,
      )
    : [];
  const activeConfigEvaluated =
    !activeNamedConfig ||
    (activeNamedConfig.configuration === evaluation.context.configuration &&
      (activeNamedConfig.targetFramework ?? null) ===
        (evaluation.context.targetFramework ?? null));
  const buildRunning = build.state === "running";
  const buildObsolete = isBuildObsolete(build, editedAt);
  useEffect(() => {
    panelBodyRef.current?.scrollTo(0, 0);
  }, [bottomTab]);
  const tests = snapshot.tests;
  const testDiscovery = tests.discovery;
  const testCandidates = evaluatedProjects.filter(
    (project) => project.kind === "test" && project.state === "ok",
  );
  const testsDisabledReason = !trusted
    ? "Trust the workspace before discovering tests; nothing runs in Restricted mode."
    : projectSdk.state !== "resolved"
      ? "Resolve the project SDK before discovering tests."
      : evaluation.state === "running"
        ? "Wait for the evaluation to finish before discovering tests."
        : evaluation.stale
          ? "The evaluated project model is stale; re-evaluate before discovering tests."
          : !["ok", "partial"].includes(evaluation.state)
            ? "Evaluate the project model before discovering tests; discovery uses its project list."
            : restoreIncomplete
              ? "Dependencies are not restored; run the explicit restore first."
              : testCandidates.length === 0
                ? "No evaluated test project is available; open a solution with a test project and evaluate it."
                : testDiscovery.state === "discovering"
                  ? "A test discovery is already running."
                  : null;
  const testDiscoveryKey = `${testDiscovery.discoveryId ?? "none"}:${testDiscovery.revision}`;
  useEffect(() => {
    if (testCandidates.some((project) => project.path === testProjectPath)) return;
    setTestProjectPath(testCandidates.length === 1 ? testCandidates[0].path : "");
    // Selection follows the evaluated project list, not typing.
  }, [testCandidates.map((project) => project.path).join("|"), testProjectPath]);
  useEffect(() => {
    setTestSelected(new Set());
  }, [testDiscoveryKey]);
  const testDiscoveryBoundary = `${workspace.id}:${snapshot.contextId}:${restore.revision}:${testProjectPath}`;
  useEffect(() => {
    // One automatic discovery per workspace/context/restore boundary and selected test project;
    // the ref keeps renders, edits and panel openings from launching it again. A running restore
    // has no revision yet and the host would reject discovery for missing assets.
    if (!testProjectPath || testsDisabledReason !== null || restoring) return;
    if (testDiscoveryAutoStarted.current === testDiscoveryBoundary) return;
    testDiscoveryAutoStarted.current = testDiscoveryBoundary;
    void startTestDiscovery(testProjectPath, testDiscoveryBoundary);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [testDiscoveryBoundary, testsDisabledReason, restoring]);
  const testProject = testCandidates.find((project) => project.path === testProjectPath) ?? null;
  const testRunBlocked = testRunBlockedReason({
    disabledReason: testsDisabledReason,
    discovery: testDiscovery,
    run: snapshot.tests.run,
    editedAt,
  });
  const testRerunBlocked = rerunFailedReason({
    disabledReason: testsDisabledReason,
    discovery: testDiscovery,
    run: snapshot.tests.run,
    editedAt,
  });
  const run = snapshot.run;
  const runRunning = run.state === "running";
  const runBuilding = run.state === "building";
  const dap = snapshot.dap;
  const debugBuilding = dap.state === "building";
  const debugActive = ["building", "preparing", "running", "paused"].includes(dap.state);
  const gates: GateContext = {
    trusted,
    snapshot,
    activeConfigEvaluated,
    restoreIncomplete,
    consoleCandidates,
    startupProject,
    activeConfigProjectInvalid: Boolean(
      activeNamedConfig && !consoleCandidates.some((project) => project.path === activeNamedConfig.project),
    ),
    runRunning,
    runBuilding,
    debugActive,
    debugBuilding,
  };
  const buildDisabledReason = buildGate(gates);
  const runAvailabilityReason = (
    project: EvaluationProject | null,
    configEvaluated: boolean,
    invalidSelectedProject = false,
  ) => runGate(gates, project, configEvaluated, invalidSelectedProject);
  const runDisabledReason = runAvailabilityReason(
    startupProject,
    activeConfigEvaluated,
    gates.activeConfigProjectInvalid,
  );
  const debugDisabledReason = debugGate(gates);
  const buildSolutionDisabledReason =
    workspace.kind !== "solution"
      ? "Build Solution needs an open solution; build the single project with Build."
      : buildDisabledReason;
  const debugCurrentFrame =
    dap.frames.find((frame) => frame.id === dap.selectedFrameId) ?? dap.frames[0] ?? null;
  const activeBreakpointEntries = active
    ? dap.breakpoints.filter((breakpoint) => breakpoint.path === active.path)
    : [];
  const activeBreakpoints = activeBreakpointEntries.map((breakpoint) => breakpoint.line);
  const activeVerifiedBreakpoints = activeBreakpointEntries
    .filter((breakpoint) => breakpoint.verified)
    .map((breakpoint) => breakpoint.line);
  const debugCurrentLine =
    active && debugCurrentFrame?.path
      ? debugCurrentFrame.path.replace(/\\/g, "/").endsWith(`/${active.path}`)
        ? debugCurrentFrame.line
        : null
      : null;
  const debugInlay = debugCurrentLine === null
    ? ""
    : (dap.locals?.scopes[0]?.variables ?? [])
        .slice(0, 4)
        .map((variable) => `${variable.name} = ${variable.value.length > 32 ? `${variable.value.slice(0, 31)}…` : variable.value}`)
        .join(", ");
  const activeProjectPath = activeEvaluationProject?.path ?? activeProject?.path ?? null;
  const [changedState, setChangedState] = useState<{ path: string; lines: ChangedLine[] }>({ path: "", lines: [] });
  const changedLines = changedState.path === active?.path ? changedState.lines : NO_CHANGED_LINES;
  const changedGitFile = active ? gitStatus?.files.find((entry) => entry.path === active.path) : undefined;
  useEffect(() => {
    // Saved file vs HEAD; staged-only files diff against the index, untracked files have no HEAD to compare.
    const path = active?.path;
    if (!path || !trusted || !changedGitFile || changedGitFile.untracked || changedGitFile.conflicted) {
      setChangedState({ path: path ?? "", lines: [] });
      return;
    }
    let live = true;
    const staged = changedGitFile.staged && changedGitFile.worktree === " ";
    call<GitDiffView>("git.diff", { path, staged })
      .then((answer) => live && setChangedState({ path, lines: changedLinesFromDiff(answer.diff ?? "") }))
      .catch(() => live && setChangedState({ path, lines: [] }));
    return () => { live = false; };
  }, [active?.path, trusted, changedGitFile?.index, changedGitFile?.worktree, gitStatus]);

  const gutterActions = useMemo(() => {
    if (
      !active ||
      !documentSymbols ||
      documentSymbols.path !== active.path ||
      documentSymbols.revision !== active.revision
    ) {
      return [];
    }
    const bulbs: GutterAction[] = [];
    const seen = new Set<number>();
    for (const snap of snapshot.diagnostics.snapshots) {
      if (snap.state !== "current" || !snap.path || !snap.path.replace(/\\/g, "/").endsWith(`/${active.path}`)) continue;
      for (const item of snap.items) {
        if (!item.position || seen.has(item.position.line)) continue;
        seen.add(item.position.line);
        bulbs.push({
          line: item.position.line + 1,
          character: item.position.character,
          kind: "fix",
          label: `Show code actions for: ${item.message}`,
          disabledReason: codeActionsDisabledReason,
          testId: null,
          testProject: null,
        });
      }
    }
    return [
      ...gutterActionsFor({
        symbols: documentSymbols.response.symbols,
        projectPath: activeProjectPath,
        startupProjectPath: startupProject?.path ?? null,
        testItems: testDiscovery.items,
        runDisabledReason,
        testDisabledReason: testRunBlocked,
      }),
      ...bulbs,
    ];
  }, [
    active?.path,
    active?.revision,
    documentSymbols,
    activeProjectPath,
    startupProject?.path,
    testDiscovery.items,
    runDisabledReason,
    testRunBlocked,
    snapshot.diagnostics,
    codeActionsDisabledReason,
  ]);

  useEffect(() => {
    if (evaluation.revision === 0) {
      seededTreeExpansion.current = 0;
      return;
    }
    // Seed the opened/closed state once; a re-evaluation (including the automatic one after an
    // external change) keeps what the user collapsed or expanded.
    if (seededTreeExpansion.current === 0) {
      setExpanded(expandableKeys(evaluatedProjects, []));
    }
    seededTreeExpansion.current = evaluation.revision;
    setSelectedProjectPath((current) => {
      if (evaluatedProjects.some((project) => project.path === current)) {
        return current;
      }
      const failed = evaluatedProjects.find((project) => project.state === "failed");
      return (failed ?? evaluatedProjects[0])?.path ?? null;
    });
  }, [evaluation.revision]);

  useEffect(() => {
    // Poll the host so process states and appending trust decisions are not stale. The
    // interval is local to this screen and cheap against the local host.
    let latest = 0;
    const timer = setInterval(() => {
      const sequence = ++latest;
      call<Snapshot>("workspace.state")
        .then((next) => {
          // A slow answer must not overwrite a newer state.
          if (sequence === latest) onSnapshot(next);
        })
        .catch(() => {});
    }, 400);
    return () => clearInterval(timer);
  }, []);


  useEffect(() => {
    if (initialFile) {
      void openFile(initialFile);
    }
  }, []);

  useEffect(() => {
    const onFocus = () => {
      const path = activePathRef.current;
      if (path) void checkDisk(path);
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, []);

  useEffect(() => {
    // A visible window re-reads the active file so a clean buffer never shows stale text.
    // A hidden window relies on the focus re-read instead of polling the disk.
    // shortcut: re-reads the whole active file every 1.5 s; add a host file.hash if this shows in profiles.
    const timer = setInterval(() => {
      if (document.hidden) return;
      const path = activePathRef.current;
      if (path) void checkDisk(path);
    }, 1500);
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    const onBeforeUnload = (event: BeforeUnloadEvent) => {
      if (!unsavedRef.current) return;
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => window.removeEventListener("beforeunload", onBeforeUnload);
  }, []);

  useEffect(() => {
    // Tauri: the native close button must go through the same in-app decision. Chrome/HTTP
    // relies on beforeunload above, and the window dialog is unreachable there.
    if (!window.__TAURI_INTERNALS__) return;
    let unlisten: (() => void) | null = null;
    let disposed = false;
    void import("@tauri-apps/api/window")
      .then(async ({ getCurrentWindow }) => {
        const stop = await getCurrentWindow().onCloseRequested((event) => {
          if (!unsavedRef.current) return;
          event.preventDefault();
          setConfirmError(null);
          setConfirm({ kind: "window" });
        });
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    // Cmd+Q and the app menu Quit reach this view only when it owns the buffers.
    if (!quitRequest || quitRequest === handledQuitRequest.current) return;
    handledQuitRequest.current = quitRequest;
    if (!unsavedRef.current) {
      void quitApp();
      return;
    }
    setConfirmError(null);
    // A decision already on screen wins; a second prompt would stack two dialogs.
    setConfirm((current) => current ?? { kind: "quit" });
  }, [quitRequest]);

  useEffect(() => {
    // Send didOpen once per session for every open buffer. A restart creates a new session,
    // so the full set is sent again with the current buffer text.
    if (!lspActive) return;
    if (lspSent.current.session !== lsp.session) {
      lspSent.current = { session: lsp.session, paths: new Set() };
    }
    for (const tab of tabs) {
      if (lspSent.current.paths.has(tab.path)) continue;
      lspSent.current.paths.add(tab.path);
      void call("lsp.didOpen", { path: tab.path, text: tab.content }).catch(() => {
        if (lspSent.current.session === lsp.session) {
          lspSent.current.paths.delete(tab.path);
        }
      });
    }
  }, [lsp.session, lsp.state, tabs]);

  useEffect(() => {
    // Gutter positions come from the server's document symbols, never from source parsing.
    // No analysis, capability or open document means no positions and therefore no icons.
    const path = active?.path;
    if (!path || !active || !trusted || !lspActive || !lsp.capabilities?.documentSymbol) {
      setDocumentSymbols(null);
      return;
    }
    const revision = active.revision;
    const session = lsp.session;
    const contextId = snapshot.contextId;
    const workspaceId = workspace.id;
    setDocumentSymbols((current) =>
      current &&
      current.path === path &&
      current.session === session &&
      current.contextId === contextId &&
      current.workspaceId === workspaceId
        ? current
        : null,
    );
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void (async () => {
        const synchronized = await flushPendingChanges();
        if (cancelled || !synchronized) return;
        try {
          const response = await call<DocumentSymbolResponse>("lsp.documentSymbol", { path });
          if (cancelled) return;
          const tab = tabsRef.current.find((item) => item.path === path);
          const live = lspViewRef.current;
          if (
            !tab ||
            tab.revision !== revision ||
            live.session !== response.session ||
            contextIdRef.current !== response.contextId ||
            workspaceIdRef.current !== response.workspaceId ||
            activePathRef.current !== path
          ) {
            return;
          }
          setDocumentSymbols({
            path,
            revision,
            session: response.session,
            contextId: response.contextId,
            workspaceId: response.workspaceId,
            response,
          });
        } catch {
          if (!cancelled) {
            setDocumentSymbols((current) =>
              current && current.path === path && current.session === session
                ? null
                : current,
            );
          }
        }
      })();
    }, 350);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [
    active?.path,
    active?.revision,
    lspActive,
    lsp.session,
    lsp.capabilities?.documentSymbol,
    snapshot.contextId,
    workspace.id,
    trusted,
  ]);

  function queueDidChange(path: string, text: string) {
    if (!lspActiveRef.current) return;
    lspPending.current.set(path, text);
    if (lspChangeTimer.current !== null) {
      window.clearTimeout(lspChangeTimer.current);
    }
    lspChangeTimer.current = window.setTimeout(() => {
      lspChangeTimer.current = null;
      for (const [pendingPath, pendingText] of lspPending.current) {
        void call("lsp.didChange", { path: pendingPath, text: pendingText }).catch(() => {});
      }
      lspPending.current.clear();
    }, 250);
  }

  useEffect(() => {
    if (editPreview) editPreviewHeading.current?.focus();
  }, [editPreview ? previewId(editPreview) : null]);

  useEffect(() => {
    if (!editPreview) return;
    if (activePath !== editPreview.sourcePath) {
      editPreviewApplySeq.current += 1;
      setEditPreview(null);
      setMessage({
        tone: "info",
        text: `${previewKindLabel(editPreview)} preview discarded: another file became active; nothing was applied.`,
      });
    }
    // For rename, buffer edits are rejected by Apply so the user sees the stale reason.
    // Code-action previews close immediately (effect below), as required by ticket 10.
  }, [activePath, editPreview ? previewId(editPreview) : null]);

  useEffect(() => {
    if (actionList) actionListHeading.current?.focus();
  }, [actionList]);

  useEffect(() => {
    if (!actionList) return;
    if (activePath !== actionList.sourcePath) {
      setActionList(null);
      return;
    }
    const tab = tabs.find((item) => item.path === actionList.sourcePath);
    if (!tab || tab.revision !== actionList.sourceRevision || tab.conflict) {
      setActionList(null);
      setMessage({
        tone: "info",
        text: "Code actions discarded: the source buffer changed; request them again.",
      });
      return;
    }
    const live = lspViewRef.current;
    if (
      !["loading", "ready", "partial", "awaiting_diagnostics"].includes(live.state) ||
      live.session !== actionList.session ||
      contextIdRef.current !== actionList.contextId ||
      workspaceIdRef.current !== actionList.workspaceId
    ) {
      setActionList(null);
      setMessage({
        tone: "error",
        text: "Code actions discarded: analysis or the workspace changed; nothing was applied.",
      });
    }
  }, [actionList, activePath, tabs, lsp.state, lsp.session, snapshot.contextId, workspace.id]);

  useEffect(() => {
    if (!editPreview || editPreview.kind !== "action") return;
    const tab = tabs.find((item) => item.path === editPreview.sourcePath);
    if (!tab || tab.revision !== editPreview.sourceRevision || tab.conflict) {
      editPreviewApplySeq.current += 1;
      setEditPreview(null);
      setMessage({
        tone: "info",
        text: "Code action preview discarded: the source buffer changed; nothing was applied.",
      });
      return;
    }
    if (
      !lspActive ||
      lsp.session !== editPreview.session ||
      contextIdRef.current !== editPreview.contextId ||
      workspaceIdRef.current !== editPreview.workspaceId
    ) {
      editPreviewApplySeq.current += 1;
      setEditPreview(null);
      setMessage({
        tone: "error",
        text: "Code action preview discarded: analysis or the workspace changed; nothing was applied.",
      });
    }
  }, [editPreview, tabs, lspActive, lsp.session, snapshot.contextId, workspace.id]);

  function toggleProject(path: string) {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }

  /** Reopens the saved tab paths once per workspace; deleted files are skipped, not fatal. */
  async function restoreSession(saved: SessionState) {
    const opened: string[] = [];
    for (const tab of saved.tabs) {
      if (await openFile(tab.path)) opened.push(tab.path);
    }
    savedBreakpoints.current = saved.breakpoints;
    // Every opened tab mounted an editor that reported 0:0; the saved cursor wins for all tabs.
    for (const tab of saved.tabs) {
      tabCursors.current[tab.path] = { line: tab.line, character: tab.character };
    }
    const target =
      saved.activePath && opened.includes(saved.activePath)
        ? saved.activePath
        : opened[opened.length - 1];
    if (target) {
      await openFile(target);
      const position = tabCursors.current[target];
      navigationSeq.current += 1;
      setEditorTarget({
        path: target,
        line: position?.line ?? 0,
        character: position?.character ?? 0,
        nonce: navigationSeq.current,
      });
    }
    setSessionReady(true);
  }

  async function openFile(path: string) {
    setCommitDiffActive(false);
    const relative = workspaceRelative(path, workspaceDir);
    if (tabs.some((tab) => tab.path === relative)) {
      setActivePath(relative);
      void checkDisk(relative);
      return true;
    }
    try {
      const file = await call<{ path: string; content: string; hash: string }>("file.read", {
        path: relative,
      });
      setTabs((current) =>
        current.some((tab) => tab.path === file.path)
          ? current
          : [
              ...current,
              {
                path: file.path,
                content: file.content,
                hash: file.hash,
                revision: 0,
                savedContent: file.content,
                conflict: null,
              },
            ],
      );
      setActivePath(file.path);
      setMessage(null);
      return true;
    } catch (error) {
      setMessage({ tone: "error", text: `Could not open ${path}: ${messageOf(error)}` });
      return false;
    }
  }

  function navEntry(): NavEntry | null {
    const path = activePathRef.current;
    return path
      ? { path, line: cursorRef.current.line, character: cursorRef.current.character }
      : null;
  }

  function syncNavEnabled() {
    const { entries, index } = navHistory.current;
    setNavEnabled({
      back: index > 0,
      forward: index >= 0 && index < entries.length - 1,
    });
  }

  /** A navigation starts at the first location only; the destination is the new history tip. */
  function recordNavigation(from: NavEntry | null, to: NavEntry) {
    if (!from) return;
    const history = navHistory.current;
    const entries = history.entries.slice(0, history.index + 1);
    if (entries.length === 0) entries.push(from);
    entries.push(to);
    const limited = entries.length > 100 ? entries.slice(entries.length - 100) : entries;
    navHistory.current = { entries: limited, index: limited.length - 1 };
    syncNavEnabled();
  }

  async function openUserFile(path: string) {
    if (path === activePathRef.current) {
      await openFile(path);
      return;
    }
    const from = navEntry();
    if (!(await openFile(path))) return;
    recordNavigation(from, { path, line: 0, character: 0 });
  }

  async function navigateTo(path: string, position: { line: number; character: number }) {
    const relative = workspaceRelative(path, workspaceDir);
    const from = navEntry();
    if (!(await openFile(relative))) return;
    recordNavigation(from, { path: relative, ...position });
    navigationSeq.current += 1;
    setEditorTarget({
      path: relative,
      line: position.line,
      character: position.character,
      nonce: navigationSeq.current,
    });
  }

  async function travel(delta: number) {
    const history = navHistory.current;
    const next = history.index + delta;
    const entry = history.entries[next];
    if (!entry) return;
    if (!(await openFile(entry.path))) return;
    navHistory.current = { ...history, index: next };
    syncNavEnabled();
    navigationSeq.current += 1;
    setEditorTarget({
      path: entry.path,
      line: entry.line,
      character: entry.character,
      nonce: navigationSeq.current,
    });
  }

  function bumpDiskEpoch(path: string) {
    diskEpoch.current.set(path, (diskEpoch.current.get(path) ?? 0) + 1);
  }

  async function checkDisk(path: string) {
    // Proactive read. A clean buffer is replaced by the disk text; a dirty one only raises a
    // conflict and is never overwritten. A read failure is reported and the buffer stays editable.
    if (writing.current.has(path) || !tabsRef.current.some((tab) => tab.path === path)) return;
    // One read at a time per path; the poll must not stack requests while a read is in flight.
    if (checkingDisk.current.has(path)) return;
    const epoch = diskEpoch.current.get(path) ?? 0;
    checkingDisk.current.add(path);
    let file: { content: string; hash: string };
    try {
      file = await call<{ content: string; hash: string }>("file.read", { path });
    } catch (error) {
      setMessage({
        tone: "error",
        text: `Could not re-read ${path} from disk; your buffer is unchanged. ${messageOf(error)}`,
      });
      return;
    } finally {
      checkingDisk.current.delete(path);
    }
    if ((diskEpoch.current.get(path) ?? 0) !== epoch) return;
    setTabs((current) =>
      current.map((tab) => {
        if (tab.path !== path) return tab;
        const known = tab.conflict?.hash ?? tab.hash;
        if (file.hash === known) return tab;
        if (!tab.conflict && tab.content === tab.savedContent) {
          return { ...tab, content: file.content, hash: file.hash, savedContent: file.content };
        }
        return { ...tab, conflict: { content: file.content, hash: file.hash } };
      }),
    );
  }

  function updateContent(path: string, content: string) {
    setTabs((current) =>
      current.map((tab) =>
        tab.path === path ? { ...tab, content, revision: tab.revision + 1 } : tab,
      ),
    );
    setEditedAt((current) => ({ ...current, [path]: Date.now() }));
    queueDidChange(path, content);
  }

  function applyConflict(path: string, error: HostCallError) {
    const details = error.details as { currentContent?: string; currentHash?: string } | undefined;
    if (details?.currentContent === undefined || !details.currentHash) {
      const text = `Save failed; buffer kept. ${error.message}`;
      setMessage({ tone: "error", text });
      return text;
    }
    const text = `${path} changed on disk. Your buffer was kept; choose Reload from disk or Overwrite disk.`;
    setTabs((current) =>
      current.map((tab) =>
        tab.path === path
          ? { ...tab, conflict: { content: details.currentContent!, hash: details.currentHash! } }
          : tab,
      ),
    );
    setMessage({ tone: "error", text });
    return text;
  }

  async function saveTab(tab: Tab): Promise<{ ok: boolean; error?: string }> {
    if (tab.conflict) {
      const error = `${tab.path} has a pending disk conflict; choose Reload from disk or Overwrite disk first. Your buffer was kept.`;
      setMessage({ tone: "error", text: error });
      return { ok: false, error };
    }
    if (!isDirty(tab)) {
      setMessage({ tone: "info", text: "No changes to save." });
      return { ok: true };
    }
    const content = tab.content;
    writing.current.add(tab.path);
    try {
      const result = await call<{ path: string; hash: string }>("file.write", {
        path: tab.path,
        content,
        expected_hash: tab.hash,
      });
      setTabs((current) =>
        current.map((item) =>
          item.path === tab.path
            ? { ...item, hash: result.hash, savedContent: content, conflict: null }
            : item,
        ),
      );
      setSavedAt((current) => ({ ...current, [tab.path]: Date.now() }));
      setMessage({ tone: "info", text: `Saved ${tab.path}` });
      return { ok: true };
    } catch (error) {
      if (error instanceof HostCallError && error.code === "conflict") {
        return { ok: false, error: applyConflict(tab.path, error) };
      }
      const text = `Save failed; buffer kept. ${messageOf(error)}`;
      setMessage({ tone: "error", text });
      return { ok: false, error: text };
    } finally {
      writing.current.delete(tab.path);
      bumpDiskEpoch(tab.path);
    }
  }

  async function saveAll() {
    const failures: string[] = [];
    for (const tab of tabsRef.current) {
      if (!isDirty(tab)) continue;
      const result = await saveTab(tab);
      if (!result.ok) failures.push(result.error ?? `Could not save ${tab.path}.`);
    }
    return failures;
  }

  async function save() {
    const tab = active;
    if (!tab) return;
    await saveTab(tab);
  }

  function noteCursor(position: { line: number; character: number }) {
    if (activePathRef.current) tabCursors.current[activePathRef.current] = position;
    cursorRef.current = position;
    setCursor(position);
    setActionList((current) => {
      if (!current) return current;
      if (current.line === position.line && current.character === position.character) {
        return current;
      }
      return null;
    });
  }

  function requestRename(position = cursorRef.current) {
    if (!active) return;
    if (renameDisabledReason) {
      setMessage(analysisNotice("Rename", renameDisabledReason, { kind: "rename", position }));
      return;
    }
    renameSeq.current += 1;
    noteCursor(position);
    setMessage(null);
    setRenameRequest((current) => current + 1);
  }

  function cancelRenameRequest() {
    renameSeq.current += 1;
    setMessage(null);
  }

  function handleGutterAction(action: GutterAction) {
    if (action.kind === "fix") {
      void requestCodeActions({ line: action.line - 1, character: action.character ?? 0 });
      return;
    }
    if (action.kind === "run") {
      requestRun();
      return;
    }
    if (!action.testId || !action.testProject) return;
    if (testRunBlocked) {
      setMessage({ tone: "error", text: `Test run unavailable: ${testRunBlocked}` });
      return;
    }
    void startTestRun(action.testProject, "selected", [action.testId]);
  }

  function openCommitPanel(path: string | null = null) {
    setCommitFocusPath(path);
    setLayout((current) => ({
      ...current,
      left: { ...current.left, active: "commit", open: true },
    }));
  }

  async function showGitDiff(path: string) {
    const file = gitStatus?.files.find((entry) => entry.path === path);
    const staged = Boolean(file && file.staged && file.worktree === " ");
    try {
      setGitDiffView(await call<GitDiffView>("git.diff", { path, staged }));
    } catch (error) {
      setGitDiffView({
        state: "failed",
        message: messageOf(error),
        path,
        staged,
        diff: "",
        truncated: false,
      });
    }
  }

  async function copyPath(path: string) {
    try {
      if (!navigator.clipboard) throw new Error("the clipboard is not accessible in this context");
      await navigator.clipboard.writeText(path);
      setMessage({ tone: "info", text: `Copied path: ${path}` });
    } catch (error) {
      setMessage({ tone: "error", text: `Copy Path unavailable: ${messageOf(error)}` });
    }
  }

  async function revealInFinder(path: string) {
    try {
      await call("workspace.reveal", { path });
    } catch (error) {
      setMessage({ tone: "error", text: `Show in Finder failed: ${messageOf(error)}` });
    }
  }

  async function requestCodeActions(position = cursorRef.current) {
    if (!active) return;
    if (codeActionsDisabledReason) {
      setMessage(
        analysisNotice("Code actions", codeActionsDisabledReason, { kind: "codeActions", position }),
      );
      return;
    }
    const sourcePath = active.path;
    const sourceRevision = active.revision;
    const sequence = ++actionSeq.current;
    setActionList(null);
    setMessage(null);
    if (!lspActiveRef.current) {
      setMessage({ tone: "error", text: "Code actions unavailable: analysis is not running." });
      return;
    }
    // csharp-ls must see the buffer the actions are requested for; a pending debounce would
    // compute them from older text.
    const synchronized = await flushPendingChanges();
    if (sequence !== actionSeq.current) return;
    if (!synchronized) {
      setMessage({
        tone: "error",
        text: "Code actions unavailable: the current buffer could not be sent to analysis.",
      });
      return;
    }
    const tab = tabsRef.current.find((item) => item.path === sourcePath);
    const live = lspViewRef.current;
    if (
      !tab ||
      tab.revision !== sourceRevision ||
      tab.conflict ||
      live.session !== lsp.session
    ) {
      setMessage({
        tone: "error",
        text: "Code actions discarded: the document or analysis session changed before the request.",
      });
      return;
    }
    let response: CodeActionsResponse;
    try {
      response = await call<CodeActionsResponse>("lsp.codeActions", {
        path: sourcePath,
        line: position.line,
        character: position.character,
        revision: sourceRevision,
      });
    } catch (error) {
      if (sequence !== actionSeq.current) return;
      setMessage({ tone: "error", text: `Code actions unavailable: ${messageOf(error)}` });
      return;
    }
    if (sequence !== actionSeq.current) return;
    const currentTab = tabsRef.current.find((item) => item.path === sourcePath);
    const liveAfter = lspViewRef.current;
    if (
      !currentTab ||
      currentTab.revision !== sourceRevision ||
      currentTab.conflict ||
      liveAfter.session !== response.session ||
      contextIdRef.current !== snapshot.contextId ||
      workspaceIdRef.current !== workspace.id ||
      activePathRef.current !== sourcePath ||
      cursorRef.current.line !== position.line ||
      cursorRef.current.character !== position.character
    ) {
      setMessage({
        tone: "error",
        text: "Code actions discarded: the document, cursor, workspace or analysis session changed while csharp-ls was answering; nothing was applied.",
      });
      return;
    }
    const tabRevisions: Record<string, number> = {};
    for (const openTab of tabsRef.current) tabRevisions[openTab.path] = openTab.revision;
    setActionList({
      response,
      sourcePath,
      sourceRevision,
      line: position.line,
      character: position.character,
      session: response.session,
      contextId: response.contextId,
      workspaceId: response.workspaceId,
      tabRevisions,
      stale: response.state === "ok" ? null : response.message,
    });
    const where = `${sourcePath} · ${position.line + 1}:${position.character + 1} · revision ${sourceRevision} · session ${response.session}`;
    if (response.state !== "ok") {
      setMessage({
        tone: "error",
        text: `Code actions ${response.state}: ${response.message ?? "csharp-ls did not answer."}`,
      });
    } else if (response.actionCount === 0) {
      setMessage({
        tone: "info",
        text: `Code actions: csharp-ls returned no actions at ${where}; editing continues.`,
      });
    } else {
      setMessage({
        tone: "info",
        text: `Code actions: ${countLabel(response.actionCount, "action")} from csharp-ls · ${where}${
          response.unsupportedActions > 0
            ? ` · ${response.unsupportedActions} listed as unsupported`
            : ""
        }; nothing has been applied.`,
      });
    }
  }

  function cancelActionList() {
    actionSeq.current += 1;
    setActionList(null);
    setMessage({ tone: "info", text: "Code actions closed; nothing was applied." });
  }

  async function requestFindUsages(position = cursorRef.current) {
    if (!active) return;
    if (findUsagesDisabledReason) {
      setMessage(
        analysisNotice("Find Usages", findUsagesDisabledReason, { kind: "findUsages", position }),
      );
      return;
    }
    const sourcePath = active.path;
    const sourceRevision = active.revision;
    const session = lsp.session;
    const contextId = snapshot.contextId;
    const workspaceId = workspace.id;
    const sequence = ++findUsagesSeq.current;
    setFindUsages(null);
    setMessage(null);
    const synchronized = await flushPendingChanges();
    if (sequence !== findUsagesSeq.current) return;
    if (!synchronized) {
      setMessage({
        tone: "error",
        text: "Find Usages unavailable: the current buffer could not be sent to analysis.",
      });
      return;
    }
    const tab = tabsRef.current.find((item) => item.path === sourcePath);
    const live = lspViewRef.current;
    if (
      !tab ||
      tab.revision !== sourceRevision ||
      tab.conflict ||
      live.session !== session ||
      activePathRef.current !== sourcePath
    ) {
      setMessage({
        tone: "error",
        text: "Find Usages discarded: the document or analysis session changed before the request.",
      });
      return;
    }
    let response: ReferencesResponse;
    try {
      response = await call<ReferencesResponse>("lsp.references", {
        path: sourcePath,
        line: position.line,
        character: position.character,
        revision: sourceRevision,
      });
    } catch (error) {
      if (sequence !== findUsagesSeq.current) return;
      setMessage({ tone: "error", text: `Find Usages unavailable: ${messageOf(error)}` });
      return;
    }
    if (sequence !== findUsagesSeq.current) return;
    const currentTab = tabsRef.current.find((item) => item.path === sourcePath);
    const liveAfter = lspViewRef.current;
    if (
      !currentTab ||
      currentTab.revision !== sourceRevision ||
      currentTab.conflict ||
      liveAfter.session !== response.session ||
      contextIdRef.current !== response.contextId ||
      workspaceIdRef.current !== response.workspaceId ||
      activePathRef.current !== sourcePath
    ) {
      setMessage({
        tone: "error",
        text: "Find Usages discarded: the document, workspace or analysis session changed while csharp-ls was answering.",
      });
      return;
    }
    const snippets: Record<string, string | null> = {};
    const byPath = new Map<string, number[]>();
    for (const reference of response.references) {
      byPath.set(reference.path, [...(byPath.get(reference.path) ?? []), reference.line]);
    }
    await Promise.all(
      [...byPath.entries()].map(async ([path, lines]) => {
        const open = tabsRef.current.find((item) => item.path === path);
        let content: string | null = open?.content ?? null;
        if (content === null) {
          try {
            content = (await call<{ content: string }>("file.read", { path })).content;
          } catch {
            content = null;
          }
        }
        for (const line of lines) {
          snippets[`${path}:${line}`] = content === null ? null : lineSnippet(content, line);
        }
      }),
    );
    if (sequence !== findUsagesSeq.current) return;
    setFindUsages({ response, snippets });
    if (response.state !== "ok") {
      setMessage({
        tone: "error",
        text: `Find Usages ${response.state}: ${response.message ?? "csharp-ls did not answer."}`,
      });
    } else if (response.referenceCount === 0) {
      setMessage({
        tone: "info",
        text: `Find Usages: csharp-ls returned no references at ${sourcePath}:${position.line + 1}:${position.character + 1}.`,
      });
    } else {
      setMessage({
        tone: "info",
        text: `Find Usages: ${countLabel(response.referenceCount, "reference")} from csharp-ls; the list is read-only.`,
      });
    }
  }

  function revalidateActionList(list: ActionList): string | null {
    const live = lspViewRef.current;
    if (!["loading", "ready", "partial", "awaiting_diagnostics"].includes(live.state)) {
      return "Analysis is not running; nothing was applied.";
    }
    if (live.session !== list.session) {
      return "The analysis session changed after the actions were listed; nothing was applied.";
    }
    if (
      contextIdRef.current !== list.contextId ||
      workspaceIdRef.current !== list.workspaceId
    ) {
      return "The workspace changed after the actions were listed; nothing was applied.";
    }
    if (activePathRef.current !== list.sourcePath) {
      return `Another file is active; select ${list.sourcePath} and request code actions again.`;
    }
    const tab = tabsRef.current.find((item) => item.path === list.sourcePath);
    if (!tab || tab.revision !== list.sourceRevision || tab.conflict) {
      return "The source buffer changed after the actions were listed; nothing was applied.";
    }
    return null;
  }

  function chooseCodeAction(action: CodeAction) {
    const list = actionList;
    if (!list) return;
    const reason = revalidateActionList(list);
    if (reason) {
      setActionList({ ...list, stale: reason });
      setMessage({ tone: "error", text: `Code action not applied: ${reason}` });
      return;
    }
    if (action.disabled) {
      const text = `Code action “${action.title}” is disabled by csharp-ls: ${action.disabled}. Nothing was applied.`;
      console.warn(text);
      setMessage({ tone: "error", text });
      return;
    }
    if (!action.supported) {
      const text = `Code action “${action.title}” was not applied: ${
        action.reason ?? "csharp-ls did not return a supported edit."
      }${action.command ? ` The server command “${action.command.command}” is never executed.` : ""}`;
      console.warn(text);
      setMessage({ tone: "error", text });
      return;
    }
    if (action.documents.length === 0) {
      const text = `Code action “${action.title}” was not applied: csharp-ls returned no documents.`;
      console.warn(text);
      setMessage({ tone: "error", text });
      return;
    }
    const tabRevisions: Record<string, number> = {};
    for (const tab of tabsRef.current) tabRevisions[tab.path] = tab.revision;
    editPreviewApplySeq.current += 1;
    setActionList(null);
    setEditPreview({
      kind: "action",
      action,
      response: list.response,
      sourcePath: list.sourcePath,
      sourceRevision: list.sourceRevision,
      session: list.session,
      contextId: list.contextId,
      workspaceId: list.workspaceId,
      tabRevisions,
      stale: null,
      applying: false,
    });
    setMessage({
      tone: "info",
      text: `Code action preview: ${action.title} · ${countLabel(
        action.documentCount,
        "file",
      )} · ${countLabel(action.editCount, "edit")}; nothing has been applied.`,
    });
  }

  async function flushPendingChanges() {
    if (lspChangeTimer.current !== null) {
      window.clearTimeout(lspChangeTimer.current);
      lspChangeTimer.current = null;
    }
    const pending = [...lspPending.current];
    lspPending.current.clear();
    if (pending.length === 0) return true;
    const results = await Promise.allSettled(
      pending.map(([path, text]) => call("lsp.didChange", { path, text })),
    );
    return results.every((result) => result.status === "fulfilled");
  }

  async function submitRenameInline(
    position: { line: number; character: number },
    newName: string,
  ): Promise<{ ok: boolean; error?: string }> {
    const tab = tabsRef.current.find((item) => item.path === activePathRef.current);
    if (!tab) return { ok: false, error: "Rename unavailable: no file is active." };
    if (renameDisabledReason) {
      return { ok: false, error: `Rename unavailable: ${renameDisabledReason}` };
    }
    if (!lspActiveRef.current) {
      return { ok: false, error: "Analysis is not running; no rename was requested." };
    }
    const sourcePath = tab.path;
    const sourceRevision = tab.revision;
    const session = lspViewRef.current.session;
    const contextId = contextIdRef.current;
    const workspaceId = workspaceIdRef.current;
    const sequence = ++renameSeq.current;
    // Analysis must see the buffer the preview is based on; a pending debounce would let
    // csharp-ls compute the rename from older text.
    const synchronized = await flushPendingChanges();
    if (sequence !== renameSeq.current) {
      return { ok: false, error: "Rename cancelled; no preview was opened." };
    }
    if (!synchronized) {
      return {
        ok: false,
        error: "Could not send the current buffer to analysis; no rename was requested.",
      };
    }
    const live = lspViewRef.current;
    const current = tabsRef.current.find((item) => item.path === sourcePath);
    if (
      !current ||
      current.revision !== sourceRevision ||
      current.conflict ||
      live.session !== session ||
      activePathRef.current !== sourcePath
    ) {
      const error =
        "Rename discarded: the document or analysis session changed before the request.";
      setMessage({ tone: "error", text: error });
      return { ok: false, error };
    }
    let response: RenameResponse;
    try {
      response = await call<RenameResponse>("lsp.rename", {
        path: sourcePath,
        line: position.line,
        character: position.character,
        newName,
        revision: sourceRevision,
      });
    } catch (error) {
      if (sequence !== renameSeq.current) {
        return { ok: false, error: "Rename cancelled; no preview was opened." };
      }
      return { ok: false, error: `Rename unavailable: ${messageOf(error)}` };
    }
    if (sequence !== renameSeq.current) {
      return { ok: false, error: "Rename cancelled; no preview was opened." };
    }
    const currentTab = tabsRef.current.find((item) => item.path === sourcePath);
    const liveAfter = lspViewRef.current;
    if (
      !currentTab ||
      currentTab.revision !== sourceRevision ||
      currentTab.conflict ||
      liveAfter.session !== session ||
      contextIdRef.current !== contextId ||
      workspaceIdRef.current !== workspaceId
    ) {
      const error =
        "Rename response discarded: the document, workspace or analysis session changed while csharp-ls was answering; nothing was applied.";
      setMessage({ tone: "error", text: error });
      return { ok: false, error };
    }
    if (response.state !== "ok" || response.documentCount === 0) {
      return {
        ok: false,
        error: `Rename ${response.state}: ${response.message ?? "csharp-ls returned no edits."}`,
      };
    }
    const tabRevisions: Record<string, number> = {};
    for (const openTab of tabsRef.current) tabRevisions[openTab.path] = openTab.revision;
    editPreviewApplySeq.current += 1;
    setEditPreview({
      kind: "rename",
      response,
      sourcePath,
      sourceRevision,
      session,
      contextId,
      workspaceId,
      tabRevisions,
      stale: null,
      applying: false,
    });
    setMessage({
      tone: "info",
      text: `Rename preview: ${countLabel(response.documentCount, "file")} · ${countLabel(
        response.editCount,
        "edit",
      )}; nothing has been applied.`,
    });
    return { ok: true };
  }

  function cancelEditPreview() {
    // Cancel is disabled while Apply is revalidating; every dismissal path (button, popup Esc, editor Esc/click)
    // must obey the same guard so a cancel cannot race the post-await application.
    if (editPreview?.applying) return;
    const preview = editPreview;
    editPreviewApplySeq.current += 1;
    setEditPreview(null);
    setMessage({
      tone: "info",
      text: `${preview ? previewKindLabel(preview) : "Preview"} cancelled; no buffer or file changed.`,
    });
  }

  // D-12: an open rename preview or code-action list must not trap the editor. A click on the editor text, or Esc
  // while the editor really has focus, cancels it with the same "nothing applied" path as its own Cancel/Esc.
  // Clicks inside the popup and an Apply already revalidating are left alone.
  useEffect(() => {
    if (!editPreview && !actionList) return;
    const close = () => {
      if (editPreview) cancelEditPreview();
      else cancelActionList();
    };
    const inEditorText = (target: EventTarget | null) =>
      target instanceof Element &&
      target.closest(".cm-editor") !== null &&
      target.closest('[data-testid="editor-shell"]') !== null;
    const onPointerDown = (event: PointerEvent) => {
      if (inEditorText(event.target)) close();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && inEditorText(document.activeElement)) close();
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKeyDown, true);
    };
  }, [editPreview, actionList]);

  function revalidateEditPreview(preview: EditPreview): string | null {
    const live = lspViewRef.current;
    const what = preview.kind === "rename" ? "rename" : "code action";
    if (!["loading", "ready", "partial", "awaiting_diagnostics"].includes(live.state)) {
      return "Analysis is not running; the preview cannot be applied.";
    }
    if (live.session !== preview.session) {
      return `The analysis session changed after the preview; apply nothing and request the ${what} again.`;
    }
    if (
      contextIdRef.current !== preview.contextId ||
      workspaceIdRef.current !== preview.workspaceId
    ) {
      return `The workspace changed after the preview; apply nothing and request the ${what} again.`;
    }
    if (activePathRef.current !== preview.sourcePath) {
      return `Another file is active; select ${preview.sourcePath} and request the ${what} again.`;
    }
    const tabsNow = tabsRef.current;
    for (const document of previewDocuments(preview)) {
      const captured = preview.tabRevisions[document.path];
      const tab = tabsNow.find((item) => item.path === document.path);
      if (captured === undefined) {
        if (tab) {
          return `${document.path} was opened after the preview; apply nothing and request the ${what} again.`;
        }
        continue;
      }
      if (!tab) {
        return `${document.path} was closed after the preview; apply nothing and request the ${what} again.`;
      }
      if (tab.revision !== captured) {
        return `${document.path} changed after the preview; nothing was applied.`;
      }
      if (tab.content !== document.baseContent) {
        return `${document.path} no longer matches the text csharp-ls previewed; apply nothing and request the ${what} again.`;
      }
      if (tab.conflict) {
        return `${document.path} has a pending disk conflict; nothing was applied.`;
      }
    }
    const source = tabsNow.find((item) => item.path === preview.sourcePath);
    if (!source || source.revision !== preview.sourceRevision || source.conflict) {
      return "The source buffer changed after the preview; nothing was applied.";
    }
    return null;
  }

  async function applyEditPreview() {
    const preview = editPreview;
    if (!preview || preview.applying) return;
    const id = previewId(preview);
    const application = ++editPreviewApplySeq.current;
    const label = preview.kind === "rename" ? "Rename" : "Code action";
    const fail = (reason: string) => {
      setEditPreview((current) =>
        current && previewId(current) === id
          ? { ...current, applying: false, stale: reason }
          : current,
      );
      setMessage({ tone: "error", text: `${label} not applied: ${reason}` });
    };
    const stillOwnsPreview = () => {
      const current = editPreviewRef.current;
      return (
        application === editPreviewApplySeq.current &&
        current !== null &&
        previewId(current) === id &&
        current.applying
      );
    };
    const reason = revalidateEditPreview(preview);
    if (reason) {
      fail(reason);
      return;
    }
    setEditPreview((current) =>
      current && previewId(current) === id
        ? { ...current, applying: true, stale: null }
        : current,
    );

    // Re-read every previewed file. Open buffers must still have their captured revision
    // and their last known disk hash; unopened files must still match the preview base.
    const documents = previewDocuments(preview);
    const problems: string[] = [];
    for (const document of documents) {
      const tab = tabsRef.current.find((item) => item.path === document.path);
      let file: { hash: string };
      try {
        file = await call<{ hash: string }>("file.read", { path: document.path });
      } catch (error) {
        problems.push(`${document.path} could not be re-read (${messageOf(error)})`);
        continue;
      }
      const expected = tab ? tab.hash : document.baseHash;
      if (file.hash !== expected) {
        problems.push(`${document.path} changed on disk after the preview`);
      }
    }
    // A late read from an old application must not mutate a newer preview or status, whether it
    // succeeded or failed. Only the owner of the current preview may report a read problem.
    if (!stillOwnsPreview()) return;
    if (problems.length > 0) {
      fail(`${problems.join("; ")}. No buffer or file was changed.`);
      return;
    }
    const afterReads = revalidateEditPreview(preview);
    if (afterReads) {
      fail(afterReads);
      return;
    }

    const byPath = new Map(documents.map((document) => [document.path, document]));
    const next = tabsRef.current.map((tab) => {
      const document = byPath.get(tab.path);
      return document
        ? { ...tab, content: document.newContent, revision: tab.revision + 1, conflict: null }
        : tab;
    });
    for (const document of documents) {
      if (next.some((tab) => tab.path === document.path)) continue;
      next.push({
        path: document.path,
        content: document.newContent,
        hash: document.baseHash,
        revision: 1,
        savedContent: null,
        conflict: null,
      });
    }
    setTabs(next);
    for (const document of documents) {
      if (document.opened) queueDidChange(document.path, document.newContent);
    }
    setEditPreview(null);
    setActionList(null);
    setMessage({
      tone: "info",
      text: `${label} applied to ${countLabel(
        documents.length,
        "file",
      )}; ${countLabel(
        documents.length,
        "buffer",
      )} ${documents.length === 1 ? "is" : "are"} modified and unsaved. Nothing was written to disk; Save remains explicit and diagnostics are not marked fixed.`,
    });
  }

  function reloadFromDisk() {
    const tab = active;
    if (!tab?.conflict) return;
    const conflict = tab.conflict;
    const hadEdits = tab.content !== tab.savedContent;
    const revision = tab.revision + 1;
    setTabs((current) =>
      current.map((item) =>
        item.path === tab.path
          ? {
              ...item,
              content: conflict.content,
              hash: conflict.hash,
              revision,
              savedContent: conflict.content,
              conflict: null,
            }
          : item,
      ),
    );
    queueDidChange(tab.path, conflict.content);
    setMessage({
      tone: "info",
      text: hadEdits
        ? `Reloaded ${tab.path} from disk. Your unsaved edits were discarded.`
        : `Reloaded ${tab.path} from disk; the buffer now matches disk.`,
    });
  }

  async function overwriteDisk() {
    const tab = active;
    if (!tab?.conflict) return;
    const conflict = tab.conflict;
    const content = tab.content;
    writing.current.add(tab.path);
    try {
      const result = await call<{ path: string; hash: string }>("file.write", {
        path: tab.path,
        content,
        expected_hash: conflict.hash,
      });
      setTabs((current) =>
        current.map((item) =>
          item.path === tab.path
            ? { ...item, hash: result.hash, savedContent: content, conflict: null }
            : item,
        ),
      );
      setSavedAt((current) => ({ ...current, [tab.path]: Date.now() }));
      setMessage({ tone: "info", text: `Overwrote ${tab.path} with the buffer.` });
    } catch (error) {
      if (error instanceof HostCallError && error.code === "conflict") {
        applyConflict(tab.path, error);
      } else {
        setMessage({ tone: "error", text: `Overwrite failed; buffer kept. ${messageOf(error)}` });
      }
    } finally {
      writing.current.delete(tab.path);
      bumpDiskEpoch(tab.path);
    }
  }

  function closeTab(path: string) {
    const remaining = tabs.filter((tab) => tab.path !== path);
    setTabs(remaining);
    if (activePath === path) {
      setActivePath(remaining.at(-1)?.path ?? null);
    }
  }

  function requestCloseTab(path: string) {
    const tab = tabs.find((item) => item.path === path);
    if (!tab) return;
    if (isDirty(tab)) {
      setConfirmError(null);
      setConfirm({ kind: "tab", path });
      return;
    }
    closeTab(path);
  }

  function switchTab(delta: number) {
    if (tabs.length < 2) return;
    const index = tabs.findIndex((tab) => tab.path === activePath);
    if (index < 0) return;
    const next = tabs[(index + delta + tabs.length) % tabs.length];
    setActivePath(next.path);
    void checkDisk(next.path);
  }

  async function closeWorkspace() {
    try {
      const closed = await call<Snapshot>("workspace.close");
      onClosed(closed);
    } catch (error) {
      setMessage({
        tone: "error",
        text: `Could not close the workspace: ${messageOf(error)}`,
      });
    }
  }

  async function redetectSdks() {
    try {
      onSnapshot(await call<Snapshot>("sdk.redetect"));
      setMessage({
        tone: "info",
        text: "SDK inventory refreshed. No project file was read or written by the detection.",
      });
    } catch (error) {
      setMessage({ tone: "error", text: `SDK detection failed: ${messageOf(error)}` });
    }
  }

  function openRunConfigDialog(id?: string) {
    setRunConfigInitialId(id ?? activeRunConfig?.id ?? DEFAULT_RUN_CONFIG_ID);
    setRunConfigOpen(true);
  }

  function setStartupProject(path: string) {
    if (!path) return;
    setStartupPath(path);
    void persistStartupProject(path).catch((error) =>
      setMessage({
        tone: "error",
        text: `The startup project could not be saved: ${messageOf(error)}`,
      }),
    );
    if (!activeNamedConfig) return;
    const configs = (runConfigStore?.configs ?? []).map((config) =>
      config.id === activeNamedConfig.id ? { ...config, project: path } : config,
    );
    void commitRunConfigs(configs, activeNamedConfig.id).catch((error) =>
      setMessage({
        tone: "error",
        text: `The run configuration could not be updated: ${messageOf(error)}`,
      }),
    );
  }

  function invalidRunConfigReason(config: RunConfig | null = activeRunConfig) {
    const issues = config
      ? (runConfigStore?.issues ?? []).filter((issue) => issue.configId === config.id)
      : [];
    if (!config || issues.length === 0) return null;
    return `The active run configuration “${config.name}” cannot run: ${issues
      .map((issue) => issue.message)
      .join(" ")} Edit the configuration before running.`;
  }

  function openConfigOnFieldError(error: unknown) {
    if (!(error instanceof HostCallError)) return;
    const field = (error.details as { field?: string } | undefined)?.field;
    if (["args", "workingDirectory", "env", "envFiles"].includes(field ?? "")) {
      openRunConfigDialog(activeRunConfig?.id);
    }
  }

  function openGlobalJson() {
    void openUserFile(projectSdk.globalJsonPath ?? "global.json");
  }

  // One grant path for every "Trust this project" control: the decision is remembered, so the
  // same workspace never asks again. The status popover keeps its own checkbox workflow.
  async function grantTrust() {
    setTrustError(null);
    try {
      onSnapshot(await call<Snapshot>("trust.grant", { remember: true }));
      setMessage(null);
    } catch (error) {
      setTrustError(messageOf(error));
    }
  }

  const {
    buildDecision, setBuildDecision, runDecision, setRunDecision,
    startBuild, requestBuild, saveAllAndBuild, buildSavedContent, cancelBuild,
    requestBuildSolution,
    startRun, requestRun, saveAllAndRebuild, runSavedBuild, cancelRun,
    prepareAndDebug, requestDebugStart, stopDebug,
  } = useExecutionFlow({
    snapshot, evaluation, build, buildRunning, runRunning, dirtyCount, consoleCandidates, startupProject,
    activeRunConfig, activeConfigEvaluated, buildDisabledReason, debugDisabledReason, runAvailabilityReason,
    onSnapshot, setMessage, setBottomTab, saveAll, invalidRunConfigReason, openRunConfigDialog, openConfigOnFieldError,
  });

  const workspaceDir = dirOf(workspace.path);
  const absolutePath = (rel: string) => (rel ? `${workspaceDir}/${rel}` : workspaceDir);

  function locateOpenedFile() {
    const owner = showsEvaluatedTree
      ? evaluatedProjects.find((item) => item.sources.includes(activePath ?? ""))
      : projects.find((item) => item.files.includes(activePath ?? ""));
    if (owner) {
      setExpanded((current) => {
        const next = new Set(current).add(ROOT_KEY).add(owner.path);
        const folder = "folder" in owner ? owner.folder : null;
        if (folder) next.add(folderKey(folder));
        // Open every source folder between the project and the opened file.
        for (let dir = dirOf(activePath ?? ""); dir && dir !== dirOf(owner.path); dir = dirOf(dir)) next.add(dirKey(dir));
        return next;
      });
    }
    setSelectedProjectPath(owner?.path ?? null);
  }

  function openTreeMenu(position: MenuPosition, target: TreeTarget, row: HTMLElement) {
    treeMenuReturnFocus.current = row;
    setTreeMenu({ x: position.x, y: position.y, target });
  }

  function closeTreeMenu() {
    const row = treeMenuReturnFocus.current;
    treeMenuReturnFocus.current = null;
    setTreeMenu(null);
    if (!row) return;
    // An action that already owns the focus (the editor, a dialog field, a panel) keeps it: the row
    // only takes the focus back while the menu itself still holds it — Esc, a click outside, or an
    // action that focuses nothing. A dialog opened by the action focuses its field on commit, after
    // this call, and wins on its own.
    const active = document.activeElement;
    const menuHoldsFocus =
      !(active instanceof HTMLElement) ||
      active === document.body ||
      active.closest(".editor-context-menu") !== null;
    if (menuHoldsFocus && row.isConnected) row.focus();
  }

  const reloadReason =
    evaluationAvailable && !evaluationBusy
      ? null
      : evaluationBusy
        ? "An evaluation is already running."
        : trusted
          ? "Evaluation needs a resolved project SDK."
          : "Trust the workspace and resolve the project SDK before evaluating.";

  // Build/Run/Debug on a tree project use that project's own gate, never the startup project's.
  function projectGates(project: EvaluationProject | undefined) {
    const buildable = Boolean(project && project.state === "ok" && project.targetPath);
    const consoleOk = buildable && project!.kind === "console";
    const context: GateContext = {
      ...gates,
      startupProject: consoleOk ? project! : null,
      activeConfigEvaluated: true,
      activeConfigProjectInvalid: false,
    };
    // An explicit tree build may target a library, so the startup console-candidate rule
    // does not apply; the rest of the build gate still does.
    const buildContext: GateContext = {
      ...context,
      consoleCandidates: buildable ? [project!] : [],
      startupProject: buildable ? project! : null,
    };
    const notConsole = "Only evaluated console projects can be run or debugged.";
    const notBuildable = "Only an evaluated project with a TargetPath can be built; re-evaluate the project model.";
    const gated = (reason: string | null) => (trusted ? (consoleOk ? reason : notConsole) : reason);
    return {
      build: !trusted ? buildGate(context) : buildable ? buildGate(buildContext) : notBuildable,
      run: gated(runGate(context, consoleOk ? project! : null, true)),
      debug: gated(debugGate(context)),
    };
  }

  function projectMenu(target: Extract<TreeTarget, { kind: "project" }>): ContextMenuItem[] {
    const project = evaluatedProjects.find((item) => item.path === target.path);
    const outline = projects.find((item) => item.path === target.path);
    const reasons = projectGates(project);
    const config = project ? (deriveDefaultRunConfig(project, evaluation) as RunConfig) : null;
    return projectMenuItems({
      name: target.name,
      projectFile: baseName(target.path),
      trusted,
      executable: (project?.kind ?? outline?.kind) === "console",
      testProject: (project?.kind ?? outline?.kind) === "test",
      reloadReason,
      buildReason: reasons.build,
      runReason: reasons.run,
      debugReason: reasons.debug,
      unitTestReason: testsDisabledReason,
      shortcuts: { build: shortcutOf("build.start"), run: shortcutOf("run.start"), debug: shortcutOf("debug.start") },
      onTrust: () => void grantTrust(),
      onNuGet: () => setBottomTab("nuget"),
      onReload: () => void runEvaluation(),
      onBuild: () => config && requestBuild(config),
      onRun: () => config && requestRun(config),
      onDebug: () => config && void prepareAndDebug(config),
      onRunUnitTest: () => void startTestRun(target.path, "all", []),
      onEditFile: () => void openUserFile(target.path),
      onFind: () => openFindInFiles(dirOf(target.path)),
      onCopyPath: () => void copyPath(absolutePath(target.path)),
      onReveal: () => void revealInFinder(target.path),
      onProperties: () => {
        setSelectedProjectPath(target.path);
        setProjectDetailsOpen(true);
      },
      onNewFile: () => startCreateFile(target),
    });
  }

  function startCreateFile(target: TreeTarget) {
    if (target.kind !== "project" && target.kind !== "dir") return;
    const directory = target.kind === "project" ? dirOf(target.path) : target.path;
    setNamePrompt({ kind: "create", path: directory, value: "", error: null, busy: false });
  }

  function startRename(target: TreeTarget) {
    if (target.kind === "root" || target.kind === "solution-folder") return;
    setNamePrompt({
      kind: "rename",
      path: target.path,
      value: baseOf(target.path),
      error: null,
      busy: false,
    });
  }

  function requestDelete(target: TreeTarget) {
    if (target.kind === "root" || target.kind === "solution-folder") return;
    setDeletePrompt({
      path: target.path,
      directory: target.kind === "dir",
      error: null,
      busy: false,
    });
  }

  async function submitNamePrompt(event: FormEvent) {
    event.preventDefault();
    const prompt = namePrompt;
    if (!prompt || prompt.busy) return;
    const name = prompt.value.trim();
    const problem = nameProblem(name);
    if (problem) {
      setNamePrompt({ ...prompt, error: problem });
      return;
    }
    const directory = prompt.kind === "create" ? prompt.path : dirOf(prompt.path);
    const to = directory ? `${directory}/${name}` : name;
    if (prompt.kind === "rename" && to === prompt.path) {
      setNamePrompt(null);
      return;
    }
    setNamePrompt({ ...prompt, busy: true, error: null });
    try {
      if (prompt.kind === "create") {
        await call("file.create", { path: to });
        setMessage({ tone: "info", text: `Created ${to}.` });
      } else {
        await call("file.rename", { from: prompt.path, to });
        setMessage({ tone: "info", text: `Renamed ${prompt.path} to ${to}.` });
      }
      setNamePrompt(null);
    } catch (error) {
      setNamePrompt((current) =>
        current ? { ...current, busy: false, error: messageOf(error) } : current,
      );
    }
  }

  async function confirmDelete() {
    const prompt = deletePrompt;
    if (!prompt || prompt.busy) return;
    setDeletePrompt({ ...prompt, busy: true, error: null });
    try {
      await call("file.delete", { path: prompt.path });
      setMessage({ tone: "info", text: `Deleted ${prompt.path}.` });
      setDeletePrompt(null);
    } catch (error) {
      setDeletePrompt((current) =>
        current ? { ...current, busy: false, error: messageOf(error) } : current,
      );
    }
  }

  /** The accessible name says which row the menu belongs to; the visible labels stay put. */
  function treeMenuLabel(target: TreeTarget): string | undefined {
    if (target.kind === "project") return "Project context menu";
    if (target.kind === "file") return "File context menu";
    if (target.kind === "dir") return "Folder context menu";
    if (target.kind === "solution-folder") return "Solution folder context menu";
    return undefined;
  }

  function treeMenuItems(target: TreeTarget): ContextMenuItem[] {
    if (target.kind === "project") return projectMenu(target);
    if (target.kind === "solution-folder") {
      return solutionFolderMenuItems({
        buildReason: buildSolutionDisabledReason,
        restoreReason: restoreDisabledReason,
        reloadReason,
        buildShortcut: shortcutOf("build.solution"),
        onBuild: requestBuildSolution,
        onRestore: () => void startRestore("restore.start"),
        onReload: () => void runEvaluation(),
        onCopyPath: () => void copyPath(absolutePath("")),
        onFind: () => openFindInFiles(""),
        onReveal: () => void revealInFinder(""),
      });
    }
    const path = target.kind === "root" ? "" : target.path;
    if (target.kind === "root") {
      return solutionMenuItems({
        buildReason: buildSolutionDisabledReason,
        restoreReason: restoreDisabledReason,
        reloadReason,
        buildShortcut: shortcutOf("build.solution"),
        onBuild: requestBuildSolution,
        onRestore: () => void startRestore("restore.start"),
        onReload: () => void runEvaluation(),
        onCopyPath: () => void copyPath(absolutePath("")),
        onFind: () => openFindInFiles(""),
        onReveal: () => void revealInFinder(""),
      });
    }
    const editable = target.kind === "file" || target.kind === "dir";
    return fileMenuItems({
      open: target.kind === "file" ? () => void openUserFile(target.path) : undefined,
      onCopyPath: () => void copyPath(absolutePath(path)),
      onFind: () => openFindInFiles(path),
      onReveal: () => void revealInFinder(path),
      newFile: target.kind === "dir" ? () => startCreateFile(target) : undefined,
      rename: editable ? () => startRename(target) : undefined,
      delete: editable ? () => requestDelete(target) : undefined,
    });
  }

  const treeMoreItems: ContextMenuItem[] = [
    { id: "reevaluate", label: "Re-evaluate project model", disabledReason: reloadReason, action: () => void runEvaluation() },
    { id: "details", label: "Project details", disabledReason: selectedProject ? null : "Select a project first.", action: () => setProjectDetailsOpen((current) => !current) },
    { id: "expand-all", label: "Expand All", action: () => setExpanded(expandableKeys(showsEvaluatedTree ? evaluatedProjects : null, projects, true)), separatorBefore: true },
    { id: "collapse-all", label: "Collapse All", action: () => setExpanded(new Set([ROOT_KEY])) },
    { id: "show-all", label: showAllFiles ? "Hide All Files" : "Show All Files", action: () => setShowAllFiles((current) => !current), separatorBefore: true },
  ];

  async function startTestDiscovery(project: string, automaticBoundary: string | null = null) {
    const automatic = automaticBoundary !== null;
    if (!automatic) {
      // A manual discovery belongs to the same boundary; the automatic effect must not repeat it.
      testDiscoveryAutoStarted.current = testDiscoveryBoundary;
      setBottomTab("tests");
    }
    setTestAutoDiscoveryError(null);
    try {
      onSnapshot(
        await call<Snapshot>("tests.discover", {
          project,
          targetFramework: evaluation.context.targetFramework,
        }),
      );
      if (!automatic) setMessage(null);
    } catch (error) {
      if (automaticBoundary !== null) {
        setTestAutoDiscoveryError({ boundary: automaticBoundary, message: messageOf(error) });
        return;
      }
      setMessage({ tone: "error", text: `Test discovery could not start: ${messageOf(error)}` });
      if (error instanceof HostCallError && error.code === "restore_required") {
        setBottomTab("tests");
      }
    }
  }

  async function cancelTestDiscovery() {
    try {
      onSnapshot(await call<Snapshot>("tests.discover_cancel"));
    } catch (error) {
      setMessage({ tone: "error", text: `Discovery cancellation failed: ${messageOf(error)}` });
    }
  }

  async function startTestRun(
    project: string,
    selection: "all" | "selected",
    testIds: string[],
  ) {
    setBottomTab("tests");
    try {
      onSnapshot(
        await call<Snapshot>("tests.run", {
          project,
          targetFramework: evaluation.context.targetFramework,
          selection,
          testIds,
        }),
      );
      setMessage(null);
    } catch (error) {
      setMessage({ tone: "error", text: `Test run could not start: ${messageOf(error)}` });
      try {
        onSnapshot(await call<Snapshot>("workspace.state"));
      } catch {
        // The message above already explains the failure.
      }
    }
  }

  async function cancelTestRun() {
    try {
      onSnapshot(await call<Snapshot>("tests.run_cancel"));
    } catch (error) {
      setMessage({ tone: "error", text: `Test run cancellation failed: ${messageOf(error)}` });
    }
  }

  function toggleTestSelected(id: string) {
    setTestSelected((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function toggleBreakpoint(line: number) {
    if (!active) {
      setMessage({ tone: "error", text: "Open a file before setting a breakpoint." });
      return;
    }
    if (!active.path.endsWith(".cs")) {
      setMessage({ tone: "error", text: "Breakpoints are supported for C# sources only." });
      return;
    }
    const next = activeBreakpoints.includes(line)
      ? activeBreakpoints.filter((value) => value !== line)
      : [...activeBreakpoints, line];
    try {
      onSnapshot(
        await call<Snapshot>("debug.setBreakpoints", {
          path: active.path,
          lines: next.sort((left, right) => left - right),
        }),
      );
    } catch (error) {
      setMessage({ tone: "error", text: `Breakpoint could not be set: ${messageOf(error)}` });
    }
  }

  async function restartDebug() {
    await stopDebug();
    await requestDebugStart();
  }

  async function runToCursor() {
    if (!active) return;
    try {
      onSnapshot(
        await call<Snapshot>("debug.runToCursor", {
          path: active.path,
          line: cursorRef.current.line + 1,
        }),
      );
    } catch (error) {
      setMessage({ tone: "error", text: `Run to Cursor failed: ${messageOf(error)}` });
    }
  }

  async function debugAction(
    method: "debug.continue" | "debug.pause" | "debug.stepOver" | "debug.stepInto" | "debug.stepOut",
  ) {
    try {
      onSnapshot(await call<Snapshot>(method));
    } catch (error) {
      setMessage({ tone: "error", text: `Debug action failed: ${messageOf(error)}` });
    }
  }

  function hideActiveToolWindow() {
    const other: ToolWindowSlot = activeSlot === "left" ? "bottom" : "left";
    const slot: ToolWindowSlot | null = layout[activeSlot].open
      ? activeSlot
      : layout[other].open
        ? other
        : null;
    if (!slot) return;
    setLayout((current) => {
      const state = current[slot];
      const next = { ...state, open: false };
      return slot === "left" ? { ...current, left: next } : { ...current, bottom: next };
    });
    document.querySelector<HTMLElement>('[data-testid="editor"]')?.focus();
  }

  function stopExecution() {
    if (debugActive) {
      void stopDebug();
    } else if (runRunning || runBuilding) {
      void cancelRun();
    }
  }

  function openSearch(tab: SearchTab | "last") {
    setKeymapOpen(false);
    setSearchTab(tab);
    // FindAction reads initialTab only on mount; a new key applies the requested tab even
    // when the palette is already open on another tab.
    setSearchSeq((current) => current + 1);
    setFindActionOpen(true);
  }

  function openFindInFiles(scope: string) {
    setKeymapOpen(false);
    setFindActionOpen(false);
    setFindInFiles((current) => ({ scope, nonce: (current?.nonce ?? 0) + 1 }));
  }

  function openFindAction() {
    openSearch("actions");
  }

  function closeSearch() {
    setFindActionOpen(false);
    focusEditor();
  }

  function focusEditor() {
    const editor = document.querySelector<HTMLElement>('[data-testid="editor"]');
    (editor?.querySelector<HTMLElement>(".cm-content") ?? editor)?.focus();
  }

  // Opening another file remounts the editor; focusing before that lands on the old one or nothing.
  // WebKit commits that remount later than a timer, so retry per frame. Focus on the document or on the
  // control that started the navigation (a panel row) is ours to move. A modal dialog, the terminal or a
  // text field keeps the focus it has. The editor is not a stop: the first try may still land on the old one.
  function focusEditorWhenMounted(frames: number) {
    const active = document.activeElement;
    const keeps =
      active !== null &&
      active !== document.body &&
      (active.closest('[role="dialog"][aria-modal="true"], [data-testid="terminal-panel"]') !== null ||
        active.matches("input, textarea, select"));
    if (keeps) return;
    focusEditor();
    if (frames > 0) window.requestAnimationFrame(() => focusEditorWhenMounted(frames - 1));
  }

  function focusEditorAfterOpen(open: () => Promise<unknown>) {
    void open().finally(() => focusEditorWhenMounted(15));
  }

  function openSearchFile(path: string) {
    setFindActionOpen(false);
    focusEditorAfterOpen(() => openUserFile(path));
  }

  function openSearchLocation(path: string, line: number, character: number) {
    setFindActionOpen(false);
    focusEditorAfterOpen(() => navigateTo(path, { line, character }));
  }

  function openKeymap() {
    setFindActionOpen(false);
    setKeymapOpen(true);
  }

  function openSettings() {
    if (settingsOpen) return;
    setFindActionOpen(false);
    setKeymapOpen(false);
    settingsBaseline.current = settings;
    setSettingsOpen(true);
  }

  /** Draft changes apply live; Cancel restores the snapshot taken when Settings opened. */
  function updateSettings(next: AppSettings) {
    setSettings(next);
  }

  function cancelSettings() {
    if (settingsBaseline.current) setSettings(settingsBaseline.current);
    settingsBaseline.current = null;
    setSettingsOpen(false);
  }

  async function saveSettings(): Promise<string | null> {
    try {
      const result = await call<{ settings: unknown }>("settings.write", {
        settings,
      });
      setSettings(normalizeSettings(result.settings));
      settingsBaseline.current = null;
      setSettingsOpen(false);
      return null;
    } catch (error) {
      return `Settings were not saved: ${messageOf(error)}`;
    }
  }

  async function saveKeymap(keymap: Keymap): Promise<string | null> {
    const next = { ...settings, keymap };
    try {
      const result = await call<{ settings: unknown }>("settings.write", {
        settings: next,
      });
      setSettings(normalizeSettings(result.settings));
      return null;
    } catch (error) {
      return `Settings were not saved: ${messageOf(error)}`;
    }
  }

  function define(action: Omit<Action, "editor"> & { editor?: boolean }): Action {
    return { editor: true, ...action };
  }

  const actions: Action[] = [
    define({
      id: "view.findAction",
      title: "Find Action",
      group: "General",
      execute: openFindAction,
    }),
    define({
      id: "view.keymap",
      title: "Keymap: change shortcuts",
      group: "General",
      execute: openKeymap,
    }),
    define({
      id: "file.save",
      title: "Save",
      group: "File",
      execute: () => void save(),
      disabledReason: active ? null : "Open a file to save.",
    }),
    define({
      id: "editor.closeTab",
      title: "Close Tab",
      group: "Editor",
      execute: () => {
        if (activePath) requestCloseTab(activePath);
      },
      disabledReason: active ? null : "Open a file to close its tab.",
    }),
    define({
      id: "editor.nextTab",
      title: "Next Tab",
      group: "Editor",
      execute: () => switchTab(1),
      disabledReason: tabs.length > 1 ? null : "Open at least two files to switch tabs.",
    }),
    define({
      id: "editor.previousTab",
      title: "Previous Tab",
      group: "Editor",
      execute: () => switchTab(-1),
      disabledReason: tabs.length > 1 ? null : "Open at least two files to switch tabs.",
    }),
    define({
      id: "navigate.back",
      title: "Back",
      group: "Navigation",
      execute: () => void travel(-1),
      disabledReason: navEnabled.back ? null : "No earlier location in history.",
    }),
    define({
      id: "navigate.forward",
      title: "Forward",
      group: "Navigation",
      execute: () => void travel(1),
      disabledReason: navEnabled.forward ? null : "No later location in history.",
    }),
    define({
      id: "toolWindow.hideActive",
      title: "Hide Active Tool Window",
      group: "Tool Windows",
      editor: false,
      execute: hideActiveToolWindow,
      disabledReason:
        layout.left.open || layout.bottom.open ? null : "No tool window is open.",
    }),
    ...toolWindows.map((tool) =>
      define({
        id: `toolWindow.${tool.id}`,
        title: tool.title,
        group: "Tool Windows",
        execute: () => toggleToolWindow(tool.slot, tool.id),
      }),
    ),
    define({
      id: "build.start",
      title: "Build",
      group: "Build",
      execute: () => requestBuild(),
      disabledReason: buildDisabledReason,
    }),
    define({
      id: "build.solution",
      title: "Build Solution",
      group: "Build",
      execute: requestBuildSolution,
      disabledReason: buildSolutionDisabledReason,
    }),
    define({
      id: "run.start",
      title: "Run",
      group: "Run",
      execute: requestRun,
      disabledReason: runDisabledReason,
    }),
    define({
      id: "debug.start",
      title: "Debug",
      group: "Debug",
      execute: () => void prepareAndDebug(),
      disabledReason: debugDisabledReason,
    }),
    define({
      id: "run.stop",
      title: "Stop",
      group: "Run",
      execute: stopExecution,
      disabledReason:
        debugActive || runRunning || runBuilding ? null : "Nothing is running; there is no session to stop.",
    }),
    define({
      id: "debug.restart",
      title: "Restart Debugging",
      group: "Debug",
      execute: () => void restartDebug(),
      disabledReason: debugActive ? null : "No debug session to restart.",
    }),
    define({
      id: "debug.stepOver",
      title: "Step Over",
      group: "Debug",
      execute: () => {
        if (dap.state === "paused") void debugAction("debug.stepOver");
      },
      disabledReason: dap.state === "paused" ? null : "No paused debug session.",
    }),
    define({
      id: "debug.stepInto",
      title: "Step Into",
      group: "Debug",
      execute: () => {
        if (dap.state === "paused") void debugAction("debug.stepInto");
      },
      disabledReason: dap.state === "paused" ? null : "No paused debug session.",
    }),
    define({
      id: "debug.continue",
      title: "Resume",
      group: "Debug",
      execute: () => {
        if (dap.state === "paused") void debugAction("debug.continue");
      },
      disabledReason: dap.state === "paused" ? null : "No paused debug session.",
    }),
    define({
      id: "debug.stepOut",
      title: "Step Out",
      group: "Debug",
      execute: () => {
        if (dap.state === "paused") void debugAction("debug.stepOut");
      },
      disabledReason: dap.state === "paused" ? null : "No paused debug session.",
    }),
    define({
      id: "debug.runToCursor",
      title: "Run to Cursor",
      group: "Debug",
      execute: () => void runToCursor(),
      disabledReason:
        dap.state !== "paused"
          ? "No paused debug session."
          : active?.path.endsWith(".cs")
            ? null
            : "Open a C# file to run to a line.",
    }),
    define({
      id: "debug.pause",
      title: "Pause",
      group: "Debug",
      execute: () => {
        if (dap.state === "running") void debugAction("debug.pause");
      },
      disabledReason: dap.state === "running" ? null : "The debuggee is not running.",
    }),
    define({
      id: "debug.stop",
      title: "Stop Debugging",
      group: "Debug",
      execute: () => void stopDebug(),
      disabledReason: debugActive ? null : "No debug session to stop.",
    }),
    define({
      id: "debug.toggleBreakpoint",
      title: "Toggle Breakpoint",
      group: "Debug",
      execute: () => void toggleBreakpoint(cursorRef.current.line + 1),
      disabledReason: !active
        ? "Open a file to set a breakpoint."
        : !active.path.endsWith(".cs")
          ? "Breakpoints are supported for C# sources only."
          : null,
    }),
    define({
      id: "code.goToDeclaration",
      title: "Go to Declaration",
      group: "Code",
      execute: () => setDefinitionRequest((current) => current + 1),
      disabledReason: goToDefinitionDisabledReason,
    }),
    define({
      id: "code.findUsages",
      title: "Find Usages",
      group: "Code",
      execute: () => void requestFindUsages(),
      disabledReason: findUsagesDisabledReason,
    }),
    define({
      id: "code.contextActions",
      title: "Show Context Actions",
      group: "Code",
      execute: () => void requestCodeActions(),
      disabledReason: codeActionsDisabledReason,
    }),
    define({
      id: "code.rename",
      title: "Rename Symbol",
      group: "Code",
      execute: () => requestRename(),
      disabledReason: renameDisabledReason,
    }),
    define({
      id: "search.everywhere",
      title: "Search Everywhere",
      group: "Search",
      execute: () => openSearch("all"),
    }),
    define({
      id: "search.file",
      title: "Go to File",
      group: "Search",
      execute: () => openSearch("files"),
    }),
    define({
      id: "search.symbol",
      title: "Go to Symbol",
      group: "Search",
      execute: () => openSearch("symbols"),
    }),
    define({
      id: "search.findInFiles",
      title: "Find in Files",
      group: "Search",
      execute: () => openFindInFiles(""),
    }),
    define({
      id: "search.recentFiles",
      title: "Recent Files",
      group: "Search",
      execute: () => {
        if (recentFiles.length === 0) return;
        setFindActionOpen(false);
        setRecentFilesOpen(true);
      },
      disabledReason: recentFiles.length === 0 ? "No recent files yet." : null,
    }),
    define({
      id: "search.text",
      title: "Find in Path",
      group: "Search",
      execute: () => openSearch("text"),
    }),
    define({
      id: "search.line",
      title: "Go to Line",
      group: "Search",
      execute: () => {
        focusEditor();
        setGotoLineRequest((current) => current + 1);
      },
      disabledReason: active ? null : "Open a file to go to a line.",
    }),
    define({
      id: "tests.runAll",
      title: "Run All Tests",
      group: "Tests",
      execute: () => {
        if (testProject) void startTestRun(testProject.path, "all", []);
      },
      disabledReason: testRunBlocked,
    }),
    define({
      id: "tests.runSelected",
      title: "Run Selected Tests",
      group: "Tests",
      execute: () => {
        if (testProject) void startTestRun(testProject.path, "selected", Array.from(testSelected));
      },
      disabledReason:
        testRunBlocked ??
        (testSelected.size === 0
          ? "Select at least one discovered test; an empty selection never runs all tests."
          : null),
    }),
    define({
      id: "tests.debugSelected",
      title: "Debug Selected Tests",
      group: "Tests",
      execute: () =>
        setMessage({ tone: "error", text: testDebugUnavailable }),
      disabledReason: testDebugUnavailable,
    }),
    define({
      id: "tests.rerunFailed",
      title: "Rerun Failed Tests",
      group: "Tests",
      execute: () => {
        if (testProject) {
          void startTestRun(
            testProject.path,
            "selected",
            failedTestIds(snapshot.tests.run, testDiscovery),
          );
        }
      },
      disabledReason: testRerunBlocked,
    }),
    define({
      id: "tests.stop",
      title: "Stop Test Run",
      group: "Tests",
      execute: () => {
        if (testDiscovery.state === "discovering") void cancelTestDiscovery();
        else if (snapshot.tests.run.state === "running") void cancelTestRun();
      },
      disabledReason:
        testDiscovery.state === "discovering" || snapshot.tests.run.state === "running"
          ? null
          : "No test discovery or run is active.",
    }),
    define({
      id: "tests.refresh",
      title: "Refresh Tests",
      group: "Tests",
      execute: () => {
        if (testProject) void startTestDiscovery(testProject.path);
      },
      disabledReason:
        testsDisabledReason ??
        (testProject
          ? null
          : "Select an evaluated test project explicitly; discovery never guesses."),
    }),
    define({
      id: "tests.quickList",
      title: "Show Test Quick List",
      group: "Tests",
      execute: () => setQuickListOpen(true),
    }),
    define({
      id: "git.commit",
      title: "Commit…",
      group: "Git",
      execute: () => openCommitPanel(),
      disabledReason: gitBlockedReason(gitStatus, gitStatusError, trusted),
    }),
    define({
      id: "git.push",
      title: "Push…",
      group: "Git",
      execute: () => openGitIntent("push"),
      disabledReason: gitPushBlockedReason(gitStatus, gitStatusError, trusted),
    }),
    define({
      id: "git.update",
      title: "Update Project…",
      group: "Git",
      execute: () => openGitIntent("update"),
      disabledReason: gitUpdateBlockedReason(gitStatus, gitStatusError, trusted),
    }),
    define({
      id: "git.branchCreate",
      title: "New Branch…",
      group: "Git",
      execute: () => openGitIntent("branchCreate"),
      disabledReason: gitBlockedReason(gitStatus, gitStatusError, trusted),
    }),
    define({
      id: "run.configure",
      title: "Edit Run Configurations…",
      group: "Run",
      execute: () => openRunConfigDialog(),
    }),
    define({
      id: "settings.open",
      title: "Open Settings",
      group: "Settings",
      execute: openSettings,
    }),
    define({
      id: "sdk.redetect",
      title: "Redetect SDKs",
      group: "Build",
      execute: () => void redetectSdks(),
    }),
    define({
      id: "sdk.openGlobalJson",
      title: "Open global.json",
      group: "Build",
      execute: () => {
        if (projectSdk.state === "sdk_missing" || projectSdk.state === "invalid_global_json") {
          openGlobalJson();
        }
      },
      disabledReason:
        projectSdk.state === "sdk_missing" || projectSdk.state === "invalid_global_json"
          ? null
          : "global.json is valid or no project SDK is pinned.",
    }),
    define({
      id: "workspace.close",
      title: "Close Workspace",
      group: "Workspace",
      execute: requestCloseWorkspace,
    }),
  ];
  const resolvedActions = resolveActions(
    actions,
    settings.keymap,
    snapshot.app.platform,
  );
  const actionById = new Map(resolvedActions.map((action) => [action.id, action]));
  const testQuickItems = [
    { id: "tests.runAll", title: "Run all" },
    { id: "tests.runSelected", title: "Run selected" },
    { id: "tests.debugSelected", title: "Debug" },
    { id: "tests.rerunFailed", title: "Rerun failed" },
    { id: "tests.stop", title: "Stop" },
  ].map(({ id, title }) => {
    const action = actionById.get(id);
    return {
      id,
      title,
      shortcut:
        [...(action?.shortcutLabels ?? []), ...(action?.chordLabels ?? [])].join(" ") ||
        "No shortcut",
      disabledReason: action ? (action.disabledReason ?? null) : "Action unavailable.",
      execute: () => action?.execute(),
    };
  });
  const closeContextMenu = useCallback(() => {
    setContextMenu(null);
    document.querySelector<HTMLElement>('[data-testid="editor"]')?.focus();
  }, []);
  const activeGitFile = active
    ? gitStatus?.files.find((entry) => entry.path === active.path) ?? null
    : null;
  const gitContextDisabledReason =
    gitBlockedReason(gitStatus, gitStatusError, trusted) ??
    (activeGitFile ? null : "This file has no uncommitted changes in Git.");
  const shortcutOf = (id: string) => actionById.get(id)?.shortcutTexts[0];
  function contextMenuItems(state: ContextMenuState): ContextMenuItem[] {
    const action = gutterActionAt(gutterActions, state.line + 1);
    const test = action?.kind === "test" ? action : null;
    const position = { line: state.line, character: state.character };
    return [
      {
        id: "context-actions",
        label: "Context Actions",
        shortcut: shortcutOf("code.contextActions"),
        disabledReason: codeActionsDisabledReason,
        action: () => void requestCodeActions(position),
      },
      {
        id: "find-usages",
        label: "Find Usages",
        shortcut: shortcutOf("code.findUsages"),
        disabledReason: findUsagesDisabledReason,
        action: () => void requestFindUsages(position),
      },
      {
        id: "go-to-definition",
        label: "Go To Definition",
        shortcut: shortcutOf("code.goToDeclaration"),
        disabledReason: goToDefinitionDisabledReason,
        action: () => setDefinitionRequest((current) => current + 1),
      },
      {
        id: "rename",
        label: "Rename",
        shortcut: shortcutOf("code.rename"),
        disabledReason: renameDisabledReason,
        action: () => requestRename(position),
      },
      {
        id: "run-test",
        label: "Run Test",
        disabledReason: test
          ? testRunBlocked
          : "Place the cursor on a test method with analysis ready.",
        action: () => {
          if (test) handleGutterAction(test);
        },
        separatorBefore: true,
      },
      {
        id: "debug-test",
        label: "Debug Test",
        disabledReason: test
          ? testDebugUnavailable
          : "Place the cursor on a test method with analysis ready.",
        action: () => {
          if (test) setMessage({ tone: "error", text: testDebugUnavailable });
        },
      },
      {
        id: "git-diff",
        label: "Git: Show Diff",
        disabledReason: gitContextDisabledReason,
        action: () => {
          if (active) void showGitDiff(active.path);
        },
        separatorBefore: true,
      },
      {
        id: "git-commit",
        label: "Git: Commit File…",
        disabledReason: gitContextDisabledReason,
        action: () => openCommitPanel(active?.path ?? null),
      },
      {
        id: "copy-path",
        label: "Copy Path",
        disabledReason: null,
        action: () => {
          if (active) void copyPath(active.path);
        },
        separatorBefore: true,
      },
    ];
  }
  const actionsRef = useRef<ResolvedAction[]>([]);
  actionsRef.current = resolvedActions;
  const lastShift = useRef<number | null>(null);
  const shiftTap = useRef<"armed" | "invalid" | null>(null);
  const chordPending = useRef<number | null>(null);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (document.querySelector('[role="dialog"][aria-modal="true"]')) {
        shiftTap.current = null;
        return;
      }
      const target = event.target instanceof HTMLElement ? event.target : null;
      const inEditor = Boolean(target?.closest(".cm-editor"));
      const textField =
        Boolean(target?.closest("input, textarea, select")) ||
        Boolean(target?.isContentEditable && !inEditor);
      // A focused tree row owns Shift+F10 and the menu key: they open its context menu, exactly like
      // a right-click, and never the keymap action behind them (IntelliJ maps Shift+F10 to Run All
      // Tests, Windows Rider to Run). Returning without preventDefault lets the row's handler run.
      if (
        target?.closest(".tree-row") &&
        (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10"))
      ) {
        // This Shift+key is not the first tap of a double-Shift, so it must not arm one.
        shiftTap.current = null;
        lastShift.current = null;
        return;
      }
      if (event.key === "Shift") {
        if (textField) {
          lastShift.current = null;
          shiftTap.current = null;
          return;
        }
        if (!event.repeat) shiftTap.current = "armed";
        return;
      }
      // Another key during the beat makes it impure and forgets the previous completed tap.
      if (shiftTap.current !== null) shiftTap.current = "invalid";
      else lastShift.current = null;
      if (event.defaultPrevented) return;
      if (textField) return;
      const profile = settings.keymap.profile;
      const pending = chordPending.current;
      chordPending.current = null;
      if (pending !== null && performance.now() - pending <= 1500) {
        const target = chordTarget(profile, event.key);
        if (target && !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey) {
          event.preventDefault();
          event.stopPropagation();
          actionsRef.current.find((candidate) => candidate.id === target)?.execute();
          return;
        }
      }
      const prefix = chordPrefixFor(profile);
      if (prefix && matches(event, prefix, snapshot.app.platform)) {
        event.preventDefault();
        event.stopPropagation();
        chordPending.current = performance.now();
        return;
      }
      const action = firstActionFor(actionsRef.current, event, snapshot.app.platform, {
        profile: settings.keymap.profile,
        debugPaused: dap.state === "paused",
      });
      if (!action) return;
      if (inEditor && !action.editor) return;
      // Capture, before CodeMirror: the app's shortcuts and chords outrank its keymap.
      event.preventDefault();
      event.stopPropagation();
      // A disabled action still reports its own reason when reached by a shortcut; the
      // palette and menus keep it disabled.
      action.execute();
    };
    const onKeyUp = (event: KeyboardEvent) => {
      if (event.key !== "Shift") return;
      const target = event.target instanceof HTMLElement ? event.target : null;
      const textField =
        Boolean(target?.closest("input, textarea, select")) ||
        Boolean(target?.isContentEditable && !target.closest(".cm-editor"));
      const clean = shiftTap.current === "armed" && !textField;
      shiftTap.current = null;
      if (!clean) {
        lastShift.current = null;
        return;
      }
      const now = performance.now();
      if (lastShift.current !== null && now - lastShift.current <= 400) {
        lastShift.current = null;
        event.preventDefault();
        openSearch("all");
        return;
      }
      lastShift.current = now;
    };
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
    };
  }, [snapshot.app.platform, settings.keymap.profile, dap.state]);

  function requestCloseWorkspace() {
    if (dirtyCount > 0) {
      setConfirmError(null);
      setConfirm({ kind: "workspace" });
      return;
    }
    void closeWorkspace();
  }

  async function destroyWindow() {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().destroy();
    } catch (error) {
      setMessage({
        tone: "error",
        text: `Could not close the window: ${messageOf(error)}`,
      });
    }
  }

  async function quitApp() {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("quit_app");
    } catch (error) {
      setMessage({
        tone: "error",
        text: `Could not quit OpenRidr: ${messageOf(error)}`,
      });
    }
  }

  async function confirmSave() {
    if (!confirm) return;
    setConfirmError(null);
    const pending = confirm;
    if (pending.kind === "tab") {
      const tab = tabs.find((item) => item.path === pending.path);
      if (!tab) {
        setConfirm(null);
        return;
      }
      const result = await saveTab(tab);
      if (!result.ok) {
        setConfirmError(result.error ?? `Could not save ${tab.path}.`);
        return;
      }
      setConfirm(null);
      closeTab(pending.path);
      return;
    }
    const failures = await saveAll();
    if (failures.length > 0) {
      setConfirmError(failures.join(" "));
      return;
    }
    setConfirm(null);
    if (pending.kind === "workspace") {
      void closeWorkspace();
    } else if (pending.kind === "window") {
      void destroyWindow();
    } else {
      void quitApp();
    }
  }

  function keepEditing() {
    const quitting = confirm?.kind === "quit";
    setConfirm(null);
    setConfirmError(null);
    if (quitting) focusEditor();
  }

  function discardConfirmed() {
    if (!confirm) return;
    const pending = confirm;
    setConfirm(null);
    setConfirmError(null);
    if (pending.kind === "tab") {
      closeTab(pending.path);
      setMessage({ tone: "info", text: `Closed ${pending.path}; unsaved edits were discarded.` });
    } else if (pending.kind === "workspace") {
      void closeWorkspace();
    } else if (pending.kind === "window") {
      void destroyWindow();
    } else {
      void quitApp();
    }
  }

  function renderToolsContent() {
    return (
      <div className="workspace-tools-body">
      <section className="restore-panel" aria-label="Restore" data-testid="restore-panel">
        <div className="restore-head">
          <strong>Restore</strong>
          <span
            className={`tag ${restore.state === "failed" ? "error" : ""}`}
            data-testid="restore-state"
            role="status"
          >
            {restoreStateLabel(restore)}
          </span>
          <span className="muted" data-testid="restore-scope">
            Scope: {workspace.kind === "solution" ? "Solution" : "Project"} ·{" "}
            {restore.revision > 0 ? baseName(restore.target) : workspace.name}
          </span>
          <span className="push" />
          {!restoring && (
            <button
              data-testid="restore-action"
              disabled={!restoreAvailable}
              onClick={() => void startRestore("restore.start")}
              title={
                restoreDisabledReason ??
                "Runs dotnet restore for this workspace with the project SDK. A Trusted workspace restores by itself; this runs it again."
              }
            >
              {restore.state === "ok" ? "Restore again" : "Restore"}
            </button>
          )}
          {restoring && (
            <button
              data-testid="restore-cancel"
              onClick={() => void cancelRestore()}
              title="Requests termination of the restore process group and reports what was observed."
            >
              Cancel restore
            </button>
          )}
          {(restore.state === "failed" || restore.state === "cancelled") && (
            <button
              data-testid="restore-retry"
              disabled={!restoreAvailable}
              onClick={() => void startRestore("restore.retry")}
              title={
                restoreDisabledReason ??
                "Runs the explicit restore again; nothing is fixed or restored silently."
              }
            >
              Retry restore
            </button>
          )}
          {restore.state === "ok" && (
            <button
              data-testid="restore-reload"
              disabled={!restoreAvailable || evaluationBusy}
              onClick={() => void runEvaluation()}
              title="Calls evaluation.run with the current configuration and TFM; the old TargetPath is not current until then."
            >
              Reload evaluated model
            </button>
          )}
        </div>
        {!restoreAvailable && !restoring && (
          <p className="muted" data-testid="restore-disabled-reason">
            {restoreDisabledReason}
          </p>
        )}
        {restore.revision > 0 && (
          <>
            <dl className="restore-meta">
              <dt>Target</dt>
              <dd>
                <code>{restore.target}</code>
              </dd>
              <dt>Command</dt>
              <dd>
                <code data-testid="restore-command">{restore.command.join(" ")}</code>
              </dd>
              <dt>Working directory</dt>
              <dd>
                <code>{restore.cwd}</code>
              </dd>
              <dt>Project SDK</dt>
              <dd>
                {restore.sdkVersion ?? "unknown"} · <code>{restore.sdkRoot ?? "unknown"}</code>
              </dd>
            </dl>
            {restore.sealed && (
              <p className="warning" data-testid="restore-sealed">
                Previous restore, not current: {restore.staleReason}
              </p>
            )}
            {restore.termination && (
              <p className="muted" data-testid="restore-termination">
                Termination requested: {restore.termination.requested}. Observed:{" "}
                {restore.termination.observed}
                {restore.termination.killedAfterGrace ? " after the grace period" : ""}.
              </p>
            )}
            {restore.awaitingDecision && (
              <p className="warning" data-testid="restore-awaiting">
                Still running; the host has not confirmed termination. Decide in the process
                panel.
              </p>
            )}
            {restoreIncompleteAfterRun && (
              <div className="notice warning" data-testid="restore-incomplete">
                <strong>Dependencies incomplete.</strong> Editing stays available; analysis and
                later build/run/test steps need these projects restored.
                <ul>
                  {restore.projects
                    .filter((project) => project.state !== "restored")
                    .map((project) => (
                      <li key={project.path} data-testid={`restore-incomplete-${project.name}`}>
                        {project.name} —{" "}
                        {project.state === "unknown" ? "restore unknown" : "not restored"}:{" "}
                        {project.reason ?? "obj/project.assets.json is missing or stale"}
                      </li>
                    ))}
                </ul>
              </div>
            )}
            {restore.projects.length > 0 && !restoreIncompleteAfterRun && (
              <p className="muted" data-testid="restore-projects">
                {restore.projects.length} project
                {restore.projects.length === 1 ? "" : "s"} report restored assets.
              </p>
            )}
            <details className="eval-log" open={restore.state === "running" || restore.state === "failed"}>
              <summary>
                Restore log · stdout/stderr{restore.logTruncated ? " · truncated" : ""}
              </summary>
              <pre className="eval-pre" data-testid="restore-log">
                {restore.log || "No output yet."}
              </pre>
            </details>
          </>
        )}
      </section>

      {restoreIncomplete && (
        <div className="notice warning" data-testid="eval-restore-banner" role="status">
          <strong>
            {restore.revision > 0
              ? "Restore incomplete — package dependencies and analysis are incomplete."
              : "Restore not requested — package dependencies and analysis are incomplete."}
          </strong>{" "}
          {restore.revision > 0
            ? "Fix the cause and retry the explicit restore; editing stays available."
            : "Opening or evaluating never restores. Use Restore to run it explicitly, then reload the evaluated model."}
        </div>
      )}

      <DebugAdapterPanel snapshot={snapshot} onChange={onSnapshot} compact />
      <ToolInstaller snapshot={snapshot} onChange={onSnapshot} />
      <LspPanel snapshot={snapshot} onChange={onSnapshot} />

      {showsProcesses && <ProcessPanel snapshot={snapshot} onChange={onSnapshot} />}
      </div>
    );
  }

  function renderBottomContent() {
    switch (bottomTab) {
      case "run":
        return (
          <RunPanel
            run={run}
            onCancelRun={() => void cancelRun()}
            onOpenFrame={(path, line) => void navigateTo(path, { line: line - 1, character: 0 })}
          />
        );
      case "build":
        return (
          <BuildPanel
            build={build}
            run={run}
            platform={snapshot.app.platform}
            shortcuts={{ run: shortcutOf("run.start"), build: shortcutOf("build.start"), solution: shortcutOf("build.solution") }}
            dirtyCount={dirtyCount}
            runDisabledReason={runDisabledReason}
            onRun={requestRun}
            onSaveAndRebuild={() => void saveAllAndRebuild()}
            evaluation={evaluation}
            startupPath={startupProject?.path ?? null}
            onStartup={setStartupProject}
            onEvaluate={(next) => void runEvaluation(next)}
            onBuild={() => requestBuild()}
            onBuildSolution={requestBuildSolution}
            solutionBuildDisabledReason={buildSolutionDisabledReason}
            onCancel={() => void cancelBuild()}
            disabledReason={buildDisabledReason}
            trusted={trusted}
            onTrust={() => void grantTrust()}
            restoreRequired={restoreIncomplete}
            restoreDisabledReason={restoreDisabledReason}
            onRestore={() => void startRestore("restore.start")}
            obsolete={buildObsolete}
            onNavigate={(path, position) => void navigateTo(path, position)}
          />
        );
      case "problems":
        return (
          <ProblemsPanel
            diagnostics={snapshot.diagnostics}
            build={build}
            tests={snapshot.tests}
            evaluation={evaluation}
            buildObsolete={buildObsolete}
            editedAt={editedAt}
            onNavigate={(path, position) => void navigateTo(path, position)}
            onOpen={(path) => void openUserFile(path)}
          />
        );
      case "debug":
        return (
          <div data-testid="debug-tab">
            <DebugPanel
              snapshot={snapshot}
              shortcuts={{ start: shortcutOf("debug.start"), stop: settings.keymap.profile === "visual-studio" ? shortcutOf("run.stop") : shortcutOf("debug.stop"), resume: shortcutOf("debug.continue"), over: shortcutOf("debug.stepOver"), into: shortcutOf("debug.stepInto"), out: shortcutOf("debug.stepOut"), breakpoint: shortcutOf("debug.toggleBreakpoint") }}
              disabledReason={debugDisabledReason}
              trusted={trusted}
              onTrust={() => void grantTrust()}
              onLaunch={() => void prepareAndDebug()}
              onSnapshot={onSnapshot}
              onError={(text) => setMessage({ tone: "error", text })}
              onRestart={() => void restartDebug()}
              onOpenFrame={(path, line) => void navigateTo(path, { line: line - 1, character: 0 })}
            />
          </div>
        );
      case "terminal":
        return (
          <TerminalPanel
            snapshot={snapshot}
            disabledReason={
              trusted ? null : "Trust the workspace before starting a terminal; no shell is spawned in Restricted mode."
            }
            onSnapshot={onSnapshot}
            onError={(text) => setMessage({ tone: "error", text })}
          />
        );
      case "tests":
        return (
          <TestsPanel
            tests={tests}
            evaluation={evaluation}
            editedAt={editedAt}
            disabledReason={testsDisabledReason}
            autoDiscoveryError={
              testAutoDiscoveryError?.boundary === testDiscoveryBoundary
                ? testAutoDiscoveryError.message
                : null
            }
            projectPath={testProjectPath}
            onProjectChange={setTestProjectPath}
            selected={testSelected}
            onToggleSelected={toggleTestSelected}
            restoreRequired={restoreIncomplete}
            restoreDisabledReason={restoreDisabledReason}
            onRestore={() => void startRestore("restore.start")}
            onEvaluate={(next) => void runEvaluation(next)}
            onDiscover={(project) => void startTestDiscovery(project)}
            onCancel={() => void cancelTestDiscovery()}
            onRun={(project, selection, testIds) =>
              void startTestRun(project, selection, testIds)
            }
            onRunCancel={() => void cancelTestRun()}
            onNavigate={(path, position) =>
              focusEditorAfterOpen(() => navigateTo(path, position))
            }
          />
        );
      case "git":
        return (
          <GitLogPanel
            trusted={trusted}
            onError={(text) => setMessage({ tone: "error", text })}
          />
        );
      case "nuget":
        return (
          <NuGetPanel
            trusted={trusted}
            projects={projects}
            restoreRunning={restoring}
            onRestore={(project) => void startRestore("restore.start", project)}
            onMessage={(text) => setMessage({ tone: "info", text })}
            onError={(text) => setMessage({ tone: "error", text })}
          />
        );
    }
  }

  const sdkSummary = `Analysis runtime ${
    snapshot.sdk.host.state === "found" ? snapshot.sdk.host.version : "missing"
  } · ${
    projectSdk.state === "resolved"
      ? `Project SDK ${projectSdk.resolvedVersion}`
      : projectSdk.state === "sdk_missing"
        ? `Project SDK missing (${projectSdk.requestedVersion ?? "pinned"})`
        : projectSdk.state === "not_inspected"
          ? "Project SDK not inspected"
          : projectSdk.state === "no_global_json"
            ? "No pinned project SDK"
            : projectSdk.state === "not_pinned"
              ? "Project SDK not pinned"
              : "Project SDK unresolved"
  }`;

  const allDiagnostics = collectDiagnostics(snapshot.diagnostics, build, buildObsolete, editedAt);
  const activeDiagnostics = active ? allDiagnostics.filter((item) => samePath(item.path, active.path)) : [];
  const problems = problemCounts(
    snapshot.diagnostics,
    build,
    snapshot.tests,
    evaluation,
    buildObsolete,
    editedAt,
  );
  const diagnosticCounts = { errors: problems.errors, warnings: problems.warnings };
  const activeInspections = active
    ? snapshot.diagnostics.snapshots
        .filter((snap) => snap.state === "current" && snap.path && snap.path.replace(/\\/g, "/").endsWith(`/${active.path}`))
        .flatMap((snap) => snap.items)
        .reduce((count, item) => {
          if (item.severityLabel === "error") count.error += 1;
          else if (item.severityLabel === "warning") count.warning += 1;
          return count;
        }, { error: 0, warning: 0 })
    : null;
  const bottomTabLabels = bottomTools.map(({ id, title }) => ({
    id: id as BottomToolId,
    label: id === "problems" && problems.total > 0 ? `Problems (${problems.total})` : title,
  }));
  const breadcrumbProject = activeEvaluationProject?.name ?? activeProject?.name ?? null;
  const statusBreadcrumb = [
    breadcrumbProject && breadcrumbProject !== workspace.name ? breadcrumbProject : null,
    active ? baseName(active.path) : null,
  ].filter((part): part is string => Boolean(part)).join(" > ");
  const evaluationStatus = evaluationStatusView(evaluation);
  const partialWorkspace = showsEvaluatedTree ? partialWorkspaceStatus(evaluatedProjects) : null;
  const evaluationRun = {
    label: evaluation.revision > 0 ? "Re-evaluate project model" : "Evaluate project model",
    disabledReason: !evaluationAvailable
      ? "Trust the workspace and resolve the project SDK before evaluating."
      : evaluationBusy
        ? "Wait for the current evaluation to finish."
        : null,
    onClick: () => void runEvaluation(),
  };

  async function openSolution(solution: string) {
    if (solution === workspace.path) return;
    if (dirtyCount > 0) {
      setMessage({ tone: "error", text: "Save or close modified buffers before opening another solution." });
      return;
    }
    try {
      await onOpenRecent(solution);
    } catch (error) {
      const candidates = candidatesOf(error);
      if (candidates.length > 0) {
        setExternalCandidates({ path: solution, candidates });
        return;
      }
      setMessage({ tone: "error", text: `Could not open ${solution}: ${messageOf(error)}` });
    }
  }

  // The request this view was mounted with was already handled by App; only new requests open.
  const handledExternalOpen = useRef(externalOpen);
  useEffect(() => {
    if (!externalOpen || externalOpen === handledExternalOpen.current) return;
    handledExternalOpen.current = externalOpen;
    setExternalCandidates(null);
    void openSolution(externalOpen.path);
  }, [externalOpen]);

  const buildAction = requireAction(actionById, "build.start");
  const buildSolutionAction = requireAction(actionById, "build.solution");
  const runAction = requireAction(actionById, "run.start");
  const debugStartAction = requireAction(actionById, "debug.start");
  const stopAction = requireAction(actionById, "run.stop");
  const searchAction = requireAction(actionById, "search.everywhere");
  const searchHint =
    searchAction.shortcutTexts.length > 0
      ? `${searchAction.shortcutTexts.join(" or ")} or Double Shift`
      : "Double Shift";
  const settingsAction = requireAction(actionById, "settings.open");
  const findActionAction = requireAction(actionById, "view.findAction");
  const keymapAction = requireAction(actionById, "view.keymap");
  const redetectAction = requireAction(actionById, "sdk.redetect");
  const openGlobalJsonAction = requireAction(actionById, "sdk.openGlobalJson");
  const commitAction = requireAction(actionById, "git.commit");
  const closeWorkspaceAction = requireAction(actionById, "workspace.close");
  const renameAction = requireAction(actionById, "code.rename");
  const contextActionsAction = requireAction(actionById, "code.contextActions");
  const saveAction = requireAction(actionById, "file.save");
  const backAction = requireAction(actionById, "navigate.back");
  const forwardAction = requireAction(actionById, "navigate.forward");

  return (
    <main
      className={`workspace-view ${leftOpen ? "" : "left-closed"} ${bottomOpen ? "" : "bottom-closed"} ${toolsOpen ? "tools-open" : ""}`}
      data-density={settings.appearance.density}
      style={{
        "--sidebar-width": `${effectiveSidebarWidth}px`,
        "--output-height": `${visibleOutputHeight}px`,
        "--ui-zoom": settings.appearance.zoom / 100,
        zoom: settings.appearance.zoom / 100,
      } as CSSProperties}
    >
      <div className="toolbar">
        <button type="button" className="toolbar-icon-button nav-button" data-testid="nav-back" disabled={backAction.disabledReason !== null} aria-label="Back" title={backAction.disabledReason ?? `Back (${backAction.shortcutTexts.join(" or ")})`} onClick={backAction.execute}><Icon name="back" /></button>
        <button type="button" className="toolbar-icon-button nav-button" data-testid="nav-forward" disabled={forwardAction.disabledReason !== null} aria-label="Forward" title={forwardAction.disabledReason ?? `Forward (${forwardAction.shortcutTexts.join(" or ")})`} onClick={forwardAction.execute}><Icon name="forward" /></button>
        <details className="toolbar-menu project-menu" onKeyDown={(event) => {
          if (event.key === "Escape") { event.currentTarget.open = false; event.currentTarget.querySelector("summary")?.focus(); }
        }}>
          <summary className="workspace-brand" title={workspace.path} data-testid="toolbar-project" aria-label={`Project: ${workspace.name}. Open recent solutions.`}>
            <span className="project-badge" aria-hidden="true">{workspace.name.slice(0, 2).toUpperCase()}</span><strong><MidText text={workspace.name} /></strong><Icon name="chevron" />
          </summary>
          <div className="menu-content" data-testid="toolbar-project-menu">
            <span className="menu-heading">Recent solutions</span>
                        {recentSolutions.map((solution, index) => (
              <button
                key={solution}
                type="button"
                data-testid={`recent-solution-${index}`}
                aria-current={solution === workspace.path ? "true" : undefined}
                title={solution}
                onClick={() => void openSolution(solution)}
              >
                {baseName(solution)}{solution === workspace.path ? " (current)" : ""}
              </button>
            ))}
            {recentSolutions.length === 0 && <span className="menu-note">No recent solutions.</span>}
          </div>
        </details>
        <BranchWidget
          status={gitStatus}
          statusError={gitStatusError}
          trusted={trusted}
          intent={gitIntent}
          onRefresh={refreshGit}
          onOpenCommit={() => openCommitPanel()}
          onMessage={(text) => setMessage({ tone: "info", text })}
        />
        <span className="push" />
        <button
          data-testid="toolbar-build"
          disabled={buildAction.disabledReason !== null}
          onClick={buildAction.execute}
          title={buildAction.disabledReason ?? `Builds the selected startup project (${buildAction.shortcutTexts.join(" or ")}). Unsaved buffers need an explicit decision.`}
          className="toolbar-icon-button"
          aria-label="Build"
        ><Icon name="build" /></button>
        <RunConfigWidget
          configs={runConfigStore?.configs ?? []}
          defaultConfig={defaultRunConfig}
          activeId={activeRunConfig?.id ?? DEFAULT_RUN_CONFIG_ID}
          disabledReason={
            !trusted
              ? "Trust the workspace before choosing a run configuration; nothing runs in Restricted mode."
              : !["ok", "partial"].includes(evaluation.state)
                ? "Evaluate the project model before choosing a run configuration."
                : consoleCandidates.length === 0
                  ? "No evaluated console project is available for a run configuration."
                  : !startupProject && !activeNamedConfig
                    ? "More than one console project; choose the startup project explicitly."
                    : null
          }
          onSelect={(id) => void selectRunConfig(id)}
          profiles={profileState?.profiles ?? []}
          onOpen={() => void refreshLaunchProfiles()}
          onSelectProfile={(profile) => {
            if (!profileProject) return;
            void selectLaunchProfile(profileProject, profile).then(() => refreshLaunchProfiles()).catch((error) => setMessage({ tone: "error", text: `Launch profile could not be selected: ${messageOf(error)}` }));
          }}
          onEdit={() => openRunConfigDialog()}
        />
        <label
          className="toolbar-startup"
          title={
            consoleCandidates.length === 0
              ? "No evaluated console project is available."
              : "Startup project for Build, Run and Debug; saved per solution."
          }
        >
          <select
            data-testid="toolbar-startup"
            aria-label="Startup project"
            value={startupProject?.path ?? ""}
            disabled={consoleCandidates.length === 0 || buildRunning}
            onChange={(event) => setStartupProject(event.target.value)}
          >
            {consoleCandidates.length === 0 && <option value="">No console project</option>}
            {consoleCandidates.map((project) => (
              <option key={project.path} value={project.path}>
                {project.name}
              </option>
            ))}
          </select>
        </label>
        {debugActive && (
          <button
            data-testid="toolbar-debug-stop"
            className="toolbar-icon-button stop-button"
            aria-label="Stop"
            onClick={stopAction.execute}
            title={stopAction.disabledReason ?? "Stops the adapter group and reports what was observed."}
          >
            <Icon name="stop" />
          </button>
        )}
        <button
          data-testid="toolbar-run"
          className="toolbar-icon-button run-button"
          aria-label={runRunning || runBuilding ? "Running" : "Run startup project"}
          aria-disabled={runAction.disabledReason !== null}
          onClick={() => runAction.execute()}
          title={
            runAction.disabledReason ??
            `Runs the saved build, or builds the startup project first when there is no current build (${runAction.shortcutTexts.join(" or ")}); unsaved buffers are never included.`
          }
        >
          <Icon name="run" />
        </button>
        <button
          data-testid="toolbar-debug"
          className="toolbar-icon-button debug-button"
          aria-label="Debug"
          aria-disabled={debugStartAction.disabledReason !== null}
          onClick={() => debugStartAction.execute()}
          title={
            debugStartAction.disabledReason ??
            `Starts the evaluated Debug assembly under netcoredbg, enabling the experimental flag and reusing or installing the approved adapter when needed, and building first when there is no current build (${debugStartAction.shortcutTexts.join(" or ")}).`
          }
        >
          <Icon name="debug" />
        </button>
        <details className="toolbar-menu" data-testid="workspace-actions-menu" onKeyDown={(event) => {
          if (event.key === "Escape") { event.currentTarget.open = false; event.currentTarget.querySelector("summary")?.focus(); }
        }}>
          <summary aria-label="Workspace actions" title="More actions">⋮</summary>
          <div className="menu-content">
            <label className="eval-field"><span>Configuration</span><select data-testid="eval-configuration" value={evaluation.context.configuration} disabled={!evaluationAvailable || evaluationBusy} onChange={(event) => void runEvaluation({ configuration: event.target.value })} title={!trusted ? "Trust the workspace and resolve the project SDK to select a configuration." : projectSdk.state !== "resolved" ? "Resolve the project SDK before selecting a configuration." : evaluationBusy ? "Wait for evaluation to finish before changing configuration." : "MSBuild configuration for evaluation."}>{configurations.map((configuration) => <option key={configuration} value={configuration}>{configuration}</option>)}</select></label>
            <label className="eval-field"><span>TFM</span><select data-testid="eval-tfm" value={evaluation.context.targetFramework ?? ""} disabled={!evaluationAvailable || evaluationBusy} onChange={(event) => void runEvaluation({ targetFramework: event.target.value || null })} title={!trusted ? "Trust the workspace and resolve the project SDK to select a TFM." : projectSdk.state !== "resolved" ? "Resolve the project SDK before selecting a TFM." : evaluationBusy ? "Wait for evaluation to finish before changing TFM." : "Target framework for evaluation."}><option value="">TFM: unset</option>{tfmOptions.map((tfm) => <option key={tfm} value={tfm}>{tfm}</option>)}</select></label>
            <button
              data-testid="build-solution-menu"
              disabled={buildSolutionAction.disabledReason !== null}
              title={buildSolutionAction.disabledReason ?? "Builds the open solution with the current configuration and reports problems from every project."}
              onClick={(event) => {
                event.currentTarget.closest("details")?.removeAttribute("open");
                buildSolutionAction.execute();
              }}
            >
              Build Solution <kbd>{buildSolutionAction.shortcutLabels[0] ?? ""}</kbd>
            </button>
            <button data-testid="open-find-action" onClick={(event) => {
              event.currentTarget.closest("details")?.removeAttribute("open");
              findActionAction.execute();
            }} title={findActionAction.shortcutTexts.join(" or ")}>
              Find Action <kbd>{findActionAction.shortcutLabels[0] ?? ""}</kbd>
            </button>
            <button data-testid="open-keymap" onClick={(event) => {
              event.currentTarget.closest("details")?.removeAttribute("open");
              keymapAction.execute();
            }}>
              {keymapAction.title}
            </button>
            <button
              data-testid="sdk-redetect"
              onClick={redetectAction.execute}
              title={redetectAction.disabledReason ?? "Re-runs only the SDK inventory; it never touches project files."}
            >
              {redetectAction.title}
            </button>
            <button
              data-testid="sdk-open-global-json"
              onClick={openGlobalJsonAction.execute}
              disabled={openGlobalJsonAction.disabledReason !== null}
              title={openGlobalJsonAction.disabledReason ?? openGlobalJsonAction.title}
            >
              {openGlobalJsonAction.title}
            </button>
            <button
              data-testid="git-commit-action"
              disabled={commitAction.disabledReason !== null}
              title={commitAction.disabledReason ?? commitAction.title}
            >
              {commitAction.title}
            </button>
            <button data-testid="close-workspace" onClick={closeWorkspaceAction.execute}>
              {closeWorkspaceAction.title}
            </button>
          </div>
        </details>
        <button
          type="button"
          className="toolbar-icon-button"
          data-testid="toolbar-search"
          disabled={Boolean(searchAction.disabledReason)}
          aria-label={searchAction.title}
          title={searchAction.disabledReason ?? searchHint}
          onClick={searchAction.execute}
        >
          <Icon name="search" />
        </button>
        <button
          type="button"
          className="toolbar-icon-button"
          data-testid="toolbar-settings"
          aria-label={settingsAction.title}
          title={settingsAction.shortcutTexts.join(" or ") || settingsAction.title}
          onClick={settingsAction.execute}
        >
          <Icon name="settings" />
        </button>
      </div>

      <div className="workspace-notices">
      {externalCandidates && (
        <div className="notice error" role="alert" data-testid="external-open-candidates">
          {externalCandidates.path} contains more than one solution or project:
          {externalCandidates.candidates.map((candidate, index) => (
            <button
              key={candidate.path}
              className="link-button"
              data-testid={`external-open-candidate-${index}`}
              onClick={() => {
                setExternalCandidates(null);
                void openSolution(candidate.path);
              }}
            >
              {candidate.name}
            </button>
          ))}
          <button className="notice-dismiss" aria-label="Dismiss candidates" onClick={() => setExternalCandidates(null)}>×</button>
        </div>
      )}
      {message && (
        <div
          className={`notice ${message.tone === "error" ? "error" : ""}`}
          role={message.tone === "error" ? "alert" : "status"}
          data-testid="app-log"
        >
          {message.text}
          {message.action && (
            <button
              type="button"
              className="notice-action"
              data-testid="notice-action"
              onClick={() => {
                const action = message.action;
                setMessage(null);
                action?.onClick();
              }}
            >
              {message.action.label}
            </button>
          )}
          <button className="notice-dismiss" aria-label="Dismiss notification" onClick={() => setMessage(null)}>×</button>
        </div>
      )}
      {trustError && (
        <div className="notice error" role="alert" data-testid="trust-grant-error">
          Trust could not be granted: {trustError}
          <button className="notice-dismiss" aria-label="Dismiss trust error" onClick={() => setTrustError(null)}>×</button>
        </div>
      )}

      </div>

      <div className="toolrail toolrail-right" data-testid="tool-windows-right" aria-label="Right tool windows">
        <button
          type="button"
          className={`toolband-button ${toolsOpen ? "active" : ""}`}
          data-testid="workspace-tools-toggle"
          aria-label="Workspace tools"
          aria-pressed={toolsOpen}
          title={`Workspace tools · ${restoreIncomplete ? "Restore required" : restoreStateLabel(restore)} · Analysis: ${lsp.state}`}
          onClick={() => setToolsOpen((open) => !open)}
        >
          <Icon name="shield" />
        </button>
        {["Monitoring", "AI Assistant", "Database"].map((title) => (
          <button
            key={title}
            type="button"
            className="toolband-button"
            disabled
            aria-label={title}
            title={`${title} is not part of OpenRidr`}
          >
            {title[0]}
          </button>
        ))}
      </div>

      {toolsOpen && (
        <aside className="tools-drawer" aria-label="Workspace tools" data-testid="workspace-tools">
          <div className="tools-drawer-head">
            <strong>Workspace tools</strong>
            <button type="button" aria-label="Close workspace tools" data-testid="workspace-tools-close" onClick={() => setToolsOpen(false)}>×</button>
          </div>
          {renderToolsContent()}
        </aside>
      )}

      <div className="toolrail" data-testid="tool-windows">
        <ToolWindowBand
          slot="left"
          items={toolWindowsIn("left")}
          actions={resolvedActions}
          activeId={layout.left.active}
          open={leftOpen}
        />
        <span className="push" />
        <ToolWindowBand
          slot="bottom"
          items={bottomTools}
          actions={resolvedActions}
          activeId={bottomTab}
          open={bottomOpen}
        />
      </div>

      <div className={`workspace-grid ${selectedProject ? "with-details" : ""}`}>
        {leftOpen && (
          <>
        {layout.left.active === "commit" ? (
          <CommitPanel
            status={gitStatus}
            statusError={gitStatusError}
            trusted={trusted}
            initialPath={commitFocusPath}
            onRefresh={refreshGit}
            onOpenDiff={(path) => { setCommitDiffPath(path); setCommitDiffActive(true); }}
          />
        ) : (
        <nav className="side" aria-label="Solution explorer" data-testid="tree">
          <div className="panel-title">
            <span className="explorer-heading">
              <span className="explorer-title">Solution<Icon name="chevron" /></span>
              <SolutionToolbar
                showAll={showAllFiles}
                searchOpen={treeSearchOpen}
                locateDisabled={!activePath}
                moreItems={treeMoreItems}
                onToggleShowAll={() => setShowAllFiles((current) => !current)}
                onToggleSearch={() => {
                  setTreeSearchOpen((current) => !current);
                  setTreeFilter("");
                }}
                onLocate={locateOpenedFile}
                onExpandAll={() => setExpanded(expandableKeys(showsEvaluatedTree ? evaluatedProjects : null, projects, true))}
                onCollapseAll={() => setExpanded(new Set([ROOT_KEY]))}
                onMinimize={() => toggleToolWindow("left", "solution")}
              />
            </span>
          </div>
          {treeSearchOpen && (
            <input
              className="tree-filter"
              data-testid="tree-filter"
              aria-label="Filter the Solution tree"
              placeholder="Filter by name"
              autoFocus
              value={treeFilter}
              onChange={(event) => setTreeFilter(event.target.value)}
              onKeyDown={(event) => {
                if (event.key !== "Escape") return;
                event.stopPropagation();
                setTreeSearchOpen(false);
                setTreeFilter("");
              }}
            />
          )}

          {trusted && (emptySolution || !evaluationAvailable) && (
            <div className="tree-note" data-testid="eval-disabled-reason">
              {emptySolution ? (
                <p>
                  {workspace.name} lists no projects; the solution file is not valid or is empty.
                  Evaluation, build and run need at least one project.
                </p>
              ) : projectSdk.state === "sdk_missing" ? (
                <>
                  <p>
                    {projectSdk.globalJsonPath ?? "global.json"} requests{" "}
                    {projectSdk.requestedVersion ?? "a pinned SDK"}. OpenRidr detects SDKs but
                    never installs one or substitutes the .NET 10 analysis runtime. Install that
                    SDK (or change global.json) and redetect.
                  </p>
                  <div className="actions">
                    <button data-testid="eval-redetect" onClick={() => void redetectSdks()}>
                      Redetect SDKs
                    </button>
                    <a
                      data-testid="eval-sdk-download"
                      href={projectSdk.instructionsUrl}
                      target="_blank"
                      rel="noreferrer noopener"
                    >
                      Official .NET SDK downloads
                    </a>
                    {projectSdk.globalJsonPath && (
                      <button data-testid="eval-open-global-json" onClick={openGlobalJson}>
                        Open global.json
                      </button>
                    )}
                  </div>
                </>
              ) : (
                <p>
                  {projectSdk.message ?? "No project SDK is resolved."} Evaluation, build and run
                  need a project SDK from global.json or the declared TargetFramework; fix or add
                  global.json.
                </p>
              )}
            </div>
          )}

          {evaluation.staleReason && (
            <div className="notice warning eval-stale" role="status" data-testid="eval-stale">
              <strong>Evaluation stale.</strong> {evaluation.staleReason}{" "}
              {hasEarlierEvaluation
                ? "Values shown are from the previous evaluation and are not current."
                : "Nothing is current yet; Re-evaluate to produce a current model."}
              <div className="actions">
                <button
                  data-testid="eval-stale-run"
                  disabled={!evaluationAvailable || evaluationBusy}
                  onClick={() => void runEvaluation()}
                >
                  Re-evaluate
                </button>
              </div>
            </div>
          )}

          <SolutionTree
            workspaceName={workspace.name}
            evaluated={showsEvaluatedTree ? evaluatedProjects : null}
            outline={projects}
            expanded={expanded}
            onToggle={toggleProject}
            selectedProjectPath={selectedProjectPath}
            onSelectProject={setSelectedProjectPath}
            activePath={activePath}
            onOpenFile={(path) => void openUserFile(path)}
            restoreIncomplete={(path) => showsEvaluatedTree && restore.revision > 0 && restoreProjectState.get(path) !== "restored"}
            evaluationRevision={evaluation.revision}
            showAll={showAllFiles}
            filter={treeFilter}
            gitStatus={gitStatus}
            workspaceDir={workspaceDir}
            onMenu={openTreeMenu}
          />
        </nav>
        )}
        <Splitter label="Solution explorer width" value={effectiveSidebarWidth} min={Math.min(220, effectiveSidebarWidth)} max={420} onChange={setSidebarWidth} />
          </>
        )}

        <section className="editor-pane" aria-label="Editor">
          <div className="tabbar">
            <div className="tabs" role="tablist" aria-label="Open files" onKeyDown={(event) => {
              const count = tabs.length + (commitDiffPath ? 1 : 0);
              if (!count || !["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
              event.preventDefault();
              const index = commitDiffActive ? tabs.length : Math.max(0, tabs.findIndex((tab) => tab.path === activePath));
              const next = event.key === "Home" ? 0 : event.key === "End" ? count - 1 : (index + (event.key === "ArrowRight" ? 1 : -1) + count) % count;
              setCommitDiffActive(next === tabs.length);
              if (next < tabs.length) { setActivePath(tabs[next].path); void checkDisk(tabs[next].path); }
              event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]')[next]?.focus();
            }}>
              {tabs.map((tab) => {
                const dirty = isDirty(tab);
                const selected = !commitDiffActive && tab.path === activePath;
                return (
                  <div className={`tab ${selected ? "active" : ""}`} key={tab.path} data-testid={`tab-${tab.path}`}>
                    <button
                      role="tab"
                      aria-selected={selected}
                      tabIndex={selected ? 0 : -1}
                      aria-controls="editor-content"
                      className="tab-label"
                      onClick={() => {
                        setCommitDiffActive(false);
                        setActivePath(tab.path);
                        void checkDisk(tab.path);
                      }}
                      title={tab.path}
                      aria-label={`${baseName(tab.path)}${dirty ? ", modified" : ""}`}
                    >
                      <FileIcon path={tab.path} />
                      {baseName(tab.path)}
                      {dirty && (
                        <span
                          className="dirty"
                          data-testid="dirty-marker"
                          aria-label="Modified"
                          title="Modified"
                        >
                          *
                        </span>
                      )}
                    </button>
                    <button
                      className="tab-close"
                      data-testid="close-buffer"
                      aria-label={`Close ${baseName(tab.path)}`}
                      title="Close file"
                      onClick={() => requestCloseTab(tab.path)}
                    >
                      ×
                    </button>
                  </div>
                );
              })}
              {commitDiffPath && <div className={`tab ${commitDiffActive ? "active" : ""}`} data-testid="commit-diff-tab">
                <button role="tab" className="tab-label" tabIndex={commitDiffActive ? 0 : -1} aria-selected={commitDiffActive} aria-controls="commit-diff-content" onClick={() => setCommitDiffActive(true)} title={commitDiffPath}>Commit: {baseName(commitDiffPath)}</button>
                <button className="tab-close" aria-label="Close diff" title="Close diff" onClick={() => { setCommitDiffPath(null); setCommitDiffActive(false); }}>×</button>
              </div>}
            </div>
            {activeInspections && (activeInspections.error > 0 || activeInspections.warning > 0) && (
              <span className="inspection-summary" data-testid="editor-inspections" title="Current diagnostics in this file">
                {activeInspections.error > 0 && <span className="insp-error">✕ {activeInspections.error}</span>}
                {activeInspections.warning > 0 && <span className="insp-warning">⚠ {activeInspections.warning}</span>}
              </span>
            )}
            <button
              className="save"
              data-testid="rename-open"
              onClick={renameAction.execute}
              disabled={renameAction.disabledReason !== null}
              title={
                renameAction.disabledReason ??
                `Rename the symbol at the cursor (${renameAction.shortcutTexts.join(" or ")}); preview before applying.`
              }
            >
              <Icon name="rename" /><span className="editor-action-label"> Rename symbol…</span>
            </button>
            <button
              className="save"
              data-testid="code-actions-open"
              onClick={contextActionsAction.execute}
              disabled={contextActionsAction.disabledReason !== null}
              title={
                contextActionsAction.disabledReason ??
                `Code actions at the cursor (${contextActionsAction.shortcutTexts.join(" or ")}).`
              }
            >
              <Icon name="code" /><span className="editor-action-label"> Code actions…</span>
            </button>
            <button
              className="save"
              data-testid="save-file"
              aria-label="Save file"
              onClick={saveAction.execute}
              disabled={saveAction.disabledReason !== null || !active || !isDirty(active)}
              title={
                saveAction.disabledReason ??
                `Save with ${saveAction.shortcutTexts.join(" or ")}`
              }
            >
              <Icon name="save" /><span className="editor-action-label"> Save</span>
            </button>
          </div>

          {commitDiffActive && commitDiffPath ? (
            <CommitDiff path={commitDiffPath} files={gitStatus?.files ?? []} onNavigate={setCommitDiffPath} />
          ) : active ? (
            <>
              <div className="breadcrumb-row">
              <div className="breadcrumb" data-testid="breadcrumb">
                {activeEvaluationProject
                  ? `${activeEvaluationProject.name} / `
                  : activeProject
                    ? `${activeProject.name} / `
                    : ""}
                {baseName(active.path)}
                {activeEvaluationProject
                  ? ` / ${activeEvaluationProject.tfms.join(", ") || "TFM not evaluated"}`
                  : activeProject
                    ? ` / ${activeProject.tfms.join(", ") || "TFM not declared"}`
                    : ""}
              </div>
              <span
                className={isDirty(active) ? "tag warning" : "muted"}
                data-testid="editor-revision"
              >
                {isDirty(active) ? "Modified" : ""}
              </span>
              </div>

              {active.conflict && (
                <div className="notice error" data-testid="conflict-banner" role="alert">
                  <strong>File changed on disk.</strong> {active.path} was modified outside
                  OpenRidr. Your buffer is preserved; nothing was overwritten.
                  <div className="actions">
                    <button data-testid="conflict-reload" onClick={reloadFromDisk}>
                      Reload from disk (discard my edits)
                    </button>
                    <button data-testid="conflict-overwrite" onClick={() => void overwriteDisk()}>
                      Overwrite disk
                    </button>
                  </div>
                  <details className="conflict-compare" data-testid="conflict-compare">
                    <summary>Compare buffer and disk</summary>
                    <div className="conflict-compare-grid">
                      <section aria-label="Buffer version">
                        <h3>Buffer · unsaved · revision {active.revision}</h3>
                        <p className="muted">
                          Based on <code data-testid="conflict-buffer-hash">{active.hash}</code>
                        </p>
                        <pre data-testid="conflict-compare-buffer">{active.content}</pre>
                      </section>
                      <section aria-label="Disk version">
                        <h3>Disk · external change</h3>
                        <p className="muted">
                          Disk hash{" "}
                          <code data-testid="conflict-disk-hash">{active.conflict.hash}</code>
                        </p>
                        <pre data-testid="conflict-compare-disk">{active.conflict.content}</pre>
                      </section>
                    </div>
                    <p className="muted" data-testid="conflict-compare-diff">
                      {activeConflictDiff
                        ? `${countLabel(activeConflictDiff.removed, "line")} removed · ${countLabel(activeConflictDiff.added, "line")} added`
                        : "Line diff skipped: one version has more than 1200 lines; compare hashes only."}
                    </p>
                  </details>
                </div>
              )}

              <div className="editor-body" id="editor-content" role="tabpanel" aria-label={baseName(active.path)}>
                <CodeEditor
                  key={active.path}
                  value={active.content}
                  path={active.path}
                  platform={snapshot.app.platform}
                  profile={settings.keymap.profile}
                  shortcutHint={`Ctrl+Space: completion · ${shortcutOf("code.goToDeclaration") ?? ""}: go to definition · ${shortcutOf("code.rename") ?? ""}: rename · ${shortcutOf("code.findUsages") ?? ""}: find usages · ${shortcutOf("code.contextActions") ?? ""}: code actions`}
                  revision={active.revision}
                  editor={settings.editor}
                  onChange={(value) => updateContent(active.path, value)}
                  onSave={() => void save()}
                  onDefinition={(location) =>
                    focusEditorAfterOpen(() =>
                      navigateTo(location.path, {
                        line: location.line,
                        character: location.character,
                      }),
                    )
                  }
                  onRename={(position) => requestRename(position)}
                  onRenameSubmit={submitRenameInline}
                  onRenameCancel={cancelRenameRequest}
                  onCodeActions={(position) => void requestCodeActions(position)}
                  onContextMenu={(position, at) => {
                    noteCursor(position);
                    setContextMenu({
                      x: at.x,
                      y: at.y,
                      line: position.line,
                      character: position.character,
                    });
                    if (trusted) void refreshGit();
                  }}
                  onGutterAction={handleGutterAction}
                  gutterActions={gutterActions}
                  changedLines={changedLines}
                  renameRequest={renameRequest}
                  definitionRequest={definitionRequest}
                  gotoLineRequest={gotoLineRequest}
                  onCursor={noteCursor}
                  diagnostics={activeDiagnostics}
                  breakpoints={activeBreakpoints}
                  verifiedBreakpoints={activeVerifiedBreakpoints}
                  currentLine={debugCurrentLine}
                  inlayText={debugInlay}
                  onHoverEvaluate={
                    trusted && dap.state === "paused"
                      ? async (expression) => {
                          try {
                            return await call<DapEvaluation>("debug.hover", { expression });
                          } catch {
                            return null;
                          }
                        }
                      : undefined
                  }
                  onToggleBreakpoint={(line) => void toggleBreakpoint(line)}
                  target={
                    editorTarget?.path === active.path ? editorTarget : null
                  }
                />
              </div>

              {findUsages && (
                <FindUsagesPanel
                  className="rename-preview find-usages editor-popup"
                  response={findUsages.response}
                  snippets={findUsages.snippets}
                  onNavigate={(path, position) =>
                    focusEditorAfterOpen(() => navigateTo(path, position))
                  }
                  onClose={() => {
                    findUsagesSeq.current += 1;
                    setFindUsages(null);
                  }}
                />
              )}

              {editPreview && (
                <section
                  className="rename-preview editor-popup"
                  role="dialog"
                  aria-modal="false"
                  aria-labelledby="edit-preview-title"
                  data-testid="rename-preview"
                  data-origin={editPreview.kind}
                  data-id={previewId(editPreview)}
                  data-files={previewDocs.length}
                  data-edits={previewEditCount(editPreview)}
                  onKeyDown={(event) => {
                    if (event.key === "Escape") cancelEditPreview();
                  }}
                >
                  <h2 id="edit-preview-title" ref={editPreviewHeading} tabIndex={-1}>
                    {previewTitle(editPreview)}
                  </h2>
                  <p className="muted" data-testid="rename-preview-summary">
                    {countLabel(previewDocs.length, "file")} ·{" "}
                    {countLabel(previewEditCount(editPreview), "edit")} · session{" "}
                    {editPreview.session} · source revision{" "}
                    {editPreview.sourceRevision} ·{" "}
                    {
                      previewDocs.filter(
                        (document) => editPreview.tabRevisions[document.path] === undefined,
                      ).length
                    }{" "}
                    not opened here. Nothing has been applied.
                  </p>
                  {editPreview.kind === "rename" && editPreview.response.prepared && (
                    <p className="muted">prepareRename accepted the position.</p>
                  )}
                  {editPreview.kind === "action" && (
                    <p className="muted" data-testid="action-preview-detail">
                      Kind:{" "}
                      {editPreview.action.kind ?? "not provided by csharp-ls"} ·{" "}
                      {editPreview.action.isPreferred ? "preferred" : "not preferred"} ·{" "}
                      {countLabel(editPreview.response.diagnosticsUsed, "current diagnostic")} at
                      the request position.
                    </p>
                  )}
                  {editPreview.stale && (
                    <p className="error" role="alert" data-testid="rename-stale">
                      {editPreview.stale}
                    </p>
                  )}
                  <ul className="rename-index" data-testid="rename-file-index">
                    {previewDocs.map((document) => (
                      <li key={document.path} data-testid={`rename-index-${document.path}`}>
                        <code>{document.path}</code>{" "}
                        <span className="tag">
                          {editPreview.tabRevisions[document.path] === undefined
                            ? "not opened here"
                            : "open buffer"}
                        </span>{" "}
                        <span className="muted">
                          {countLabel(document.editCount, "edit")}
                        </span>
                      </li>
                    ))}
                  </ul>
                  <ol className="rename-files">
                    {previewDocs.map((document) => (
                      <li key={document.path} data-testid={`rename-file-${document.path}`}>
                        <div className="rename-file-head">
                          <code>{document.path}</code>
                          <span className="tag">
                            {editPreview.tabRevisions[document.path] === undefined
                              ? "not opened here"
                              : "open buffer"}
                          </span>
                          <span className="muted" data-testid={`rename-count-${document.path}`}>
                            {countLabel(document.editCount, "edit")}
                          </span>
                        </div>
                        <div
                          className="rename-diff"
                          data-testid={`rename-diff-${document.path}`}
                        >
                          {document.changes.slice(0, 8).map((change, index) => (
                            <div className="rename-change" key={`${change.line}-${index}`}>
                              <span className="muted">line {change.line}</span>
                              <pre className="rename-before">− {change.before}</pre>
                              <pre className="rename-after">+ {change.after}</pre>
                            </div>
                          ))}
                          {document.changes.length > 8 && (
                            <p className="muted">
                              … {document.changes.length - 8} more change
                              {document.changes.length - 8 === 1 ? "" : "s"} in this file.
                            </p>
                          )}
                          {document.displayTruncated && (
                            <p className="muted">
                              Long lines are shortened here; the applied text and counts are
                              complete.
                            </p>
                          )}
                        </div>
                      </li>
                    ))}
                  </ol>
                  <div className="actions">
                    <button
                      data-testid="rename-apply"
                      onClick={() => void applyEditPreview()}
                      disabled={editPreview.applying}
                    >
                      {editPreview.applying ? "Revalidating…" : "Apply to buffers"}
                    </button>
                    <button
                      data-testid="rename-cancel-preview"
                      onClick={cancelEditPreview}
                      disabled={editPreview.applying}
                    >
                      Cancel
                    </button>
                  </div>
                </section>
              )}

              {actionList && (
                <section
                  className="rename-preview code-actions editor-popup"
                  role="dialog"
                  aria-modal="false"
                  aria-labelledby="code-actions-title"
                  data-testid="code-actions"
                  data-actions={actionList.response.actionCount}
                  data-state={actionList.response.state}
                  data-session={actionList.session}
                  data-revision={actionList.sourceRevision}
                  onKeyDown={(event) => {
                    if (event.key === "Escape") cancelActionList();
                  }}
                >
                  <h2 id="code-actions-title" ref={actionListHeading} tabIndex={-1}>
                    Code actions
                  </h2>
                  <p className="muted" data-testid="code-actions-summary">
                    {actionList.response.state === "ok"
                      ? `${countLabel(actionList.response.actionCount, "action")} from csharp-ls · session ${actionList.session} · ${countLabel(
                          actionList.response.diagnosticsUsed,
                          "current diagnostic",
                        )} at this position · revision ${actionList.sourceRevision}. Nothing has been applied.`
                      : `Code actions ${actionList.response.state}: ${
                          actionList.response.message ?? "csharp-ls did not answer."
                        }`}
                  </p>
                  {actionList.response.truncated && (
                    <p className="muted">
                      Showing {actionList.response.actionCount} of{" "}
                      {actionList.response.totalActions} actions returned by csharp-ls.
                    </p>
                  )}
                  {actionList.stale && (
                    <p className="error" role="alert" data-testid="code-actions-stale">
                      {actionList.stale}
                    </p>
                  )}
                  {actionList.response.actions.length === 0 ? (
                    <p className="muted" data-testid="code-actions-empty">
                      {actionList.response.state === "ok"
                        ? `No code actions from csharp-ls at ${actionList.line + 1}:${
                            actionList.character + 1
                          }; nothing was offered and editing continues.`
                        : "No actions are available. Editing continues."}
                    </p>
                  ) : (
                    <ul className="code-actions-list" data-testid="code-actions-list">
                      {actionList.response.actions.map((action, index) => (
                        <li key={action.actionId}>
                          <button
                            type="button"
                            data-testid={`code-action-${index}`}
                            data-title={action.title}
                            data-supported={action.supported}
                            data-disabled={action.disabled ?? ""}
                            aria-disabled={!action.supported || action.disabled !== null}
                            title={
                              action.supported && !action.disabled
                                ? "Preview this action"
                                : action.reason ??
                                  action.disabled ??
                                  "This action cannot be applied."
                            }
                            onClick={() => chooseCodeAction(action)}
                          >
                            {action.title}
                          </button>
                          <span className="tag">{action.kind ?? "no kind"}</span>
                          {action.isPreferred && <span className="tag">preferred</span>}
                          {action.disabled && (
                            <span className="muted" data-testid={`code-action-disabled-${index}`}>
                              Disabled: {action.disabled}
                            </span>
                          )}
                          {!action.supported && !action.disabled && (
                            <span className="muted" data-testid={`code-action-reason-${index}`}>
                              {action.reason ?? "No supported edit was returned."}
                              {action.command
                                ? ` Command ${action.command.command} is never executed.`
                                : ""}
                            </span>
                          )}
                        </li>
                      ))}
                    </ul>
                  )}
                  <div className="actions">
                    <button data-testid="code-actions-close" onClick={cancelActionList}>
                      Close
                    </button>
                  </div>
                </section>
              )}
            </>
          ) : (
            <div className="empty-editor">
              <Icon name="code" />
              <h1>Your code, in focus.</h1>
              <p className="muted">Open a file from the solution explorer to start editing.</p>
              <dl className="shortcut-list">
                <dt>Search Everywhere</dt><dd><kbd>{searchAction.shortcutLabels[0] ?? ""}</kbd> <span className="muted">or Double Shift</span></dd>
                <dt>Save file</dt><dd><kbd>{actionById.get("file.save")?.shortcutLabels[0]}</kbd></dd>
                <dt>Go to definition</dt><dd><kbd>{actionById.get("code.goToDeclaration")?.shortcutLabels[0]}</kbd></dd>
                <dt>Rename symbol</dt><dd><kbd>{actionById.get("code.rename")?.shortcutLabels[0]}</kbd></dd>
                <dt>Run saved build</dt><dd><kbd>{actionById.get("run.start")?.shortcutLabels[0]}</kbd></dd>
              </dl>
              <p className="hint">Text editing is always available. Analysis and execution require trust.</p>
            </div>
          )}
        </section>

        <aside className="eval-details-pane" hidden={!selectedProject || !projectDetailsOpen} aria-label="Project inspector">
            {selectedProject && (
              <section
                className="eval-details"
                aria-label="Evaluated project details"
                tabIndex={0}
                data-testid="eval-details"
              >
                <div className="panel-title inspector-heading">
                  <span>{selectedProject.name} <span className="tag">Evaluated project</span></span>
                  <button className="inspector-close" onClick={() => setProjectDetailsOpen(false)}>Close project details</button>
                </div>
                <dl>
                  <dt>State</dt>
                  <dd data-testid="eval-details-state">
                    {selectedProject.state === "ok" ? "Evaluated" : "Evaluation failed"}
                    {evaluation.stale
                      ? hasEarlierEvaluation
                        ? " · stale (previous evaluation)"
                        : " · not current"
                      : ""}
                  </dd>
                  <dt>Path</dt>
                  <dd>
                    <code className="detail-path" title={selectedProject.path}>{selectedProject.path}</code>
                  </dd>
                  <dt>Kind</dt>
                  <dd>{selectedProject.kind}</dd>
                  <dt>TFMs</dt>
                  <dd>{selectedProject.tfms.join(", ") || "unknown"}</dd>
                  <dt>Configuration</dt>
                  <dd>{evaluation.context.configuration}</dd>
                  <dt>TFM selection</dt>
                  <dd>{evaluation.context.targetFramework ?? "Not selected"}</dd>
                  <dt>Evaluated TFM</dt>
                  <dd>{selectedProject.evaluatedTfm ?? "unknown"}</dd>
                  <dt>TargetPath</dt>
                  <dd data-testid="eval-target-path">
                    {selectedProject.targetPath
                      ? evaluation.stale && hasEarlierEvaluation
                        ? <>Previous evaluation (stale): <code className="detail-path" title={selectedProject.targetPath}>{selectedProject.targetPath}</code></>
                        : <code className="detail-path" title={selectedProject.targetPath}>{selectedProject.targetPath}</code>
                      : "Not available"}
                  </dd>
                  <dt>OutputType</dt>
                  <dd>{selectedProject.outputType ?? "unknown"}</dd>
                  <dt>Project references</dt>
                  <dd>
                    {selectedProject.projectRefs.length > 0
                      ? selectedProject.projectRefs.map((ref) => ref.name).join(", ")
                      : "None declared"}
                  </dd>
                  <dt>Packages</dt>
                  <dd data-testid="eval-packages">
                    {selectedProject.packageRefs.length} package
                    {selectedProject.packageRefs.length === 1 ? "" : "s"}
                    {(() => {
                      const unknown = selectedProject.packageRefs.filter(
                        (pkg) => !pkg.version,
                      ).length;
                      return unknown > 0 ? ` · ${unknown} version${unknown === 1 ? "" : "s"} unknown` : "";
                    })()}
                  </dd>
                  <dt>Restore</dt>
                  <dd data-testid="eval-restore">
                    {selectedProject.restored === "restored"
                      ? "Restored"
                      : selectedProject.restored === "unknown"
                        ? `Unknown — ${selectedProject.restoredReason ?? "could not inspect obj/project.assets.json"}`
                        : `Not restored — ${selectedProject.restoredReason ?? "obj/project.assets.json is missing or stale"}`}
                  </dd>
                  <dt>Evaluation</dt>
                  <dd data-testid="eval-revision">
                    revision {evaluation.revision} · context {evaluation.contextId ?? "unknown"} ·
                    workspace {evaluation.workspaceId ?? "unknown"}
                  </dd>
                </dl>
                {selectedProject.note && <p className="hint">{selectedProject.note}</p>}
                {selectedProject.error && (
                  <div className="notice error" role="alert">
                    <strong>Evaluation failed.</strong>{" "}
                    <span data-testid="eval-error">{selectedProject.error}</span>
                  </div>
                )}
                <div className="actions">
                  <button
                    data-testid="eval-open-project"
                    onClick={() => void openUserFile(selectedProject.path)}
                  >
                    Open project file
                  </button>
                  {selectedProject.state === "failed" && (
                    <button
                      data-testid="eval-retry"
                      disabled={!evaluationAvailable || evaluationBusy}
                      onClick={() => void runEvaluation()}
                    >
                      Retry evaluation
                    </button>
                  )}
                </div>
                <pre className="eval-pre" data-testid="eval-project-log">
                  {selectedProject.log || "No log was captured."}
                </pre>
              </section>
            )}

            {evaluation.log && (
              <details className="eval-log" data-testid="eval-log">
                <summary>Evaluation log</summary>
                <pre className="eval-pre">{evaluation.log}</pre>
              </details>
            )}
        </aside>

      </div>

      {bottomOpen && (
        <>
      <Splitter label="Output panel height" value={visibleOutputHeight} min={120} max={outputMax} horizontal onChange={setOutputHeight} />
      <BottomToolWindow tabs={bottomTabLabels} active={bottomTab} onSelect={setBottomTab} onHide={() => setLayout((current) => ({ ...current, bottom: { ...current.bottom, open: false } }))} panelRef={panelBodyRef}>
        {renderBottomContent()}
      </BottomToolWindow>
        </>
      )}

      {findActionOpen && (
        <FindAction
          key={`${searchTab}:${searchSeq}`}
          actions={resolvedActions}
          platform={snapshot.app.platform}
          profile={settings.keymap.profile}
          initialTab={searchTab}
          onExecute={(action) => {
            setFindActionOpen(false);
            focusEditor();
            if (!action.disabledReason) action.execute();
          }}
          onOpenFile={openSearchFile}
          onNavigate={openSearchLocation}
          onClose={closeSearch}
        />
      )}

      {findInFiles && (
        <FindInFiles
          key={findInFiles.nonce}
          scope={findInFiles.scope}
          onClose={() => {
            setFindInFiles(null);
            focusEditor();
          }}
          onNavigate={(path, line, character) => {
            setFindInFiles(null);
            focusEditorAfterOpen(() => navigateTo(path, { line, character }));
          }}
        />
      )}

      {recentFilesOpen && (
        <RecentFilesDialog
          files={recentFiles}
          onOpen={(path) => {
            setRecentFilesOpen(false);
            focusEditorAfterOpen(() => openUserFile(path));
          }}
          onClose={() => {
            setRecentFilesOpen(false);
            focusEditor();
          }}
        />
      )}

      {keymapOpen && (
        <KeymapDialog
          actions={resolvedActions}
          keymap={settings.keymap}
          platform={snapshot.app.platform}
          onChange={saveKeymap}
          onClose={() => setKeymapOpen(false)}
        />
      )}

      {settingsOpen && (
        <SettingsDialog
          actions={resolvedActions}
          settings={settings}
          platform={snapshot.app.platform}
          snapshot={snapshot}
          onChange={updateSettings}
          onSave={saveSettings}
          onCancel={cancelSettings}
          onRedetect={async () => {
            onSnapshot(await call<Snapshot>("sdk.redetect"));
          }}
        />
      )}

      {quickListOpen && (
        <TestsQuickList items={testQuickItems} onClose={() => setQuickListOpen(false)} />
      )}

      <StatusBar
        workspace={workspace}
        snapshot={snapshot}
        onSnapshot={onSnapshot}
        sdkSummary={sdkSummary}
        evaluation={{ ...evaluationStatus, reevaluate: evaluationRun }}
        partial={partialWorkspace}
        restore={{
          state: restore.state,
          label: restoreStateLabel(restore),
          onCancel: () => void cancelRestore(),
          onRetry: restoreAvailable ? () => void startRestore("restore.retry") : null,
        }}
        dirtyCount={dirtyCount}
        diagnostics={{ ...diagnosticCounts, onOpen: () => setBottomTab("problems") }}
        analysis={analysisStatus}
        breadcrumb={statusBreadcrumb}
        cursor={active ? `Ln ${cursor.line + 1}, Col ${cursor.character + 1}` : "No file open"}
        indent={{
          label: settings.editor.useTabs
            ? `Tabs (${settings.editor.indentSize})`
            : `${settings.editor.indentSize} spaces`,
          title: "Indentation comes from Settings (Editor). No per-file detection is claimed.",
        }}
        keymapProfile={settings.keymap.profile}
      />

      {buildDecision && (
        <div className="modal-backdrop">
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="build-decision-title"
            data-testid="build-dirty-dialog"
            onKeyDown={(event) => {
              if (event.key === "Escape") setBuildDecision(false);
            }}
          >
            <h2 id="build-decision-title">Build with unsaved buffers?</h2>
            <p>
              {countLabel(dirtyCount, "modified buffer")}{" "}
              {dirtyCount === 1 ? "is" : "are"} not saved. A build reads saved disk content;
              OpenRidr never saves or overwrites a buffer silently.
            </p>
            <div className="actions">
              <button data-testid="build-save-and-start" onClick={() => void saveAllAndBuild()}>
                Save all and build
              </button>
              <button data-testid="build-saved-only" onClick={() => void buildSavedContent()}>
                Build saved content ({dirtyCount} unsaved buffer{dirtyCount === 1 ? "" : "s"}{" "}
                excluded)
              </button>
              <button data-testid="build-decision-cancel" onClick={() => setBuildDecision(false)}>
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {runDecision && (
        <div className="modal-backdrop">
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="run-decision-title"
            data-testid="run-dirty-dialog"
            onKeyDown={(event) => {
              if (event.key === "Escape") setRunDecision(false);
            }}
          >
            <h2 id="run-decision-title">Run the saved build with unsaved buffers?</h2>
            <p>
              {countLabel(dirtyCount, "modified buffer")}{" "}
              {dirtyCount === 1 ? "is" : "are"} not saved. Running the saved build;{" "}
              {dirtyCount} unsaved buffer{dirtyCount === 1 ? "" : "s"}{" "}
              {dirtyCount === 1 ? "is" : "are"} not included. OpenRidr never saves or
              overwrites a buffer silently.
            </p>
            <div className="actions">
              <button data-testid="run-saved-build" onClick={runSavedBuild}>
                Run saved build
              </button>
              <button data-testid="run-save-rebuild" onClick={() => void saveAllAndRebuild()}>
                Save all and rebuild
              </button>
              <button data-testid="run-decision-cancel" onClick={() => setRunDecision(false)}>
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {runConfigOpen && (
        <RunConfigDialog
          evaluation={evaluation}
          configs={runConfigStore?.configs ?? []}
          defaultConfig={defaultRunConfig}
          activeId={runConfigStore?.activeId ?? DEFAULT_RUN_CONFIG_ID}
          issues={runConfigStore?.issues ?? []}
          initialId={runConfigInitialId}
          onCommit={commitRunConfigDialog}
          onRun={(config) => requestRun(config)}
          onClose={() => setRunConfigOpen(false)}
        />
      )}

      {namePrompt && (
        <div className="modal-backdrop">
          <form
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="tree-name-title"
            data-testid="tree-name-dialog"
            onSubmit={(event) => void submitNamePrompt(event)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                setNamePrompt(null);
              }
            }}
          >
            <h2 id="tree-name-title">
              {namePrompt.kind === "create" ? "New File" : "Rename"}
            </h2>
            <p className="muted">
              {namePrompt.kind === "create"
                ? `Create a file in ${namePrompt.path || "the workspace root"}.`
                : `Rename ${namePrompt.path}.`}
            </p>
            <label className="rename-field">
              Name
              <input
                data-testid="tree-name-input"
                value={namePrompt.value}
                autoFocus
                onFocus={(event) => event.currentTarget.select()}
                onChange={(event) =>
                  setNamePrompt({ ...namePrompt, value: event.target.value, error: null })
                }
              />
            </label>
            {namePrompt.error && (
              <p className="error" role="alert" data-testid="tree-name-error">
                {namePrompt.error}
              </p>
            )}
            <div className="actions">
              <button
                className="primary"
                type="submit"
                data-testid="tree-name-confirm"
                disabled={namePrompt.busy}
              >
                {namePrompt.busy
                  ? "Working…"
                  : namePrompt.kind === "create"
                    ? "Create"
                    : "Rename"}
              </button>
              <button type="button" data-testid="tree-name-cancel" onClick={() => setNamePrompt(null)}>
                Cancel
              </button>
            </div>
          </form>
        </div>
      )}

      {deletePrompt && (
        <div className="modal-backdrop">
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="tree-delete-title"
            data-testid="tree-delete-dialog"
            onKeyDown={(event) => {
              if (event.key === "Escape") setDeletePrompt(null);
            }}
          >
            <h2 id="tree-delete-title">
              Delete {deletePrompt.directory ? "folder" : "file"}?
            </h2>
            <p>
              {deletePrompt.path} is removed from disk
              {deletePrompt.directory ? " with everything inside it" : ""}. This cannot be
              undone.
            </p>
            {deletePrompt.error && (
              <p className="error" role="alert" data-testid="tree-delete-error">
                {deletePrompt.error}
              </p>
            )}
            <div className="actions">
              <button autoFocus data-testid="tree-delete-cancel" onClick={() => setDeletePrompt(null)}>
                Cancel
              </button>
              <button
                className="primary"
                data-testid="tree-delete-confirm"
                disabled={deletePrompt.busy}
                onClick={() => void confirmDelete()}
              >
                {deletePrompt.busy ? "Deleting…" : "Delete"}
              </button>
            </div>
          </div>
        </div>
      )}

      {confirm && (
        <div className="modal-backdrop">
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="confirm-title"
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                keepEditing();
              }
              if (event.key === "Tab") {
                const focusable = Array.from(
                  event.currentTarget.querySelectorAll<HTMLElement>("button"),
                );
                const first = focusable[0];
                const last = focusable[focusable.length - 1];
                if (!first || !last) return;
                if (event.shiftKey && document.activeElement === first) {
                  event.preventDefault();
                  last.focus();
                } else if (!event.shiftKey && document.activeElement === last) {
                  event.preventDefault();
                  first.focus();
                }
              }
            }}
          >
            <h2 id="confirm-title">
              {confirm.kind === "tab"
                ? "Close modified file?"
                : confirm.kind === "workspace"
                  ? "Close workspace with unsaved changes?"
                  : "Close OpenRidr with unsaved changes?"}
            </h2>
            <p>
              {confirm.kind === "tab"
                ? `${confirm.path} has unsaved changes. Save it, keep editing, or discard it explicitly.`
                : `${countLabel(dirtyCount, "modified buffer")} ${
                    dirtyCount === 1 ? "has" : "have"
                  } unsaved changes. Save all, keep editing, or discard explicitly.`}
            </p>
            {confirmError && (
              <p className="error" role="alert" data-testid="confirm-error">
                {confirmError}
              </p>
            )}
            <div className="actions">
              <button className="primary" data-testid="confirm-save" onClick={() => void confirmSave()}>
                {confirm.kind === "tab" ? "Save" : "Save all"}
              </button>
              <button autoFocus data-testid="confirm-keep" onClick={keepEditing}>
                Keep editing
              </button>
              <button
                data-testid="confirm-discard"
                onClick={discardConfirmed}
              >
                <Icon name="problems" /> {confirm.kind === "tab"
                  ? "Discard and close"
                  : confirm.kind === "workspace"
                    ? "Discard and close workspace"
                    : "Discard and close app"}
              </button>
            </div>
          </div>
        </div>
      )}

      {treeMenu && (
        <EditorContextMenu
          compact
          testId="tree-context-menu"
          itemPrefix="tree-menu"
          label={treeMenuLabel(treeMenu.target)}
          x={treeMenu.x}
          y={treeMenu.y}
          items={treeMenuItems(treeMenu.target)}
          onClose={closeTreeMenu}
        />
      )}

      {contextMenu && (
        <EditorContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={contextMenuItems(contextMenu)}
          onClose={closeContextMenu}
        />
      )}

      {gitDiffView && (
        <div className="modal-backdrop">
          <div
            className="modal git-diff-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="git-diff-title"
            data-testid="editor-git-diff"
            onKeyDown={(event) => {
              if (event.key === "Escape") setGitDiffView(null);
            }}
          >
            <h2 id="git-diff-title">Git diff · {gitDiffView.path}</h2>
            <p className="muted" data-testid="editor-git-diff-state">
              {gitDiffView.state === "ok"
                ? `${gitDiffView.staged ? "Staged" : "Working tree"} diff`
                : gitDiffView.state === "empty"
                  ? "No diff for this file."
                  : "Git diff failed."}
              {gitDiffView.truncated ? " · truncated by the host" : ""}
            </p>
            {gitDiffView.message && (
              <p className="error" role="alert" data-testid="editor-git-diff-message">
                {gitDiffView.message}
              </p>
            )}
            {gitDiffView.diff ? (
              <pre className="eval-pre" data-testid="editor-git-diff-content">
                {gitDiffView.diff}
              </pre>
            ) : (
              <p className="muted">Nothing to show; OpenRidr did not invent a diff.</p>
            )}
            <div className="actions">
              <button
                autoFocus
                data-testid="editor-git-diff-close"
                onClick={() => setGitDiffView(null)}
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}
