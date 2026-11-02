import type { Snapshot } from "../host/client";
import { StatusMode } from "./StatusMode";

type EvaluationStatus = { text: string; title?: string; testId?: string };

type RestoreStatus = {
  state: "idle" | "running" | "ok" | "failed" | "cancelled";
  label: string;
  onCancel: () => void;
  onRetry: (() => void) | null;
};

export function StatusBar({ workspace, snapshot, onSnapshot, sdkSummary, evaluation, restore, dirtyCount, diagnostics, breadcrumb, cursor, indent }: {
  workspace: { name: string; path: string };
  snapshot: Snapshot;
  onSnapshot: (snapshot: Snapshot) => void;
  sdkSummary: string;
  evaluation: EvaluationStatus;
  restore: RestoreStatus;
  dirtyCount: number;
  diagnostics: { errors: number; warnings: number; onOpen: () => void };
  breadcrumb: string;
  cursor: string;
  indent: { label: string; title: string };
}) {
  return (
    <footer className="status">
      <span data-testid="status-workspace" title={workspace.path}>{workspace.name}</span>
      <span data-testid="status-breadcrumb" title={breadcrumb}>{breadcrumb}</span>
      <StatusMode snapshot={snapshot} onSnapshot={onSnapshot} />
      <span data-testid="status-sdk" title={sdkSummary}>{sdkSummary}</span>
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
      {evaluation.testId ? (
        <span data-testid={evaluation.testId} title={evaluation.title}>{evaluation.text}</span>
      ) : (
        <span>{evaluation.text}</span>
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
        Keymap: IntelliJ / Rider
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
