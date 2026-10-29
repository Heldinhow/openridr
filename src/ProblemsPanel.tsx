import { useState } from "react";
import type { Build, DiagnosticSnapshot, Diagnostics, ProblemItem } from "./host/client";

type Row = { key: string; snapshot: DiagnosticSnapshot; item: ProblemItem };

type Props = {
  diagnostics: Diagnostics;
  build: Build;
  /** A buffer was edited after the build finished. */
  buildObsolete?: boolean;
  /** Local edit times; a publication older than the buffer is shown as awaiting at once. */
  editedAt?: Record<string, number>;
  onNavigate: (path: string, position: { line: number; character: number }) => void;
  onOpen: (path: string) => void;
};

function buildProblemState(build: Build, obsolete: boolean) {
  if (build.sealed) return "Sealed";
  if (build.stale) return "Stale";
  if (obsolete) return "Obsolete";
  return "Current";
}

function buildProblemLocation(problem: Build["problems"][number]) {
  return problem.position
    ? `${problem.position.line + 1}:${problem.position.character + 1}`
    : "position unknown";
}

function stateLabel(snapshot: DiagnosticSnapshot) {
  switch (snapshot.state) {
    case "awaiting":
      return "Awaiting diagnostics";
    case "stale":
      return "Stale";
    default:
      return "Current";
  }
}

function revisionLabel(snapshot: DiagnosticSnapshot) {
  return snapshot.clientRevision === null
    ? "revision unknown"
    : `revision ${snapshot.clientRevision}`;
}

function locationLabel(item: ProblemItem) {
  return item.position
    ? `${item.position.line + 1}:${item.position.character + 1}`
    : "position unknown";
}

function codeLabel(item: ProblemItem) {
  return item.code === null || item.code === "" ? "no code" : String(item.code);
}

function countText(snapshots: DiagnosticSnapshot[]) {
  const counts = { error: 0, warning: 0, information: 0, hint: 0, unknown: 0 };
  for (const snapshot of snapshots) {
    if (snapshot.state !== "current") continue;
    for (const item of snapshot.items) {
      counts[item.severityLabel === "unknown" ? "unknown" : item.severityLabel] += 1;
    }
  }
  const { error, warning, information, hint, unknown } = counts;
  return `${error} error${error === 1 ? "" : "s"} · ${warning} warning${
    warning === 1 ? "" : "s"
  } · ${information} information · ${hint} hint${hint === 1 ? "" : "s"}${
    unknown ? ` · ${unknown} unknown severity` : ""
  }`;
}

/** A local edit newer than the publication makes its items awaiting before the next poll. */
export function withLocalEdits(
  diagnostics: Diagnostics,
  editedAt: Record<string, number>,
) {
  return diagnostics.snapshots.map((snapshot) =>
    snapshot.state === "current" &&
    snapshot.path !== null &&
    (editedAt[snapshot.path] ?? 0) > snapshot.receivedAt
      ? {
          ...snapshot,
          state: "awaiting" as const,
          staleReason:
            "The buffer was edited after this publication; awaiting diagnostics for the new revision.",
        }
      : snapshot,
  );
}

