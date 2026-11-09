import { profileTitle, type ProfileId } from "../actions/keymap";
import type { ProjectSdk, Snapshot } from "../host/client";
import type { PartialWorkspaceStatus } from "../evaluationStatus";
import { Icon } from "../Icon";
import { StatusMode } from "./StatusMode";

type EvaluationStatus = {
  text: string;
  title?: string;
  testId?: string;
  /** Failed/partial evaluations keep the old red cue in the status bar. */
  tone?: "error";
  /** The single action that produces a current model; null hides the affordance. */
  reevaluate: { label: string; disabledReason: string | null; onClick: () => void } | null;
};

type RestoreStatus = {
  state: "idle" | "running" | "ok" | "failed" | "cancelled";
  label: string;
  onCancel: () => void;
  onRetry: (() => void) | null;
};

export function StatusBar({ workspace, snapshot, onSnapshot, sdkSummary, sdk, evaluation, partial, restore, dirtyCount, diagnostics, analysis, breadcrumb, cursor, indent, keymapProfile }: {
  workspace: { name: string; path: string };
  snapshot: Snapshot;
  onSnapshot: (snapshot: Snapshot) => void;
  sdkSummary: string;
  /** The project SDK block, for the popover: the bar itself is one line and holds no link. */
  sdk: { project: ProjectSdk; onOpenGlobalJson: () => void; onCheckAgain: () => void };
  evaluation: EvaluationStatus;
  /** Incomplete evaluated context; the old tree footnote now lives here. */
  partial: PartialWorkspaceStatus | null;
  restore: RestoreStatus;
  dirtyCount: number;
  diagnostics: { errors: number; warnings: number; onOpen: () => void };
  /** Analysis (csharp-ls) state with the one action that moves it forward; null in Restricted mode. */
  analysis: { text: string; action: { label: string; disabledReason: string | null; onClick: () => void } | null } | null;
  breadcrumb: string;
  cursor: string;
  indent: { label: string; title: string };
  keymapProfile: ProfileId;
}) {
  return (
    // The status bar lives inside the app's main region, where `<footer>` alone has no landmark
    // role: the explicit role is what puts it (and its cursor/analysis/toolchain text) in the
    // accessibility tree as a named region.
    <footer className="status" role="contentinfo" aria-label="Status bar">
      <span className="sr-only" data-testid="status-workspace" title={workspace.path}>{workspace.name}</span>
      <span data-testid="status-breadcrumb" title={breadcrumb}>{breadcrumb}</span>
      <StatusMode
        snapshot={snapshot}
        onSnapshot={onSnapshot}
        sdkSummary={sdkSummary}
        sdk={sdk}
      />
      <span className="sr-only" data-testid="status-sdk" title={sdkSummary}>{sdkSummary}</span>
      {restore.state === "running" && (
        <span data-testid="status-restore">
          Restoring packages…{" "}
          <button type="button" data-testid="status-restore-cancel" onClick={restore.onCancel}>Cancel restore</button>
        </span>
      )}
      {(restore.state === "failed" || restore.state === "cancelled") && (
        <span className="error" data-testid="status-restore" title={restore.label}>
          {restore.label}{" "}
          {restore.onRetry && (
            <button type="button" data-testid="status-restore-retry" onClick={restore.onRetry}>Retry restore</button>
          )}
        </span>
      )}
      {evaluation.reevaluate && (
        <button
          type="button"
          className="status-evaluation-action"
          data-testid="eval-run"
          aria-label={evaluation.reevaluate.label}
          title={evaluation.reevaluate.disabledReason ?? evaluation.reevaluate.label}
          disabled={evaluation.reevaluate.disabledReason !== null}
          onClick={evaluation.reevaluate.onClick}
        >
          <Icon name="refresh" />
        </button>
      )}
      {evaluation.reevaluate?.disabledReason && (
        <span className="status-reason" data-testid="eval-blocked-reason">
          {evaluation.reevaluate.disabledReason}
        </span>
      )}
      {evaluation.testId ? (
        <span
          data-testid={evaluation.testId}
          title={evaluation.title}
          className={[evaluation.tone === "error" && "error", evaluation.text.includes("not current") && "is-stale"].filter(Boolean).join(" ")}
        >
          <span
            data-testid="eval-state"
            role="status"
            className={evaluation.tone === "error" ? "error" : undefined}
          >
            {evaluation.text}
          </span>
        </span>
      ) : (
        <span>{evaluation.text}</span>
      )}
      {partial && (
        <span className="status-partial" data-testid="tree-partial" title={partial.title}>
          {partial.text}
        </span>
      )}
      {analysis && (
        <span data-testid="status-analysis">
          {analysis.text}
          {analysis.action && (
            <>
              {" "}
              <button
                type="button"
                className="status-analysis-action"
                data-testid="status-analysis-action"
                disabled={analysis.action.disabledReason !== null}
                title={analysis.action.disabledReason ?? analysis.action.label}
                onClick={analysis.action.onClick}
              >
                {analysis.action.label}
              </button>
              {analysis.action.disabledReason && (
                <span className="status-reason" data-testid="status-analysis-blocked">
                  {analysis.action.disabledReason}
                </span>
              )}
            </>
          )}
        </span>
      )}
      {dirtyCount > 0 && (
        <span data-testid="status-dirty">
          {dirtyCount} modified buffer{dirtyCount === 1 ? "" : "s"}
        </span>
      )}
      {(diagnostics.errors > 0 || diagnostics.warnings > 0) && (
        <button
          type="button"
          className={`status-diagnostics ${diagnostics.errors > 0 ? "error" : "warning"}`}
          data-testid="status-diagnostics"
          title="Open Problems"
          onClick={diagnostics.onOpen}
        >
          {[
            diagnostics.errors > 0 && `${diagnostics.errors} error${diagnostics.errors === 1 ? "" : "s"}`,
            diagnostics.warnings > 0 && `${diagnostics.warnings} warning${diagnostics.warnings === 1 ? "" : "s"}`,
          ].filter(Boolean).join(" · ")}
        </button>
      )}
      <span className="push" data-testid="status-keymap">
        Keymap: {profileTitle(keymapProfile)}
      </span>
      <span data-testid="status-encoding" title="UTF-8 is the only encoding OpenRidr reads and writes; nothing is detected.">
        UTF-8
      </span>
      {/* Indentation comes from Settings; no per-file detection is claimed. */}
      <span data-testid="status-indent" title={indent.title}>
        {indent.label}
      </span>
      <span data-testid="status-cursor">{cursor}</span>
    </footer>
  );
}
