import { useState } from "react";
import { call, HostCallError, messageOf, type EvaluationProject, type RunConfig, type Snapshot } from "./host/client";
import {
  cancelBuild as buildCancel,
  cancelRun as runCancel,
  waitForHost as waitHost,
} from "./executionCommands";
import type { RunConfigView } from "./runConfigs";
import type { BottomToolId } from "./shell/toolWindows";

// What the workspace hands over: read-only render values plus the few callbacks that start flows.
export interface ExecutionFlowInput {
  snapshot: Snapshot;
  evaluation: Snapshot["evaluation"];
  build: Snapshot["build"];
  buildRunning: boolean;
  runRunning: boolean;
  dirtyCount: number;
  consoleCandidates: EvaluationProject[];
  startupProject: EvaluationProject | null;
  activeRunConfig: RunConfigView | null;
  activeConfigEvaluated: boolean;
  buildDisabledReason: string | null;
  debugDisabledReason: string | null;
  runAvailabilityReason: (
    project: EvaluationProject | null,
    configEvaluated: boolean,
    invalidSelectedProject?: boolean,
  ) => string | null;
  onSnapshot: (snapshot: Snapshot) => void;
  setMessage: (message: { tone: "info" | "error"; text: string } | null) => void;
  setBottomTab: (tab: BottomToolId) => void;
  saveAll: () => Promise<string[]>;
  invalidRunConfigReason: (config?: RunConfig | null) => string | null;
  openRunConfigDialog: (id?: string) => void;
  openConfigOnFieldError: (error: unknown) => void;
}

