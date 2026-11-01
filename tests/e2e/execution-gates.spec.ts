import { expect, test } from "@playwright/test";
import { buildDisabledReason, debugDisabledReason, runAvailabilityReason, type GateContext } from "../../src/executionGates";
import type { EvaluationProject, Snapshot } from "../../src/host/client";

// Text of every Build/Run/Debug gate in src/executionGates.ts (pure functions: no host, no browser).
const project = { path: "src/Cli/Cli.csproj" } as EvaluationProject;

function context(patch: {
  trusted?: boolean;
  sdk?: string;
  evaluation?: Record<string, unknown>;
  build?: Record<string, unknown>;
  debug?: Record<string, unknown>;
  app?: Record<string, unknown>;
  ctx?: Partial<GateContext>;
} = {}): GateContext {
  const snapshot = {
    sdk: { project: { state: patch.sdk ?? "resolved" } },
    evaluation: { state: "ok", stale: false, ...patch.evaluation },
    build: {
      revision: 1, state: "ok", exit: 0, sealed: false, stale: false, staleReason: null,
      project: { path: project.path }, configuration: "Debug", ...patch.build,
    },
    debug: { os: "macos", arch: "aarch64", adapterState: "ready", gateReason: null, adapterMessage: "", appleSiliconExperimental: true, ...patch.debug },
    app: { platform: "macos", arch: "arm64", ...patch.app },
  } as unknown as Snapshot;
  return {
    trusted: patch.trusted ?? true,
    snapshot,
    activeConfigEvaluated: true,
    restoreIncomplete: false,
    consoleCandidates: [project],
    startupProject: project,
    activeConfigProjectInvalid: false,
    runRunning: false,
    runBuilding: false,
    debugActive: false,
    debugBuilding: false,
    ...patch.ctx,
  };
}

const run = (c: GateContext) => runAvailabilityReason(c, c.startupProject, c.activeConfigEvaluated, c.activeConfigProjectInvalid);

test("a fully prepared workspace has no gate", () => {
  const c = context();
  expect(buildDisabledReason(c)).toBeNull();
  expect(run(c)).toBeNull();
  expect(debugDisabledReason(c)).toBeNull();
});

test("Build gates name their reason", () => {
  expect(buildDisabledReason(context({ trusted: false }))).toBe("Trust the workspace before building; nothing runs in Restricted mode.");
  expect(buildDisabledReason(context({ sdk: "missing" }))).toBe("Resolve the project SDK before building; the host SDK is never substituted.");
  expect(buildDisabledReason(context({ evaluation: { state: "running" } }))).toBe("Wait for the evaluation to finish before building.");
  expect(buildDisabledReason(context({ evaluation: { stale: true } }))).toBe("The evaluated project model is stale; re-evaluate before building.");
  expect(buildDisabledReason(context({ evaluation: { state: "idle" } }))).toBe("Evaluate the project model before building; the build uses its TargetPath.");
  expect(buildDisabledReason(context({ ctx: { activeConfigEvaluated: false } }))).toBe("The run configuration has not been evaluated yet; re-evaluate the project model.");
  expect(buildDisabledReason(context({ ctx: { restoreIncomplete: true } }))).toBe("Dependencies are not restored; run the explicit restore first.");
  expect(buildDisabledReason(context({ ctx: { consoleCandidates: [], startupProject: null } }))).toBe(
    "No console executable project is available in the current evaluation; libraries and test projects cannot be startup projects.",
  );
  expect(buildDisabledReason(context({ ctx: { activeConfigProjectInvalid: true } }))).toBe(
    "The active run configuration selects a project that is not an evaluated console project; edit the configuration.",
  );
  expect(buildDisabledReason(context({ ctx: { startupProject: null } }))).toBe("More than one console project; choose the startup project explicitly.");
  expect(buildDisabledReason(context({ build: { state: "running" } }))).toBe("A build is already running.");
});

