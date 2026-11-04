import { useEffect, useRef, useState } from "react";
import { messageOf, type RunConfig, type RunConfigStore, type Snapshot } from "./host/client";
import { evaluateContext as evaluate } from "./executionCommands";
import {
  cancelRestore as restoreCancel,
  startRestore as restoreStart,
  type RestoreReporter,
} from "./restoreCommands";
import type { RunConfigView } from "./runConfigs";

// What the workspace hands over: read-only render values plus the callbacks that write back.
export interface EvaluationFlowInput {
  snapshot: Snapshot;
  trusted: boolean;
  restoreAvailable: boolean;
  evaluationAvailable: boolean;
  runConfigsLoaded: boolean;
  activeNamedConfig: RunConfigView | null;
  runConfigStore: RunConfigStore | null;
  hasUnsavedBuffers: boolean;
  commitRunConfigs: (configs: RunConfig[], activeId: string) => Promise<RunConfigStore>;
  onSnapshot: (snapshot: Snapshot) => void;
  setMessage: (message: { tone: "info" | "error"; text: string } | null) => void;
}

// Restore and evaluation: the manual commands and the automatic ones (after Trust, after a restore, after a project
// file changes on disk). Moved out of WorkspaceView unchanged.
export function useEvaluationFlow(c: EvaluationFlowInput) {
  const {
    snapshot, trusted, restoreAvailable, evaluationAvailable, runConfigsLoaded, activeNamedConfig, runConfigStore,
    hasUnsavedBuffers, commitRunConfigs, onSnapshot, setMessage,
  } = c;
  const evaluation = snapshot.evaluation;
  const restore = snapshot.restore;
  const [evaluating, setEvaluating] = useState(false);
  const [visible, setVisible] = useState(() => !document.hidden);
  const autoEvaluation = useRef<number | null>(null);
  const autoRestore = useRef<number | null>(null);
  const autoEvaluatedAfterRestore = useRef<number | null>(null);
  const autoReevaluated = useRef<string | null>(null);

  useEffect(() => {
    const onVisibilityChange = () => setVisible(!document.hidden);
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => document.removeEventListener("visibilitychange", onVisibilityChange);
  }, []);

  async function evaluateContext(
    configuration: string,
    targetFramework: string | null,
  ) {
    setEvaluating(true);
    try {
      await evaluate({ onSnapshot, setMessage }, configuration, targetFramework);
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

  const restoreReporter: RestoreReporter = { onSnapshot, setMessage };
  const startRestore = (method: "restore.start" | "restore.retry", project?: string) =>
    restoreStart(restoreReporter, method, project);
  const cancelRestore = () => restoreCancel(restoreReporter);


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

  // docs/decisions.md: a Trusted workspace restores by itself, once per context; Restricted runs nothing.
  useEffect(() => {
    if (!trusted || restore.state !== "idle" || !restoreAvailable) return;
    if (restore.projects.length > 0 && restore.projects.every((project) => project.state === "restored")) return;
    if (autoRestore.current === snapshot.contextId) return;
    autoRestore.current = snapshot.contextId;
    void startRestore("restore.start");
  }, [trusted, restore.state, restoreAvailable, snapshot.contextId]);

  // The host's state poll marks the model stale when a project file or a watched source
  // directory changes on disk. Coalesce a burst of external creates/renames into one
  // evaluation; a hidden window, a dirty buffer or a running build/restore waits.
  useEffect(() => {
    if (!trusted || !evaluation.staleReason?.includes("changed on disk")) return;
    if (!visible || hasUnsavedBuffers || evaluating) return;
    if (
      evaluation.state === "running" ||
      snapshot.build.state === "running" ||
      snapshot.restore.state === "running"
    ) {
      return;
    }
    const key = `${evaluation.revision}:${evaluation.staleReason}`;
    if (autoReevaluated.current === key) return;
    const timer = window.setTimeout(() => {
      if (document.hidden) return;
      autoReevaluated.current = key;
      void runEvaluation();
    }, 500);
    return () => window.clearTimeout(timer);
  }, [
    trusted,
    visible,
    hasUnsavedBuffers,
    evaluating,
    evaluation.state,
    evaluation.revision,
    evaluation.staleReason,
    snapshot.build.state,
    snapshot.restore.state,
  ]);

  // A finished restore makes the old model stale; evaluate once per restore so the tree matches the assets.
  useEffect(() => {
    if (!trusted || restore.state !== "ok" || autoEvaluatedAfterRestore.current === restore.revision) return;
    autoEvaluatedAfterRestore.current = restore.revision;
    void runEvaluation();
  }, [trusted, restore.state, restore.revision]);

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

  return { evaluating, evaluateContext, runEvaluation, startRestore, cancelRestore, selectRunConfig, commitRunConfigDialog };
}
