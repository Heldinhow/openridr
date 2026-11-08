import { describe, expect, test } from "bun:test";
import type { EvaluationProject, Lsp } from "../../src/host/client";
import {
  buildDisabledReason,
  debugDisabledReason,
  runAvailabilityReason,
  type GateContext,
} from "../../src/executionGates";

// The reason the host and the UI name while the automatic discovery builds the test project
// (issue #275).
const discoveryReason =
  "Test discovery is building the test project; wait for it to finish or cancel the discovery.";

// The reason both sides name while the analysis is still loading the solution (#286).
const analysisReason = "Analysis is loading the solution; wait for it to finish or stop analysis.";

const consoleProject: EvaluationProject = {
  name: "OrderDesk.Cli",
  path: "src/OrderDesk.Cli/OrderDesk.Cli.csproj",
  folder: null,
  state: "ok",
  kind: "console",
  tfms: ["net8.0"],
  evaluatedTfm: "net8.0",
  targetPath: "src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll",
  assemblyPath: null,
  outputType: "Exe",
  projectRefs: [],
  packageRefs: [],
  sources: [],
  error: null,
  log: "",
  exit: 0,
  restored: "restored",
  restoredReason: null,
  note: null,
};

function gates(
  overrides: {
    discovery?: "idle" | "discovering";
    build?: "idle" | "running";
    lsp?: Lsp["state"];
    workspace?: "solution" | "project";
  } = {},
): GateContext {
  return {
    trusted: true,
    snapshot: {
      sdk: { project: { state: "resolved" } },
      evaluation: { state: "ok", stale: false },
      build: { state: overrides.build ?? "idle" },
      tests: { discovery: { state: overrides.discovery ?? "idle" } },
      lsp: { state: overrides.lsp ?? "ready" },
      workspace: { kind: overrides.workspace ?? "solution" },
    },
    activeConfigEvaluated: true,
    restoreIncomplete: false,
    consoleCandidates: [consoleProject],
    startupProject: consoleProject,
    activeConfigProjectInvalid: false,
    runRunning: false,
    runBuilding: false,
    debugActive: false,
    debugBuilding: false,
  };
}

describe("the execution gates name the automatic discovery before the click", () => {
  test("Build is disabled with the discovery reason while it runs", () => {
    expect(buildDisabledReason(gates({ discovery: "discovering" }))).toBe(discoveryReason);
  });

  test("Run and Debug name the same reason, because both build implicitly", () => {
    const discovering = gates({ discovery: "discovering" });
    expect(runAvailabilityReason(discovering, consoleProject, true)).toBe(discoveryReason);
    expect(debugDisabledReason(discovering)).toBe(discoveryReason);
  });

  test("nothing is disabled when no automation is running", () => {
    expect(buildDisabledReason(gates())).toBeNull();
    expect(runAvailabilityReason(gates(), consoleProject, true)).toBeNull();
    expect(debugDisabledReason(gates())).toBeNull();
  });
});

describe("the execution gates name the analysis that is still loading", () => {
  test("Build, Run and Debug are disabled with the analysis reason while it loads", () => {
    const loading = gates({ lsp: "loading" });
    expect(buildDisabledReason(loading)).toBe(analysisReason);
    expect(runAvailabilityReason(loading, consoleProject, true)).toBe(analysisReason);
    expect(debugDisabledReason(loading)).toBe(analysisReason);
  });

  test("a loaded analysis never blocks, and neither does an absent or starting one", () => {
    for (const state of [
      "ready",
      "partial",
      "awaiting_diagnostics",
      "stopped",
      "unavailable",
      "starting",
      "failed",
    ] as const) {
      const loaded = gates({ lsp: state });
      expect(buildDisabledReason(loaded)).toBeNull();
      expect(runAvailabilityReason(loaded, consoleProject, true)).toBeNull();
      expect(debugDisabledReason(loaded)).toBeNull();
    }
  });

  test("the running build and the discovery win over the loading analysis", () => {
    expect(buildDisabledReason(gates({ build: "running", lsp: "loading" }))).toBe(
      "A build is already running.",
    );
    expect(buildDisabledReason(gates({ discovery: "discovering", lsp: "loading" }))).toBe(
      discoveryReason,
    );
    expect(runAvailabilityReason(gates({ build: "running", lsp: "loading" }), consoleProject, true)).toBe(
      "A build is running; wait for it to finish before running.",
    );
    expect(debugDisabledReason(gates({ build: "running", lsp: "loading" }))).toBe(
      "A build is running; wait for it to finish before debugging.",
    );
    expect(buildDisabledReason(gates({ discovery: "discovering", build: "running", lsp: "loading" }))).toBe(
      discoveryReason,
    );
  });

  test("a project workspace never waits for the analysis, which never finishes loading there", () => {
    const project = gates({ lsp: "loading", workspace: "project" });
    expect(buildDisabledReason(project)).toBeNull();
    expect(runAvailabilityReason(project, consoleProject, true)).toBeNull();
    expect(debugDisabledReason(project)).toBeNull();
  });

  test("the loading analysis wins over a program or debug session that is already up", () => {
    const running = gates({ lsp: "loading" });
    running.runRunning = true;
    running.debugActive = true;
    expect(runAvailabilityReason(running, consoleProject, true)).toBe(analysisReason);
    expect(debugDisabledReason(running)).toBe(analysisReason);
  });
});
