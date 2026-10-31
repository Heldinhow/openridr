import type { EvaluationProject, Snapshot } from "./host/client";

// Pure Build/Run/Debug gates: the workspace gathers the inputs, these only decide the disabled reason.
export interface GateContext {
  trusted: boolean;
  snapshot: Snapshot;
  activeConfigEvaluated: boolean;
  restoreIncomplete: boolean;
  consoleCandidates: EvaluationProject[];
  startupProject: EvaluationProject | null;
  // The active named configuration points at a project that is not an evaluated console project.
  activeConfigProjectInvalid: boolean;
  runRunning: boolean;
  savedSourceAfterBuild: boolean;
  debugActive: boolean;
}

// Why the saved build cannot be used for `verb`, or null when it can.
function savedBuildReason(
  { snapshot: { build }, savedSourceAfterBuild }: GateContext,
  project: EvaluationProject,
  verb: string,
  needsDebug = false,
) {
  return build.revision === 0
    ? needsDebug
      ? "Build the startup project in Debug first; Debug never builds or restores implicitly."
      : "Build the startup project first; Run never builds or restores implicitly."
    : needsDebug && build.configuration.toLowerCase() !== "debug"
      ? "The saved build is not a Debug build; build with Debug before debugging."
      : build.sealed
        ? `The saved build belongs to a previous context; rebuild before ${verb}.`
        : build.stale
          ? `The saved build is stale: ${(build.staleReason ?? "no current build").replace(/\.$/, "")}.`
          : build.state !== "ok" || build.exit !== 0
            ? `The last build did not succeed (${
                build.exit === null ? "exit unknown" : `exit ${build.exit}`
              }); fix it and rebuild.`
            : build.project?.path !== project.path
              ? "The saved build is for another startup project; build the selected one."
              : savedSourceAfterBuild
                ? `Sources were saved after this build; rebuild before ${verb}.`
                : null;
}

export function buildDisabledReason(c: GateContext) {
  const { evaluation } = c.snapshot;
  return !c.trusted
    ? "Trust the workspace before building; nothing runs in Restricted mode."
    : c.snapshot.sdk.project.state !== "resolved"
      ? "Resolve the project SDK before building; the host SDK is never substituted."
      : evaluation.state === "running"
        ? "Wait for the evaluation to finish before building."
        : evaluation.stale
          ? "The evaluated project model is stale; re-evaluate before building."
          : !["ok", "partial"].includes(evaluation.state)
            ? "Evaluate the project model before building; the build uses its TargetPath."
            : !c.activeConfigEvaluated
              ? "The run configuration has not been evaluated yet; re-evaluate the project model."
              : c.restoreIncomplete
                ? "Dependencies are not restored; run the explicit restore first."
                : c.consoleCandidates.length === 0
                  ? "No console executable project is available in the current evaluation; libraries and test projects cannot be startup projects."
                  : c.activeConfigProjectInvalid
                    ? "The active run configuration selects a project that is not an evaluated console project; edit the configuration."
                    : !c.startupProject
                      ? "More than one console project; choose the startup project explicitly."
                      : c.snapshot.build.state === "running"
                        ? "A build is already running."
                        : null;
}

export function runAvailabilityReason(
  c: GateContext,
  project: EvaluationProject | null,
  configEvaluated: boolean,
  invalidSelectedProject = false,
) {
  const { evaluation, build } = c.snapshot;
  return !c.trusted
    ? "Trust the workspace before running; nothing runs in Restricted mode."
    : c.snapshot.sdk.project.state !== "resolved"
      ? "Resolve the project SDK before running; the host SDK is never substituted."
      : evaluation.state === "running"
        ? "Wait for the evaluation to finish before running."
        : evaluation.stale
          ? "The evaluated project model is stale; re-evaluate before running."
          : !["ok", "partial"].includes(evaluation.state)
            ? "Evaluate the project model before running; Run uses the saved build's TargetPath."
            : !configEvaluated
              ? "The run configuration has not been evaluated yet; re-evaluate the project model."
              : c.consoleCandidates.length === 0
                ? "No console executable project is available in the current evaluation; libraries and test projects cannot be run."
                : invalidSelectedProject
                  ? "The run configuration selects a project that is not an evaluated console project; edit the configuration."
                  : !project
                    ? "More than one console project; choose the startup project explicitly."
                    : build.state === "running"
                      ? "A build is running; wait for it to finish before running."
                      : c.runRunning
                        ? "A program is already running."
                        : savedBuildReason(c, project, "running");
}

export function debugDisabledReason(c: GateContext) {
  const { evaluation, build, debug, app } = c.snapshot;
  return !c.trusted
    ? "Trust the workspace before debugging; nothing runs in Restricted mode."
    : debug.adapterState !== "ready"
      ? `The debug adapter is not ready: ${debug.gateReason ?? debug.adapterMessage}`
      : app.platform === "macos" && app.arch === "arm64" && !debug.appleSiliconExperimental
        ? "Enable the Apple Silicon experimental flag before debugging; nothing was launched."
        : c.snapshot.sdk.project.state !== "resolved"
          ? "Resolve the project SDK before debugging; the host SDK is never substituted."
          : evaluation.state === "running"
            ? "Wait for the evaluation to finish before debugging."
            : evaluation.stale
              ? "The evaluated project model is stale; re-evaluate before debugging."
              : !["ok", "partial"].includes(evaluation.state)
                ? "Evaluate the project model before debugging; Debug uses its managed assembly."
                : !c.activeConfigEvaluated
                  ? "The run configuration has not been evaluated yet; re-evaluate the project model."
                  : c.consoleCandidates.length === 0
                    ? "No console executable project is available in the current evaluation."
                    : c.activeConfigProjectInvalid
                      ? "The active run configuration selects a project that is not an evaluated console project; edit the configuration."
                      : !c.startupProject
                        ? "More than one console project; choose the startup project explicitly."
                        : build.state === "running"
                          ? "A build is running; wait for it to finish before debugging."
                          : (savedBuildReason(c, c.startupProject, "debugging", true) ??
                            (c.debugActive ? "A debug session is already active." : null));
}
