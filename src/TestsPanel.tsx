import { useEffect, useState } from "react";
import type { Evaluation, TestItem, TestResult, TestRun, Tests, TestsDiscovery } from "./host/client";

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
  onRun: (project: string, selection: "all" | "selected", testIds: string[]) => void;
  onRunCancel: () => void;
  onNavigate: (path: string, position: { line: number; character: number }) => void;
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
  return `${item.fullyQualifiedName} · ${item.project} · ${item.tfm ?? "TFM not selected"}${
    item.occurrences > 1 ? " · listed more than once by VSTest" : ""
  }`;
}

export function runStateLabel(run: TestRun) {
  const exit = run.exit === null ? "exit unknown" : `exit ${run.exit}`;
  switch (run.state) {
    case "running":
      return "Running tests…";
    case "completed":
      return `Run completed · ${run.counts.total} result${run.counts.total === 1 ? "" : "s"} · ${exit}`;
    case "failed":
      return `Run failed · ${run.infrastructureFailure ? "infrastructure" : "run"} · ${exit}`;
    case "cancelled":
      return `Run cancelled · ${run.counts.total} partial result${run.counts.total === 1 ? "" : "s"} · ${exit}`;
    default:
      return "No test run";
  }
}

/** Buffer edits after the run started make the results obsolete, never corrected. */
export function isRunObsolete(run: TestRun, editedAt: Record<string, number>) {
  if (run.revision === 0) return false;
  const reference = run.startedAt ?? run.finishedAt ?? 0;
  return Object.values(editedAt).some((edited) => edited > reference);
}

function outcomeTone(result: TestResult) {
  return result.outcome === "failed" ? "error" : result.outcome === "passed" ? "success" : "";
}

