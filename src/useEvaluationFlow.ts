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
  commitRunConfigs: (configs: RunConfig[], activeId: string) => Promise<RunConfigStore>;
  onSnapshot: (snapshot: Snapshot) => void;
  setMessage: (message: { tone: "info" | "error"; text: string } | null) => void;
}

// Restore and evaluation: the manual commands and the automatic ones (after Trust, after a restore, after a project
// file changes on disk). Moved out of WorkspaceView unchanged.
export function useEvaluationFlow(c: EvaluationFlowInput) {
  const {
    snapshot, trusted, restoreAvailable, evaluationAvailable, runConfigsLoaded, activeNamedConfig, runConfigStore,
    commitRunConfigs, onSnapshot, setMessage,
  } = c;
  const evaluation = snapshot.evaluation;
  const restore = snapshot.restore;
  const [evaluating, setEvaluating] = useState(false);
  const autoEvaluation = useRef<number | null>(null);
  const autoRestore = useRef<number | null>(null);
  const autoEvaluatedAfterRestore = useRef<number | null>(null);
  const autoReevaluated = useRef<string | null>(null);

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

  // The host's state poll marks the model stale when a project file changes on disk; a Trusted workspace re-evaluates it.
  useEffect(() => {
    if (!trusted || !evaluation.staleReason?.includes("changed on disk")) return;
    const key = `${evaluation.revision}:${evaluation.staleReason}`;
    if (autoReevaluated.current === key) return;
    autoReevaluated.current = key;
    void runEvaluation();
  }, [trusted, evaluation.revision, evaluation.staleReason]);

  // A finished restore makes the old model stale; evaluate once per restore so the tree matches the assets.
  useEffect(() => {
    if (!trusted || restore.state !== "ok" || autoEvaluatedAfterRestore.current === restore.revision) return;
    autoEvaluatedAfterRestore.current = restore.revision;
    void runEvaluation();
  }, [trusted, restore.state, restore.revision]);
  return { evaluating, evaluateContext, runEvaluation, startRestore, cancelRestore };
}
