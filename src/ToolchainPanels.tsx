import { useState } from "react";
import { call, HostCallError, type Snapshot } from "./host/client";

function messageOf(error: unknown) {
  return error instanceof HostCallError ? error.message : String(error);
}

const activeStates = ["downloading", "installing", "verifying"];

function installStateLabel(state: Snapshot["tools"]["csharp-ls"]["state"]) {
  switch (state) {
    case "planned":
      return "Plan ready — nothing downloaded yet";
    case "downloading":
      return "Downloading…";
    case "installing":
      return "Installing…";
    case "verifying":
      return "Verifying executable and version…";
    case "ready":
      return "Ready";
    case "cancelled":
      return "Cancelled";
    case "failed":
      return "Failed";
    default:
      return "Not installed";
  }
}

function lspStateLabel(state: Snapshot["lsp"]["state"]) {
  switch (state) {
    case "starting":
      return "Starting…";
    case "loading":
      return "Loading solution…";
    case "ready":
      return "Ready";
    case "partial":
      return "Partial coverage";
    case "awaiting_diagnostics":
      return "Awaiting diagnostics";
    case "failed":
      return "Failed";
    case "stopped":
      return "Stopped";
    default:
      return "Not running";
  }
}

type Props = {
  snapshot: Snapshot;
  onChange: (snapshot: Snapshot) => void;
};

