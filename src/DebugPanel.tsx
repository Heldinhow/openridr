import { DebugVariable } from "./DebugVariable";
import { Icon } from "./Icon";
import { useState } from "react";
import {
  call,
  HostCallError,
  type Dap,
  type Snapshot,
} from "./host/client";

type Props = {
  snapshot: Snapshot;
  disabledReason: string | null;
  onLaunch: () => void;
  onSnapshot: (snapshot: Snapshot) => void;
  onError: (message: string) => void;
};

function messageOf(error: unknown): string {
  if (error instanceof HostCallError || error instanceof Error) return error.message;
  return String(error);
}

const stateLabels: Record<Dap["state"], string> = {
  idle: "Not started",
  preparing: "Preparing",
  running: "Running",
  paused: "Paused",
  terminated: "Terminated",
  cancelled: "Cancelled",
  failed: "Failed",
};

export function DebugPanel({
  snapshot,
  disabledReason,
  onLaunch,
  onSnapshot,
  onError,
}: Props) {
  const dap = snapshot.dap;
  const [busy, setBusy] = useState<string | null>(null);
  const active = dap.state === "preparing" || dap.state === "running" || dap.state === "paused";
  const paused = dap.state === "paused";
  const terminal = ["terminated", "cancelled", "failed"].includes(dap.state);
  const phaseLabel = dap.phase.replace(
    new RegExp(`^${stateLabels[dap.state]}\\s*(?:[·:]\\s*)?`, "i"),
    "",
  );

  async function invoke(label: string, method: string, params: Record<string, unknown> = {}) {
    setBusy(label);
    try {
      onSnapshot(await call<Snapshot>(method, params));
    } catch (error) {
      onError(`${label} failed: ${messageOf(error)}`);
    } finally {
      setBusy(null);
    }
  }

  return (
    <section className="debug-panel" aria-label="Debug session" data-testid="dap-panel">
      <div className="panel-title">
        <strong>Debug</strong>
        <span
          className={`tag ${dap.state === "failed" ? "error" : ""}`}
          data-testid="dap-state"
          role="status"
        >
          {stateLabels[dap.state]}
        </span>
        {phaseLabel !== "" && (
          <span className="muted" data-testid="dap-phase">
            {phaseLabel}
          </span>
        )}
        <span className="push" />
        {!active && (
          <button
            data-testid="debug-launch"
            className="toolbar-icon-button"
            aria-label="Start debugging"
            disabled={disabledReason !== null || busy !== null}
            onClick={onLaunch}
            title={
              disabledReason ??
              `Starts the evaluated managed assembly under the verified adapter (${snapshot.app.platform === "windows" ? "Shift+F9" : "Ctrl+D"}).`
            }
          >
            <Icon name="debug" />
          </button>
        )}
        {active && (
          <button
            data-testid="debug-stop"
            className="toolbar-icon-button"
            aria-label="Stop"
            disabled={busy !== null}
            onClick={() => void invoke("Debug stop", "debug.stop")}
            title="Sends DAP disconnect, terminates the adapter group and reports what was observed; no cleanup is presumed."
          >
            <Icon name="stop" />
          </button>
        )}
        {paused && (
          <>
            <button
              data-testid="debug-continue"
            className="toolbar-icon-button"
            aria-label="Continue"
              disabled={busy !== null}
              onClick={() => void invoke("Continue", "debug.continue")}
              title="Resumes every thread (F5)."
            >
              <Icon name="resume" />
            </button>
            <button
              data-testid="debug-step-over"
            className="toolbar-icon-button"
            aria-label="Step over"
              disabled={busy !== null}
              onClick={() => void invoke("Step over", "debug.stepOver")}
              title="Steps over the current line (F8)."
            >
              <Icon name="stepOver" />
            </button>
            <button
              data-testid="debug-step-into"
            className="toolbar-icon-button"
            aria-label="Step into"
              disabled={busy !== null}
              onClick={() => void invoke("Step into", "debug.stepInto")}
              title="Steps into the current call (F7)."
            >
              <Icon name="stepInto" />
            </button>
            <button
              data-testid="debug-step-out"
            className="toolbar-icon-button"
            aria-label="Step out"
              disabled={busy !== null}
              onClick={() => void invoke("Step out", "debug.stepOut")}
              title="Steps out of the current function."
            >
              <Icon name="stepOut" />
            </button>
          </>
        )}
      </div>

      {disabledReason && !active && (
        <p className="muted" data-testid="dap-disabled-reason">
          {disabledReason}
        </p>
      )}
      {terminal && (
        <p className="muted" data-testid="dap-terminal-note">
          {dap.phase.replace(/[.]+$/, "")}. Logs and debuggee output below are preserved from this session.
        </p>
      )}
      {dap.sealed && (
        <p className="warning" data-testid="dap-sealed">
          Previous session, not current: {dap.staleReason}
        </p>
      )}

      {dap.termination && (
        <p className="muted" data-testid="dap-termination">
          Termination requested: {dap.termination.requested}. Observed:{" "}
          {dap.termination.observed}
          {dap.termination.killedAfterGrace ? " after the grace period" : ""}; exit{" "}
          {dap.termination.exit === null ? "unknown" : dap.termination.exit}.
        </p>
      )}

      <div className="debug-columns">
        <section aria-label="Threads" data-testid="debug-threads">
          <h3>Threads</h3>
          {dap.threads.length === 0 ? (
            <p className="muted">No threads.</p>
          ) : (
            <ul data-testid="dap-thread-list">
              {dap.threads.map((thread) => (
                <li key={thread.id} data-testid={`debug-thread-${thread.id}`}>
                  {thread.name} <span className="muted">({thread.id})</span>
                </li>
              ))}
            </ul>
          )}
        </section>
        <section aria-label="Call frames" data-testid="debug-frames">
          <h3>Call frames</h3>
          {dap.frames.length === 0 ? (
            <p className="muted" data-testid="debug-frames-empty">
              {paused ? "The adapter reported a pause with no frames yet." : "No paused frames."}
            </p>
          ) : (
            <ul>
              {dap.frames.map((frame) => (
                <li key={frame.id}>
                  <button
                    data-testid={`debug-frame-${frame.id}`}
                    aria-pressed={dap.selectedFrameId === frame.id}
                    onClick={() =>
                      void invoke("Frame variables", "debug.variables", {
                        frameId: frame.id,
                      })
                    }
                    title={`Load locals for frame ${frame.id}`}
                  >
                    {frame.name}
                  </button>{" "}
                  <span className="muted" title={frame.path ?? frame.sourceName ?? "no source"}>
                    {frame.path ?? frame.sourceName ?? "no source"}:{frame.line}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>

        <section aria-label="Locals" data-testid="debug-locals">
          <h3>Locals</h3>
          {!dap.locals ? (
            <p className="muted" data-testid="debug-locals-empty">
              {paused ? "Select a frame to load its scopes." : "No current locals."}
            </p>
          ) : (
            <div key={dap.stopGeneration}>
              {dap.locals.scopes.map((scope, index) => (
                <div key={`${scope.name}-${index}`} data-testid={`debug-scope-${index}`}>
                  <strong>{scope.name}</strong>
                  {scope.expensive ? (
                    <span className="muted"> · marked expensive by the adapter</span>
                  ) : null}
                  <ul>
                    {scope.variables.map((variable, position) => (
                      <DebugVariable
                        key={`${variable.name}-${position}`}
                        variable={variable}
                        testId={`debug-local-${position}`}
                        expansions={dap.children}
                        onExpand={(variablesReference) =>
                          void invoke("Expand variable", "debug.children", { variablesReference })
                        }
                      />
                    ))}
                    {scope.variables.length === 0 && (
                      <li className="muted">No top-level variables in this scope.</li>
                    )}
                  </ul>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>

      <details className="debug-context-details">
        <summary>Session details</summary>
        <dl className="debug-identity" data-testid="dap-identity">
          <dt>Session</dt>
          <dd data-testid="dap-session">{dap.session || "none"}</dd>
          <dt>Project</dt>
          <dd>{dap.project ? `${dap.project.name} · ${dap.project.path}` : "none"}</dd>
          <dt>Build</dt>
          <dd data-testid="dap-build-id">
            {dap.buildId ?? "none"} · {dap.configuration || "no configuration"}
            {dap.targetFramework ? ` / ${dap.targetFramework}` : ""}
          </dd>
          <dt>Assembly</dt>
          <dd><code data-testid="dap-assembly">{dap.assembly ?? "none"}</code></dd>
          <dt>Adapter</dt>
          <dd data-testid="dap-adapter">
            {dap.adapter.binary
              ? `${dap.adapter.version} · ${dap.adapter.platform} · ${dap.adapter.argv.join(" ")}`
              : "not started"}
          </dd>
          <dt>Adapter capabilities</dt>
          <dd data-testid="dap-capabilities">
            {Object.entries(dap.adapterCapabilities)
              .filter(([, supported]) => supported)
              .map(([name]) => name)
              .join(", ") || "none reported"}
          </dd>
          <dt>Working directory</dt>
          <dd><code>{dap.cwd || "none"}</code></dd>
          <dt>Process</dt>
          <dd data-testid="dap-process">
            {dap.processId === null
              ? "none"
              : `registry ${dap.processId} · adapter pid ${dap.pid ?? "unknown"}`}
            {dap.debuggeePid === null ? "" : ` · debuggee pid ${dap.debuggeePid}`}
            {dap.adapterExit !== null && dap.adapterExit !== undefined
              ? ` · adapter exit ${dap.adapterExit}`
              : ""}
          </dd>
          <dt>Exit</dt>
          <dd data-testid="dap-exit">
            {dap.exitKnown ? `exit ${dap.exit}` : "exit unknown"}
          </dd>
        </dl>
      </details>

      <section aria-label="Breakpoints" data-testid="debug-breakpoints">
        <h3>Breakpoints</h3>
        {dap.breakpoints.length === 0 ? (
          <p className="muted" data-testid="debug-breakpoints-empty">
            No breakpoints. Click the gutter marker column or press {snapshot.app.platform === "macos" ? "Cmd+F8" : "Ctrl+F8"} on a line.
          </p>
        ) : (
          <ul>
            {dap.breakpoints.map((breakpoint) => (
              <li
                key={`${breakpoint.path}:${breakpoint.line}`}
                data-testid={`debug-breakpoint-${breakpoint.path}:${breakpoint.line}`}
              >
                <code>
                  {breakpoint.path}:{breakpoint.line}
                </code>{" "}
                <span className="muted">
                  {breakpoint.verified
                    ? "verified by the adapter"
                    : `pending${breakpoint.message ? ` · ${breakpoint.message}` : ""}`}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>

      <details className="debug-log-block">
        <summary>DAP, adapter and host log</summary>
        <pre data-testid="dap-log">{dap.log || "No DAP traffic yet."}</pre>
      </details>
      <details className="debug-log-block">
        <summary>Debuggee output</summary>
        <pre data-testid="dap-output">{dap.output || "No debuggee output yet."}</pre>
      </details>
    </section>
  );
}
