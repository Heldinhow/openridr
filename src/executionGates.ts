import type { Build, Evaluation, EvaluationProject, ProjectSdk, TestsDiscovery } from "./host/client";

/** The snapshot fields the gates read; a full `Snapshot` satisfies it, so the workspace passes the
 * whole state and the unit test builds only what a decision depends on. */
export type GateSnapshot = {
  sdk: { project: { state: ProjectSdk["state"] } };
  evaluation: { state: Evaluation["state"]; stale: boolean };
  build: { state: Build["state"] };
  tests: { discovery: { state: TestsDiscovery["state"] } };
};

// The host refuses the build with the same reason while its own automation builds the project.
const discoveryReason =
  "Test discovery is building the test project; wait for it to finish or cancel the discovery.";

// Pure Build/Run/Debug gates: the workspace gathers the inputs, these only decide the disabled reason.
export interface GateContext {
  trusted: boolean;
  snapshot: GateSnapshot;
  activeConfigEvaluated: boolean;
  restoreIncomplete: boolean;
  consoleCandidates: EvaluationProject[];
  startupProject: EvaluationProject | null;
  // The active named configuration points at a project that is not an evaluated console project.
  activeConfigProjectInvalid: boolean;
  runRunning: boolean;
  runBuilding: boolean;
  debugActive: boolean;
  debugBuilding: boolean;
}

export function buildDisabledReason(c: GateContext) {
  const { evaluation, tests } = c.snapshot;
  return !c.trusted
    ? "Trust the workspace before building; nothing runs in Restricted mode."
    : c.snapshot.sdk.project.state !== "resolved"
      ? "Resolve the project SDK before building."
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
                      : tests.discovery.state === "discovering"
                        ? discoveryReason
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
  const { evaluation, build, tests } = c.snapshot;
  return !c.trusted
    ? "Trust the workspace before running; nothing runs in Restricted mode."
    : c.snapshot.sdk.project.state !== "resolved"
      ? "Resolve the project SDK before running."
      : evaluation.state === "running"
        ? "Wait for the evaluation to finish before running."
        : evaluation.stale
          ? "The evaluated project model is stale; re-evaluate before running."
          : !["ok", "partial"].includes(evaluation.state)
            ? "Evaluate the project model before running; Run needs its evaluated TargetPath."
            : !configEvaluated
              ? "The run configuration has not been evaluated yet; re-evaluate the project model."
              : c.consoleCandidates.length === 0
                ? "No console executable project is available in the current evaluation; libraries and test projects cannot be run."
                : invalidSelectedProject
                  ? "The run configuration selects a project that is not an evaluated console project; edit the configuration."
                  : !project
                    ? "More than one console project; choose the startup project explicitly."
                    : c.runBuilding
                      ? "Run is already building; wait for it to finish or press Stop."
                      : tests.discovery.state === "discovering"
                        ? discoveryReason
                        : build.state === "running"
                          ? "A build is running; wait for it to finish before running."
                          : c.runRunning
                            ? "A program is already running."
                            : null;
}

export function debugDisabledReason(c: GateContext) {
  const { evaluation, build, tests } = c.snapshot;
  // The automatic Debug flow enables the experimental flag and installs the adapter itself,
  // so neither is a UI gate; the host still validates both before launching.
  return !c.trusted
    ? "Trust the workspace before debugging; nothing runs in Restricted mode."
    : c.snapshot.sdk.project.state !== "resolved"
      ? "Resolve the project SDK before debugging."
      : evaluation.state === "running"
        ? "Wait for the evaluation to finish before debugging."
        : evaluation.stale
          ? "The evaluated project model is stale; re-evaluate before debugging."
          : !["ok", "partial"].includes(evaluation.state)
            ? "Evaluate the project model before debugging; Debug needs its managed assembly."
            : !c.activeConfigEvaluated
              ? "The run configuration has not been evaluated yet; re-evaluate the project model."
              : c.consoleCandidates.length === 0
                ? "No console executable project is available in the current evaluation."
                : c.activeConfigProjectInvalid
                  ? "The active run configuration selects a project that is not an evaluated console project; edit the configuration."
                  : !c.startupProject
                    ? "More than one console project; choose the startup project explicitly."
                    : c.debugBuilding
                      ? "Debug is already building; wait for it to finish or press Stop."
                      : tests.discovery.state === "discovering"
                        ? discoveryReason
                        : build.state === "running"
                          ? "A build is running; wait for it to finish before debugging."
                          : c.debugActive
                            ? "A debug session is already active."
                            : null;
}