export function ProblemsPanel({
  diagnostics,
  build,
  buildObsolete = false,
  editedAt = {},
  onNavigate,
  onOpen,
}: Props) {
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [document, setDocument] = useState("");
  const snapshots = withLocalEdits(diagnostics, editedAt);
  const documents = Array.from(
    new Set(
      snapshots
        .map((snapshot) => snapshot.path)
        .filter((path): path is string => Boolean(path)),
    ),
  ).sort();
  const visible = snapshots.filter(
    (snapshot) => document === "" || snapshot.path === document,
  );
  const rows: Row[] = visible.flatMap((snapshot) =>
    snapshot.items.map((item) => ({
      key: `${snapshot.id}:${item.id}`,
      snapshot,
      item,
    })),
  );
  const selected = rows.find((row) => row.key === selectedKey) ?? null;
  const currentEmpty = visible.filter(
    (snapshot) => snapshot.state === "current" && snapshot.items.length === 0,
  );
  const staleSnapshots = snapshots.filter(
    (snapshot) => snapshot.state !== "current",
  );
  const notCurrent =
    diagnostics.staleItems > 0 ||
    diagnostics.awaiting.length > 0 ||
    staleSnapshots.length > 0 ||
    diagnostics.analysisUnavailable !== null;
  const canNavigate =
    selected !== null &&
    selected.snapshot.state === "current" &&
    selected.item.position !== null &&
    selected.snapshot.path !== null;
  const navigateReason = selected
    ? selected.snapshot.state !== "current"
      ? "Not navigating: this diagnostic is not from the current revision/session. The published revision is not current. Open the document without positioning if you need to inspect it."
      : selected.item.position === null
        ? "The source published no range for this diagnostic; OpenRidr does not guess a line or column."
        : selected.snapshot.path === null
          ? "The publication has no document path; navigation is not offered."
          : "Moves the editor cursor to the published position."
    : "Select a problem first.";

  function selectRow(row: Row) {
    setSelectedKey(row.key);
    if (
      row.snapshot.state === "current" &&
      row.item.position !== null &&
      row.snapshot.path !== null
    ) {
      onNavigate(row.snapshot.path, row.item.position);
    }
  }

  return (
    <div className="problems" data-testid="problems-panel">
      <div className="problems-head">
        <span data-testid="problems-coverage">
          {diagnostics.summaries.length > 0
            ? diagnostics.summaries
                .map(
                  (summary) =>
                    `${summary.origin.label} · session ${summary.session} · ${summary.currentItems} current`,
                )
                .join("  |  ")
            : "No analysis origin has published diagnostics"}
          {" · coverage "}
          {diagnostics.coverage}
          {" · Analyzer coverage: not verified"}
          {" · No solution-wide clean state is claimed"}
        </span>
        <span className="muted" data-testid="problems-counts">
          {countText(snapshots)}
        </span>
        {build.revision > 0 && (
          <span className="muted" data-testid="problems-build-count">
            Build · dotnet · {build.problems.length} problem
            {build.problems.length === 1 ? "" : "s"} · separate origin
          </span>
        )}
        <label className="problems-filter">
          <span>Document</span>
          <select
            data-testid="problems-filter"
            value={document}
            onChange={(event) => {
              setDocument(event.target.value);
              setSelectedKey(null);
            }}
            title="Filters the list to one document; coverage statements remain per document."
          >
            <option value="">All documents</option>
            {documents.map((path) => (
              <option key={path} value={path}>
                {path}
              </option>
            ))}
          </select>
        </label>
      </div>

      {notCurrent && (
        <div className="notice warning" data-testid="problem-stale" role="status">
          <strong>Some diagnostics are not current.</strong> Previous items are kept and
          labelled stale or awaiting; editing does not resolve them by anticipation.
          {diagnostics.analysisUnavailable && <> {diagnostics.analysisUnavailable}</>}
        </div>
      )}

      {diagnostics.coverage === "partial" && (
        <div className="notice warning" data-testid="problems-partial" role="status">
          <strong>Analysis coverage is partial.</strong>{" "}
          {diagnostics.coverageNote ??
            "The project model or restore is incomplete; diagnostics may cover only part of the solution."}
        </div>
      )}

      <div className="problems-body">
        <div className="problems-list-wrap">
          <div className="problems-list" data-testid="problems-list" aria-label="Problems">
            {rows.length > 0 && (
              <div className="problem-row problem-head" aria-hidden="true">
                <span>Severity</span>
                <span>Code</span>
                <span>Message</span>
                <span>Location</span>
                <span>Document</span>
                <span>Origin</span>
                <span>Category</span>
                <span>Revision</span>
                <span>State</span>
              </div>
            )}
            {rows.map((row, index) => (
              <button
                key={row.key}
                type="button"
                className="problem-row"
                data-testid={`problem-row-${index}`}
                data-state={row.snapshot.state}
                aria-current={selectedKey === row.key ? "true" : undefined}
                onClick={() => selectRow(row)}
                title={row.item.message}
              >
                <span className={`problem-severity severity-${row.item.severityLabel}`}>
                  {row.item.severityLabel}
                </span>
                <span className="problem-code">{codeLabel(row.item)}</span>
                <span className="problem-message">{row.item.message}</span>
                <span>{locationLabel(row.item)}</span>
                <span>{row.snapshot.path ?? row.snapshot.uri}</span>
                <span>{row.snapshot.origin.label}</span>
                <span>{row.item.category}</span>
                <span>{revisionLabel(row.snapshot)}</span>
                <span className={`problem-state state-${row.snapshot.state}`}>
                  {stateLabel(row.snapshot)}
                </span>
              </button>
            ))}
            {currentEmpty.length > 0 && (
              <div className="problems-empty" data-testid="problems-empty">
                {currentEmpty.map((snapshot) => (
                  <p key={snapshot.id}>
                    No problems reported for {snapshot.path ?? snapshot.uri} ·{" "}
                    {revisionLabel(snapshot)}. Only this covered document/publication is
                    claimed; the solution is not claimed clean.
                  </p>
                ))}
              </div>
            )}
            {rows.length === 0 && currentEmpty.length === 0 && (
              <p className="muted" data-testid="problems-empty">
                {diagnostics.analysisUnavailable ??
                  (diagnostics.session === 0
                    ? "No analysis session has published diagnostics. Absence of a server or publication is not 'no problems'."
                    : "No current diagnostics publication covers the selected documents. Previous publications stay visible as stale when present.")}
              </p>
            )}
            {rows.length === 0 && diagnostics.awaiting.length > 0 && (
              <p className="muted" data-testid="problems-awaiting">
                Awaiting diagnostics for{" "}
                {diagnostics.awaiting
                  .map((item) => `${item.path} (revision ${item.revision})`)
                  .join(", ")}
                .
              </p>
            )}
          </div>
        </div>

        <aside className="problem-details" data-testid="problems-details" aria-label="Problem details">
          <div className="panel-title">Problem details</div>
          {selected ? (
            <>
              <p className="problem-details-message">{selected.item.message}</p>
              <dl>
                <dt>Severity</dt>
                <dd>{selected.item.severityLabel}</dd>
                <dt>Code</dt>
                <dd>{codeLabel(selected.item)}</dd>
                <dt>Category</dt>
                <dd>{selected.item.category}</dd>
                <dt>Origin</dt>
                <dd>{selected.snapshot.origin.label}</dd>
                <dt>Source</dt>
                <dd>{selected.item.source ?? "not reported"}</dd>
                <dt>Document</dt>
                <dd>{selected.snapshot.path ?? selected.snapshot.uri}</dd>
                <dt>Location</dt>
                <dd>
                  {selected.item.position
                    ? `line ${selected.item.position.line + 1}, column ${selected.item.position.character + 1} (1-based)`
                    : "not published"}
                </dd>
                <dt>Session / revision</dt>
                <dd>
                  session {selected.snapshot.session} · {revisionLabel(selected.snapshot)}
                  {selected.snapshot.currentRevision !== null
                    ? ` · current revision ${selected.snapshot.currentRevision}`
                    : ""}
                </dd>
                <dt>State</dt>
                <dd data-testid="problem-details-state">{stateLabel(selected.snapshot)}</dd>
              </dl>
              {selected.snapshot.state !== "current" && (
                <p className="warning" data-testid="problem-details-stale">
                  {selected.snapshot.staleReason}
                </p>
              )}
              {selected.item.position === null && (
                <p className="muted" data-testid="problem-no-position">
                  No position was published for this diagnostic. Navigation is not offered;
                  open the document without positioning if needed.
                </p>
              )}
              <div className="actions">
                <button
                  type="button"
                  data-testid="problem-navigate"
                  disabled={!canNavigate}
                  onClick={() => {
                    if (
                      selected.snapshot.path !== null &&
                      selected.item.position !== null
                    ) {
                      onNavigate(selected.snapshot.path, selected.item.position);
                    }
                  }}
                  title={navigateReason}
                >
                  Go to position
                </button>
                <button
                  type="button"
                  data-testid="problem-open-document"
                  disabled={selected.snapshot.path === null}
                  onClick={() => {
                    if (selected.snapshot.path !== null) {
                      onOpen(selected.snapshot.path);
                    }
                  }}
                  title={
                    selected.snapshot.path === null
                      ? "The publication has no document path."
                      : "Opens the document without moving the cursor to the published position."
                  }
                >
                  Open document without positioning
                </button>
              </div>
            </>
          ) : (
            <p className="muted">
              Select a problem to see its full message, origin, category, session and
              revision. A diagnostic is never opened at a guessed position.
            </p>
          )}
        </aside>
      </div>

      {build.revision > 0 && (
        <section className="build-problems" data-testid="build-problems" aria-label="Build problems">
          <div className="build-problems-head">
            <strong>Build · dotnet</strong>
            <span data-testid="build-problems-state">
              {buildProblemState(build, buildObsolete)}
            </span>
            <span className="muted">
              {build.problems.length} problem{build.problems.length === 1 ? "" : "s"} ·{" "}
              {build.buildId ?? "unknown build"} · revision {build.revision} · separate from
              LSP · csharp-ls
            </span>
            <span className="push" />
            <span className="muted">
              A build result covers the compiled startup project; it is not a clean-state claim
              for the solution.
            </span>
          </div>
          {build.sealed && (
            <p className="warning" data-testid="build-problem-stale">
              Not current: {build.staleReason ?? "the build belongs to a previous context."}
            </p>
          )}
          {!build.sealed && build.stale && (
            <p className="warning" data-testid="build-problem-stale">
              Stale build: {build.staleReason ?? "the evaluated context changed."}
            </p>
          )}
          {!build.sealed && !build.stale && buildObsolete && (
            <p className="warning" data-testid="build-problem-obsolete">
              Obsolete: a buffer was edited after this build. These items are not marked fixed;
              rebuild to verify.
            </p>
          )}
          {build.problemsTruncated && (
            <p className="warning" data-testid="build-problems-capped">
              More diagnostics were printed than listed; the raw build log keeps every line.
            </p>
          )}
          <div className="build-problems-list" aria-label="Build diagnostics">
            {build.problems.length > 0 && (
              <div className="build-problem-row build-problem-head" aria-hidden="true">
                <span>Severity</span>
                <span>Code</span>
                <span>Message</span>
                <span>Location</span>
                <span>Document</span>
                <span>Origin</span>
                <span>Category</span>
                <span>State</span>
              </div>
            )}
            {build.problems.map((problem, index) => (
              <button
                key={problem.id}
                type="button"
                className="build-problem-row"
                data-testid={`build-problem-row-${index}`}
                data-has-position={problem.position !== null ? "true" : "false"}
                data-position={
                  problem.position
                    ? `${problem.position.line}:${problem.position.character}`
                    : undefined
                }
                onClick={() => {
                  if (problem.position) {
                    onNavigate(problem.path, problem.position);
                  } else {
                    onOpen(problem.path);
                  }
                }}
                title={
                  problem.position
                    ? `Open ${problem.path} at line ${problem.position.line + 1}`
                    : `Open ${problem.path} without positioning (MSBuild printed no position)`
                }
              >
                <span className={`problem-severity severity-${problem.severityLabel}`}>
                  {problem.severityLabel}
                </span>
                <span className="problem-code">{problem.code ?? "no code"}</span>
                <span className="problem-message">{problem.message}</span>
                <span>{buildProblemLocation(problem)}</span>
                <span>{problem.path}</span>
                <span>Build · dotnet</span>
                <span>{problem.category}</span>
                <span className={`problem-state state-${buildProblemState(build, buildObsolete).toLowerCase()}`}>
                  {buildProblemState(build, buildObsolete)}
                </span>
              </button>
            ))}
            {build.problems.length === 0 && (
              <p className="muted" data-testid="build-problems-empty">
                No MSBuild diagnostics were parsed from this build's log. Unrecognized lines stay
                in the raw log; absence of parsed problems is not a clean-solution claim.
              </p>
            )}
          </div>
        </section>
      )}
    </div>
  );
}
