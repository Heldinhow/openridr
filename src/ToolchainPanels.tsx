import { useState } from "react";
import { call, HostCallError, type Snapshot } from "./host/client";
import { ToolDownloadStatus } from "./ToolDownloadStatus";

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

export function lspStateLabel(state: Snapshot["lsp"]["state"]) {
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
  const active = activeStates.includes(tool.state);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const lspRunning = ["starting", "loading", "ready", "partial", "awaiting_diagnostics"].includes(
    snapshot.lsp.state,
  );
  // The drawer is diagnosis only: setup already installed both fixed tools. Reinstall is the
  // one action, and it refuses to replace a launcher a running server still uses.
  const reinstallBlocked = busy
    ? "An action is already running."
    : active
      ? "An installation is already running."
      : lspRunning
        ? "Stop analysis before reinstalling csharp-ls; the running server uses that launcher."
        : null;

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
        <button
          data-testid="install-reinstall"
          disabled={reinstallBlocked !== null}
          onClick={() => void run(() => call<Snapshot>("setup.reinstall"))}
          title={
            reinstallBlocked ??
            "Removes only the app-home installation, downloads the pinned package again and verifies the launcher. The workspace is never touched."
          }
        >
          Reinstall
        </button>
      </div>

      {reinstallBlocked && (
        <p className="muted" data-testid="install-disabled-reason">
          {reinstallBlocked}
        </p>
      )}
      {tool.state === "ready" && (
        <p className="success" data-testid="install-ready">
          Executable and version verified. This does not prove that the language server loads a
          solution.
        </p>
      )}
      <ToolDownloadStatus
        testId="install"
        tool="csharp-ls"
        state={tool.state}
        progress={tool.progress}
        error={tool.error}
        note={tool.note}
        busy={busy}
        retryDisabledReason={null}
        onRetry={() => void run(() => call<Snapshot>("setup.start"))}
      />
      {!active && tool.state !== "failed" && tool.note && (
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
        >
          <summary>Installation details</summary>
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

function debugStateLabel(state: Snapshot["debug"]["adapterState"]) {
  switch (state) {
    case "unavailable":
      return "Unavailable";
    case "experimental_disabled":
      return "Experimental · disabled";
    case "not_installed":
      return "Not installed";
    case "planned":
      return "Plan ready — nothing downloaded yet";
    case "downloading":
      return "Downloading…";
    case "installing":
      return "Extracting…";
    case "verifying":
      return "Verifying executable and version…";
    case "ready":
      return "Adapter verified — DAP not proven";
    case "cancelled":
      return "Cancelled";
    case "failed":
      return "Failed";
    default:
      return "Not checked yet";
  }
}

export function DebugAdapterPanel({
  snapshot,
  onChange,
  compact = false,
}: Props & { compact?: boolean }) {
  const debug = snapshot.debug;
  const plan = debug.plan;
  const active =
    !debug.bundled &&
    (debug.busy || ["downloading", "installing", "verifying"].includes(debug.adapterState));

  return (
    <section
      className={`analysis-panel debug-panel${compact ? " compact" : ""}`}
      aria-label="Debug adapter"
      data-testid="debug-panel"
    >
      <div className="analysis-head">
        <strong>
          Debug adapter · {debug.version}
        </strong>
        <span
          className={`tag ${
            debug.adapterState === "failed"
              ? "error"
              : debug.adapterState === "ready"
                ? ""
                : "warning"
          }`}
          data-testid="debug-state"
          role="status"
        >
          {debug.bundled && debug.adapterState === "ready"
            ? "Bundled with OpenRidr — DAP not proven"
            : debugStateLabel(debug.adapterState)}
        </span>
        <span className="muted" data-testid="debug-platform">
          {debug.os} / {debug.arch}
        </span>
        <span className="push" />
      </div>

      <div data-testid="sdk-debug-adapter">
        <strong data-testid="debug-asset-state">
          {debug.asset
            ? `${debug.asset.name} · ${debug.availability}`
            : `No asset · ${debug.availability}`}
        </strong>
        <p className="muted" data-testid="debug-source">
          Source: <a href={debug.source.url} target="_blank" rel="noreferrer noopener">{debug.source.name}</a>{" "}
          · approved version {debug.version} · DAP proven by installation: {debug.dapProven ? "yes" : "no"}
        </p>
        {debug.bundled && (
          <dl className="restore-meta" data-testid="debug-bundled-adapter">
            <dt>Origin</dt>
            <dd data-testid="debug-bundled-origin">Bundled with OpenRidr</dd>
            <dt>Version</dt>
            <dd data-testid="debug-bundled-version">{debug.version}</dd>
            <dt>Checksum</dt>
            <dd data-testid="debug-bundled-checksum">
              {debug.sha256 ? <code>{debug.sha256}</code> : "not measured"}
              <span className="muted"> · SHA-256 of the embedded executable; --version is verified when a session starts.</span>
            </dd>
            <dt>Executable</dt>
            <dd data-testid="debug-bundled-binary">
              <code>{debug.installedBinary}</code>
            </dd>
          </dl>
        )}
        {debug.installed && !debug.bundled && (
          <dl className="restore-meta" data-testid="debug-installed-adapter">
            <dt>Installed origin</dt>
            <dd data-testid="debug-installed-origin">
              {debug.source.name} · <code>{debug.source.url}</code>
            </dd>
            <dt>Installed version</dt>
            <dd data-testid="debug-installed-version">
              {debug.verifiedVersion
                ? `${debug.verifiedVersion} · verified with --version`
                : `${debug.version} · approved; --version is verified when a session starts`}
            </dd>
            <dt>Installed checksum</dt>
            <dd data-testid="debug-installed-checksum">
              {debug.installedSha256 ? (
                <code>{debug.installedSha256}</code>
              ) : (
                "not measured"
              )}
              <span className="muted">
                {" "}
                · SHA-256 of the installed executable; the archive hash is checked against the
                release before execution.
              </span>
            </dd>
            <dt>Executable</dt>
            <dd data-testid="debug-installed-binary">
              <code>{debug.installedBinary}</code>
            </dd>
          </dl>
        )}
        <p className={debug.adapterState === "failed" ? "error" : "muted"} data-testid="debug-message">
          {debug.adapterMessage}
        </p>
      </div>

      {debug.error && (
        <p className="error" role="alert" data-testid="debug-error">
          {debug.error}
        </p>
      )}
      {debug.note && (
        <p className="muted" data-testid="debug-note">
          {debug.note}
        </p>
      )}
      {!debug.bundled && debug.progress && (
        <p className="muted" data-testid="debug-progress">
          Downloaded {debug.progress.downloaded} of{" "}
          {debug.progress.total === null ? "an unknown total" : `${debug.progress.total}`} bytes
          (measured; no percentage)
        </p>
      )}
      {!debug.bundled && debug.sha256 && (
        <p className="muted" data-testid="debug-sha256">
          SHA-256 of the installed executable: <code>{debug.sha256}</code>
        </p>
      )}
      {debug.verifiedOutput && (
        <p className="muted debug-verified" data-testid="debug-verified">
          <code>{debug.verifiedOutput}</code>
        </p>
      )}
      <p className="muted" data-testid="debug-dap">
        DAP launch is implemented (ticket 14). A verified executable or a successful install is
        still not proof that a session will succeed; the Debug tab reports the real adapter
        messages, exit code, processes and preserved logs.
      </p>

      {!debug.bundled && plan && (
        <details className="eval-log analysis-plan" data-testid="debug-plan">
          <summary>Installation details</summary>
          <dl className="restore-meta">
            <dt>Tool</dt>
            <dd data-testid="debug-plan-tool">
              {plan.tool} {plan.version}
            </dd>
            <dt>Source</dt>
            <dd data-testid="debug-plan-source">
              {plan.source.name} · <code>{plan.source.url}</code>
            </dd>
            <dt>Asset</dt>
            <dd data-testid="debug-plan-asset">
              {plan.asset ? (
                <>
                  <code>{plan.asset.name}</code> · {plan.availability}
                  <br />
                  {plan.asset.size > 0 ? `${plan.asset.size} bytes · ` : ""}
                  <code>{plan.asset.url}</code>
                  <br />
                  SHA-256 <code>{plan.asset.sha256}</code>
                  {plan.asset.testOverride ? " · controlled test artifact" : ""}
                  {plan.asset.reason ? <p className="muted">{plan.asset.reason}</p> : null}
                </>
              ) : (
                <>No asset. {plan.availabilityReason}</>
              )}
            </dd>
            <dt>Platform</dt>
            <dd data-testid="debug-plan-platform">
              {plan.os} / {plan.arch} ({plan.platform})
            </dd>
            <dt>Destination</dt>
            <dd data-testid="debug-plan-destination">
              <code>{plan.destination}</code>
            </dd>
            <dt>Staging</dt>
            <dd data-testid="debug-plan-staging">
              <code>{plan.staging}</code>
            </dd>
            <dt>Operation</dt>
            <dd data-testid="debug-plan-operation">{plan.operation}</dd>
            <dt>Permissions</dt>
            <dd data-testid="debug-plan-permissions">
              <ul className="plain-list">
                {plan.permissions.map((permission) => (
                  <li key={permission}>{permission}</li>
                ))}
              </ul>
            </dd>
            <dt>Integrity</dt>
            <dd data-testid="debug-plan-integrity">{plan.integrity}</dd>
            <dt>Limits</dt>
            <dd data-testid="debug-plan-limits">
              <ul className="plain-list">
                {plan.limits.map((limit) => (
                  <li key={limit}>{limit}</li>
                ))}
              </ul>
            </dd>
            <dt>Existing</dt>
            <dd data-testid="debug-plan-existing">
              {plan.existing.state === "present"
                ? `A destination exists; confirming verifies and reuses it when valid (${plan.existing.reason ?? "unverified"}).`
                : "No adapter at the destination."}
            </dd>
            {plan.blockedReason && (
              <>
                <dt>Blocked</dt>
                <dd className="warning" data-testid="debug-plan-blocked">
                  {plan.blockedReason}{" "}
                  {plan.blockedCode ? <code>{plan.blockedCode}</code> : null}
                </dd>
              </>
            )}
            <dt>Plan id</dt>
            <dd>
              <code>{plan.planId}</code>
            </dd>
          </dl>
        </details>
      )}

      {(debug.log || debug.revision > 0) && (
        <details
          className="eval-log"
          open={active || debug.adapterState === "failed" || debug.adapterState === "cancelled"}
        >
          <summary>
            Debug adapter log{debug.logTruncated ? " · truncated" : ""}
          </summary>
          <pre className="eval-pre" data-testid="debug-log">
            {debug.log || "No output yet."}
          </pre>
        </details>
      )}
    </section>
  );
}

/** Why analysis cannot start now (null = it can). One answer for the panel, the status bar and auto-start. */
export function lspStartBlocker(snapshot: Snapshot): string | null {
  const tool = snapshot.tools["csharp-ls"];
  const trusted = snapshot.trust.granted;
  const projectSdk = snapshot.sdk.project;
  const restore = snapshot.restore;
  const evaluated = snapshot.evaluation.projects;
  const restoreCurrent =
    restore.revision > 0
      ? restore.state === "ok" &&
        restore.projects.length > 0 &&
        restore.projects.every((project) => project.state === "restored")
      : evaluated.length > 0 && evaluated.every((project) => project.restored === "restored");
  return !trusted
    ? "Trust the workspace before analysis."
    : tool.state !== "ready"
      ? "Install and verify csharp-ls 0.28.0 first; the LSP never installs itself."
      : projectSdk.state !== "resolved"
        ? "Resolve the project SDK before analysis."
        : restore.state === "running"
          ? "A restore is running; analysis starts after it completes."
          : !restoreCurrent
            ? "Restore is not current. Analysis never restores; run the explicit Restore first."
            : null;
}

export function LspPanel({ snapshot, onChange }: Props) {
  const lsp = snapshot.lsp;
  const active = ["starting", "loading", "ready", "partial", "awaiting_diagnostics"].includes(
    lsp.state,
  );
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const startDisabledReason = lspStartBlocker(snapshot);

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
          {lsp.coverage === "partial" &&
          lsp.state !== "partial" &&
          lsp.state !== "unavailable"
            ? " · partial coverage"
            : ""}
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
                lsp.exit === null ? "not observed" : lsp.exit
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
