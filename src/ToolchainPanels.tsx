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
  const trusted = snapshot.trust.granted;
  const active =
    debug.busy || ["downloading", "installing", "verifying"].includes(debug.adapterState);
  const appleSilicon = debug.os === "macos" && debug.arch === "aarch64";
  const [acknowledged, setAcknowledged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const installBlocker = !debug.installable
    ? debug.adapterMessage
    : !trusted
      ? "Trust the workspace before installing a debug adapter; trust and installation are separate decisions."
      : busy || active
        ? "An install attempt is still running or winding down."
        : !plan
          ? "Review the exact install plan before confirming."
          : plan.blockedReason
            ? plan.blockedReason
            : null;
  const canCancel =
    debug.adapterState === "planned" ||
    debug.adapterState === "failed" ||
    ["downloading", "installing", "verifying"].includes(debug.adapterState);
  const canRetry =
    (debug.adapterState === "failed" || debug.adapterState === "cancelled") &&
    debug.revision > 0;

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
          {debugStateLabel(debug.adapterState)}
        </span>
        <span className="muted" data-testid="debug-platform">
          {debug.os} / {debug.arch}
        </span>
        <span className="push" />
        <button
          data-testid="debug-plan-open"
          disabled={busy}
          onClick={() => void run(() => call<Snapshot>("debug.plan"))}
          title="Shows the approved version, official source, platform asset, destination, operation, permissions, integrity and limits. It does not download or execute anything."
        >
          Review install plan
        </button>
        {canCancel && (
          <button
            data-testid="debug-cancel"
            disabled={busy}
            onClick={() => void run(() => call<Snapshot>("debug.cancel"))}
            title={
              active
                ? "Requests cancellation of the download/extraction; a partial staging directory is never reported ready."
                : "Cancels the plan; no download and no process were started."
            }
          >
            {active ? "Cancel install" : "Cancel plan"}
          </button>
        )}
        {canRetry && (
          <button
            data-testid="debug-retry"
            disabled={busy || active || !trusted}
            onClick={() => void run(() => call<Snapshot>("debug.retry"))}
            title={
              !trusted
                ? "Trust the workspace before retrying an installation."
                : "Runs the confirmed plan again; a changed plan requires a new confirmation."
            }
          >
            Retry install
          </button>
        )}
        {!active && (
          <button
            className="primary"
            data-testid="debug-install"
            disabled={busy || installBlocker !== null}
            onClick={() =>
              void run(() =>
                call<Snapshot>("debug.install", { planId: plan?.planId, confirm: true }),
              )
            }
            title={
              installBlocker ??
              "Downloads the official asset, verifies its published SHA-256, extracts it safely and only then verifies and promotes it."
            }
          >
            {debug.adapterState === "ready" ? "Reuse verified adapter" : "Confirm and install"}
          </button>
        )}
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
        <p className={debug.adapterState === "failed" ? "error" : "muted"} data-testid="debug-message">
          {debug.adapterMessage}
        </p>
      </div>

      {installBlocker && !active && (
        <p className="muted" data-testid="debug-disabled-reason">
          {installBlocker}
        </p>
      )}
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
      {message && (
        <p className="error" role="alert" data-testid="debug-message-error">
          {message}
        </p>
      )}
      {debug.progress && (
        <p className="muted" data-testid="debug-progress">
          Downloaded {debug.progress.downloaded} of{" "}
          {debug.progress.total === null ? "an unknown total" : `${debug.progress.total}`} bytes
          (measured; no percentage)
        </p>
      )}
      {debug.sha256 && (
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

      {appleSilicon && (
        <div data-testid="sdk-apple-silicon">
          <strong>Apple Silicon debugger</strong>{" "}
          <span className="tag">
            {debug.appleSiliconExperimental ? "experimental · enabled" : "experimental · disabled"}
          </span>
          <p className="muted" data-testid="debug-flag-note">
            The opt-in is independent of trust and installation. It neither installs netcoredbg
            nor proves compatibility. It only unlocks the install plan, which still requires trust
            and an explicit confirmation.
          </p>
          <label className="check-row">
            <input
              type="checkbox"
              data-testid="debug-flag-checkbox"
              checked={acknowledged}
              disabled={busy || debug.appleSiliconExperimental}
              onChange={(event) => setAcknowledged(event.target.checked)}
            />
            I understand the experimental flag does not install or prove the adapter
          </label>
          {debug.appleSiliconExperimental ? (
            <button
              data-testid="debug-flag-disable"
              disabled={busy}
              onClick={() =>
                void run(() => call<Snapshot>("debug.flag", { enabled: false }))
              }
              title="Disables the experimental flag. It never installed or claimed compatibility."
            >
              Disable experimental flag
            </button>
          ) : (
            <button
              data-testid="debug-flag-enable"
              disabled={busy || !acknowledged}
              onClick={() =>
                void run(() =>
                  call<Snapshot>("debug.flag", { enabled: true, confirm: true }),
                )
              }
              title={
                acknowledged
                  ? "Records the experimental opt-in for this session only. Nothing is installed and no DAP support is claimed."
                  : "Acknowledge the limitation before enabling the experimental flag."
              }
            >
              Enable experimental flag
            </button>
          )}
        </div>
      )}

      {plan && (
        <details className="eval-log analysis-plan" data-testid="debug-plan">
          <summary>Install plan details</summary>
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
