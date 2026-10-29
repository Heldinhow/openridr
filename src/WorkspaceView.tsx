import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import {
  call,
  HostCallError,
  type CodeAction,
  type CodeActionsResponse,
  type EvaluationProject,
  type RenameResponse,
  type RunConfig,
  type RunConfigStore,
  type Snapshot,
  type OutlineProject,
} from "./host/client";
import { CodeEditor } from "./CodeEditor";
import { ProblemsPanel } from "./ProblemsPanel";
import { BuildPanel, isBuildObsolete, runShortcutLabel } from "./BuildPanel";
import {
  TestsPanel,
  failedTestIds,
  rerunFailedReason,
  testDebugUnavailable,
  testRunBlockedReason,
} from "./TestsPanel";
import { TestsQuickList } from "./TestsQuickList";
import { TrustControls } from "./TrustControls";
import { ProcessPanel } from "./ProcessPanel";
import { DebugAdapterPanel, LspPanel, ToolInstaller } from "./ToolchainPanels";
import { DebugPanel } from "./DebugPanel";
import { TerminalPanel } from "./TerminalPanel";
import { Icon } from "./Icon";
import { Splitter } from "./Splitter";
import { BottomToolWindow } from "./shell/BottomToolWindow";
import { StatusBar } from "./shell/StatusBar";
import { ToolWindowBand } from "./shell/ToolWindowBand";
import { FindAction, type SearchTab } from "./actions/FindAction";
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
  type GitIntent,
  type GitStatus,
} from "./git";
import { NuGetPanel } from "./NuGetPanel";
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
  savedRevision: number;
  conflict: null | { content: string; hash: string };
};

