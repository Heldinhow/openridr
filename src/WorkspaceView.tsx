import { useEffect, useRef, useState } from "react";
import {
  call,
  HostCallError,
  type EvaluationProject,
  type Snapshot,
  type OutlineProject,
} from "./host/client";
import { CodeEditor } from "./CodeEditor";
import { ProblemsPanel } from "./ProblemsPanel";
import { TrustControls } from "./TrustControls";
import { ProcessPanel } from "./ProcessPanel";
import { LspPanel, ToolInstaller } from "./ToolchainPanels";

type Tab = {
  path: string;
  content: string;
  saved: string;
  hash: string;
  conflict: null | { content: string; hash: string };
};

type Confirmation = { kind: "tab"; path: string } | { kind: "workspace" };

type BottomTab = "build" | "problems" | "debug" | "terminal" | "tests";

type Props = {
  snapshot: Snapshot;
  initialFile: string | null;
  onSnapshot: (snapshot: Snapshot) => void;
  onClosed: (snapshot: Snapshot) => void;
};

const bottomTabs: { id: BottomTab; label: string }[] = [
  { id: "build", label: "Build" },
  { id: "problems", label: "Problems" },
  { id: "debug", label: "Debug" },
  { id: "terminal", label: "Terminal" },
  { id: "tests", label: "Tests" },
];

function baseName(path: string) {
  return path.split("/").pop() ?? path;
}