// Build, Run and Debug start flows plus the decision dialogs' state. Moved out of WorkspaceView unchanged.
export function useExecutionFlow(c: ExecutionFlowInput) {
  const { snapshot, evaluation, dirtyCount, consoleCandidates, startupProject,
    activeRunConfig, activeConfigEvaluated, buildDisabledReason, debugDisabledReason, runAvailabilityReason,
    onSnapshot, setMessage, setBottomTab, saveAll, invalidRunConfigReason, openRunConfigDialog, openConfigOnFieldError,
  } = c;
  const [buildDecision, setBuildDecision] = useState(false);
  const [runDecision, setRunDecision] = useState(false);
  const [pendingRunConfig, setPendingRunConfig] = useState<RunConfig | null>(null);
  // A Solution-tree Build on a specific project survives the unsaved-buffers decision.
  const [pendingBuildConfig, setPendingBuildConfig] = useState<RunConfig | null>(null);

  async function startBuild(savedNote: string, requestedConfig?: RunConfig | null): Promise<boolean> {
    const project = requestedConfig
      ? consoleCandidates.find((item) => item.path === requestedConfig.project) ?? null
      : startupProject;
    if (!project) {
      setMessage({
        tone: "error",
        text: "Choose a console startup project before building.",
      });
      return false;
    }
    setBottomTab("build");
    try {
      onSnapshot(
        await call<Snapshot>("build.start", {
          project: project.path,
          configuration:
            requestedConfig?.configuration ?? activeRunConfig?.configuration ?? evaluation.context.configuration,
          targetFramework:
            requestedConfig?.targetFramework ?? activeRunConfig?.targetFramework ?? evaluation.context.targetFramework,
          savedNote,
        }),
      );
      setMessage(null);
      return true;
    } catch (error) {
      const text = `Build could not start: ${messageOf(error)}`;
      setMessage({ tone: "error", text });
      if (error instanceof HostCallError && error.code === "restore_required") {
        setBottomTab("build");
      }
      return false;
    }
  }

  // `requestedConfig` comes from the Solution tree; its caller already applied the project's gate.
  function requestBuild(requestedConfig?: RunConfig) {
    // A tree-requested config skips the startup-project gate, never the trust gate.
    const reason = requestedConfig && snapshot.trust.granted ? null : buildDisabledReason;
    if (reason) {
      setMessage({ tone: "error", text: `Build unavailable: ${reason}` });
      return;
    }
    if (dirtyCount > 0) {
      setPendingBuildConfig(requestedConfig ?? null);
      setBuildDecision(true);
      return;
    }
    void startBuild("Built saved content; no unsaved buffers were open.", requestedConfig);
  }

  async function saveAllAndBuild(requested?: RunConfig | null): Promise<boolean> {
    const requestedConfig = requested ?? pendingBuildConfig;
    setPendingBuildConfig(null);
    const dirty = dirtyCount;
    const failures = await saveAll();
    if (failures.length > 0) {
      setBuildDecision(false);
      setMessage({ tone: "error", text: failures.join(" ") });
      return false;
    }
    setBuildDecision(false);
    return startBuild(
      `Saved all ${dirty} modified buffer${dirty === 1 ? "" : "s"} before building.`,
      requestedConfig,
    );
  }

  async function buildSavedContent() {
    const excluded = dirtyCount;
    setBuildDecision(false);
    const requestedConfig = pendingBuildConfig;
    setPendingBuildConfig(null);
    await startBuild(
      `Built saved content; ${excluded} unsaved buffer${excluded === 1 ? "" : "s"} excluded.`,
      requestedConfig,
    );
  }

  const cancelBuild = () => buildCancel({ onSnapshot, setMessage });

  async function startRun(requestedConfig?: RunConfig | null) {
    const config = requestedConfig ?? activeRunConfig;
    const invalid = invalidRunConfigReason(config);
    if (invalid) {
      setMessage({ tone: "error", text: invalid });
      openRunConfigDialog(config?.id);
      return;
    }
    const selectedProject = config
      ? consoleCandidates.find((project) => project.path === config.project) ?? null
      : startupProject;
    if (!selectedProject) {
      setMessage({
        tone: "error",
        text: "Choose a console startup project before running.",
      });
      return;
    }
    setBottomTab("run");
    try {
      onSnapshot(
        await call<Snapshot>("run.start", {
          project: selectedProject.path,
          configuration: config?.configuration ?? evaluation.context.configuration,
          targetFramework:
            config?.targetFramework ?? evaluation.context.targetFramework,
          args: config?.args ?? [],
          workingDirectory: config?.workingDirectory ?? "",
          env: config?.env ?? {},
          envFiles: config?.envFiles ?? [],
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

  function requestRun(requestedConfig?: RunConfig) {
    const configProject = requestedConfig
      ? consoleCandidates.find((project) => project.path === requestedConfig.project) ?? null
      : startupProject;
    const configEvaluated = requestedConfig
      ? requestedConfig.configuration === evaluation.context.configuration &&
        (requestedConfig.targetFramework ?? null) === (evaluation.context.targetFramework ?? null)
      : activeConfigEvaluated;
    const configDisabledReason = runAvailabilityReason(
      configProject,
      configEvaluated,
      Boolean(requestedConfig && !configProject),
    );
    if (configDisabledReason) {
      setMessage({ tone: "error", text: `Run unavailable: ${configDisabledReason}` });
      return;
    }
    const invalid = invalidRunConfigReason(requestedConfig ?? activeRunConfig);
    if (invalid) {
      setMessage({ tone: "error", text: invalid });
      openRunConfigDialog(requestedConfig?.id ?? activeRunConfig?.id);
      return;
    }
    if (dirtyCount > 0) {
      setPendingRunConfig(requestedConfig ?? null);
      setRunDecision(true);
      return;
    }
    void startRun(requestedConfig);
  }

  async function saveAllAndRebuild() {
    setRunDecision(false);
    const requestedConfig = pendingRunConfig;
    setPendingRunConfig(null);
    const dirty = dirtyCount;
    const failures = await saveAll();
    if (failures.length > 0) {
      setMessage({ tone: "error", text: failures.join(" ") });
      return;
    }
    if (dirty > 0) {
      setMessage({
        tone: "info",
        text: `Saved all ${dirty} modified buffer${dirty === 1 ? "" : "s"}; the host rebuilds only when the saved build cannot run.`,
      });
    }
    void startRun(requestedConfig);
  }

  function runSavedBuild() {
    setRunDecision(false);
    void startRun(pendingRunConfig);
    setPendingRunConfig(null);
  }

  const cancelRun = () => runCancel({ onSnapshot, setMessage });

  function debugStartParams(requested?: RunConfig) {
    const config = requested ?? activeRunConfig;
    return {
      project: requested ? requested.project : startupProject?.path,
      configuration: config?.configuration ?? evaluation.context.configuration,
      targetFramework: config?.targetFramework ?? evaluation.context.targetFramework,
      args: config?.args ?? [],
      workingDirectory: config?.workingDirectory ?? "",
      env: config?.env ?? {},
      envFiles: config?.envFiles ?? [],
    };
  }

  const waitForHost = (done: (s: Snapshot) => boolean) => waitHost({ onSnapshot, setMessage }, done);

  // One-click Debug: there is no consent question. Clicking Debug enables the experimental
  // macOS arm64 flag for the session, reuses or installs the approved adapter only when it is
  // missing, then lets the host start the session and build implicitly when no current build
  // can launch it. Restore stays explicit.
  async function prepareAndDebug(requested?: RunConfig) {
    const reason = requested && snapshot.trust.granted ? null : debugDisabledReason;
    if (reason) {
      setMessage({ tone: "error", text: `Debug unavailable: ${reason}` });
      return;
    }
    const invalid = invalidRunConfigReason(requested);
    if (invalid) {
      setMessage({ tone: "error", text: invalid });
      openRunConfigDialog(activeRunConfig?.id);
      return;
    }
    if (["building", "preparing", "running", "paused"].includes(snapshot.dap.state)) {
      setMessage({
        tone: "error",
        text: `Debug unavailable: ${debugDisabledReason ?? "a debug session is already active."}`,
      });
      return;
    }
    setBottomTab("debug");
    try {
      let snap = snapshot;
      if (snap.debug.os === "macos" && snap.debug.arch === "aarch64" && !snap.debug.appleSiliconExperimental) {
        snap = await call<Snapshot>("debug.flag", { enabled: true, confirm: true });
        onSnapshot(snap);
      }
      if (snap.debug.adapterState !== "ready") {
        // Reuses the verified executable when present; downloads the approved release otherwise.
        const planned = await call<Snapshot>("debug.plan");
        snap = await call<Snapshot>("debug.install", { planId: planned.debug.plan?.planId, confirm: true });
        onSnapshot(snap);
        if (snap.debug.adapterState === "downloading" || snap.debug.adapterState === "installing" || snap.debug.adapterState === "verifying") {
          snap = await waitForHost((s) => !["downloading", "installing", "verifying"].includes(s.debug.adapterState));
        }
      }
      if (snap.debug.adapterState !== "ready") throw new Error(snap.debug.gateReason ?? snap.debug.adapterMessage);
      onSnapshot(await call<Snapshot>("debug.start", debugStartParams(requested)));
      setMessage(null);
    } catch (error) {
      setMessage({ tone: "error", text: `Debug could not start: ${messageOf(error)}` });
      openConfigOnFieldError(error);
    }
  }

  // Restart calls this directly: the guards in prepareAndDebug read the render that still shows the stopped-by-restart session.
  async function requestDebugStart() {
    setBottomTab("debug");
    try {
      onSnapshot(await call<Snapshot>("debug.start", debugStartParams()));
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

  return {
    buildDecision, setBuildDecision, runDecision, setRunDecision,
    startBuild, requestBuild, saveAllAndBuild, buildSavedContent, cancelBuild,
    startRun, requestRun, saveAllAndRebuild, runSavedBuild, cancelRun,
    prepareAndDebug, requestDebugStart, stopDebug,
  };
}
