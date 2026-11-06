import { useEffect, useRef } from "react";
import { call, type Snapshot } from "./host/client";

type Props = {
  snapshot: Snapshot;
  error: string | null;
  onSnapshot: (snapshot: Snapshot) => void;
  onRetry: () => void;
  onCancel: () => void;
};

const activeStates = ["downloading", "installing", "verifying"];

/** The workspace stays unreachable until both fixed tools are verified. Retry and cancel are
 * the only actions; there is no plan to review and no consent checkbox. */
export function SetupGate({ snapshot, error, onSnapshot, onRetry, onCancel }: Props) {
  const setup = snapshot.setup;
  const tool = snapshot.tools["csharp-ls"];
  const debug = snapshot.debug;
  const running = setup.state === "running" || activeStates.includes(tool.state);
  const onSnapshotRef = useRef(onSnapshot);
  onSnapshotRef.current = onSnapshot;

  useEffect(() => {
    if (!running) return;
    let active = true;
    const timer = setInterval(() => {
      call<Snapshot>("workspace.state")
        .then((next) => {
          if (active) onSnapshotRef.current(next);
        })
        .catch(() => {
          // The visible state keeps the last observed step; the next poll retries.
        });
    }, 400);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [running]);

  const debugReady = debug.adapterState === "ready";
  const toolReady = tool.state === "ready";
  const runtimeReady = setup.runtime.state === "found";
  const progress = tool.progress;
  const failure = error ?? setup.error;

  return (
    <main className="setup-screen setup-gate-screen" data-testid="setup-gate">
      <header className="chrome">
        <span className="brand">Setting up {snapshot.workspace?.name ?? "the project"}</span>
        <span
          className={`tag ${setup.state === "failed" ? "error" : setup.state === "ready" ? "" : "warning"}`}
          data-testid="setup-gate-state"
          role="status"
        >
          {setup.state === "ready"
            ? "Ready"
            : setup.state === "failed"
              ? "Setup failed"
              : setup.state === "blocked"
                ? "Runtime needed"
                : "Installing tools…"}
        </span>
      </header>

      <section className="setup-gate-body" aria-labelledby="setup-gate-title">
        <h1 id="setup-gate-title">Fixed tools are part of the project setup</h1>
        <p className="muted">
          OpenRidr verifies these tools once, outside the workspace. csharp-ls and the .NET
          runtime that runs it ship inside the app, next to netcoredbg; nothing of the project is
          executed during setup, and the workspace is not shown until both tools are ready.
          Version and checksum never change between projects.
        </p>

        <ol className="setup-gate-steps">
          <li data-testid="setup-step-analysis" data-state={tool.state}>
            <strong>Analysis tool · csharp-ls {tool.version}</strong>
            <span className="menu-note">
              {tool.state === "ready"
                ? tool.bundled
                  ? "Bundled and verified"
                  : "Verified"
                : activeStates.includes(tool.state)
                  ? tool.state === "downloading"
                    ? "Downloading the pinned package"
                    : tool.state === "installing"
                      ? "Installing into the app home"
                      : "Verifying the launcher"
                  : tool.state === "failed"
                    ? "Failed"
                    : "Not installed yet"}
            </span>
            {progress && activeStates.includes(tool.state) && (
              <span className="muted" data-testid="setup-gate-progress">
                Downloaded {progress.downloaded} of{" "}
                {progress.total === null ? "an unknown total" : `${progress.total}`} bytes
                {progress.total === null || progress.total === 0
                  ? " (measured; no percentage)"
                  : ` (${Math.min(100, Math.floor((progress.downloaded / progress.total) * 100))}%)`}
              </span>
            )}
            {tool.state === "ready" && (
              <span className="muted">
                <code data-testid="setup-gate-tool-destination">{tool.destination}</code>
                {tool.sha256 && (
                  <>
                    {" "}
                    · SHA-256 <code data-testid="setup-gate-tool-sha256">{tool.sha256}</code>
                  </>
                )}
              </span>
            )}
          </li>
          <li data-testid="setup-step-debug" data-state={debug.adapterState}>
            <strong>Debug adapter · netcoredbg {debug.version}</strong>
            <span className="menu-note">
              {debugReady
                ? "Bundled and verified"
                : debug.adapterState === "experimental_disabled"
                  ? "Bundled adapter could not be enabled"
                  : "Not available"}
            </span>
            {debug.bundled && (
              <span className="muted">
                <code data-testid="setup-gate-debug-binary">{debug.installedBinary}</code>
              </span>
            )}
          </li>
          <li data-testid="setup-step-runtime" data-state={setup.runtime.state}>
            <strong>Analysis runtime · .NET 10</strong>
            <span className="menu-note">
              {runtimeReady
                ? setup.runtime.bundled
                  ? `Bundled ${setup.runtime.version ?? ""} with the app`
                  : `Found ${setup.runtime.version ?? ""} at ${setup.runtime.root ?? ""}`
                : "Missing"}
            </span>
            {setup.runtime.bundled && setup.runtime.root && (
              <span className="muted">
                <code data-testid="setup-gate-runtime-bundled">{setup.runtime.root}</code>
              </span>
            )}
          </li>
        </ol>

        {!runtimeReady && (
          <div className="notice warning" data-testid="setup-gate-runtime">
            <p>{setup.runtime.message}</p>
            {!setup.runtime.bundled && (
              <>
                <a
                  href={setup.runtime.instructionsUrl}
                  target="_blank"
                  rel="noreferrer noopener"
                  data-testid="setup-gate-runtime-link"
                >
                  Get the .NET 10 runtime
                </a>
                <p className="muted">
                  Install it from that page, then retry. OpenRidr never installs an SDK or edits
                  global.json.
                </p>
              </>
            )}
          </div>
        )}

        {failure && runtimeReady && (
          <p className="error" role="alert" data-testid="setup-gate-error">
            {failure}
          </p>
        )}

        <div className="actions">
          <button
            className="primary"
            data-testid="setup-gate-retry"
            disabled={running}
            onClick={onRetry}
            title={
              running
                ? "Setup is already running."
                : "Verifies again; an installation that is already valid is reused without downloading."
            }
          >
            {running ? "Setting up…" : "Retry setup"}
          </button>
          <button
            data-testid="setup-gate-cancel"
            onClick={onCancel}
            title="Closes the workspace; a running install is stopped and the verified tools stay installed for the next project."
          >
            Back to projects
          </button>
        </div>

        {(tool.log || error || tool.error) && (
          <details className="eval-log" open={setup.state === "failed"}>
            <summary>Setup log</summary>
            <pre className="eval-pre" data-testid="setup-gate-log">
              {tool.log || "No output yet."}
            </pre>
          </details>
        )}
        {toolReady && !debugReady && (
          <p className="muted" data-testid="setup-gate-debug-reason">
            {debug.adapterMessage}
          </p>
        )}
      </section>
    </main>
  );
}
