import { useEffect, useState } from "react";
import type { Evaluation, TestItem, Tests, TestsDiscovery } from "./host/client";

type Props = {
  tests: Tests;
  evaluation: Evaluation;
  editedAt: Record<string, number>;
  disabledReason: string | null;
  restoreRequired: boolean;
  restoreDisabledReason: string | null;
  onRestore: () => void;
  onEvaluate: (next: { targetFramework?: string | null }) => void;
  onDiscover: (project: string) => void;
  onCancel: () => void;
};

export function discoveryStateLabel(discovery: TestsDiscovery) {
  const exit = discovery.exit === null ? "exit unknown" : `exit ${discovery.exit}`;
  switch (discovery.state) {
    case "discovering":
      return "Discovering…";
    case "completed":
      return `Discovered ${discovery.items.length} test${discovery.items.length === 1 ? "" : "s"} · ${exit}`;
    case "empty":
      return `No tests listed · ${exit}`;
    case "failed":
      return `Discovery failed · ${exit}`;
    case "cancelled":
      return `Discovery cancelled · ${exit}`;
    case "adapter_missing":
      return "No VSTest adapter";
    case "mtp_unsupported":
      return "Microsoft.Testing.Platform";
    default:
      return "No discovery has run";
  }
}

/** Unsaved edits or saves after the discovery started make the list stale, never failed. */
export function isDiscoveryObsolete(
  discovery: TestsDiscovery,
  editedAt: Record<string, number>,
) {
  if (discovery.revision === 0) return false;
  const reference = discovery.startedAt ?? discovery.finishedAt ?? 0;
  return Object.values(editedAt).some((edited) => edited > reference);
}

function itemLabel(item: TestItem) {
  return `${item.fullyQualifiedName} · ${item.project} · ${item.tfm ?? "TFM not selected"}`;
}