function messageOf(error: unknown) {
  return error instanceof HostCallError ? error.message : String(error);
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

export function WorkspaceView({ snapshot, initialFile, onSnapshot, onClosed }: Props) {
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
  const [bottomTab, setBottomTab] = useState<BottomTab>("build");
  const [editorTarget, setEditorTarget] = useState<{
    path: string;
    line: number;
    character: number;
    nonce: number;
  } | null>(null);
  const [editedAt, setEditedAt] = useState<Record<string, number>>({});
  const navigationSeq = useRef(0);
  const [selectedProjectPath, setSelectedProjectPath] = useState<string | null>(null);
  const [evaluating, setEvaluating] = useState(false);
  const autoEvaluation = useRef<number | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(
    () => new Set(projects.map((project) => project.path)),
  );

  const active = tabs.find((tab) => tab.path === activePath) ?? null;
  const activeProject = active ? projectFor(active.path, projects) : undefined;
  const activeEvaluationProject = active
    ? evaluatedProjectFor(active.path, evaluatedProjects)
    : undefined;
  const selectedProject =
    evaluatedProjects.find((project) => project.path === selectedProjectPath) ?? null;
  const dirtyCount = tabs.filter((tab) => tab.content !== tab.saved || tab.conflict).length;
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

  useEffect(() => {
    // Evaluate automatically once per trusted context; a stale model always needs an
    // explicit re-evaluate so a late result cannot be mistaken for the latest one.
    if (!evaluationAvailable || evaluation.state !== "idle" || evaluation.staleReason) {
      return;
    }
    if (autoEvaluation.current === snapshot.contextId) {
      return;
    }
    autoEvaluation.current = snapshot.contextId;
    void runEvaluation();
  }, [evaluationAvailable, evaluation.state, evaluation.staleReason, snapshot.contextId]);

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
      return true;
    }
    try {
      const file = await call<{ path: string; content: string; hash: string }>("file.read", {
        path,
      });
      setTabs((current) => [
        ...current,
        { path: file.path, content: file.content, saved: file.content, hash: file.hash, conflict: null },
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

  function updateContent(path: string, content: string) {
    setTabs((current) =>
      current.map((tab) => (tab.path === path ? { ...tab, content } : tab)),
    );
    setEditedAt((current) => ({ ...current, [path]: Date.now() }));
    queueDidChange(path, content);
  }

  function applyConflict(path: string, error: HostCallError) {
    const details = error.details as { currentContent?: string; currentHash?: string } | undefined;
    if (details?.currentContent === undefined || !details.currentHash) {
      setMessage({ tone: "error", text: `Save failed: ${error.message}` });
      return;
    }
    setTabs((current) =>
      current.map((tab) =>
        tab.path === path
          ? { ...tab, conflict: { content: details.currentContent!, hash: details.currentHash! } }
          : tab,
      ),
    );
    setMessage({
      tone: "error",
      text: `${path} changed on disk. Your buffer was kept; choose Reload from disk or Overwrite disk.`,
    });
  }

  async function save() {
    const tab = active;
    if (!tab) return;
    if (tab.conflict) {
      setMessage({
        tone: "error",
        text: "A disk conflict is pending. Choose Reload from disk or Overwrite disk explicitly.",
      });
      return;
    }
    if (tab.content === tab.saved) {
      setMessage({ tone: "info", text: "No changes to save." });
      return;
    }
    const content = tab.content;
    try {
      const result = await call<{ path: string; hash: string }>("file.write", {
        path: tab.path,
        content,
        expected_hash: tab.hash,
      });
      setTabs((current) =>
        current.map((item) =>
          item.path === tab.path
            ? { ...item, saved: content, hash: result.hash, conflict: null }
            : item,
        ),
      );
      setMessage({ tone: "info", text: `Saved ${tab.path}` });
    } catch (error) {
      if (error instanceof HostCallError && error.code === "conflict") {
        applyConflict(tab.path, error);
      } else {
        setMessage({ tone: "error", text: `Save failed; buffer kept. ${messageOf(error)}` });
      }
    }
  }

  function reloadFromDisk() {
    if (!active?.conflict) return;
    const conflict = active.conflict;
    setTabs((current) =>
      current.map((tab) =>
        tab.path === active.path
          ? { ...tab, content: conflict.content, saved: conflict.content, hash: conflict.hash, conflict: null }
          : tab,
      ),
    );
    setMessage({
      tone: "info",
      text: `Reloaded ${active.path} from disk. Your edits were discarded.`,
    });
  }

  async function overwriteDisk() {
    const tab = active;
    if (!tab?.conflict) return;
    const conflict = tab.conflict;
    const content = tab.content;
    try {
      const result = await call<{ path: string; hash: string }>("file.write", {
        path: tab.path,
        content,
        expected_hash: conflict.hash,
      });
      setTabs((current) =>
        current.map((item) =>
          item.path === tab.path
            ? { ...item, saved: content, hash: result.hash, conflict: null }
            : item,
        ),
      );
      setMessage({ tone: "info", text: `Overwrote ${tab.path} with the buffer.` });
    } catch (error) {
      if (error instanceof HostCallError && error.code === "conflict") {
        applyConflict(tab.path, error);
      } else {
        setMessage({ tone: "error", text: `Overwrite failed; buffer kept. ${messageOf(error)}` });
      }
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
    if (tab.content !== tab.saved || tab.conflict) {
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

  async function runEvaluation(next?: {
    configuration?: string;
    targetFramework?: string | null;
  }) {
    const configuration = next?.configuration ?? evaluation.context.configuration;
    const targetFramework =
      next && "targetFramework" in next
        ? next.targetFramework ?? null
        : evaluation.context.targetFramework;
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

  function openGlobalJson() {
    void openFile(projectSdk.globalJsonPath ?? "global.json");
  }

  async function startRestore(method: "restore.start" | "restore.retry") {
    try {
      onSnapshot(await call<Snapshot>(method, {}));
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

  function requestCloseWorkspace() {
    if (dirtyCount > 0) {
      setConfirm({ kind: "workspace" });
      return;
    }
    void closeWorkspace();
  }

  function discardConfirmed() {
    if (!confirm) return;
    const pending = confirm;
    setConfirm(null);
    if (pending.kind === "tab") {
      closeTab(pending.path);
      setMessage({ tone: "info", text: `Closed ${pending.path}; unsaved edits were discarded.` });
    } else {
      void closeWorkspace();
    }
  }

  function renderBottomContent() {
    switch (bottomTab) {
      case "build":
        return (
          <p className="muted">
            No build has run. Build is not available in this ticket; it arrives with ticket 11.
          </p>
        );
      case "problems":
        return (
          <ProblemsPanel
            diagnostics={snapshot.diagnostics}
            editedAt={editedAt}
            onNavigate={(path, position) => void navigateTo(path, position)}
            onOpen={(path) => void openFile(path)}
          />
        );
      case "debug":
        return <p className="muted">Debug is not available in this build (ticket 14).</p>;
      case "terminal":
        return <p className="muted">Terminal is not available in this build (ticket 15).</p>;
      case "tests":
        return <p className="muted">Tests are not available in this build (ticket 17).</p>;
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

  return (
    <main className="workspace-view">
      <header className="chrome">
        <span className="brand">OpenRidr · {workspace.name}</span>
        <span
          className={trusted ? "success" : "warning"}
          data-testid="mode-badge"
        >
          {trusted ? "Trusted mode" : "Restricted mode"}
        </span>
      </header>

      <div className="toolbar">
        <span>
          {workspace.kind === "solution" ? "Solution" : "Project"}:{" "}
          <strong>{workspace.name}</strong>
        </span>
        <span className="push" />
        <button disabled title="Build is not available in this ticket (ticket 11).">
          Build
        </button>
        <button disabled title="Run is not available in this ticket (ticket 12).">
          Run
        </button>
        <button disabled title="Debug is not available in this ticket (ticket 14).">
          Debug
        </button>
        <button
          data-testid="sdk-redetect"
          onClick={() => void redetectSdks()}
          title="Re-runs only the SDK inventory; it never touches project files."
        >
          Redetect SDKs
        </button>
        {(projectSdk.state === "sdk_missing" || projectSdk.state === "invalid_global_json") && (
          <button data-testid="sdk-open-global-json" onClick={openGlobalJson}>
            Open global.json
          </button>
        )}
        <button data-testid="close-workspace" onClick={requestCloseWorkspace}>
          Close workspace
        </button>
      </div>

      <div className="workspace-notices">
      <div className={`notice ${trusted ? "" : "warning"}`} data-testid="trust-banner">
        {!trusted && (
          <p>
            <strong>Blocked in Restricted mode.</strong> MSBuild, restore, LSP, tool
            installation, terminal, build, run, debug and tests need a trusted workspace. Text
            editing and saving stay available.
          </p>
        )}
        <TrustControls snapshot={snapshot} onChange={onSnapshot} compact />
      </div>

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

      <ToolInstaller snapshot={snapshot} onChange={onSnapshot} />
      <LspPanel snapshot={snapshot} onChange={onSnapshot} />

      {message && (
        <div
          className={`notice ${message.tone === "error" ? "error" : ""}`}
          role={message.tone === "error" ? "alert" : "status"}
          data-testid="app-log"
        >
          {message.text}
        </div>
      )}

      {showsProcesses && <ProcessPanel snapshot={snapshot} onChange={onSnapshot} />}
      </div>

      <div className={`workspace-grid ${selectedProject ? "with-details" : ""}`}>
        <nav className="side" aria-label="Solution explorer" data-testid="tree">
          <div className="panel-title">
            Solution Explorer{" "}
            <span className="tag">
              {showsEvaluatedTree ? "MSBuild evaluated" : "Text outline · not evaluated"}
            </span>
          </div>

          <div className="eval-controls">
            <label className="eval-field">
              <span>Configuration</span>
              <select
                data-testid="eval-configuration"
                value={evaluation.context.configuration}
                disabled={!evaluationAvailable || evaluationBusy}
                onChange={(event) =>
                  void runEvaluation({ configuration: event.target.value })
                }
                title={
                  evaluationAvailable
                    ? "MSBuild configuration for the evaluated project model."
                    : "Trust the workspace and resolve the project SDK to select a configuration."
                }
              >
                {configurations.map((configuration) => (
                  <option key={configuration} value={configuration}>
                    {configuration}
                  </option>
                ))}
              </select>
            </label>
            <label className="eval-field">
              <span>TFM</span>
              <select
                data-testid="eval-tfm"
                value={evaluation.context.targetFramework ?? ""}
                disabled={!evaluationAvailable || evaluationBusy}
                onChange={(event) =>
                  void runEvaluation({
                    targetFramework: event.target.value === "" ? null : event.target.value,
                  })
                }
                title={
                  evaluationAvailable
                    ? "Select a TFM to evaluate multi-targeted projects. Single-target projects evaluate their declared TFM."
                    : "Trust the workspace and resolve the project SDK to select a TFM."
                }
              >
                <option value="">Not selected</option>
                {tfmOptions.map((tfm) => (
                  <option key={tfm} value={tfm}>
                    {tfm}
                  </option>
                ))}
              </select>
            </label>
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
                          <span>{project.name}</span>
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
                                {baseName(file)}
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
                      <span>{project.name}</span>
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
                            {baseName(file)}
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

        <section className="editor-pane" aria-label="Editor">
          <div className="tabbar">
            <div className="tabs" role="tablist" aria-label="Open files">
              {tabs.map((tab) => {
                const dirty = tab.content !== tab.saved || Boolean(tab.conflict);
                const selected = tab.path === activePath;
                return (
                  <div className={`tab ${selected ? "active" : ""}`} key={tab.path} data-testid={`tab-${tab.path}`}>
                    <button
                      role="tab"
                      aria-selected={selected}
                      className="tab-label"
                      onClick={() => setActivePath(tab.path)}
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
              data-testid="save-file"
              onClick={() => void save()}
              disabled={!active || (active.content === active.saved && !active.conflict)}
              title={active ? "Save with Cmd+S (macOS) or Ctrl+S (Windows)" : "No file open"}
            >
              Save
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
                </div>
              )}

              <div className="editor-body">
                <CodeEditor
                  key={active.path}
                  value={active.content}
                  onChange={(value) => updateContent(active.path, value)}
                  onSave={() => void save()}
                  target={
                    editorTarget?.path === active.path ? editorTarget : null
                  }
                />
              </div>
            </>
          ) : (
            <div className="empty-editor">
              <p>Open a file from the solution explorer to edit it.</p>
              <p className="muted">Saving checks the disk first and never overwrites silently.</p>
            </div>
          )}
        </section>

        <aside className="eval-details-pane" hidden={!selectedProject}>
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

      <section
        className={`output ${bottomTab === "problems" ? "problems-output" : ""}`}
        aria-label="Bottom panel"
      >
        <div className="tabs bottom-tabs" role="tablist" aria-label="Output panels">
          {bottomTabs.map((tab) => (
            <button
              key={tab.id}
              role="tab"
              aria-selected={bottomTab === tab.id}
              className={`tab-label ${bottomTab === tab.id ? "active" : ""}`}
              data-testid={`bottom-tab-${tab.id}`}
              onClick={() => setBottomTab(tab.id)}
            >
              {tab.id === "problems" && snapshot.diagnostics.currentItems > 0
                ? `Problems (${snapshot.diagnostics.currentItems})`
                : tab.label}
            </button>
          ))}
        </div>
        <div className="panel-body" role="tabpanel">
          {renderBottomContent()}
        </div>
      </section>

      <footer className="status">
        <span data-testid="status-workspace">{workspace.name}</span>
        <span className={trusted ? "success" : "warning"} data-testid="status-mode">
          {trusted ? "Trusted mode" : "Restricted mode"}
        </span>
        <span data-testid="status-sdk">{sdkSummary}</span>
        {showsEvaluatedTree ? (
          <span data-testid="status-evaluation">
            {evaluationStateLabel(evaluation.state)} · Configuration{" "}
            {evaluation.context.configuration} · TFM{" "}
            {evaluation.context.targetFramework ?? "not selected"} · revision{" "}
            {evaluation.revision}
            {evaluation.stale ? " · stale" : ""}
          </span>
        ) : (
          <span>Text outline · not evaluated</span>
        )}
        {dirtyCount > 0 && (
          <span data-testid="status-dirty">
            {dirtyCount} modified buffer{dirtyCount === 1 ? "" : "s"}
          </span>
        )}
        <span className="push" data-testid="status-keymap">
          Keymap: IntelliJ / Rider
        </span>
      </footer>

      {confirm && (
        <div className="modal-backdrop">
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="confirm-title"
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                setConfirm(null);
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
              {confirm.kind === "tab" ? "Close modified file?" : "Close workspace with unsaved changes?"}
            </h2>
            <p>
              {confirm.kind === "tab"
                ? `${confirm.path} has unsaved changes.`
                : `${dirtyCount} modified buffer${dirtyCount === 1 ? "" : "s"} will be lost.`}
            </p>
            <div className="actions">
              <button autoFocus data-testid="confirm-keep" onClick={() => setConfirm(null)}>
                Keep editing
              </button>
              <button
                className="danger"
                data-testid="confirm-discard"
                onClick={discardConfirmed}
              >
                {confirm.kind === "tab" ? "Discard and close" : "Discard and close workspace"}
              </button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}
