type Props = {
  /** Prefix for the testids, e.g. "debug-install" → debug-install-progress/retry. */
  testId: string;
  /** Tool name used in the progress sentence. */
  tool: string;
  state: string;
  progress: { downloaded: number; total: number | null } | null;
  error: string | null;
  note: string | null;
  busy: boolean;
  retryDisabledReason: string | null;
  onRetry: () => void;
};

const active = ["downloading", "installing", "verifying"];

/** Observed download progress and retry shared by tool installers; never invents a percentage. */
export function ToolDownloadStatus({
  testId,
  tool,
  state,
  progress,
  error,
  note,
  busy,
  retryDisabledReason,
  onRetry,
}: Props) {
  const installing = active.includes(state);
  if (!installing && state !== "failed") return null;
  return (
    <div data-testid={`${testId}-status`}>
      {installing && (
        <p className="muted" data-testid={`${testId}-progress`}>
          {progress
            ? `${tool}: downloaded ${progress.downloaded} of ${
                progress.total === null ? "an unknown total" : `${progress.total}`
              } bytes (measured; no percentage)`
            : `${tool}: waiting for the first observed bytes…`}
        </p>
      )}
      {error ? (
        <p className="error" role="alert" data-testid={`${testId}-error`}>
          {error}
        </p>
      ) : note ? (
        <p className="muted" data-testid={`${testId}-note`}>
          {note}
        </p>
      ) : null}
      {state === "failed" && (
        <button
          data-testid={`${testId}-retry`}
          disabled={busy || retryDisabledReason !== null}
          onClick={onRetry}
          title={
            retryDisabledReason ??
            "Runs the confirmed plan again; a changed plan requires a new confirmation."
          }
        >
          Try again
        </button>
      )}
    </div>
  );
}
