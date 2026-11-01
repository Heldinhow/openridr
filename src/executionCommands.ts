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
