import { Fragment } from "react";
import type { Build, BuildLogLink, Evaluation, Run } from "./host/client";

type Props = {
  build: Build;
  run: Run;
  shortcuts: { run?: string; build?: string; solution?: string };
  dirtyCount: number;
  runDisabledReason: string | null;
  onRun: () => void;
  onSaveAndRebuild: () => void;
  evaluation: Evaluation;
  startupPath: string | null;
  onStartup: (path: string) => void;
  onEvaluate: (next: { configuration?: string; targetFramework?: string | null }) => void;
  onBuild: () => void;
  onBuildSolution: () => void;
  solutionBuildDisabledReason: string | null;
  onCancel: () => void;
  disabledReason: string | null;
  trusted: boolean;
  onTrust: () => void;
  restoreRequired: boolean;
  restoreDisabledReason: string | null;
  onRestore: () => void;
  obsolete: boolean;
  onNavigate: (path: string, position: { line: number; character: number }) => void;
};

export function buildStateLabel(build: Build) {
  const exit = build.exit === null ? "exit unknown" : `exit ${build.exit}`;
  switch (build.state) {
    case "running":
      return "Building…";
    case "ok":
      return `Build succeeded · ${exit}`;
    case "failed":
      return `Build failed · ${exit}`;
    case "cancelled":
      return `Build cancelled · ${exit}`;
    default:
      return "No build has run";
  }
}

export function isBuildObsolete(build: Build, editedAt: Record<string, number>) {
  if (build.revision === 0) return false;
  const reference = build.finishedAt ?? build.startedAt ?? 0;
  return Object.values(editedAt).some((edited) => edited > reference);
}

export function runStateLabel(run: Run) {
  const exit = run.exit === null ? "exit unknown" : `exit ${run.exit}`;
  switch (run.state) {
    case "building":
      return "Building…";
    case "running":
      return "Running…";
    case "exited":
      return run.signal
        ? `Run crashed · ${run.signal}`
        : `Run exited · ${exit}`;
    case "failed":
      return "Run failed to start";
    case "cancelled":
      return `Run cancelled · ${exit}`;
    default:
      return "No program has run";
  }
}

export function runShortcutLabel(platform: string) {
  return platform === "windows" ? "Shift+F10" : "Ctrl+R";
}

export function timeLabel(value: number | null) {
  return value === null ? "unknown" : new Date(value).toLocaleString();
}

/** Renders the raw log, replacing only the spans the host marked as navigable. The log is
 * never reparsed here: an unknown line is plain text by construction. */
function buildLogContent(
  log: string,
  links: BuildLogLink[],
  onNavigate: Props["onNavigate"],
) {
  if (!log) return "No output yet.";
  const byLine = new Map(links.map((link) => [link.line, link]));
  const lines = log.split("\n");
  return lines.map((line, index) => {
    const link = byLine.get(index);
    const newline = index < lines.length - 1 ? "\n" : "";
    if (!link) {
      return (
        <Fragment key={index}>
          {line}
          {newline}
        </Fragment>
      );
    }
    const chars = Array.from(line);
    const start = Math.min(link.start, chars.length);
    const end = Math.min(Math.max(link.end, start), chars.length);
    return (
      <Fragment key={index}>
        {chars.slice(0, start).join("")}
        <button
          type="button"
          className="link-button"
          data-testid={`build-log-link-${index}`}
          data-position={`${link.position.line}:${link.position.character}`}
          data-path={link.path}
          title={`Open ${link.path}:${link.position.line + 1}:${link.position.character + 1}`}
          onClick={() => onNavigate(link.path, link.position)}
        >
          {chars.slice(start, end).join("")}
        </button>
        {chars.slice(end).join("")}
        {newline}
      </Fragment>
    );
  });
}

