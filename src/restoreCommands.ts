import { call, messageOf, type Snapshot } from "./host/client";

// Host calls for restore. The workspace owns the snapshot and the message line; these only report into them.
export interface RestoreReporter {
  onSnapshot: (snapshot: Snapshot) => void;
  setMessage: (message: { tone: "info" | "error"; text: string } | null) => void;
}

export async function startRestore(
  reporter: RestoreReporter,
  method: "restore.start" | "restore.retry",
  project?: string,
) {
  try {
    reporter.onSnapshot(await call<Snapshot>(method, project ? { project } : {}));
    reporter.setMessage(null);
  } catch (error) {
    reporter.setMessage({ tone: "error", text: `Restore could not start: ${messageOf(error)}` });
  }
}

export async function cancelRestore(reporter: RestoreReporter) {
  try {
    reporter.onSnapshot(await call<Snapshot>("restore.cancel"));
  } catch (error) {
    reporter.setMessage({ tone: "error", text: `Restore cancellation failed: ${messageOf(error)}` });
  }
}
