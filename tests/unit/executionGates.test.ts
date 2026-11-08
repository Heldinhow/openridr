import { describe, expect, test } from "bun:test";
import type { EvaluationProject } from "../../src/host/client";
import {
  buildDisabledReason,
  debugDisabledReason,
  runAvailabilityReason,
  type GateContext,
} from "../../src/executionGates";

// The reason the host and the UI name while the automatic discovery builds the test project
// (docs/SPEC-CONCURRENT-BUILDS.md, issue #275).
const discoveryReason =
  "Test discovery is building the test project; wait for it to finish or cancel the discovery.";

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
  overrides: { discovery?: "idle" | "discovering"; build?: "idle" | "running" } = {},
): GateContext {
  return {
    trusted: true,
    snapshot: {
      sdk: { project: { state: "resolved" } },
      evaluation: { state: "ok", stale: false },
      build: { state: overrides.build ?? "idle" },
      tests: { discovery: { state: overrides.discovery ?? "idle" } },
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