export function ToolInstaller({ snapshot, onChange }: Props) {
  const tool = snapshot.tools["csharp-ls"];
  const plan = tool.plan;
  const trusted = snapshot.trust.granted;
  const active = activeStates.includes(tool.state);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [planOpen, setPlanOpen] = useState(false);

  const disabledReason = !trusted
    ? "Trust the workspace before installing a tool; trust and installation are separate decisions."
    : !plan
      ? "Review the exact install plan before confirming."
      : active
        ? "An installation is already running."
        : null;
  const canCancel = tool.state === "planned" || tool.state === "downloading";
  const canRetry =
    (tool.state === "failed" || tool.state === "cancelled") && tool.revision > 0;

  async function run(action: () => Promise<Snapshot>) {
    setBusy(true);
    setMessage(null);
    try {
      onChange(await action());
    } catch (error) {
      setMessage(messageOf(error));
      try {
        onChange(await call<Snapshot>("workspace.state"));
      } catch {
        // The action message already explains the failure.
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="analysis-panel" aria-label="Tool installation" data-testid="install-panel">
      <div className="analysis-head">
        <strong>
          Analysis tool · {tool.version}
        </strong>
        <span
          className={`tag ${tool.state === "failed" ? "error" : tool.state === "ready" ? "" : "warning"}`}
          data-testid="install-state"
          role="status"
        >
          {installStateLabel(tool.state)}
        </span>
        <span className="push" />
        {!active && (
          <button
            data-testid="install-plan-open"
            disabled={busy}
            onClick={() => {
              setPlanOpen(true);
              void run(() => call<Snapshot>("tools.plan", { tool: "csharp-ls" }));
            }}
            title="Shows the exact version, source, platform, destination, command and permissions. It does not download or execute anything."
          >
            Review install plan
          </button>
        )}
        {canCancel && (
          <button
            data-testid="install-cancel"
            disabled={busy}
            onClick={() => void run(() => call<Snapshot>("tools.cancel"))}
            title={
              active
                ? "Requests termination of the install process group and keeps the partial staging directory labelled as not ready."
                : "Cancels the plan; no download or process was started."
            }
          >
            {active ? "Cancel install" : "Cancel plan"}
          </button>
        )}
        {canRetry && (
          <button
            data-testid="install-retry"
            disabled={busy || !plan}
            onClick={() => void run(() => call<Snapshot>("tools.retry"))}
            title={
              disabledReason ??
              "Runs the confirmed plan again. A changed plan requires a new confirmation."
            }
          >
            Retry install
          </button>
        )}
        {!active && plan && (
          <button
            className="primary"
            data-testid="install-confirm"
            disabled={busy || disabledReason !== null}
            onClick={() =>
              void run(() =>
                call<Snapshot>("tools.install", { planId: plan.planId, confirm: true }),
              )
            }
            title={disabledReason ?? "Downloads and installs into the staging directory, then verifies before ready."}
          >
            {tool.state === "ready" ? "Reuse verified installation" : "Confirm and install"}
          </button>
        )}
      </div>

      {disabledReason && !active && (
        <p className="muted" data-testid="install-disabled-reason">
          {disabledReason}
        </p>
      )}
      {tool.state === "ready" && (
        <p className="success" data-testid="install-ready">
          Executable and version verified. This does not prove that the language server loads a
          solution.
        </p>
      )}
      {tool.error && (
        <p className="error" role="alert" data-testid="install-error">
          {tool.error}
        </p>
      )}
      {tool.note && (
        <p className="muted" data-testid="install-note">
          {tool.note}
        </p>
      )}
      {message && (
        <p className="error" role="alert" data-testid="install-message">
          {message}
        </p>
      )}
      {tool.sha256 && (
        <p className="muted" data-testid="install-sha256">
          SHA-256 of the installed launcher: <code>{tool.sha256}</code>
        </p>
      )}
      {tool.verifiedOutput && (
        <p className="muted" data-testid="install-verified">
          <code>{tool.verifiedOutput}</code>
        </p>
      )}
      {tool.revision > 0 && (
        <p className="muted" data-testid="install-exit">
          Install attempt {tool.revision} · exit{" "}
          {tool.exit === null ? "unknown" : tool.exit}
          {tool.termination
            ? ` · termination ${tool.termination.requested} / ${tool.termination.observed}`
            : ""}
        </p>
      )}

      {plan && (
        <details
          className="eval-log analysis-plan"
          data-testid="install-plan"
          open={planOpen}
          onToggle={(event) => setPlanOpen(event.currentTarget.open)}
        >
          <summary>Install plan details</summary>
          <dl className="restore-meta">
            <dt>Tool</dt>
            <dd data-testid="install-tool">
              {plan.tool} {plan.version}
            </dd>
            <dt>Source</dt>
            <dd data-testid="install-source">
              {plan.source.name} · <code>{plan.source.url}</code>
              <br />
              <code>{plan.source.packageUrl}</code>
            </dd>
            <dt>Platform</dt>
            <dd data-testid="install-platform">
              {plan.os} / {plan.arch}
            </dd>
            <dt>Destination</dt>
            <dd data-testid="install-destination">
              <code>{plan.destination}</code>
            </dd>
            <dt>Staging</dt>
            <dd data-testid="install-staging">
              <code>{plan.staging}</code>
            </dd>
            <dt>Command</dt>
            <dd data-testid="install-command">
              <code>{plan.argv.join(" ")}</code>
            </dd>
            <dt>Working directory</dt>
            <dd data-testid="install-cwd">
              <code>{plan.cwd}</code>
            </dd>
            <dt>Permissions</dt>
            <dd data-testid="install-permissions">
              <ul className="plain-list">
                {plan.permissions.map((permission) => (
                  <li key={permission}>{permission}</li>
                ))}
              </ul>
            </dd>
            <dt>Integrity</dt>
            <dd data-testid="install-integrity">{plan.integrity}</dd>
            <dt>Existing</dt>
            <dd data-testid="install-existing">
              {plan.existing.state === "present"
                ? `A destination exists; confirming verifies and reuses it when valid (${plan.existing.reason ?? "unverified"})`
                : "No launcher at the destination."}
            </dd>
          </dl>
        </details>
      )}

      {(tool.log || tool.revision > 0) && (
        <details
          className="eval-log"
          open={active || tool.state === "failed" || tool.state === "cancelled"}
        >
          <summary>
            Install log · stdout/stderr{tool.logTruncated ? " · truncated" : ""}
          </summary>
          <pre className="eval-pre" data-testid="install-log">
            {tool.log || "No output yet."}
          </pre>
        </details>
      )}
    </section>
  );
}

export function LspPanel({ snapshot, onChange }: Props) {
  const lsp = snapshot.lsp;
  const tool = snapshot.tools["csharp-ls"];
  const trusted = snapshot.trust.granted;
  const projectSdk = snapshot.sdk.project;
  const restore = snapshot.restore;
  const evaluated = snapshot.evaluation.projects;
  const active = ["starting", "loading", "ready", "partial", "awaiting_diagnostics"].includes(
    lsp.state,
  );
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const restoreCurrent =
    restore.revision > 0
      ? restore.state === "ok" &&
        restore.projects.length > 0 &&
        restore.projects.every((project) => project.state === "restored")
      : evaluated.length > 0 && evaluated.every((project) => project.restored === "restored");
  const restoreReason = !trusted
    ? "Trust the workspace before analysis."
    : tool.state !== "ready"
      ? "Install and verify csharp-ls 0.28.0 first; the LSP never installs itself."
      : projectSdk.state !== "resolved"
        ? "Resolve the project SDK before analysis; host SDK 10 is never substituted."
        : restore.state === "running"
          ? "A restore is running; analysis starts after it completes."
          : !restoreCurrent
            ? "Restore is not current. Analysis never restores; run the explicit Restore first."
            : null;
  const startDisabledReason = restoreReason;

  async function run(action: () => Promise<Snapshot>) {
    setBusy(true);
    setMessage(null);
    try {
      onChange(await action());
    } catch (error) {
      setMessage(messageOf(error));
      try {
        onChange(await call<Snapshot>("workspace.state"));
      } catch {
        // Keep the action message.
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="analysis-panel" aria-label="Analysis" data-testid="lsp-panel">
      <div className="analysis-head">
        <strong>Analysis · csharp-ls</strong>
        <span
          className={`tag ${lsp.state === "failed" ? "error" : lsp.state === "ready" ? "" : "warning"}`}
          data-testid="lsp-state"
          role="status"
        >
          {lspStateLabel(lsp.state)}
          {lsp.coverage === "partial" && lsp.state !== "unavailable" ? " · partial coverage" : ""}
        </span>
        <span className="muted" data-testid="lsp-session">
          session {lsp.session} · {lsp.logLevel} log
        </span>
        <span className="push" />
        {!active && (
          <button
            className="primary"
            data-testid="lsp-start"
            disabled={busy || startDisabledReason !== null}
            onClick={() => void run(() => call<Snapshot>("lsp.start"))}
            title={startDisabledReason ?? "Starts the verified csharp-ls with the composite analysis host root."}
          >
            Start analysis
          </button>
        )}
        {active && (
          <>
            <button
              data-testid="lsp-restart"
              disabled={busy}
              onClick={() => void run(() => call<Snapshot>("lsp.restart", {}))}
              title="Shuts the session down (shutdown → exit) and starts a new one."
            >
              Restart analysis
            </button>
            <button
              data-testid="lsp-stop"
              disabled={busy}
              onClick={() => void run(() => call<Snapshot>("lsp.stop"))}
              title="Requests shutdown, then exit, then terminates the process group if needed."
            >
              Stop analysis
            </button>
            <button
              data-testid="lsp-verbose"
              disabled={busy}
              onClick={() =>
                void run(() =>
                  call<Snapshot>("lsp.restart", { verbose: lsp.logLevel !== "debug" }),
                )
              }
              title="Restarts the language server with --loglevel debug or info."
            >
              {lsp.logLevel === "debug" ? "Use info log" : "Verbose log"}
            </button>
          </>
        )}
        {!active && (lsp.state === "failed" || lsp.state === "stopped") && (
          <button
            data-testid="lsp-restart"
            disabled={busy || startDisabledReason !== null}
            onClick={() => void run(() => call<Snapshot>("lsp.restart", {}))}
            title={startDisabledReason ?? "Starts a fresh analysis session."}
          >
            Restart analysis
          </button>
        )}
      </div>

      {startDisabledReason && !active && (
        <p className="muted" data-testid="lsp-disabled-reason">
          {startDisabledReason}
        </p>
      )}
      {lsp.state === "starting" && (
        <p className="muted">The server is starting; the editor is not blocked.</p>
      )}
      {lsp.state === "loading" && (
        <p className="muted">Solution loading is in progress; no file counts are invented.</p>
      )}
      {lsp.coverageNote && lsp.state !== "unavailable" && (
        <p className="warning" data-testid="lsp-coverage-note">
          {lsp.coverageNote}
        </p>
      )}
      {lsp.error && (
        <p className="error" role="alert" data-testid="lsp-error">
          {lsp.error}
        </p>
      )}
      {lsp.stoppedReason && lsp.state === "stopped" && (
        <p className="muted" data-testid="lsp-stopped-reason">
          {lsp.stoppedReason}
        </p>
      )}
      {message && (
        <p className="error" role="alert" data-testid="lsp-message">
          {message}
        </p>
      )}

      <div className="analysis-meta">
        <span data-testid="lsp-capabilities">
          Capabilities:{" "}
          {lsp.capabilities
            ? lsp.capabilities.summary.join(" · ") || "server did not advertise optional features"
            : "not negotiated yet"}
        </span>
        <span data-testid="lsp-diagnostics">
          Diagnostics: {lsp.publicationCount} publication
          {lsp.publicationCount === 1 ? "" : "s"} · {lsp.diagnosticItems} item
          {lsp.diagnosticItems === 1 ? "" : "s"}
          {lsp.awaitingDocuments.length > 0
            ? ` · awaiting ${lsp.awaitingDocuments.length} document${
                lsp.awaitingDocuments.length === 1 ? "" : "s"
              }`
            : ""}
        </span>
        <span data-testid="lsp-process">
          {lsp.processId
            ? `process ${lsp.processId} · pid ${lsp.pid ?? "unknown"} · exit ${
                lsp.exit === null ? "unknown" : lsp.exit
              }`
            : "no process"}
          {lsp.termination
            ? ` · termination ${lsp.termination.requested} / ${lsp.termination.observed}`
            : ""}
        </span>
        {lsp.hostRoot && (
          <span className="muted" data-testid="lsp-hostroot">
            DOTNET_ROOT <code>{lsp.hostRoot}</code>
            {lsp.hostRootForced ? " · forced by test seam" : ""}
            {lsp.hostRootRebuilt ? " · rebuilt this start" : ""}
          </span>
        )}
      </div>

      {(lsp.log || lsp.session > 0) && (
        <details
          className="eval-log"
          open={lsp.state === "failed" || lsp.state === "partial"}
        >
          <summary>
            LSP log · server/stdout/stderr{lsp.logTruncated ? " · truncated" : ""}
          </summary>
          <pre className="eval-pre" data-testid="lsp-log">
            {lsp.log || "No output yet."}
          </pre>
        </details>
      )}
    </section>
  );
}