test("Run gates name their reason and do not require a saved build", () => {
  expect(run(context({ trusted: false }))).toBe("Trust the workspace before running; nothing runs in Restricted mode.");
  expect(run(context({ sdk: "missing" }))).toBe("Resolve the project SDK before running; the host SDK is never substituted.");
  expect(run(context({ evaluation: { state: "running" } }))).toBe("Wait for the evaluation to finish before running.");
  expect(run(context({ evaluation: { stale: true } }))).toBe("The evaluated project model is stale; re-evaluate before running.");
  expect(run(context({ evaluation: { state: "idle" } }))).toBe("Evaluate the project model before running; Run needs its evaluated TargetPath.");
  expect(run(context({ ctx: { activeConfigEvaluated: false } }))).toBe("The run configuration has not been evaluated yet; re-evaluate the project model.");
  expect(run(context({ ctx: { consoleCandidates: [], startupProject: null } }))).toBe(
    "No console executable project is available in the current evaluation; libraries and test projects cannot be run.",
  );
  expect(run(context({ ctx: { activeConfigProjectInvalid: true } }))).toBe(
    "The run configuration selects a project that is not an evaluated console project; edit the configuration.",
  );
  expect(run(context({ ctx: { startupProject: null } }))).toBe("More than one console project; choose the startup project explicitly.");
  expect(run(context({ build: { state: "running" } }))).toBe("A build is running; wait for it to finish before running.");
  expect(run(context({ ctx: { runRunning: true } }))).toBe("A program is already running.");
  expect(run(context({ ctx: { runBuilding: true } }))).toBe("Run is already building; wait for it to finish or press Stop.");
  // The host builds implicitly: missing, stale or failed builds are not a UI gate.
  expect(run(context({ build: { revision: 0, state: "idle", exit: null } }))).toBeNull();
  expect(run(context({ build: { sealed: true } }))).toBeNull();
  expect(run(context({ build: { stale: true, staleReason: "sources changed" } }))).toBeNull();
  expect(run(context({ build: { state: "failed", exit: 1 } }))).toBeNull();
  expect(run(context({ build: { project: { path: "other.csproj" } } }))).toBeNull();
});

test("Debug gates name their reason and do not require a saved build", () => {
  expect(debugDisabledReason(context({ trusted: false }))).toBe("Trust the workspace before debugging; nothing runs in Restricted mode.");
  expect(debugDisabledReason(context({ debug: { adapterState: "absent", gateReason: "no adapter" } }))).toBe("The debug adapter is not ready: no adapter");
  expect(debugDisabledReason(context({ debug: { adapterState: "absent", adapterMessage: "missing launcher" } }))).toBe("The debug adapter is not ready: missing launcher");
  expect(debugDisabledReason(context({ debug: { appleSiliconExperimental: false } }))).toBe(
    "Enable the Apple Silicon experimental flag before debugging; nothing was launched.",
  );
  expect(debugDisabledReason(context({ debug: { appleSiliconExperimental: false, arch: "x64" } }))).toBeNull();
  expect(debugDisabledReason(context({ sdk: "missing" }))).toBe("Resolve the project SDK before debugging; the host SDK is never substituted.");
  expect(debugDisabledReason(context({ evaluation: { state: "running" } }))).toBe("Wait for the evaluation to finish before debugging.");
  expect(debugDisabledReason(context({ evaluation: { stale: true } }))).toBe("The evaluated project model is stale; re-evaluate before debugging.");
  expect(debugDisabledReason(context({ evaluation: { state: "idle" } }))).toBe("Evaluate the project model before debugging; Debug needs its managed assembly.");
  expect(debugDisabledReason(context({ ctx: { activeConfigEvaluated: false } }))).toBe("The run configuration has not been evaluated yet; re-evaluate the project model.");
  expect(debugDisabledReason(context({ ctx: { consoleCandidates: [], startupProject: null } }))).toBe(
    "No console executable project is available in the current evaluation.",
  );
  expect(debugDisabledReason(context({ ctx: { activeConfigProjectInvalid: true } }))).toBe(
    "The active run configuration selects a project that is not an evaluated console project; edit the configuration.",
  );
  expect(debugDisabledReason(context({ build: { state: "running" } }))).toBe("A build is running; wait for it to finish before debugging.");
  expect(debugDisabledReason(context({ ctx: { debugBuilding: true } }))).toBe("Debug is already building; wait for it to finish or press Stop.");
  expect(debugDisabledReason(context({ ctx: { debugActive: true } }))).toBe("A debug session is already active.");
  expect(debugDisabledReason(context({ ctx: { startupProject: null } }))).toBe("More than one console project; choose the startup project explicitly.");
  // The host builds implicitly: missing, stale or failed builds are not a UI gate.
  expect(debugDisabledReason(context({ build: { revision: 0, state: "idle", exit: null } }))).toBeNull();
  expect(debugDisabledReason(context({ build: { stale: true, staleReason: "sources changed" } }))).toBeNull();
  expect(debugDisabledReason(context({ build: { state: "failed", exit: 2 } }))).toBeNull();
  expect(debugDisabledReason(context({ build: { configuration: "Release" } }))).toBeNull();
  expect(debugDisabledReason(context({ build: { sealed: true } }))).toBeNull();
  expect(debugDisabledReason(context({ build: { project: { path: "other.csproj" } } }))).toBeNull();
});
