import type { Run } from "./host/client";
import { runStateLabel, timeLabel } from "./BuildPanel";

// Run is its own bottom tool window: program output first, metadata folded in details.
export function RunPanel({ run, onCancelRun }: { run: Run; onCancelRun: () => void }) {
  const running = run.state === "running" || run.state === "building";
  return (
    <div className="run-window">
      <div className="run-window-head">
        <span
          className={run.revision === 0 ? "muted" : `tag ${run.state === "failed" ? "error" : ""}`}
          data-testid="run-state"
          role="status"
        >
          {runStateLabel(run)}
          {run.stale ? " · stale" : ""}
        </span>
        {running && (
          <button
            data-testid="run-stop"
            onClick={onCancelRun}
            title="Requests termination of the program's process group, children included, and reports what was observed. While the host is building, it also stops that build."
          >
            Stop
          </button>
        )}
      </div>
      {run.revision === 0 && (
        <p className="muted" data-testid="run-empty">
          Run starts the saved build here; when there is no current build the host builds the
          startup project first and its output stays in the Build tab.
        </p>
      )}
      {run.state === "building" && (
        <p className="muted" data-testid="run-building">
          Building the startup project before the program starts. Build output and diagnostics
          stay in the Build tab; Stop cancels the build.
        </p>
      )}
      {run.revision > 0 && (
    <section className="run-output" aria-label="Run output" data-testid="run-panel">
      <details className="build-context-details">
        <summary>Run details</summary>
        <dl className="restore-meta build-meta">
        <dt>Run</dt>
        <dd data-testid="run-identity">
          {run.runId ?? "unknown"} · project {run.project?.name ?? "unknown"} ·{" "}
          <code>{run.targetPath ?? "unknown"}</code>
        </dd>
        <dt>Build identity</dt>
        <dd data-testid="run-build-id">
          {run.buildId ?? "unknown"} · revision {run.revision} · context{" "}
          {run.contextId ?? "unknown"}
        </dd>
        <dt>Configuration / TFM</dt>
        <dd>
          {run.configuration} · {run.targetFramework ?? "single-target project"}
        </dd>
        <dt>Command</dt>
        <dd>
          <code className="detail-command" data-testid="run-command">{run.command.join(" ")}</code>
        </dd>
        <dt>Working directory</dt>
        <dd>
          <code data-testid="run-cwd">{run.cwd}</code>
        </dd>
        <dt>Exit</dt>
        <dd data-testid="run-exit">
          {run.exit === null ? "exit unknown" : `exit ${run.exit}`}
        </dd>
        <dt>Started / finished</dt>
        <dd>
          {timeLabel(run.startedAt)} / {timeLabel(run.finishedAt)}
        </dd>
        </dl>
      </details>
      {run.sealed && (
        <p className="warning" data-testid="run-sealed">
          Previous run, not current: {run.staleReason}
        </p>
      )}
      {!run.sealed && run.stale && (
        <p className="warning" data-testid="run-stale" role="status">
          Run result is stale: {run.staleReason}
        </p>
      )}
      {run.termination && (
        <p className="muted" data-testid="run-termination">
          Termination requested: {run.termination.requested}. Observed:{" "}
          {run.termination.observed}
          {run.termination.killedAfterGrace ? " after the grace period" : ""}.
        </p>
      )}
      {run.awaitingDecision && (
        <p className="warning" data-testid="run-awaiting">
          Still running; the host has not confirmed termination. Decide in the process panel.
        </p>
      )}
      <h3>Run output</h3>
      <pre className="eval-pre build-log run-log" data-testid="run-log">
        {run.log || "No output yet."}
      </pre>
      {run.exit !== null && !run.stale && (
        <p className="process-finished">Process finished with exit code {run.exit}.</p>
      )}
      {run.logTruncated && (
        <p className="muted" data-testid="run-log-truncated">
          The run log is bounded and the earliest lines were dropped; the note at the top of
          the log says how many.
        </p>
      )}
    </section>
          )}
    </div>
  );
}