export function BuildPanel({
  build,
  run,
  shortcuts,
  dirtyCount,
  runDisabledReason,
  onRun,
  onSaveAndRebuild,
  evaluation,
  startupPath,
  onStartup,
  onEvaluate,
  onBuild,
  onBuildSolution,
  solutionBuildDisabledReason,
  onCancel,
  disabledReason,
  trusted,
  onTrust,
  restoreRequired,
  restoreDisabledReason,
  onRestore,
  obsolete,
  onNavigate,
}: Props) {
  const candidates = evaluation.projects.filter(
    (project) => project.kind === "console" && project.state === "ok" && project.targetPath,
  );
  const baseConfigurations =
    evaluation.configurations.length > 0 ? evaluation.configurations : ["Debug", "Release"];
  const configurations = baseConfigurations.includes(evaluation.context.configuration)
    ? baseConfigurations
    : [evaluation.context.configuration, ...baseConfigurations];
  const tfmOptions =
    evaluation.context.targetFramework &&
    !evaluation.availableTfms.includes(evaluation.context.targetFramework)
      ? [evaluation.context.targetFramework, ...evaluation.availableTfms]
      : evaluation.availableTfms;
  const running = build.state === "running";
  const shortcut = shortcuts.run ?? "";
  const buildShortcut = shortcuts.build ?? "";
  const solutionShortcut = shortcuts.solution ?? "";
  const startup =
    candidates.find((project) => project.path === startupPath) ?? null;

  const buildLabel = running
    ? "Building…"
    : `Build startup${startup ? ` '${startup.name}'` : ""} (${buildShortcut})`;

  return (
    <div className="build-panel" data-testid="build-panel">
      <div className="build-controls">
        <label className="eval-field">
          <span>Startup project</span>
          <select
            data-testid="build-startup"
            value={startup?.path ?? ""}
            disabled={candidates.length === 0 || running}
            onChange={(event) => onStartup(event.target.value)}
            title={
              candidates.length === 0
                ? "No console executable project is available in the current evaluation; libraries and test projects cannot be startup projects."
                : candidates.length > 1 && !startup
                  ? "More than one console project; choose the startup project explicitly."
                  : "Console executable project evaluated for this context."
            }
          >
            {!startup && (
              <option value="">
                {candidates.length === 0 ? "No console project" : "Select a startup project"}
              </option>
            )}
            {candidates.map((project) => (
              <option key={project.path} value={project.path}>
                {project.name}
              </option>
            ))}
          </select>
        </label>
        <label className="eval-field">
          <span>Configuration</span>
          <select
            data-testid="build-configuration"
            value={evaluation.context.configuration}
            disabled={!evaluation.context.configuration || running}
            onChange={(event) => onEvaluate({ configuration: event.target.value })}
            title="Changing the configuration re-evaluates the project model instead of reusing the previous TargetPath."
          >
            {configurations.map((configuration) => (
              <option key={configuration} value={configuration}>
                {configuration}
              </option>
            ))}
          </select>
        </label>
        <label className="eval-field">
          <span>TFM</span>
          <select
            data-testid="build-tfm"
            value={evaluation.context.targetFramework ?? ""}
            disabled={running}
            onChange={(event) =>
              onEvaluate({ targetFramework: event.target.value === "" ? null : event.target.value })
            }
            title="Changing the TFM re-evaluates the project model before building."
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
          data-testid="build-action"
          disabled={disabledReason !== null}
          onClick={onBuild}
          title={`${buildLabel} — ${
            disabledReason ??
            `runs an incremental dotnet build of the startup project; use Build Solution (${solutionShortcut}) for the whole solution.`
          }`}
        >
          {buildLabel}
        </button>
        <button
          data-testid="build-solution"
          disabled={solutionBuildDisabledReason !== null}
          onClick={onBuildSolution}
          title={
            solutionBuildDisabledReason ??
            "Runs dotnet build for the open solution with the current evaluation. Never restores and never saves buffers."
          }
        >
          Build Solution
        </button>
        {running && (
          <button
            data-testid="build-cancel"
            onClick={onCancel}
            title="Requests termination of the build process group and reports what was observed."
          >
            Cancel build
          </button>
        )}
        {(build.state === "failed" || build.state === "cancelled") && (
          <button
            data-testid="build-retry"
            disabled={disabledReason !== null}
            onClick={onBuild}
            title={disabledReason ?? "Runs a new explicit build of the same startup project."}
          >
            Retry build
          </button>
        )}
        <button
          data-testid="run-action"
          disabled={runDisabledReason !== null || run.state === "running" || run.state === "building"}
          onClick={() => onRun()}
          title={
            runDisabledReason ??
            `Runs the saved build's TargetPath, building the startup project first when the saved build is missing or cannot be used (${shortcut}). Restore stays explicit.`
          }
        >
          {run.state === "building" ? "Building…" : run.state === "running" ? "Running…" : `Run (${shortcut})`}
        </button>
        <span
          className={build.revision === 0 ? "muted" : `tag ${build.state === "failed" ? "error" : ""}`}
          data-testid="build-state"
          role="status"
        >
          {buildStateLabel(build)}
          {build.stale ? " · stale" : ""}
        </span>
      </div>

      {/* Restricted mode is the only gate where trusting from the reason itself makes sense. */}
      {disabledReason && (
        <p className="muted" data-testid="build-disabled-reason">
          {disabledReason}
          {!trusted && (
            <>
              {" "}
              <button
                type="button"
                data-testid="build-trust"
                onClick={onTrust}
                title="Grants trust for this project path and remembers it; the Build gate is re-evaluated."
              >
                Trust this project
              </button>
            </>
          )}
        </p>
      )}
      {runDisabledReason && (
        <p className="muted" data-testid="run-disabled-reason">
          Run unavailable: {runDisabledReason}
          {!trusted && (
            <>
              {" "}
              <button
                type="button"
                data-testid="run-trust"
                onClick={onTrust}
                title="Grants trust for this project path and remembers it; the Run gate is re-evaluated."
              >
                Trust this project
              </button>
            </>
          )}
        </p>
      )}

      {build.state === "ok" && dirtyCount > 0 && (
        <div className="notice warning" data-testid="run-dirty-warning" role="status">
          Running the saved build; {dirtyCount} unsaved buffer{dirtyCount === 1 ? "" : "s"}{" "}
          {dirtyCount === 1 ? "is" : "are"} not included.
          <div className="actions">
            <button
              data-testid="run-warning-save-rebuild"
              onClick={onSaveAndRebuild}
              title="Saves every modified buffer and runs the existing build flow; nothing is saved silently."
            >
              Save all and rebuild
            </button>
          </div>
        </div>
      )}

      {restoreRequired && (
        <div className="notice warning" data-testid="build-restore-required" role="status">
          <strong>Build needs restored assets.</strong> OpenRidr never restores inside a build;
          run the explicit restore first.
          <div className="actions">
            <button
              data-testid="build-restore-action"
              disabled={restoreDisabledReason !== null}
              onClick={onRestore}
              title={restoreDisabledReason ?? "Calls restore.start for this workspace."}
            >
              Restore now
            </button>
          </div>
        </div>
      )}

      {build.revision > 0 && (
        <>
          <details className="build-context-details" aria-label="Build details">
            <summary>Build details</summary>
            <dl className="restore-meta build-meta">
            {build.buildId && (
              <>
                <dt>Build identity</dt>
                <dd data-testid="build-id">
                  {build.buildId} · revision {build.revision} · context {build.contextId ?? "unknown"}
                </dd>
              </>
            )}
            <dt>Startup project</dt>
            <dd data-testid="build-project">
              {build.solution
                ? `${build.project?.name ?? "solution"} · whole solution`
                : `${build.project?.name ?? "unknown"} · ${build.project?.kind ?? "unknown"} · ${build.project?.path ?? "unknown"}`}
            </dd>
            <dt>TargetPath</dt>
            <dd>
              <code data-testid="build-target-path">
                {build.solution ? "every evaluated project" : build.project?.targetPath ?? "unknown"}
              </code>
            </dd>
            <dt>Configuration / TFM</dt>
            <dd>
              {build.configuration} · {build.targetFramework ?? "single-target project"}
            </dd>
            <dt>Command</dt>
            <dd>
              <code className="detail-command" data-testid="build-command">{build.command.join(" ")}</code>
            </dd>
            <dt>Working directory</dt>
            <dd>
              <code data-testid="build-cwd">{build.cwd}</code>
            </dd>
            <dt>Project SDK</dt>
            <dd data-testid="build-sdk">
              {build.sdkVersion ?? "unknown"} · <code>{build.sdkRoot ?? "unknown"}</code>
            </dd>
            <dt>Saved content</dt>
            <dd data-testid="build-saved-note">{build.savedNote ?? "not recorded"}</dd>
            </dl>
          </details>
          {obsolete && (
            <p className="warning" data-testid="build-obsolete" role="status">
              This build predates the current buffer edits. Its result is obsolete, not fixed;
              rebuild to verify the saved content.
            </p>
          )}
          {build.sealed && (
            <p className="warning" data-testid="build-sealed">
              Previous build, not current: {build.staleReason}
            </p>
          )}
          {!build.sealed && build.stale && (
            <p className="warning" data-testid="build-stale" role="status">
              Build result is stale: {build.staleReason}
            </p>
          )}
          {build.termination && (
            <p className="muted" data-testid="build-termination">
              Termination requested: {build.termination.requested}. Observed:{" "}
              {build.termination.observed}
              {build.termination.killedAfterGrace ? " after the grace period" : ""}.
            </p>
          )}
          {build.awaitingDecision && (
            <p className="warning" data-testid="build-awaiting">
              Still running; the host has not confirmed termination. Decide in the process panel.
            </p>
          )}
          <p className="muted" data-testid="build-problems-count">
            {build.problems.length} build problem{build.problems.length === 1 ? "" : "s"} · origin
            Build · dotnet{build.problemsTruncated ? " · capped" : ""}
          </p>
          {build.problemsTruncated && (
            <p className="warning" data-testid="build-problems-capped">
              More than {build.problems.length} diagnostics were printed; only the first{" "}
              {build.problems.length} are listed. The raw log keeps every line.
            </p>
          )}
          <pre className="eval-pre build-log" data-testid="build-log">
            {buildLogContent(build.log, build.logLinks, onNavigate)}
          </pre>
          {build.logTruncated && (
            <p className="muted" data-testid="build-log-truncated">
              The log is bounded and the earliest lines were dropped; the note at the top of the
              log says how many.
            </p>
          )}
        </>
      )}

    </div>
  );
}
