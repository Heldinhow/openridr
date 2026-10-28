import type { Build, Evaluation } from "./host/client";

type Props = {
  build: Build;
  evaluation: Evaluation;
  startupPath: string | null;
  onStartup: (path: string) => void;
  onEvaluate: (next: { configuration?: string; targetFramework?: string | null }) => void;
  onBuild: () => void;
  onCancel: () => void;
  disabledReason: string | null;
  restoreRequired: boolean;
  restoreDisabledReason: string | null;
  onRestore: () => void;
  obsolete: boolean;
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

export function BuildPanel({
  build,
  evaluation,
  startupPath,
  onStartup,
  onEvaluate,
  onBuild,
  onCancel,
  disabledReason,
  restoreRequired,
  restoreDisabledReason,
  onRestore,
  obsolete,
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
  const startup =
    candidates.find((project) => project.path === startupPath) ?? null;

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
            {candidates.length === 0 && <option value="">No console project</option>}
            {candidates.length > 1 && !startup && (
              <option value="">Select a startup project</option>
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
          title={
            disabledReason ??
            "Runs dotnet build for the selected startup project with the current evaluation. Never restores and never saves buffers."
          }
        >
          {running ? "Building…" : build.revision > 0 ? "Rebuild (Cmd/Ctrl+F9)" : "Build (Cmd/Ctrl+F9)"}
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
        <span
          className={`tag ${build.state === "failed" ? "error" : ""}`}
          data-testid="build-state"
          role="status"
        >
          {buildStateLabel(build)}
          {build.stale ? " · stale" : ""}
        </span>
        {build.buildId && (
          <span className="muted" data-testid="build-id">
            {build.buildId} · revision {build.revision} · context {build.contextId ?? "unknown"}
          </span>
        )}
      </div>

      {disabledReason && <p className="muted" data-testid="build-disabled-reason">{disabledReason}</p>}

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
          <dl className="restore-meta build-meta">
            <dt>Startup project</dt>
            <dd data-testid="build-project">
              {build.project?.name ?? "unknown"} · {build.project?.kind ?? "unknown"} ·{" "}
              <code>{build.project?.path ?? "unknown"}</code>
            </dd>
            <dt>TargetPath</dt>
            <dd>
              <code data-testid="build-target-path">{build.project?.targetPath ?? "unknown"}</code>
            </dd>
            <dt>Configuration / TFM</dt>
            <dd>
              {build.configuration} · {build.targetFramework ?? "single-target project"}
            </dd>
            <dt>Command</dt>
            <dd>
              <code data-testid="build-command">{build.command.join(" ")}</code>
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
            {build.log || "No output yet."}
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
