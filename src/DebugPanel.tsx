import { DebugBreakpoint } from "./DebugBreakpoint";
import { DebugEvaluate } from "./DebugEvaluate";
import { DebugVariable } from "./DebugVariable";
import { Icon } from "./Icon";
import { ToolDownloadStatus } from "./ToolDownloadStatus";
import { useState } from "react";
import {
  call,
  HostCallError,
  type Dap,
  type Snapshot,
} from "./host/client";

type Props = {
  snapshot: Snapshot;
  shortcuts: { start?: string; stop?: string; resume?: string; over?: string; into?: string; out?: string; breakpoint?: string };
  disabledReason: string | null;
  trusted: boolean;
  onTrust: () => void;
  onLaunch: () => void;
  onSnapshot: (snapshot: Snapshot) => void;
  onError: (message: string) => void;
  /** Opens a frame's source at its 1-based DAP line in the editor. */
  onOpenFrame: (path: string, line: number) => void;
  /** Stops the session and starts it again with the active run configuration. */
  onRestart: () => void;
};

function messageOf(error: unknown): string {
  if (error instanceof HostCallError || error instanceof Error) return error.message;
  return String(error);
}

const stateLabels: Record<Dap["state"], string> = {
  idle: "Not started",
  building: "Building",
  preparing: "Preparing",
  running: "Running",
  paused: "Paused",
  terminated: "Terminated",
  cancelled: "Cancelled",
  failed: "Failed",
};