export function TestsPanel({
  tests,
  evaluation,
  editedAt,
  disabledReason,
  restoreRequired,
  restoreDisabledReason,
  onRestore,
  onEvaluate,
  onDiscover,
  onCancel,
}: Props) {
  const discovery = tests.discovery;
  const candidates = evaluation.projects.filter(
    (project) => project.kind === "test" && project.state === "ok",
  );
  const [projectPath, setProjectPath] = useState("");
  const selectedProject = candidates.find((project) => project.path === projectPath) ?? null;
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const discoveryKey = `${discovery.discoveryId ?? "none"}:${discovery.revision}`;

  useEffect(() => {
    if (candidates.some((project) => project.path === projectPath)) return;
    setProjectPath(candidates.length === 1 ? candidates[0].path : "");
    // Selection follows the evaluated project list, not typing.
  }, [candidates.map((project) => project.path).join("|"), projectPath]);

  useEffect(() => {
    setSelected(new Set());
  }, [discoveryKey]);

  const staleByEdit = isDiscoveryObsolete(discovery, editedAt);
  const stale = discovery.stale || staleByEdit;
  const staleReason =
    discovery.staleReason ??
    "A buffer was edited after discovery; the list may not match the saved code.";
  const running = discovery.state === "discovering";
  const selectionBlocked =
    disabledReason !== null || running || stale || discovery.sealed || discovery.state !== "completed";
  const tfmOptions =
    evaluation.context.targetFramework &&
    !evaluation.availableTfms.includes(evaluation.context.targetFramework)
      ? [evaluation.context.targetFramework, ...evaluation.availableTfms]
      : evaluation.availableTfms;
  const terminalFailure = [
    "failed",
    "cancelled",
    "empty",
    "adapter_missing",
    "mtp_unsupported",
  ].includes(discovery.state);
  const discoverReason =
    disabledReason ??
    (!selectedProject
      ? "Select an evaluated test project explicitly; discovery never guesses."
      : null);
  const items = discovery.items;

  function toggle(id: string) {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  return (
    <div className="build-panel" data-testid="tests-panel">
      <div className="build-controls">
        <label className="eval-field">
          <span>Test project</span>
          <select
            data-testid="tests-project"
            value={selectedProject?.path ?? ""}
            disabled={running || candidates.length === 0}
            onChange={(event) => setProjectPath(event.target.value)}
            title={
              candidates.length === 0
                ? "No evaluated test project is available; evaluate a project whose IsTestProject is true."
                : "The test project is selected explicitly; discovery never scans other projects."
            }
          >
            {candidates.length === 0 && <option value="">No test project</option>}
            {candidates.length > 1 && !selectedProject && (
              <option value="">Select a test project</option>
            )}
            {candidates.map((project) => (
              <option key={project.path} value={project.path}>
                {project.name}
              </option>
            ))}
          </select>
        </label>
        <label className="eval-field">
          <span>TFM</span>
          <select
            data-testid="tests-tfm"
            value={evaluation.context.targetFramework ?? ""}
            disabled={running || evaluation.state === "running"}
            onChange={(event) =>
              onEvaluate({ targetFramework: event.target.value === "" ? null : event.target.value })
            }
            title="The TFM is part of the evaluated model; changing it re-evaluates before discovery."
          >
            <option value="">Not selected</option>
            {tfmOptions.map((tfm) => (
              <option key={tfm} value={tfm}>
                {tfm}
              </option>
            ))}
          </select>
        </label>
        <button
          data-testid="tests-discover"
          disabled={discoverReason !== null || running}
          onClick={() => selectedProject && onDiscover(selectedProject.path)}
          title={
            discoverReason ??
            "Runs the project's VSTest adapter through dotnet test --list-tests. Never restores, builds a solution or edits packages."
          }
        >
          {running ? "Discovering…" : discovery.revision > 0 ? "Discover again" : "Discover tests"}
        </button>
        {running && (
          <button
            data-testid="tests-cancel"
            onClick={onCancel}
            title="Requests termination of the discovery process group and keeps the partial list."
          >
            Cancel discovery
          </button>
        )}
        {terminalFailure && !running && (
          <button
            data-testid="tests-retry"
            disabled={discoverReason !== null}
            onClick={() => selectedProject && onDiscover(selectedProject.path)}
            title={discoverReason ?? "Runs a new discovery of the same project and TFM."}
          >
            Retry discovery
          </button>
        )}
        <span
          className={`tag ${discovery.state === "failed" ? "error" : ""}`}
          data-testid="tests-state"
          role="status"
        >
          {discoveryStateLabel(discovery)}
        </span>
        {discovery.discoveryId && (
          <span className="muted" data-testid="tests-discovery-id">
            {discovery.discoveryId} · revision {discovery.revision} · context{" "}
            {discovery.contextId ?? "unknown"}
          </span>
        )}
      </div>

      {disabledReason && (
        <p className="muted" data-testid="tests-disabled-reason">
          {disabledReason}
        </p>
      )}

      {restoreRequired && (
        <div className="notice warning" data-testid="tests-restore-required" role="status">
          <strong>Test discovery needs restored assets.</strong> OpenRidr never restores inside
          discovery; run the explicit restore first.
          <div className="actions">
            <button
              data-testid="tests-restore-action"
              disabled={restoreDisabledReason !== null}
              onClick={onRestore}
              title={restoreDisabledReason ?? "Calls restore.start for this workspace."}
            >
              Restore now
            </button>
          </div>
        </div>
      )}

      {discovery.state === "mtp_unsupported" && (
        <p className="warning" data-testid="tests-mtp-message" role="status">
          {discovery.mtpMessage}
        </p>
      )}
      {discovery.state === "adapter_missing" && (
        <p className="warning" data-testid="tests-guidance" role="status">
          {discovery.error}
        </p>
      )}
      {discovery.state === "empty" && (
        <p className="notice" data-testid="tests-guidance" role="status">
          {discovery.error}
        </p>
      )}
      {discovery.state === "failed" && (
        <p className="warning" data-testid="tests-guidance" role="status">
          {discovery.error} The raw log below is complete for this run; Retry starts a new
          discovery.
        </p>
      )}
      {discovery.state === "cancelled" && (
        <p className="warning" data-testid="tests-guidance" role="status">
          Discovery was cancelled; the {discovery.items.length} test
          {discovery.items.length === 1 ? "" : "s"} found so far {discovery.partial ? "are" : "is"}{" "}
          partial and cannot be selected. The log is preserved.
        </p>
      )}

      {discovery.revision > 0 && (
        <>
          <dl className="restore-meta build-meta">
            <dt>Scope</dt>
            <dd data-testid="tests-scope">
              {discovery.project?.name ?? "unknown"} ·{" "}
              <code>{discovery.project?.path ?? "unknown"}</code> · TFM{" "}
              {discovery.targetFramework ?? "not selected"}
            </dd>
            <dt>Command</dt>
            <dd>
              <code data-testid="tests-command">
                {discovery.command.length > 0
                  ? discovery.command.join(" ")
                  : "No command was run."}
              </code>
            </dd>
            <dt>Working directory</dt>
            <dd>
              <code data-testid="tests-cwd">{discovery.cwd || "unknown"}</code>
            </dd>
            <dt>Project SDK</dt>
            <dd data-testid="tests-sdk">
              {discovery.sdkVersion ?? "unknown"} ·{" "}
              <code>{discovery.sdkRoot ?? "unknown"}</code>
            </dd>
            <dt>Exit</dt>
            <dd data-testid="tests-exit">
              {discovery.exit === null ? "exit unknown" : `exit ${discovery.exit}`}
            </dd>
          </dl>

          {discovery.sealed && (
            <p className="warning" data-testid="tests-sealed">
              Previous discovery, not current: {discovery.staleReason}
            </p>
          )}
          {!discovery.sealed && stale && (
            <p className="warning" data-testid="tests-stale" role="status">
              This discovery is stale, not failed: {staleReason} Discover again before selecting
              tests.
            </p>
          )}
          {discovery.termination && (
            <p className="muted" data-testid="tests-termination">
              Termination requested: {discovery.termination.requested}. Observed:{" "}
              {discovery.termination.observed}
              {discovery.termination.killedAfterGrace ? " after the grace period" : ""}.
            </p>
          )}
          {discovery.awaitingDecision && (
            <p className="warning" data-testid="tests-awaiting">
              Still running; the host has not confirmed termination. Decide in the process panel.
            </p>
          )}

          <p className="muted" data-testid="tests-counts">
            {items.length} test{items.length === 1 ? "" : "s"} · {selected.size} selected ·{" "}
            {discovery.discoveryId ?? "no discovery"} · revision {discovery.revision}
            {discovery.partial ? " · partial" : ""}
          </p>
          {items.length > 0 && (
            <ul className="test-list" data-testid="tests-items" aria-label="Discovered tests">
              {items.map((item, index) => (
                <li key={`${item.discoveryId}:${item.id}`} data-testid={`test-item-${index}`}>
                  <label>
                    <input
                      type="checkbox"
                      data-testid={`test-checkbox-${index}`}
                      checked={selected.has(item.id)}
                      disabled={selectionBlocked}
                      onChange={() => toggle(item.id)}
                      aria-label={itemLabel(item)}
                    />
                    <span>
                      <code data-testid={`test-identity-${index}`}>
                        {item.fullyQualifiedName}
                      </code>
                      <span className="muted"> display name: {item.displayName}</span>
                      <span className="muted">
                        {" "}
                        · {item.project} · {item.tfm ?? "TFM not selected"} · {item.source}
                      </span>
                    </span>
                  </label>
                </li>
              ))}
            </ul>
          )}
          {selected.size > 0 && stale && (
            <p className="muted" data-testid="tests-selection-blocked">
              Selection is disabled until discovery runs again for the current code.
            </p>
          )}
          <pre className="eval-pre build-log" data-testid="tests-log">
            {discovery.log || "No output yet."}
          </pre>
          {discovery.logTruncated && (
            <p className="muted" data-testid="tests-log-truncated">
              The discovery log is bounded and the earliest lines were dropped; the note at the top
              of the log says how many.
            </p>
          )}
        </>
      )}
    </div>
  );
}