type RenamePrompt = {
  path: string;
  line: number;
  character: number;
  revision: number;
  session: number;
  contextId: number;
  workspaceId: number;
  busy: boolean;
  error: string | null;
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
  | { kind: "window" };

type Props = {
  snapshot: Snapshot;
  initialFile: string | null;
  recentSolutions: string[];
  onOpenRecent: (path: string) => Promise<void>;
  onSnapshot: (snapshot: Snapshot) => void;
  onClosed: (snapshot: Snapshot) => void;
};

const bottomTools = toolWindowsIn("bottom");

function baseName(path: string) {
  return path.split("/").pop() ?? path;
}

function messageOf(error: unknown) {
  return error instanceof HostCallError ? error.message : String(error);
}

function isDirty(tab: Tab) {
  return tab.revision !== tab.savedRevision || tab.conflict !== null;
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

function evaluationStateLabel(state: Snapshot["evaluation"]["state"]) {
  switch (state) {
    case "running":
      return "Evaluating…";
    case "ok":
      return "Evaluated";
    case "partial":
      return "Evaluated with errors";
    case "failed":
      return "Evaluation failed";
    default:
      return "Not evaluated";
  }
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

export function WorkspaceView({ snapshot, initialFile, recentSolutions, onOpenRecent, onSnapshot, onClosed }: Props) {
  const workspace = snapshot.workspace!;
  const projects = snapshot.outline?.projects ?? [];
  const trusted = snapshot.trust.granted;
  const projectSdk = snapshot.sdk.project;
  const evaluation = snapshot.evaluation;
  const evaluatedProjects = evaluation.projects;
  const showsEvaluatedTree =
    evaluation.state !== "idle" && evaluatedProjects.length > 0;
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activePath, setActivePath] = useState<string | null>(null);
  const [message, setMessage] = useState<{ tone: "info" | "error"; text: string } | null>(null);
  const [confirm, setConfirm] = useState<Confirmation | null>(null);
  const [confirmError, setConfirmError] = useState<string | null>(null);
  const [layout, setLayout] = useState<ShellLayout>(loadShellLayout);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [settingsPath, setSettingsPath] = useState<string | null>(null);
  const [findActionOpen, setFindActionOpen] = useState(false);
  const [searchTab, setSearchTab] = useState<SearchTab | "last">("last");
  const [keymapOpen, setKeymapOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const settingsBaseline = useRef<AppSettings | null>(null);
  const [quickListOpen, setQuickListOpen] = useState(false);
  const [testProjectPath, setTestProjectPath] = useState("");
  const [testSelected, setTestSelected] = useState<Set<string>>(new Set());
  const [gitStatus, setGitStatus] = useState<GitStatus | null>(null);
  const [gitStatusError, setGitStatusError] = useState<string | null>(null);
  const [gitIntent, setGitIntent] = useState<GitIntent | null>(null);
  const refreshGit = useCallback(async () => {
    if (!trusted) {
      setGitStatus(null);
      setGitStatusError(null);
      return;
    }
    try {
      setGitStatus(await call<GitStatus>("git.status"));
      setGitStatusError(null);
    } catch (error) {
      setGitStatusError(messageOf(error));
    }
  }, [trusted]);
  useEffect(() => {
    // Refresh on workspace/trust changes; tool windows refresh on open and after operations.
    void refreshGit();
  }, [refreshGit, workspace.id]);
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
  const [viewportHeight, setViewportHeight] = useState(window.innerHeight);
  const [projectDetailsOpen, setProjectDetailsOpen] = useState(false);
  const [editorTarget, setEditorTarget] = useState<{
    path: string;
    line: number;
    character: number;
    nonce: number;
  } | null>(null);
  const outputMax = Math.max(120, Math.min(500, Math.floor(viewportHeight * 0.42)));
  const visibleOutputHeight = Math.min(outputHeight, outputMax);
  useEffect(() => {
    const resize = () => setViewportHeight(window.innerHeight);
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);
  useEffect(() => {
    saveShellLayout(layout);
  }, [layout]);
  useEffect(() => {
    let active = true;
    call<{ path: string; settings: unknown }>("settings.read")
      .then((result) => {
        if (!active) return;
        setSettingsPath(result.path);
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

  useEffect(() => {
    // Poll the host so process states and appending trust decisions are not stale. The
    // interval is local to this screen and cheap against the local host.
    const timer = setInterval(() => {
      call<Snapshot>("workspace.state")
        .then(onSnapshot)
        .catch(() => {});
    }, 400);
    return () => clearInterval(timer);
  }, []);
  const [editedAt, setEditedAt] = useState<Record<string, number>>({});
  const [savedAt, setSavedAt] = useState<Record<string, number>>({});
  const navigationSeq = useRef(0);
  const tabsRef = useRef(tabs);
  tabsRef.current = tabs;
  const activePathRef = useRef(activePath);
  activePathRef.current = activePath;
  const unsavedRef = useRef(false);
  const writing = useRef(new Set<string>());
  const diskEpoch = useRef(new Map<string, number>());
  const [selectedProjectPath, setSelectedProjectPath] = useState<string | null>(null);
  const [startupPath, setStartupPath] = useState<string | null>(null);
  const [runConfigStore, setRunConfigStore] = useState<RunConfigStore | null>(null);
  const [runConfigsLoaded, setRunConfigsLoaded] = useState(false);
  const [runConfigOpen, setRunConfigOpen] = useState(false);
  const [runConfigInitialId, setRunConfigInitialId] = useState(DEFAULT_RUN_CONFIG_ID);
  const [buildDecision, setBuildDecision] = useState(false);
  const [runDecision, setRunDecision] = useState(false);
  const [evaluating, setEvaluating] = useState(false);
  const autoEvaluation = useRef<number | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(
    () => new Set(projects.map((project) => project.path)),
  );
  const [renamePrompt, setRenamePrompt] = useState<RenamePrompt | null>(null);
  const [editPreview, setEditPreview] = useState<EditPreview | null>(null);
  const [actionList, setActionList] = useState<ActionList | null>(null);
  const [renameName, setRenameName] = useState("");
  const renameSeq = useRef(0);
  const actionSeq = useRef(0);
  const cursorRef = useRef({ line: 0, character: 0 });
  const [cursor, setCursor] = useState({ line: 0, character: 0 });
  const editPreviewHeading = useRef<HTMLHeadingElement | null>(null);
  const actionListHeading = useRef<HTMLHeadingElement | null>(null);

  const active = tabs.find((tab) => tab.path === activePath) ?? null;
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
  const evaluationAvailable = trusted && projectSdk.state === "resolved";
  const evaluationBusy = evaluating || evaluation.state === "running";
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
  const lspSent = useRef<{ session: number; paths: Set<string> }>({
    session: 0,
    paths: new Set(),
  });
  const lspChangeTimer = useRef<number | null>(null);
  const lspPending = useRef<Map<string, string>>(new Map());
  const restoreDisabledReason = !trusted
    ? "Trust the workspace before restoring; nothing runs in Restricted mode."
    : projectSdk.state !== "resolved"
      ? "Resolve the project SDK before restoring; the host SDK is never substituted."
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
  const defaultStartupProject =
    consoleCandidates.find((project) => project.path === startupPath) ??
    (consoleCandidates.length === 1 ? consoleCandidates[0] : null);
  const activeNamedConfig =
    runConfigStore && runConfigStore.activeId !== DEFAULT_RUN_CONFIG_ID
      ? runConfigStore.configs.find((config) => config.id === runConfigStore.activeId) ?? null
      : null;
  const startupProject = activeNamedConfig
    ? consoleCandidates.find((project) => project.path === activeNamedConfig.project) ?? null
    : defaultStartupProject;
  const defaultRunConfig = deriveDefaultRunConfig(defaultStartupProject, evaluation);
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
  const buildDisabledReason = !trusted
    ? "Trust the workspace before building; nothing runs in Restricted mode."
    : projectSdk.state !== "resolved"
      ? "Resolve the project SDK before building; the host SDK is never substituted."
      : evaluation.state === "running"
        ? "Wait for the evaluation to finish before building."
        : evaluation.stale
          ? "The evaluated project model is stale; re-evaluate before building."
          : !["ok", "partial"].includes(evaluation.state)
            ? "Evaluate the project model before building; the build uses its TargetPath."
            : !activeConfigEvaluated
              ? "The active run configuration has not been evaluated yet; re-evaluate the project model."
              : restoreIncomplete
              ? "Dependencies are not restored; run the explicit restore first."
              : consoleCandidates.length === 0
                ? "No console executable project is available in the current evaluation; libraries and test projects cannot be startup projects."
                : activeNamedConfig && !consoleCandidates.some((project) => project.path === activeNamedConfig.project)
                  ? "The active run configuration selects a project that is not an evaluated console project; edit the configuration."
                  : !startupProject
                    ? "More than one console project; choose the startup project explicitly."
                    : buildRunning
                      ? "A build is already running."
                      : null;
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
      ? "Resolve the project SDK before discovering tests; the host SDK is never substituted."
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
  const runShortcut = runShortcutLabel(snapshot.app.platform);
  const buildReference = build.finishedAt ?? build.startedAt ?? 0;
  const savedSourceAfterBuild = Object.entries(savedAt).some(
    ([path, saved]) => path.endsWith(".cs") && saved > buildReference,
  );
  const runDisabledReason = !trusted
    ? "Trust the workspace before running; nothing runs in Restricted mode."
    : projectSdk.state !== "resolved"
      ? "Resolve the project SDK before running; the host SDK is never substituted."
      : evaluation.state === "running"
        ? "Wait for the evaluation to finish before running."
        : evaluation.stale
          ? "The evaluated project model is stale; re-evaluate before running."
          : !["ok", "partial"].includes(evaluation.state)
            ? "Evaluate the project model before running; Run uses the saved build's TargetPath."
            : !activeConfigEvaluated
              ? "The active run configuration has not been evaluated yet; re-evaluate the project model."
              : consoleCandidates.length === 0
              ? "No console executable project is available in the current evaluation; libraries and test projects cannot be run."
              : activeNamedConfig && !consoleCandidates.some((project) => project.path === activeNamedConfig.project)
                ? "The active run configuration selects a project that is not an evaluated console project; edit the configuration."
                : !startupProject
                  ? "More than one console project; choose the startup project explicitly."
                : buildRunning
                  ? "A build is running; wait for it to finish before running."
                  : runRunning
                    ? "A program is already running."
                    : build.revision === 0
                      ? "Build the startup project first; Run never builds or restores implicitly."
                      : build.sealed
                        ? "The saved build belongs to a previous context; rebuild before running."
                        : build.stale
                          ? `The saved build is stale: ${build.staleReason ?? "no current build"}.`
                          : build.state !== "ok" || build.exit !== 0
                            ? `The last build did not succeed (${
                                build.exit === null ? "exit unknown" : `exit ${build.exit}`
                              }); fix it and rebuild.`
                            : build.project?.path !== startupProject.path
                              ? "The saved build is for another startup project; build the selected one."
                              : savedSourceAfterBuild
                                ? "Sources were saved after this build; rebuild before running."
                                : null;
  const dap = snapshot.dap;
  const debugActive = ["preparing", "running", "paused"].includes(dap.state);
  const debugDisabledReason = !trusted
    ? "Trust the workspace before debugging; nothing runs in Restricted mode."
    : snapshot.debug.adapterState !== "ready"
      ? `The debug adapter is not ready: ${snapshot.debug.gateReason ?? snapshot.debug.adapterMessage}`
      : snapshot.app.platform === "macos" &&
          snapshot.app.arch === "arm64" &&
          !snapshot.debug.appleSiliconExperimental
        ? "Enable the Apple Silicon experimental flag before debugging; nothing was launched."
        : projectSdk.state !== "resolved"
          ? "Resolve the project SDK before debugging; the host SDK is never substituted."
          : evaluation.state === "running"
            ? "Wait for the evaluation to finish before debugging."
            : evaluation.stale
              ? "The evaluated project model is stale; re-evaluate before debugging."
              : !["ok", "partial"].includes(evaluation.state)
                ? "Evaluate the project model before debugging; Debug uses its managed assembly."
                : !activeConfigEvaluated
                  ? "The active run configuration has not been evaluated yet; re-evaluate the project model."
                  : consoleCandidates.length === 0
                  ? "No console executable project is available in the current evaluation."
                  : activeNamedConfig && !consoleCandidates.some((project) => project.path === activeNamedConfig.project)
                    ? "The active run configuration selects a project that is not an evaluated console project; edit the configuration."
                    : !startupProject
                      ? "More than one console project; choose the startup project explicitly."
                    : buildRunning
                      ? "A build is running; wait for it to finish before debugging."
                      : build.revision === 0
                        ? "Build the startup project in Debug first; Debug never builds or restores implicitly."
                        : build.configuration.toLowerCase() !== "debug"
                          ? "The saved build is not a Debug build; build with Debug before debugging."
                          : build.sealed
                            ? "The saved build belongs to a previous context; rebuild before debugging."
                            : build.stale
                              ? `The saved build is stale: ${build.staleReason ?? "no current build"}.`
                              : build.state !== "ok" || build.exit !== 0
                                ? `The last build did not succeed (${
                                    build.exit === null ? "exit unknown" : `exit ${build.exit}`
                                  }); fix it and rebuild.`
                                : build.project?.path !== startupProject.path
                                  ? "The saved build is for another startup project; build the selected one."
                                  : savedSourceAfterBuild
                                    ? "Sources were saved after this build; rebuild before debugging."
                                    : debugActive
                                      ? "A debug session is already active."
                                      : null;
  const debugCurrentFrame =
    dap.frames.find((frame) => frame.id === dap.selectedFrameId) ?? dap.frames[0] ?? null;
  const activeBreakpoints = active
    ? dap.breakpoints
        .filter((breakpoint) => breakpoint.path === active.path)
        .map((breakpoint) => breakpoint.line)
    : [];
  const debugCurrentLine =
    active && debugCurrentFrame?.path
      ? debugCurrentFrame.path.replace(/\\/g, "/").endsWith(`/${active.path}`)
        ? debugCurrentFrame.line
        : null
      : null;

  useEffect(() => {
    // The active configuration is the source of truth, so the first evaluation waits for the
    // per-solution store and uses its Configuration/TFM instead of a second default.
    if (!runConfigsLoaded) return;
    if (!evaluationAvailable || evaluation.state !== "idle" || evaluation.staleReason) {
      return;
    }
    if (autoEvaluation.current === snapshot.contextId) {
      return;
    }
    autoEvaluation.current = snapshot.contextId;
    void evaluateContext(
      activeNamedConfig?.configuration ?? evaluation.context.configuration,
      activeNamedConfig?.targetFramework ?? evaluation.context.targetFramework,
    );
  }, [
    evaluationAvailable,
    evaluation.state,
    evaluation.staleReason,
    snapshot.contextId,
    runConfigsLoaded,
  ]);

  useEffect(() => {
    if (evaluation.revision === 0) return;
    setExpanded(new Set(evaluatedProjects.map((project) => project.path)));
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
    const timer = setInterval(() => {
      call<Snapshot>("workspace.state")
        .then(onSnapshot)
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
    if (!renamePrompt) return;
    if (activePath !== renamePrompt.path) {
      renameSeq.current += 1;
      setRenamePrompt(null);
      setMessage({ tone: "info", text: "Rename cancelled: another file became active." });
      return;
    }
    const tab = tabs.find((item) => item.path === renamePrompt.path);
    if (!tab || tab.revision !== renamePrompt.revision || tab.conflict) {
      renameSeq.current += 1;
      setRenamePrompt(null);
      setMessage({
        tone: "error",
        text: "Rename prompt discarded: the source buffer changed; nothing was requested.",
      });
    }
  }, [activePath, tabs, renamePrompt]);

  useEffect(() => {
    if (editPreview) editPreviewHeading.current?.focus();
  }, [editPreview ? previewId(editPreview) : null]);

  useEffect(() => {
    if (!editPreview) return;
    if (activePath !== editPreview.sourcePath) {
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

  async function openFile(path: string) {
    if (tabs.some((tab) => tab.path === path)) {
      setActivePath(path);
      void checkDisk(path);
      return true;
    }
    try {
      const file = await call<{ path: string; content: string; hash: string }>("file.read", {
        path,
      });
      setTabs((current) => [
        ...current,
        {
          path: file.path,
          content: file.content,
          hash: file.hash,
          revision: 0,
          savedRevision: 0,
          conflict: null,
        },
      ]);
      setActivePath(file.path);
      setMessage(null);
      return true;
    } catch (error) {
      setMessage({ tone: "error", text: `Could not open ${path}: ${messageOf(error)}` });
      return false;
    }
  }

  async function navigateTo(path: string, position: { line: number; character: number }) {
    if (!(await openFile(path))) return;
    navigationSeq.current += 1;
    setEditorTarget({
      path,
      line: position.line,
      character: position.character,
      nonce: navigationSeq.current,
    });
  }

  function bumpDiskEpoch(path: string) {
    diskEpoch.current.set(path, (diskEpoch.current.get(path) ?? 0) + 1);
  }

  async function checkDisk(path: string) {
    // Proactive read. It only raises a conflict; it never replaces buffer content. A read
    // failure is reported and the buffer stays editable.
    if (writing.current.has(path) || !tabsRef.current.some((tab) => tab.path === path)) return;
    const epoch = diskEpoch.current.get(path) ?? 0;
    let file: { content: string; hash: string };
    try {
      file = await call<{ content: string; hash: string }>("file.read", { path });
    } catch (error) {
      setMessage({
        tone: "error",
        text: `Could not re-read ${path} from disk; your buffer is unchanged. ${messageOf(error)}`,
      });
      return;
    }
    if ((diskEpoch.current.get(path) ?? 0) !== epoch) return;
    setTabs((current) =>
      current.map((tab) => {
        if (tab.path !== path) return tab;
        const known = tab.conflict?.hash ?? tab.hash;
        if (file.hash === known) return tab;
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
    if (tab.revision === tab.savedRevision) {
      setMessage({ tone: "info", text: "No changes to save." });
      return { ok: true };
    }
    const content = tab.content;
    const revision = tab.revision;
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
            ? { ...item, hash: result.hash, savedRevision: revision, conflict: null }
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
      setMessage({ tone: "error", text: `Rename unavailable: ${renameDisabledReason}` });
      return;
    }
    renameSeq.current += 1;
    setRenameName("");
    setMessage(null);
    setRenamePrompt({
      path: active.path,
      line: position.line,
      character: position.character,
      revision: active.revision,
      session: lsp.session,
      contextId: snapshot.contextId,
      workspaceId: workspace.id,
      busy: false,
      error: null,
    });
  }

  async function requestCodeActions(position = cursorRef.current) {
    if (!active) return;
    if (codeActionsDisabledReason) {
      setMessage({ tone: "error", text: `Code actions unavailable: ${codeActionsDisabledReason}` });
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

  function cancelRenamePrompt() {
    renameSeq.current += 1;
    setRenamePrompt(null);
    setMessage({ tone: "info", text: "Rename cancelled; no request was kept." });
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

  async function submitRename() {
    const prompt = renamePrompt;
    if (!prompt || prompt.busy) return;
    const name = renameName.trim();
    if (!name) {
      setRenamePrompt({ ...prompt, error: "Enter the new name." });
      return;
    }
    const sequence = ++renameSeq.current;
    setRenamePrompt({ ...prompt, busy: true, error: null });
    if (!lspActiveRef.current) {
      setRenamePrompt((current) =>
        current
          ? {
              ...current,
              busy: false,
              error: "Analysis is not running; no rename was requested.",
            }
          : current,
      );
      return;
    }
    // Analysis must see the buffer the preview is based on; a pending debounce would let
    // csharp-ls compute the rename from older text.
    const synchronized = await flushPendingChanges();
    if (sequence !== renameSeq.current) return;
    if (!synchronized) {
      setRenamePrompt((current) =>
        current
          ? {
              ...current,
              busy: false,
              error:
                "Could not send the current buffer to analysis; no rename was requested.",
            }
          : current,
      );
      return;
    }
    const tab = tabsRef.current.find((item) => item.path === prompt.path);
    const live = lspViewRef.current;
    if (!tab || tab.revision !== prompt.revision || tab.conflict || live.session !== prompt.session) {
      setRenamePrompt(null);
      setMessage({
        tone: "error",
        text: "Rename discarded: the document or analysis session changed while the prompt was open.",
      });
      return;
    }
    let response: RenameResponse;
    try {
      response = await call<RenameResponse>("lsp.rename", {
        path: prompt.path,
        line: prompt.line,
        character: prompt.character,
        newName: name,
        revision: prompt.revision,
      });
    } catch (error) {
      if (sequence !== renameSeq.current) return;
      setRenamePrompt((current) =>
        current
          ? { ...current, busy: false, error: `Rename unavailable: ${messageOf(error)}` }
          : current,
      );
      return;
    }
    if (sequence !== renameSeq.current) return;
    const currentTab = tabsRef.current.find((item) => item.path === prompt.path);
    const liveAfter = lspViewRef.current;
    if (
      !currentTab ||
      currentTab.revision !== prompt.revision ||
      currentTab.conflict ||
      liveAfter.session !== prompt.session ||
      contextIdRef.current !== prompt.contextId ||
      workspaceIdRef.current !== prompt.workspaceId
    ) {
      setRenamePrompt(null);
      setMessage({
        tone: "error",
        text: "Rename response discarded: the document, workspace or analysis session changed while csharp-ls was answering; nothing was applied.",
      });
      return;
    }
    if (response.state !== "ok" || response.documentCount === 0) {
      setRenamePrompt((current) =>
        current
          ? {
              ...current,
              busy: false,
              error: `Rename ${response.state}: ${
                response.message ?? "csharp-ls returned no edits."
              }`,
            }
          : current,
      );
      return;
    }
    const tabRevisions: Record<string, number> = {};
    for (const tab of tabsRef.current) tabRevisions[tab.path] = tab.revision;
    setRenamePrompt(null);
    setEditPreview({
      kind: "rename",
      response,
      sourcePath: prompt.path,
      sourceRevision: prompt.revision,
      session: prompt.session,
      contextId: prompt.contextId,
      workspaceId: prompt.workspaceId,
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
  }

  function cancelEditPreview() {
    const preview = editPreview;
    setEditPreview(null);
    setMessage({
      tone: "info",
      text: `${preview ? previewKindLabel(preview) : "Preview"} cancelled; no buffer or file changed.`,
    });
  }

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
    const label = preview.kind === "rename" ? "Rename" : "Code action";
    const fail = (reason: string) => {
      setEditPreview((current) =>
        current && previewId(current) === id
          ? { ...current, applying: false, stale: reason }
          : current,
      );
      setMessage({ tone: "error", text: `${label} not applied: ${reason}` });
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
        savedRevision: 0,
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
    const hadEdits = tab.revision !== tab.savedRevision;
    const revision = tab.revision + 1;
    setTabs((current) =>
      current.map((item) =>
        item.path === tab.path
          ? {
              ...item,
              content: conflict.content,
              hash: conflict.hash,
              revision,
              savedRevision: revision,
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
    const revision = tab.revision;
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
            ? { ...item, hash: result.hash, savedRevision: revision, conflict: null }
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

  useEffect(() => {
    let alive = true;
    call<RunConfigStore>("runConfigs.read", { solution: workspace.path })
      .then((store) => {
        if (alive) setRunConfigStore(store);
      })
      .catch((error) => {
        if (alive) {
          setMessage({
            tone: "error",
            text: `Run configurations could not be read: ${messageOf(error)}`,
          });
        }
      })
      .finally(() => {
        if (alive) setRunConfigsLoaded(true);
      });
    return () => {
      alive = false;
    };
  }, [workspace.path]);

  async function commitRunConfigs(configs: RunConfig[], activeId: string) {
    const store = await call<RunConfigStore>("runConfigs.write", {
      solution: workspace.path,
      activeId,
      configs,
    });
    setRunConfigStore(store);
    return store;
  }

  async function evaluateContext(
    configuration: string,
    targetFramework: string | null,
  ) {
    setEvaluating(true);
    try {
      onSnapshot(
        await call<Snapshot>("evaluation.run", { configuration, targetFramework }),
      );
      setMessage(null);
    } catch (error) {
      setMessage({ tone: "error", text: `Evaluation could not run: ${messageOf(error)}` });
      try {
        onSnapshot(await call<Snapshot>("workspace.state"));
      } catch {
        // The message above already explains the failure.
      }
    } finally {
      setEvaluating(false);
    }
  }

  async function runEvaluation(next?: {
    configuration?: string;
    targetFramework?: string | null;
  }) {
    const configuration = next?.configuration ?? evaluation.context.configuration;
    const targetFramework =
      next && "targetFramework" in next
        ? next.targetFramework ?? null
        : evaluation.context.targetFramework;
    if (
      activeNamedConfig &&
      (activeNamedConfig.configuration !== configuration ||
        (activeNamedConfig.targetFramework ?? null) !== targetFramework)
    ) {
      // Every toolbar/panel Configuration or TFM change writes the active configuration;
      // the evaluation follows it so there is a single source of truth.
      const configs = (runConfigStore?.configs ?? []).map((config) =>
        config.id === activeNamedConfig.id
          ? { ...config, configuration, targetFramework }
          : config,
      );
      try {
        await commitRunConfigs(configs, activeNamedConfig.id);
      } catch (error) {
        setMessage({
          tone: "error",
          text: `The active run configuration could not be updated: ${messageOf(error)}`,
        });
      }
    }
    await evaluateContext(configuration, targetFramework);
  }

  function openRunConfigDialog(id?: string) {
    setRunConfigInitialId(id ?? activeRunConfig?.id ?? DEFAULT_RUN_CONFIG_ID);
    setRunConfigOpen(true);
  }

  async function selectRunConfig(id: string) {
    try {
      const store = await commitRunConfigs(runConfigStore?.configs ?? [], id);
      setMessage(null);
      const selected = store.configs.find((config) => config.id === id);
      if (
        selected &&
        (selected.configuration !== evaluation.context.configuration ||
          (selected.targetFramework ?? null) !==
            (evaluation.context.targetFramework ?? null))
      ) {
        await evaluateContext(selected.configuration, selected.targetFramework ?? null);
      }
    } catch (error) {
      setMessage({
        tone: "error",
        text: `The run configuration could not be selected: ${messageOf(error)}`,
      });
    }
  }

  async function commitRunConfigDialog(configs: RunConfig[], activeId: string) {
    const store = await commitRunConfigs(configs, activeId);
    const selected = store.configs.find((config) => config.id === activeId);
    if (
      selected &&
      (selected.configuration !== evaluation.context.configuration ||
        (selected.targetFramework ?? null) !==
          (evaluation.context.targetFramework ?? null))
    ) {
      await evaluateContext(selected.configuration, selected.targetFramework ?? null);
    }
    return store.issues;
  }

  function setStartupProject(path: string) {
    if (!activeNamedConfig) {
      setStartupPath(path);
      return;
    }
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

  function invalidRunConfigReason() {
    if (!activeRunConfig || activeRunConfigIssues.length === 0) return null;
    return `The active run configuration “${activeRunConfig.name}” cannot run: ${activeRunConfigIssues
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
    void openFile(projectSdk.globalJsonPath ?? "global.json");
  }

  async function startRestore(method: "restore.start" | "restore.retry", project?: string) {
    try {
      onSnapshot(await call<Snapshot>(method, project ? { project } : {}));
      setMessage(null);
    } catch (error) {
      setMessage({ tone: "error", text: `Restore could not start: ${messageOf(error)}` });
    }
  }

  async function cancelRestore() {
    try {
      onSnapshot(await call<Snapshot>("restore.cancel"));
    } catch (error) {
      setMessage({ tone: "error", text: `Restore cancellation failed: ${messageOf(error)}` });
    }
  }

  async function startBuild(savedNote: string) {
    if (!startupProject) {
      setMessage({
        tone: "error",
        text: "Choose a console startup project before building.",
      });
      return;
    }
    setBottomTab("build");
    try {
      onSnapshot(
        await call<Snapshot>("build.start", {
          project: startupProject.path,
          configuration: activeRunConfig?.configuration ?? evaluation.context.configuration,
          targetFramework:
            activeRunConfig?.targetFramework ?? evaluation.context.targetFramework,
          savedNote,
        }),
      );
      setMessage(null);
    } catch (error) {
      const text = `Build could not start: ${messageOf(error)}`;
      setMessage({ tone: "error", text });
      if (error instanceof HostCallError && error.code === "restore_required") {
        setBottomTab("build");
      }
    }
  }

  function requestBuild() {
    if (buildDisabledReason) {
      setMessage({ tone: "error", text: `Build unavailable: ${buildDisabledReason}` });
      return;
    }
    if (dirtyCount > 0) {
      setBuildDecision(true);
      return;
    }
    void startBuild("Built saved content; no unsaved buffers were open.");
  }

  async function saveAllAndBuild() {
    const dirty = dirtyCount;
    const failures = await saveAll();
    if (failures.length > 0) {
      setBuildDecision(false);
      setMessage({ tone: "error", text: failures.join(" ") });
      return;
    }
    setBuildDecision(false);
    await startBuild(
      `Saved all ${dirty} modified buffer${dirty === 1 ? "" : "s"} before building.`,
    );
  }

  async function buildSavedContent() {
    const excluded = dirtyCount;
    setBuildDecision(false);
    await startBuild(
      `Built saved content; ${excluded} unsaved buffer${excluded === 1 ? "" : "s"} excluded.`,
    );
  }

  async function cancelBuild() {
    try {
      onSnapshot(await call<Snapshot>("build.cancel"));
    } catch (error) {
      setMessage({ tone: "error", text: `Build cancellation failed: ${messageOf(error)}` });
    }
  }

  async function startTestDiscovery(project: string) {
    setBottomTab("tests");
    try {
      onSnapshot(
        await call<Snapshot>("tests.discover", {
          project,
          targetFramework: evaluation.context.targetFramework,
        }),
      );
      setMessage(null);
    } catch (error) {
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

  async function startRun() {
    const invalid = invalidRunConfigReason();
    if (invalid) {
      setMessage({ tone: "error", text: invalid });
      openRunConfigDialog(activeRunConfig?.id);
      return;
    }
    if (!startupProject) {
      setMessage({
        tone: "error",
        text: "Choose a console startup project before running.",
      });
      return;
    }
    setBottomTab("build");
    try {
      onSnapshot(
        await call<Snapshot>("run.start", {
          project: startupProject.path,
          configuration: activeRunConfig?.configuration ?? evaluation.context.configuration,
          targetFramework:
            activeRunConfig?.targetFramework ?? evaluation.context.targetFramework,
          args: activeRunConfig?.args ?? [],
          workingDirectory: activeRunConfig?.workingDirectory ?? "",
          env: activeRunConfig?.env ?? {},
          envFiles: activeRunConfig?.envFiles ?? [],
        }),
      );
      setMessage(null);
    } catch (error) {
      setMessage({ tone: "error", text: `Run could not start: ${messageOf(error)}` });
      openConfigOnFieldError(error);
      try {
        onSnapshot(await call<Snapshot>("workspace.state"));
      } catch {
        // The message above already explains the failure.
      }
    }
  }

  function requestRun() {
    if (runDisabledReason) {
      setMessage({ tone: "error", text: `Run unavailable: ${runDisabledReason}` });
      return;
    }
    const invalid = invalidRunConfigReason();
    if (invalid) {
      setMessage({ tone: "error", text: invalid });
      openRunConfigDialog(activeRunConfig?.id);
      return;
    }
    if (dirtyCount > 0) {
      setRunDecision(true);
      return;
    }
    void startRun();
  }

  async function saveAllAndRebuild() {
    setRunDecision(false);
    await saveAllAndBuild();
  }

  function runSavedBuild() {
    setRunDecision(false);
    void startRun();
  }

  async function cancelRun() {
    try {
      onSnapshot(await call<Snapshot>("run.cancel"));
    } catch (error) {
      setMessage({ tone: "error", text: `Run cancellation failed: ${messageOf(error)}` });
    }
  }

  async function startDebug() {
    if (debugDisabledReason) {
      setMessage({ tone: "error", text: `Debug unavailable: ${debugDisabledReason}` });
      return;
    }
    const invalid = invalidRunConfigReason();
    if (invalid) {
      setMessage({ tone: "error", text: invalid });
      openRunConfigDialog(activeRunConfig?.id);
      return;
    }
    setBottomTab("debug");
    try {
      onSnapshot(
        await call<Snapshot>("debug.start", {
          project: startupProject?.path,
          configuration: activeRunConfig?.configuration ?? evaluation.context.configuration,
          targetFramework:
            activeRunConfig?.targetFramework ?? evaluation.context.targetFramework,
          args: activeRunConfig?.args ?? [],
          workingDirectory: activeRunConfig?.workingDirectory ?? "",
          env: activeRunConfig?.env ?? {},
          envFiles: activeRunConfig?.envFiles ?? [],
        }),
      );
      setMessage(null);
    } catch (error) {
      setMessage({ tone: "error", text: `Debug could not start: ${messageOf(error)}` });
      openConfigOnFieldError(error);
      try {
        onSnapshot(await call<Snapshot>("workspace.state"));
      } catch {
        // The message above already explains the failure.
      }
    }
  }

  async function stopDebug() {
    try {
      onSnapshot(await call<Snapshot>("debug.stop"));
    } catch (error) {
      setMessage({ tone: "error", text: `Debug stop failed: ${messageOf(error)}` });
    }
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

  async function debugAction(method: "debug.continue" | "debug.stepOver" | "debug.stepInto") {
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
    } else if (runRunning) {
      void cancelRun();
    }
  }

  function openSearch(tab: SearchTab | "last") {
    setKeymapOpen(false);
    setSearchTab(tab);
    setFindActionOpen(true);
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

  function openSearchFile(path: string) {
    setFindActionOpen(false);
    focusEditor();
    void openFile(path);
  }

  function openSearchLocation(path: string, line: number, character: number) {
    setFindActionOpen(false);
    focusEditor();
    void navigateTo(path, { line, character });
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
      const result = await call<{ path: string; settings: unknown }>("settings.write", {
        settings,
      });
      setSettingsPath(result.path);
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
      const result = await call<{ path: string; settings: unknown }>("settings.write", {
        settings: next,
      });
      setSettingsPath(result.path);
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
      execute: requestBuild,
      disabledReason: buildDisabledReason,
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
      execute: () => void startDebug(),
      disabledReason: debugDisabledReason,
    }),
    define({
      id: "run.stop",
      title: "Stop",
      group: "Run",
      execute: stopExecution,
      disabledReason:
        debugActive || runRunning ? null : "Nothing is running; there is no session to stop.",
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
      execute: () => undefined,
      disabledReason: "Go to Declaration with F12 arrives in F7; Ctrl+B works in the editor.",
    }),
    define({
      id: "code.findUsages",
      title: "Find Usages",
      group: "Code",
      execute: () => undefined,
      disabledReason: "Find Usages arrives in F7.",
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
      execute: () => openSearch("last"),
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
      id: "search.text",
      title: "Find in Path",
      group: "Search",
      execute: () => openSearch("text"),
    }),
    define({
      id: "search.line",
      title: "Go to Line",
      group: "Search",
      execute: () => undefined,
      disabledReason: "Go to Line arrives in F3.",
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
        setMessage({ tone: "error", text: `Debug selected unavailable: ${testDebugUnavailable}` }),
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
      execute: () => {
        setLayout((current) => ({
          ...current,
          left: { ...current.left, active: "commit", open: true },
        }));
      },
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
  const actionsRef = useRef<ResolvedAction[]>([]);
  actionsRef.current = resolvedActions;
  const lastShift = useRef<number | null>(null);
  const chordPending = useRef<number | null>(null);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
      const target = event.target instanceof HTMLElement ? event.target : null;
      const inEditor = Boolean(target?.closest(".cm-editor"));
      const textField =
        Boolean(target?.closest("input, textarea, select")) ||
        Boolean(target?.isContentEditable && !inEditor);
      if (event.key === "Shift" && !event.repeat) {
        if (textField) {
          lastShift.current = null;
          return;
        }
        const now = performance.now();
        if (lastShift.current !== null && now - lastShift.current <= 400) {
          lastShift.current = null;
          event.preventDefault();
          openSearch("last");
          return;
        }
        lastShift.current = now;
        return;
      }
      if (textField) return;
      const profile = settings.keymap.profile;
      const pending = chordPending.current;
      chordPending.current = null;
      if (pending !== null && performance.now() - pending <= 1500) {
        const target = chordTarget(profile, event.key);
        if (target && !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey) {
          event.preventDefault();
          actionsRef.current.find((candidate) => candidate.id === target)?.execute();
          return;
        }
      }
      const prefix = chordPrefixFor(profile);
      if (prefix && matches(event, prefix, snapshot.app.platform)) {
        event.preventDefault();
        chordPending.current = performance.now();
        return;
      }
      const action = firstActionFor(actionsRef.current, event, snapshot.app.platform);
      if (!action) return;
      if (inEditor && !action.editor) return;
      // Bubble, CodeMirror first: a consumed key never reaches this handler as an action.
      event.preventDefault();
      // A disabled action still reports its own reason when reached by a shortcut; the
      // palette and menus keep it disabled.
      action.execute();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [snapshot.app.platform, settings.keymap.profile]);

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
    } else {
      void destroyWindow();
    }
  }

  function keepEditing() {
    setConfirm(null);
    setConfirmError(null);
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
    } else {
      void destroyWindow();
    }
  }

  function renderBottomContent() {
    switch (bottomTab) {
      case "build":
        return (
          <BuildPanel
            build={build}
            run={run}
            platform={snapshot.app.platform}
            dirtyCount={dirtyCount}
            runDisabledReason={runDisabledReason}
            onRun={requestRun}
            onCancelRun={() => void cancelRun()}
            onSaveAndRebuild={() => void saveAllAndRebuild()}
            evaluation={evaluation}
            startupPath={startupProject?.path ?? null}
            onStartup={setStartupProject}
            onEvaluate={(next) => void runEvaluation(next)}
            onBuild={requestBuild}
            onCancel={() => void cancelBuild()}
            disabledReason={buildDisabledReason}
            restoreRequired={restoreIncomplete}
            restoreDisabledReason={restoreDisabledReason}
            onRestore={() => void startRestore("restore.start")}
            obsolete={buildObsolete}
          />
        );
      case "problems":
        return (
          <ProblemsPanel
            diagnostics={snapshot.diagnostics}
            build={build}
            buildObsolete={buildObsolete}
            editedAt={editedAt}
            onNavigate={(path, position) => void navigateTo(path, position)}
            onOpen={(path) => void openFile(path)}
          />
        );
      case "debug":
        return (
          <div data-testid="debug-tab">
            <DebugPanel
              snapshot={snapshot}
              disabledReason={debugDisabledReason}
              onLaunch={() => void startDebug()}
              onSnapshot={onSnapshot}
              onError={(text) => setMessage({ tone: "error", text })}
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
            onNavigate={(path, position) => void navigateTo(path, position)}
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

  const sdkSummary = `Host SDK ${
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

  const bottomTabLabels = bottomTools.map(({ id, title }) => ({
    id: id as BottomToolId,
    label: id === "problems" && snapshot.diagnostics.currentItems > 0 ? `Problems (${snapshot.diagnostics.currentItems})` : title,
  }));
  const breadcrumbProject = activeEvaluationProject?.name ?? activeProject?.name ?? null;
  const statusBreadcrumb = [
    workspace.name,
    breadcrumbProject && breadcrumbProject !== workspace.name ? breadcrumbProject : null,
    active ? baseName(active.path) : null,
  ].filter((part): part is string => Boolean(part)).join(" > ");
  const evaluationStatus = showsEvaluatedTree
    ? {
        testId: "status-evaluation",
        title: `${evaluationStateLabel(evaluation.state)} · ${evaluation.context.configuration} · TFM ${evaluation.context.targetFramework ?? "not selected"} · revision ${evaluation.revision}${evaluation.stale ? " · stale" : ""}`,
        text: `${evaluationStateLabel(evaluation.state)} · Configuration ${evaluation.context.configuration} · TFM ${evaluation.context.targetFramework ?? "not selected"} · revision ${evaluation.revision}${evaluation.stale ? " · stale" : ""}`,
      }
    : { text: "Text outline · not evaluated" };

  async function openSolution(solution: string) {
    if (solution === workspace.path) return;
    if (dirtyCount > 0) {
      setMessage({ tone: "error", text: "Save or close modified buffers before opening another solution." });
      return;
    }
    try {
      await onOpenRecent(solution);
    } catch (error) {
      setMessage({ tone: "error", text: `Could not open ${solution}: ${messageOf(error)}` });
    }
  }

  const buildAction = actionById.get("build.start")!;
  const runAction = actionById.get("run.start")!;
  const debugStartAction = actionById.get("debug.start")!;
  const stopAction = actionById.get("run.stop")!;
  const searchAction = actionById.get("search.everywhere")!;
  const settingsAction = actionById.get("settings.open")!;
  const findActionAction = actionById.get("view.findAction")!;
  const keymapAction = actionById.get("view.keymap")!;
  const redetectAction = actionById.get("sdk.redetect")!;
  const openGlobalJsonAction = actionById.get("sdk.openGlobalJson")!;
  const commitAction = actionById.get("git.commit")!;
  const closeWorkspaceAction = actionById.get("workspace.close")!;
  const renameAction = actionById.get("code.rename")!;
  const contextActionsAction = actionById.get("code.contextActions")!;
  const saveAction = actionById.get("file.save")!;

  return (
    <main
      className={`workspace-view ${leftOpen ? "" : "left-closed"} ${bottomOpen ? "" : "bottom-closed"}`}
      data-density={settings.appearance.density}
      style={{
        "--sidebar-width": `${sidebarWidth}px`,
        "--output-height": `${visibleOutputHeight}px`,
        zoom: settings.appearance.zoom / 100,
      } as CSSProperties}
    >
      <div className="toolbar">
        <details className="toolbar-menu project-menu" onKeyDown={(event) => {
          if (event.key === "Escape") { event.currentTarget.open = false; event.currentTarget.querySelector("summary")?.focus(); }
        }}>
          <summary className="workspace-brand" title={workspace.path} data-testid="toolbar-project" aria-label={`Project: ${workspace.name}. Open recent solutions.`}>
            <Icon name="project" /><strong>{workspace.name}</strong>
          </summary>
          <div className="menu-content" data-testid="toolbar-project-menu">
            <span className="menu-heading">Recent solutions</span>
            <span className="menu-note">This session only; recent solutions are not persisted yet.</span>
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
        <details className="toolbar-menu trust-menu" onKeyDown={(event) => {
          if (event.key === "Escape") { event.currentTarget.open = false; event.currentTarget.querySelector("summary")?.focus(); }
        }}>
          <summary className={trusted ? "success" : "warning"} data-testid="mode-badge">{trusted ? "Trusted mode" : "Restricted mode"}</summary>
          <div className="menu-content" data-testid="trust-banner">
            <TrustControls snapshot={snapshot} onChange={onSnapshot} />
          </div>
        </details>
        <BranchWidget
          status={gitStatus}
          statusError={gitStatusError}
          trusted={trusted}
          intent={gitIntent}
          onRefresh={refreshGit}
          onOpenCommit={() => {
            setLayout((current) => ({
              ...current,
              left: { ...current.left, active: "commit", open: true },
            }));
          }}
          onMessage={(text) => setMessage({ tone: "info", text })}
        />
        <label className="eval-field toolbar-startup">
          <span>Startup project</span>
          <select aria-label="Startup project" value={startupProject?.path ?? ""} disabled={consoleCandidates.length === 0 || buildRunning} onChange={(event) => setStartupProject(event.target.value)}>
            {!startupProject && <option value="">{consoleCandidates.length === 0 ? "No console project" : "Select a startup project"}</option>}
            {consoleCandidates.map((project) => <option key={project.path} value={project.path}>{project.name}</option>)}
          </select>
        </label>
        <div className="toolbar-configuration">
          <label className="eval-field"><span>Configuration</span><select data-testid="eval-configuration" value={evaluation.context.configuration} disabled={!evaluationAvailable || evaluationBusy} onChange={(event) => void runEvaluation({ configuration: event.target.value })} title={evaluationAvailable ? "MSBuild configuration for the evaluated project model." : "Trust the workspace and resolve the project SDK to select a configuration."}>
            {configurations.map((configuration) => <option key={configuration} value={configuration}>{configuration}</option>)}
          </select></label>
          <label className="eval-field"><span>TFM</span><select data-testid="eval-tfm" value={evaluation.context.targetFramework ?? ""} disabled={!evaluationAvailable || evaluationBusy} onChange={(event) => void runEvaluation({ targetFramework: event.target.value === "" ? null : event.target.value })} title={evaluationAvailable ? "Select a TFM to evaluate multi-targeted projects. Single-target projects evaluate their declared TFM." : "Trust the workspace and resolve the project SDK to select a TFM."}>
            <option value="">TFM: unset</option>{tfmOptions.map((tfm) => <option key={tfm} value={tfm}>{tfm}</option>)}
          </select></label>
        </div>
        <span className="push" />
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
          onEdit={() => openRunConfigDialog()}
        />
        <button
          data-testid="toolbar-build"
          disabled={buildAction.disabledReason !== null}
          onClick={buildAction.execute}
          title={
            buildAction.disabledReason ??
            `Builds the selected startup project (${buildAction.shortcutTexts.join(" or ")}). Unsaved buffers need an explicit decision.`
          }
        >
          <Icon name="build" /> Build <kbd>{buildAction.shortcutLabels[0] ?? ""}</kbd>
        </button>
        <button
          data-testid="toolbar-run"
          disabled={runAction.disabledReason !== null}
          onClick={runAction.execute}
          title={
            runAction.disabledReason ??
            `Runs the saved build (${runAction.shortcutTexts.join(" or ")}); unsaved buffers are never included.`
          }
        >
          <Icon name="run" /> {runRunning ? "Running…" : "Run"}{" "}
          <kbd>{runAction.shortcutLabels[0] ?? ""}</kbd>
        </button>
        <button
          data-testid="toolbar-debug"
          disabled={debugStartAction.disabledReason !== null}
          onClick={debugStartAction.execute}
          title={
            debugStartAction.disabledReason ??
            `Starts the evaluated Debug assembly under netcoredbg (${debugStartAction.shortcutTexts.join(" or ")}).`
          }
        >
          <Icon name="debug" /> Debug <kbd>{debugStartAction.shortcutLabels[0] ?? ""}</kbd>
        </button>
        {debugActive && (
          <button
            data-testid="toolbar-debug-stop"
            onClick={stopAction.execute}
            title={stopAction.disabledReason ?? "Stops the adapter group and reports what was observed."}
          >
            Stop
          </button>
        )}
        <span className="toolbar-disabled" title="Search Everywhere arrives in F3.">
          <button
            type="button"
            className="toolbar-icon-button"
            data-testid="toolbar-search"
            disabled={Boolean(searchAction.disabledReason)}
            aria-label={searchAction.title}
            title={searchAction.disabledReason ?? searchAction.shortcutTexts.join(" or ")}
            onClick={searchAction.execute}
          >
            <Icon name="search" />
          </button>
        </span>
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
        <details className="toolbar-menu" data-testid="workspace-actions-menu" onKeyDown={(event) => {
          if (event.key === "Escape") { event.currentTarget.open = false; event.currentTarget.querySelector("summary")?.focus(); }
        }}>
          <summary aria-label="Workspace actions">•••</summary>
          <div className="menu-content">
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
      </div>

      <div className="workspace-notices">
      <details className="workspace-tools">
        <summary data-testid="workspace-tools-toggle">Workspace tools <span className={restoreIncomplete || restore.state === "failed" ? "warning" : "muted"}>{restoreIncomplete ? "Restore required" : restoreStateLabel(restore)} · Analysis: {lsp.state}</span></summary>
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
                "Runs dotnet restore for this workspace with the project SDK. Nothing restores automatically."
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
      </details>

      {message && (
        <div
          className={`notice ${message.tone === "error" ? "error" : ""}`}
          role={message.tone === "error" ? "alert" : "status"}
          data-testid="app-log"
        >
          {message.text}
          <button className="notice-dismiss" aria-label="Dismiss notification" onClick={() => setMessage(null)}>×</button>
        </div>
      )}

      {showsProcesses && <ProcessPanel snapshot={snapshot} onChange={onSnapshot} />}
      </div>

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
            onRefresh={refreshGit}
            onMessage={(text) => setMessage({ tone: "info", text })}
            onError={(text) => setMessage({ tone: "error", text })}
          />
        ) : (
        <nav className="side" aria-label="Solution explorer" data-testid="tree">
          <div className="panel-title">
            Solution Explorer{" "}
            <span className="tag">
              {showsEvaluatedTree ? "MSBuild evaluated" : "Text outline · not evaluated"}
            </span>
          </div>

          <div className="eval-controls">
            <button
              data-testid="eval-run"
              disabled={!evaluationAvailable || evaluationBusy}
              onClick={() => void runEvaluation()}
              title={
                evaluationAvailable
                  ? "Evaluate every project with MSBuild. No restore, no build."
                  : "Trust the workspace and resolve the project SDK before evaluating."
              }
            >
              {evaluationBusy
                ? "Evaluating…"
                : evaluation.revision > 0
                  ? "Re-evaluate"
                  : "Evaluate project model"}
            </button>
            <span
              className={`tag ${evaluation.state === "failed" || evaluation.state === "partial" ? "error" : ""}`}
              data-testid="eval-state"
              role="status"
            >
              {evaluationStateLabel(evaluation.state)}
              {evaluation.stale ? " · stale" : ""}
              {evaluation.revision > 0 ? ` · revision ${evaluation.revision}` : ""}
            </span>
            {selectedProject && <button onClick={() => setProjectDetailsOpen(!projectDetailsOpen)} aria-expanded={projectDetailsOpen}>Project details</button>}
          </div>

          {!evaluationAvailable && trusted && (
            <p className="tree-note" data-testid="eval-disabled-reason">
              Evaluation needs a resolved project SDK. Host SDK 10 is never substituted.
            </p>
          )}

          {evaluation.staleReason && (
            <div className="notice warning eval-stale" role="status" data-testid="eval-stale">
              <strong>Evaluation stale.</strong> {evaluation.staleReason} Values shown are from
              the previous evaluation and are not current.
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

          {showsEvaluatedTree ? (
            <div className="tree">
              <div className="tree-root">{workspace.name}</div>
              {projectGroups.map(([folder, items]) => (
                <div key={folder || "root"}>
                  {folder && (
                    <div className="tree-folder l1" data-testid={`tree-folder-${folder}`}>
                      {folder}
                    </div>
                  )}
                  {items.map((project) => {
                    const open = expanded.has(project.path);
                    const selected = selectedProject?.path === project.path;
                    return (
                      <div key={project.path}>
                        <button
                          className={`tree-row ${folder ? "l2" : "l1"} ${selected ? "selected" : ""}`}
                          data-testid={`tree-project-${project.name}`}
                          aria-expanded={open}
                          aria-current={selected ? "true" : undefined}
                          onClick={() => {
                            toggleProject(project.path);
                            setSelectedProjectPath(project.path);
                          }}
                          title={project.path}
                        >
                          <span aria-hidden="true">{open ? "▾" : "▸"}</span>
                          <Icon name="project" /><span className="tree-name">{project.name}</span>
                          <span className="tag">{project.kind}</span>
                          {project.state === "failed" && <span className="tag error">error</span>}
                          {project.tfms.map((tfm) => (
                            <span className="tag" key={tfm}>
                              {tfm}
                            </span>
                          ))}
                          {project.restored !== "restored" && (
                            <span className="tag warning">
                              {project.restored === "unknown" ? "restore unknown" : "partial context"}
                            </span>
                          )}
                          {restore.revision > 0 &&
                            restoreProjectState.get(project.path) !== "restored" && (
                              <span className="tag warning">restore incomplete</span>
                            )}
                        </button>
                        {open && (
                          <>
                            {project.error && (
                              <div
                                className="tree-note error"
                                data-testid={`eval-project-error-${project.name}`}
                              >
                                Evaluation failed: {project.error}
                              </div>
                            )}
                            {project.note && <div className="tree-note">{project.note}</div>}
                            {project.projectRefs.length > 0 && (
                              <div className="tree-note">
                                References: {project.projectRefs.map((ref) => ref.name).join(", ")}
                              </div>
                            )}
                            {project.packageRefs.length > 0 && (
                              <div className="tree-note">
                                Packages:{" "}
                                {project.packageRefs
                                  .map((pkg) =>
                                    pkg.version ? `${pkg.name} ${pkg.version}` : `${pkg.name} (version unknown)`,
                                  )
                                  .join(", ")}
                              </div>
                            )}
                            {project.sources.map((file) => (
                              <button
                                key={file}
                                className={`tree-row l3 ${activePath === file ? "selected" : ""}`}
                                data-testid={`tree-file-${file}`}
                                aria-current={activePath === file ? "page" : undefined}
                                onClick={() => void openFile(file)}
                                title={file}
                              >
                                <Icon name="file" /><span className="tree-name">{baseName(file)}</span>
                              </button>
                            ))}
                            {project.sources.length === 0 && !project.error && (
                              <div className="tree-note">No source files evaluated.</div>
                            )}
                          </>
                        )}
                      </div>
                    );
                  })}
                </div>
              ))}
            </div>
          ) : (
            <div className="tree">
              <div className="tree-root">{workspace.name}</div>
              {projects.map((project) => {
                const open = expanded.has(project.path);
                return (
                  <div key={project.path}>
                    <button
                      className="tree-row l1"
                      data-testid={`tree-project-${project.name}`}
                      aria-expanded={open}
                      onClick={() => toggleProject(project.path)}
                      title={project.path}
                    >
                      <span aria-hidden="true">{open ? "▾" : "▸"}</span>
                      <Icon name="project" /><span className="tree-name">{project.name}</span>
                      <span className="tag">{project.kind}</span>
                      {project.tfms.map((tfm) => (
                        <span className="tag" key={tfm}>
                          {tfm}
                        </span>
                      ))}
                    </button>
                    {open && (
                      <>
                        {project.references.length > 0 && (
                          <div className="tree-note l2">
                            References: {project.references.join(", ")}
                          </div>
                        )}
                        {project.files.map((file) => (
                          <button
                            key={file}
                            className={`tree-row l2 ${activePath === file ? "selected" : ""}`}
                            data-testid={`tree-file-${file}`}
                            aria-current={activePath === file ? "page" : undefined}
                            onClick={() => void openFile(file)}
                            title={file}
                          >
                            <Icon name="file" /><span className="tree-name">{baseName(file)}</span>
                          </button>
                        ))}
                        {project.files.length === 0 && (
                          <div className="tree-note l2">No .cs files found (bin/obj hidden).</div>
                        )}
                      </>
                    )}
                  </div>
                );
              })}
            </div>
          )}


          {!showsEvaluatedTree && (
            <p className="tree-footnote">
              Read from project text only. MSBuild has not run and nothing is evaluated.
            </p>
          )}
        </nav>
        )}
        <Splitter label="Solution explorer width" value={sidebarWidth} min={220} max={420} onChange={setSidebarWidth} />
          </>
        )}

        <section className="editor-pane" aria-label="Editor">
          <div className="tabbar">
            <div className="tabs" role="tablist" aria-label="Open files" onKeyDown={(event) => {
              if (!tabs.length || !["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
              event.preventDefault();
              const index = tabs.findIndex((tab) => tab.path === activePath);
              const next = event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : (index + (event.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length;
              setActivePath(tabs[next].path);
              void checkDisk(tabs[next].path);
              event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]')[next]?.focus();
            }}>
              {tabs.map((tab) => {
                const dirty = isDirty(tab);
                const selected = tab.path === activePath;
                return (
                  <div className={`tab ${selected ? "active" : ""}`} key={tab.path} data-testid={`tab-${tab.path}`}>
                    <button
                      role="tab"
                      aria-selected={selected}
                      tabIndex={selected ? 0 : -1}
                      aria-controls="editor-content"
                      className="tab-label"
                      onClick={() => {
                        setActivePath(tab.path);
                        void checkDisk(tab.path);
                      }}
                      title={tab.path}
                      aria-label={`${baseName(tab.path)}${dirty ? ", modified" : ""}`}
                    >
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
            </div>
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

          {active ? (
            <>
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
                <span
                  className={isDirty(active) ? "warning" : "muted"}
                  data-testid="editor-revision"
                >
                  {" · "}revision {active.revision} · saved {active.savedRevision} ·{" "}
                  {isDirty(active) ? "modified" : "saved"}
                </span>
              </div>

              {editPreview && (
                <section
                  className="rename-preview"
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
                  className="rename-preview code-actions"
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
                  revision={active.revision}
                  editor={settings.editor}
                  onChange={(value) => updateContent(active.path, value)}
                  onSave={() => void save()}
                  onDefinition={(location) =>
                    void navigateTo(location.path, {
                      line: location.line,
                      character: location.character,
                    })
                  }
                  onRename={(position) => requestRename(position)}
                  onCodeActions={(position) => void requestCodeActions(position)}
                  onCursor={noteCursor}
                  breakpoints={activeBreakpoints}
                  currentLine={debugCurrentLine}
                  onToggleBreakpoint={(line) => void toggleBreakpoint(line)}
                  target={
                    editorTarget?.path === active.path ? editorTarget : null
                  }
                />
              </div>
            </>
          ) : (
            <div className="empty-editor">
              <Icon name="code" />
              <h1>Your code, in focus.</h1>
              <p className="muted">Open a file from the solution explorer to start editing.</p>
              <dl className="shortcut-list">
                <dt>Save file</dt><dd><kbd>{snapshot.app.platform === "macos" ? "⌘S" : "Ctrl+S"}</kbd></dd>
                <dt>Go to definition</dt><dd><kbd>{snapshot.app.platform === "macos" ? "⌘B" : "Ctrl+B"}</kbd></dd>
                <dt>Rename symbol</dt><dd><kbd>Shift+F6</kbd></dd>
                <dt>Run saved build</dt><dd><kbd>{runShortcut}</kbd></dd>
              </dl>
              <p className="hint">Text editing is always available. Analysis and execution require trust.</p>
            </div>
          )}
        </section>

        <aside className="eval-details-pane" hidden={!selectedProject || !projectDetailsOpen} aria-label="Project inspector">
          <button className="inspector-close" onClick={() => setProjectDetailsOpen(false)}>Close project details</button>
            {selectedProject && (
              <section
                className="eval-details"
                aria-label="Evaluated project details"
                tabIndex={0}
                data-testid="eval-details"
              >
                <div className="panel-title">
                  {selectedProject.name} <span className="tag">Evaluated project</span>
                </div>
                <dl>
                  <dt>State</dt>
                  <dd data-testid="eval-details-state">
                    {selectedProject.state === "ok" ? "Evaluated" : "Evaluation failed"}
                    {evaluation.stale ? " · stale (previous evaluation)" : ""}
                  </dd>
                  <dt>Path</dt>
                  <dd>
                    <code>{selectedProject.path}</code>
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
                      ? evaluation.stale
                        ? `Previous evaluation (stale): ${selectedProject.targetPath}`
                        : selectedProject.targetPath
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
                    onClick={() => void openFile(selectedProject.path)}
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
      <BottomToolWindow tabs={bottomTabLabels} active={bottomTab} onSelect={setBottomTab} panelRef={panelBodyRef}>
        {renderBottomContent()}
      </BottomToolWindow>
        </>
      )}

      {findActionOpen && (
        <FindAction
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

      {keymapOpen && (
        <KeymapDialog
          actions={resolvedActions}
          keymap={settings.keymap}
          path={settingsPath}
          platform={snapshot.app.platform}
          onChange={saveKeymap}
          onClose={() => setKeymapOpen(false)}
        />
      )}

      {settingsOpen && (
        <SettingsDialog
          actions={resolvedActions}
          settings={settings}
          path={settingsPath}
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
        trusted={trusted}
        sdkSummary={sdkSummary}
        evaluation={evaluationStatus}
        dirtyCount={dirtyCount}
        breadcrumb={statusBreadcrumb}
        cursor={active ? `Ln ${cursor.line + 1}, Col ${cursor.character + 1}` : "No file open"}
        indent={{
          label: settings.editor.useTabs
            ? `Tabs (${settings.editor.indentSize})`
            : `${settings.editor.indentSize} spaces`,
          title: "Indentation comes from Settings (Editor). No per-file detection is claimed.",
        }}
      />

      {renamePrompt && (
        <div className="modal-backdrop">
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="rename-title"
            onKeyDown={(event) => {
              if (event.key === "Escape") cancelRenamePrompt();
              if (event.key === "Tab") {
                const focusable = Array.from(
                  event.currentTarget.querySelectorAll<HTMLElement>("input, button"),
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
            <h2 id="rename-title">Rename symbol</h2>
            <p className="muted" data-testid="rename-source">
              {renamePrompt.path} · {renamePrompt.line + 1}:{renamePrompt.character + 1} ·
              revision {renamePrompt.revision} · session {renamePrompt.session}
            </p>
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void submitRename();
              }}
            >
              <label className="rename-field">
                <span>New name</span>
                <input
                  data-testid="rename-input"
                  autoFocus
                  value={renameName}
                  onChange={(event) => setRenameName(event.target.value)}
                  disabled={renamePrompt.busy}
                  aria-describedby={renamePrompt.error ? "rename-error" : undefined}
                />
              </label>
              {renamePrompt.error && (
                <p className="error" role="alert" id="rename-error" data-testid="rename-error">
                  {renamePrompt.error}
                </p>
              )}
              <p className="hint">
                csharp-ls returns the real edits; nothing changes until the preview is
                applied.
              </p>
              <div className="actions">
                <button type="submit" data-testid="rename-submit" disabled={renamePrompt.busy}>
                  {renamePrompt.busy ? "Waiting for csharp-ls…" : "Preview rename"}
                </button>
                <button type="button" data-testid="rename-cancel" onClick={cancelRenamePrompt}>
                  Cancel
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

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
          onClose={() => setRunConfigOpen(false)}
        />
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
              <button data-testid="confirm-save" onClick={() => void confirmSave()}>
                {confirm.kind === "tab" ? "Save" : "Save all"}
              </button>
              <button autoFocus data-testid="confirm-keep" onClick={keepEditing}>
                Keep editing
              </button>
              <button
                className="danger"
                data-testid="confirm-discard"
                onClick={discardConfirmed}
              >
                {confirm.kind === "tab"
                  ? "Discard and close"
                  : confirm.kind === "workspace"
                    ? "Discard and close workspace"
                    : "Discard and close app"}
              </button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}