export function DebugPanel({
  snapshot,
  shortcuts,
  disabledReason,
  trusted,
  onTrust,
  onLaunch,
  onSnapshot,
  onError,
  onOpenFrame,
  onRestart,
}: Props) {
  const stopKey = shortcuts.stop ?? "";
  const stepOutKey = shortcuts.out ?? "";
  const dap = snapshot.dap;
  const [busy, setBusy] = useState<string | null>(null);
  const [logTab, setLogTab] = useState<"console" | "output">("console");
  const active = dap.state === "building" || dap.state === "preparing" || dap.state === "running" || dap.state === "paused";
  const building = dap.state === "building";
  const paused = dap.state === "paused";
  const terminal = ["terminated", "cancelled", "failed"].includes(dap.state);
  const exceptionDisabledReason =
    dap.adapterExceptionFilters.length > 0 && !dap.adapterExceptionFilters.includes("all")
      ? "The debug adapter does not offer thrown-exception breakpoints."
      : null;
  const conditionDisabledReason =
    dap.adapterCapabilities.supportsConditionalBreakpoints === false
      ? "The debug adapter did not advertise conditional breakpoints."
      : null;
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
              `Starts the evaluated managed assembly under the verified adapter (${shortcuts.start ?? ""}).`
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
            title={`Stop (${stopKey}). Sends DAP disconnect, terminates the adapter group and reports what was observed; no cleanup is presumed.`}
          >
            <Icon name="stop" />
          </button>
        )}
        {active && !building && (
          <button
            data-testid="debug-restart"
            className="toolbar-icon-button"
            aria-label="Restart"
            disabled={busy !== null}
            onClick={onRestart}
            title="Stops the session and starts it again with the active run configuration."
          >
            <Icon name="restart" />
          </button>
        )}
        {dap.state === "running" && (
          <button
            data-testid="debug-pause"
            className="toolbar-icon-button"
            aria-label="Pause"
            disabled={busy !== null}
            onClick={() => void invoke("Pause", "debug.pause")}
            title="Pauses the running debuggee; the stop shows its frames and locals."
          >
            <Icon name="pause" />
          </button>
        )}
        {paused && (
          <>
            <button
              data-testid="debug-continue"
            className="toolbar-icon-button"
            aria-label="Resume"
              disabled={busy !== null}
              onClick={() => void invoke("Resume", "debug.continue")}
              title={`Resumes every thread (${shortcuts.resume ?? ""}).`}
            >
              <Icon name="resume" />
            </button>
            <button
              data-testid="debug-step-over"
            className="toolbar-icon-button"
            aria-label="Step over"
              disabled={busy !== null}
              onClick={() => void invoke("Step over", "debug.stepOver")}
              title={`Steps over the current line (${shortcuts.over ?? ""}).`}
            >
              <Icon name="stepOver" />
            </button>
            <button
              data-testid="debug-step-into"
            className="toolbar-icon-button"
            aria-label="Step into"
              disabled={busy !== null}
              onClick={() => void invoke("Step into", "debug.stepInto")}
              title={`Steps into the current call (${shortcuts.into ?? ""}).`}
            >
              <Icon name="stepInto" />
            </button>
            <button
              data-testid="debug-step-out"
            className="toolbar-icon-button"
            aria-label="Step out"
              disabled={busy !== null}
              onClick={() => void invoke("Step out", "debug.stepOut")}
              title={`Steps out of the current function (${stepOutKey}).`}
            >
              <Icon name="stepOut" />
            </button>
          </>
        )}
      </div>

      {disabledReason && !active && (
        <p className="muted" data-testid="dap-disabled-reason" title={disabledReason}>
          {disabledReason}
          {!trusted && (
            <>
              {" "}
              <button
                type="button"
                data-testid="dap-trust"
                onClick={onTrust}
                title="Grants trust for this project path and remembers it; the Debug gate is re-evaluated."
              >
                Trust this project
              </button>
            </>
          )}
        </p>
      )}
      <ToolDownloadStatus
        testId="debug-install"
        tool="netcoredbg"
        state={snapshot.debug.adapterState}
        progress={snapshot.debug.progress}
        error={snapshot.debug.error}
        note={snapshot.debug.note}
        busy={busy !== null}
        retryDisabledReason={
          trusted ? null : "Trust the workspace before retrying an installation."
        }
        onRetry={() => void invoke("Retry adapter install", "debug.retry")}
      />
      {snapshot.debug.os === "macos" &&
        snapshot.debug.arch === "aarch64" &&
        snapshot.debug.appleSiliconExperimental && (
          <p className="muted" data-testid="dap-experimental-note">
            macOS arm64 debugging is experimental and community supported; it runs the approved
            netcoredbg 3.2.0-1092 build.
          </p>
        )}
      {terminal && !dap.noStopExplanation && (
        <p className="muted" data-testid="dap-terminal-note">
          {dap.phase.replace(/[.]+$/, "")}. Logs and debuggee output below are preserved from this session.
        </p>
      )}
      {dap.noStopExplanation && (
        <p className="info" data-testid="dap-no-stop">
          {dap.noStopExplanation}
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
                  <button
                    type="button"
                    aria-pressed={dap.selectedThreadId === thread.id}
                    onClick={() =>
                      void invoke("Select thread", "debug.selectThread", {
                        threadId: thread.id,
                      })
                    }
                  >
                    {thread.name} <span className="muted">({thread.id})</span>
                  </button>
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
                    onClick={() => {
                      void invoke("Frame variables", "debug.variables", {
                        frameId: frame.id,
                      });
                      if (frame.path) onOpenFrame(frame.path, frame.line);
                    }}
                    title={`Load locals for frame ${frame.id}`}
                  >
                    {frame.name}
                  </button>{" "}
                  <span className="muted" title={frame.path ?? frame.sourceName ?? "no source"}>
                    {(frame.path ?? frame.sourceName ?? "no source").split(/[\\/]/).pop()}:{frame.line}
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

      <DebugEvaluate
        paused={paused}
        evaluation={dap.evaluation}
        watches={dap.watches}
        expansions={dap.children}
        onEvaluate={(expression) =>
          void invoke("Evaluate", "debug.evaluate", { expression })
        }
        onAddWatch={(expression) => void invoke("Add watch", "debug.addWatch", { expression })}
        onRemoveWatch={(expression) =>
          void invoke("Remove watch", "debug.removeWatch", { expression })
        }
        onExpand={(variablesReference) =>
          void invoke("Expand variable", "debug.children", { variablesReference })
        }
      />

      <section aria-label="Breakpoints" data-testid="debug-breakpoints">
        <div className="debug-breakpoints-head">
          <h3>Breakpoints</h3>
          {dap.breakpoints.length > 0 && (
            <button
              type="button"
              data-testid="debug-breakpoints-mute"
              aria-pressed={dap.breakpointsMuted}
              disabled={busy !== null}
              onClick={() =>
                void invoke(
                  dap.breakpointsMuted ? "Unmute breakpoints" : "Mute breakpoints",
                  "debug.setBreakpointsMuted",
                  { muted: !dap.breakpointsMuted },
                )
              }
              title="Mute all ignores every breakpoint without deleting it; unmute restores them with their conditions."
            >
              {dap.breakpointsMuted ? "Unmute all" : "Mute all"}
            </button>
          )}
        </div>
        <label className="debug-exceptions" title={exceptionDisabledReason ?? "Off: only unhandled exceptions stop. On: every thrown exception stops."}>
          <input
            type="checkbox"
            data-testid="debug-exceptions-thrown"
            checked={dap.exceptionFilters.includes("all")}
            disabled={exceptionDisabledReason !== null || busy !== null}
            onChange={(event) =>
              void invoke("Exception filter", "debug.setExceptionBreakpoints", {
                thrown: event.target.checked,
              })
            }
          />{" "}
          Break on thrown exceptions
        </label>
        {dap.breakpoints.length === 0 ? (
          <p className="muted" data-testid="debug-breakpoints-empty">
            No breakpoints. Click the gutter marker column or press {shortcuts.breakpoint} on a line.
          </p>
        ) : (
          <>
            <ul>
              {dap.breakpoints.map((breakpoint) => (
                <DebugBreakpoint
                  key={`${breakpoint.path}:${breakpoint.line}`}
                  breakpoint={breakpoint}
                  conditionDisabledReason={conditionDisabledReason}
                  busy={busy !== null}
                  onSaveCondition={(condition) =>
                    void invoke("Save condition", "debug.setBreakpointCondition", {
                      path: breakpoint.path,
                      line: breakpoint.line,
                      condition,
                    })
                  }
                  onSaveHitCount={(hitCount) => {
                    const trimmed = hitCount.trim();
                    const value = trimmed === "" ? null : /^\d+$/.test(trimmed) ? Number(trimmed) : Number.NaN;
                    if (Number.isNaN(value as number)) {
                      onError("The hit count must be a positive whole number.");
                      return;
                    }
                    void invoke("Save hit count", "debug.setBreakpointHitCount", {
                      path: breakpoint.path,
                      line: breakpoint.line,
                      hitCount: value,
                    });
                  }}
                  onRemove={() =>
                    void invoke("Remove breakpoint", "debug.setBreakpoints", {
                      path: breakpoint.path,
                      lines: dap.breakpoints
                        .filter((item) => item.path === breakpoint.path && item.line !== breakpoint.line)
                        .map((item) => item.line),
                    })
                  }
                />
              ))}
            </ul>
            <p className="muted" data-testid="debug-breakpoints-unsupported">
              Log message is not offered: netcoredbg 3.2.0-1092 does not advertise support for it.
            </p>
          </>
        )}
      </section>

      <div className="debug-log-block">
        <div className="debug-log-tabs" role="tablist" aria-label="Debug logs">
          <button
            type="button"
            role="tab"
            id="debug-tab-console"
            aria-selected={logTab === "console"}
            aria-controls="debug-panel-console"
            data-testid="debug-tab-console"
            onClick={() => setLogTab("console")}
          >
            Console
          </button>
          <button
            type="button"
            role="tab"
            id="debug-tab-output"
            aria-selected={logTab === "output"}
            aria-controls="debug-panel-output"
            data-testid="debug-tab-output"
            onClick={() => setLogTab("output")}
          >
            Debug Output
          </button>
        </div>
        {/* Both panes stay mounted so their text remains readable while the other tab is open. */}
        <pre
          role="tabpanel"
          id="debug-panel-console"
          aria-labelledby="debug-tab-console"
          hidden={logTab !== "console"}
          data-testid="dap-output"
        >
          {dap.output || "No debuggee output yet."}
        </pre>
        <pre
          role="tabpanel"
          id="debug-panel-output"
          aria-labelledby="debug-tab-output"
          hidden={logTab !== "output"}
          data-testid="dap-log"
        >
          {dap.log || "No DAP traffic yet."}
        </pre>
      </div>
    </section>
  );
}