function resultDuration(result: TestResult) {
  return result.durationMs === null ? "duration unknown" : `${result.durationMs} ms`;
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
  onRun,
  onRunCancel,
  onNavigate,
}: Props) {
  const discovery = tests.discovery;
  const run = tests.run;
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
  const runRunning = run.state === "running";
  const runObsolete = isRunObsolete(run, editedAt);
  const runBlocked =
    disabledReason ??
    (running
      ? "A discovery is running; wait for it to finish before running tests."
      : discovery.state !== "completed"
        ? "A completed, current discovery is required before running tests."
        : stale
          ? staleReason
          : discovery.sealed
            ? "The discovery was sealed; discover again before running tests."
            : items.length === 0
              ? "The discovery listed no tests."
              : runRunning
                ? "A test run is already running."
                : null);
  const retryBlocked =
    runBlocked ??
    (run.discoveryId !== discovery.discoveryId || run.discoveryRevision !== discovery.revision
      ? "This run belongs to another discovery; run again from the current list."
      : null);

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
        <button
          data-testid="tests-run-all"
          disabled={runBlocked !== null}
          onClick={() => selectedProject && onRun(selectedProject.path, "all", [])}
          title={
            runBlocked ??
            "Runs every identity of the current discovery with VSTest --no-build --no-restore and the built-in TRX logger."
          }
        >
          Run all
        </button>
        <button
          data-testid="tests-run-selected"
          disabled={runBlocked !== null || selected.size === 0}
          onClick={() =>
            selectedProject && onRun(selectedProject.path, "selected", Array.from(selected))
          }
          title={
            runBlocked ??
            (selected.size === 0
              ? "Select at least one discovered test; an empty selection never runs all tests."
              : `Runs exactly ${selected.size} selected identit${selected.size === 1 ? "y" : "ies"}.`)
          }
        >
          Run selected
        </button>
        {runRunning && (
          <button
            data-testid="tests-run-cancel"
            onClick={onRunCancel}
            title="Requests termination of the test process group and keeps the partial results, TRX and log."
          >
            Cancel run
          </button>
        )}
        {(run.state === "failed" || run.state === "cancelled") && !runRunning && (
          <button
            data-testid="tests-run-retry"
            disabled={retryBlocked !== null}
            onClick={() =>
              selectedProject &&
              onRun(
                selectedProject.path,
                run.selection,
                run.selection === "selected" ? run.selectedTestIds : [],
              )
            }
            title={
              retryBlocked ??
              "Starts a new explicit run with the same selection; nothing is retried automatically."
            }
          >
            Retry run
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
                      disabled={selectionBlocked || item.occurrences > 1}
                      onChange={() => toggle(item.id)}
                      aria-label={itemLabel(item)}
                    />
                    <span>
                      <code data-testid={`test-identity-${index}`}>
                        {item.fullyQualifiedName}
                      </code>
                      {item.occurrences > 1 && (
                        <span className="warning" data-testid={`test-ambiguous-${index}`}>
                          {" "}
                          listed {item.occurrences} times by VSTest; the runner gave no unique
                          identity, so it cannot be run individually.
                        </span>
                      )}
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

      {run.revision > 0 && (
        <section className="build-panel" aria-label="Test run" data-testid="tests-run-panel">
          <div className="build-controls">
            <span
              className={`tag ${run.state === "failed" ? "error" : ""}`}
              data-testid="tests-run-state"
              role="status"
            >
              {runStateLabel(run)}
            </span>
            <span className="muted" data-testid="tests-run-id">
              {run.runId ?? "no run"} · {run.discoveryId ?? "no discovery"} · discovery revision{" "}
              {run.discoveryRevision} · run revision {run.revision} · context{" "}
              {run.contextId ?? "unknown"}
            </span>
            <span className="muted" data-testid="tests-run-selection">
              {run.selection === "all"
                ? "Selection: all discovered tests"
                : `Selection: ${run.requestedCount} selected identit${run.requestedCount === 1 ? "y" : "ies"}`}
            </span>
          </div>
          {run.stale && (
            <p className="warning" data-testid="tests-run-stale" role="status">
              These results are preserved but not current: {run.staleReason}
            </p>
          )}
          {!run.stale && runObsolete && (
            <p className="warning" data-testid="tests-run-stale" role="status">
              These results are preserved but not current: a buffer was edited after this run.
              Run tests again for the saved code.
            </p>
          )}
          {run.sealed && (
            <p className="warning" data-testid="tests-run-sealed">
              Previous run, not current: {run.staleReason}
            </p>
          )}
          {run.error && (
            <p
              className={run.infrastructureFailure ? "warning" : "muted"}
              data-testid={run.infrastructureFailure ? "tests-run-infrastructure" : "tests-run-note"}
              role="status"
            >
              {run.error}
            </p>
          )}
          <p className="muted" data-testid="tests-run-counts">
            {run.counts.total} result{run.counts.total === 1 ? "" : "s"} · {run.counts.passed} passed ·{" "}
            {run.counts.failed} failed · {run.counts.skipped} skipped · {run.counts.notExecuted} not
            executed · {run.counts.unknown} unknown
            {run.resultsTruncated ? " · results list truncated" : ""}
          </p>
          <dl className="restore-meta build-meta">
            <dt>Command</dt>
            <dd>
              <code data-testid="tests-run-command">
                {run.command.length > 0 ? run.command.join(" ") : "No command was run."}
              </code>
            </dd>
            <dt>Exit</dt>
            <dd data-testid="tests-run-exit">
              {run.exit === null ? "exit unknown" : `exit ${run.exit}`}
            </dd>
            <dt>TRX</dt>
            <dd data-testid="tests-run-trx">
              <code>{run.trxPath ?? "no TRX"}</code>{" "}
              <span className="muted" data-testid="tests-run-trx-state">
                {run.trxState}
              </span>
            </dd>
            <dt>Raw log</dt>
            <dd data-testid="tests-run-log-path">
              <code>{run.logPath ?? "no log file"}</code>
            </dd>
            <dt>Results directory</dt>
            <dd>
              <code data-testid="tests-run-results">{run.resultsDirectory ?? "unknown"}</code>
            </dd>
          </dl>
          {run.termination && (
            <p className="muted" data-testid="tests-run-termination">
              Termination requested: {run.termination.requested}. Observed:{" "}
              {run.termination.observed}
              {run.termination.killedAfterGrace ? " after the grace period" : ""}.
            </p>
          )}
          {run.awaitingDecision && (
            <p className="warning" data-testid="tests-run-awaiting">
              Still running; the host has not confirmed termination. Decide in the process panel.
            </p>
          )}
          {run.results.length > 0 && (
            <ul className="test-list" data-testid="tests-run-results-list" aria-label="Test results">
              {run.results.map((result, index) => (
                <li key={`${run.runId}:${index}`} data-testid={`test-result-${index}`}>
                  <span className={`tag ${outcomeTone(result)}`} data-testid={`test-result-outcome-${index}`}>
                    {result.outcome}
                  </span>{" "}
                  <code data-testid={`test-result-name-${index}`}>
                    {result.fullyQualifiedName || "unattributed (no TestDefinitions identity)"}
                  </code>{" "}
                  <span className="muted">
                    {resultDuration(result)} · source {result.outcomeRaw}
                    {result.matched
                      ? ""
                      : result.fullyQualifiedName
                        ? " · not matched to the discovery"
                        : ` · TRX testName only: ${result.displayName}`}
                  </span>
                  <details data-testid={`test-result-details-${index}`}>
                    <summary>Details</summary>
                    {result.message && (
                      <pre className="eval-pre" data-testid={`test-result-message-${index}`}>
                        {result.message}
                      </pre>
                    )}
                    {result.stackTrace && (
                      <>
                        <strong>Stack trace</strong>
                        <pre className="eval-pre" data-testid={`test-result-stack-${index}`}>
                          {result.stackTrace}
                        </pre>
                      </>
                    )}
                    {result.stdout && (
                      <>
                        <strong>stdout</strong>
                        <pre className="eval-pre" data-testid={`test-result-stdout-${index}`}>
                          {result.stdout}
                        </pre>
                      </>
                    )}
                    {result.stderr && (
                      <>
                        <strong>stderr</strong>
                        <pre className="eval-pre" data-testid={`test-result-stderr-${index}`}>
                          {result.stderr}
                        </pre>
                      </>
                    )}
                    {result.position ? (
                      <button
                        data-testid={`test-result-navigate-${index}`}
                        onClick={() =>
                          result.position &&
                          onNavigate(result.position.path, {
                            line: result.position.line,
                            character: result.position.character,
                          })
                        }
                        title="Opens the file at the position the stack trace reported."
                      >
                        Open source position
                      </button>
                    ) : (
                      <p className="muted" data-testid={`test-result-no-position-${index}`}>
                        The source reported no `in &lt;file&gt;:line &lt;n&gt;` position inside the
                        workspace; use the raw run log instead.
                      </p>
                    )}
                  </details>
                </li>
              ))}
            </ul>
          )}
          <pre className="eval-pre build-log" data-testid="tests-run-log">
            {run.log || "No output yet."}
          </pre>
          {run.logTruncated && (
            <p className="muted" data-testid="tests-run-log-truncated">
              The run log is bounded and the earliest lines were dropped; the note at the top of
              the log says how many.
            </p>
          )}
        </section>
      )}
    </div>
  );
}
