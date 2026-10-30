import { call, messageOf, type Snapshot } from "./host/client";
import type { RestoreReporter } from "./restoreCommands";

// Host calls for evaluation, build and run. The workspace owns the snapshot and the message line;
// these only report into them.
export type ExecutionReporter = RestoreReporter;

export async function evaluateContext(
  reporter: ExecutionReporter,
  configuration: string,
  targetFramework: string | null,
) {
  try {
    reporter.onSnapshot(await call<Snapshot>("evaluation.run", { configuration, targetFramework }));
    reporter.setMessage(null);
  } catch (error) {
    reporter.setMessage({ tone: "error", text: `Evaluation could not run: ${messageOf(error)}` });
    try {
      reporter.onSnapshot(await call<Snapshot>("workspace.state"));
    } catch {
      // The message above already explains the failure.
    }
  }
}

export async function cancelBuild(reporter: ExecutionReporter) {
  try {
    reporter.onSnapshot(await call<Snapshot>("build.cancel"));
  } catch (error) {
    reporter.setMessage({ tone: "error", text: `Build cancellation failed: ${messageOf(error)}` });
  }
}

export async function cancelRun(reporter: ExecutionReporter) {
  try {
    reporter.onSnapshot(await call<Snapshot>("run.cancel"));
  } catch (error) {
    reporter.setMessage({ tone: "error", text: `Run cancellation failed: ${messageOf(error)}` });
  }
}

export async function waitForHost(reporter: ExecutionReporter, done: (s: Snapshot) => boolean) {
  for (let i = 0; i < 480; i++) {
    const next = await call<Snapshot>("workspace.state");
    reporter.onSnapshot(next);
    if (done(next)) return next;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error("timed out waiting for the host");
}

interface LaunchConfig {
  configuration: string;
  targetFramework: string | null;
  args: string[];
  workingDirectory: string;
  env: Record<string, string>;
  envFiles: string[];
}

export function launchParams(
  config: Partial<LaunchConfig> | null,
  project: string,
  configuration: string,
  targetFramework: string | null,
) {
  return {
    project,
    configuration,
    targetFramework,
    args: config?.args ?? [],
    workingDirectory: config?.workingDirectory ?? "",
    env: config?.env ?? {},
    envFiles: config?.envFiles ?? [],
  };
}

// Rider-style preparation shared by one-click Run and Debug: restore if needed, evaluate the
// project model and build. Every host gate still applies; nothing here bypasses trust.
export async function prepareBuild(
  reporter: ExecutionReporter,
  snapshot: Snapshot,
  config: { configuration: string; targetFramework: string | null } | null,
  verb: "Run" | "Debug",
) {
  let snap = snapshot;
  if (snap.restore.state !== "ok" && !(snap.restore.projects.length > 0 && snap.restore.projects.every((p) => p.state === "restored"))) {
    reporter.setMessage({ tone: "info", text: `Preparing ${verb}: restoring packages…` });
    reporter.onSnapshot(await call<Snapshot>("restore.start", {}));
    snap = await waitForHost(reporter, (s) => s.restore.state !== "running");
    if (snap.restore.state !== "ok") throw new Error("restore did not succeed; see the Build tab.");
  }
  const consoleOf = (s: Snapshot) => {
    const ready = s.evaluation.projects.filter((p) => p.kind === "console" && p.state === "ok" && p.targetPath);
    return ready.length === 1 ? ready[0] : null;
  };
  const configuration = config?.configuration ?? "Debug";
  const context = snap.evaluation.context;
  if (!consoleOf(snap) || snap.evaluation.stale || context.configuration !== configuration) {
    reporter.setMessage({ tone: "info", text: `Preparing ${verb}: evaluating the project model…` });
    reporter.onSnapshot(await call<Snapshot>("evaluation.run", {
      configuration,
      targetFramework: config?.targetFramework ?? context.targetFramework ?? (snap.evaluation.availableTfms.length === 1 ? snap.evaluation.availableTfms[0] : null),
    }));
    snap = await waitForHost(reporter, (s) => s.evaluation.state !== "running");
  }
  const project = consoleOf(snap);
  if (!project) throw new Error("no single evaluated console project; choose a startup project or TFM first.");
  reporter.setMessage({ tone: "info", text: `Preparing ${verb}: building…` });
  reporter.onSnapshot(await call<Snapshot>("build.start", {
    project: project.path,
    configuration,
    targetFramework: snap.evaluation.context.targetFramework,
    savedNote: `Built by ${verb}; saved content only.`,
  }));
  snap = await waitForHost(reporter, (s) => s.build.state !== "running");
  if (snap.build.state !== "ok" || snap.build.exit !== 0) throw new Error(`the ${verb} build failed; see the Build tab.`);
  return { snap, project, configuration };
}
