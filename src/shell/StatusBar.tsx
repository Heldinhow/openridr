type EvaluationStatus = { text: string; title?: string; testId?: string };

export function StatusBar({ workspace, trusted, sdkSummary, evaluation, dirtyCount, breadcrumb, cursor }: {
  workspace: { name: string; path: string };
  trusted: boolean;
  sdkSummary: string;
  evaluation: EvaluationStatus;
  dirtyCount: number;
  breadcrumb: string;
  cursor: string;
}) {
  return (
    <footer className="status">
      <span data-testid="status-workspace" title={workspace.path}>{workspace.name}</span>
      <span data-testid="status-breadcrumb" title={breadcrumb}>{breadcrumb}</span>
      <span className={trusted ? "success" : "warning"} data-testid="status-mode">
        {trusted ? "Trusted mode" : "Restricted mode"}
      </span>
      <span data-testid="status-sdk" title={sdkSummary}>{sdkSummary}</span>
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
      <span className="push" data-testid="status-keymap">
        Keymap: IntelliJ / Rider
      </span>
      <span data-testid="status-encoding" title="UTF-8 is the only encoding OpenRidr reads and writes; nothing is detected.">
        UTF-8
      </span>
      {/* Settings arrive in F8; until then the status bar reports the default instead of guessing from the file. */}
      <span data-testid="status-indent" title="Indentation comes from editor settings (default until Settings arrives in F8).">
        4 spaces
      </span>
      <span data-testid="status-cursor">{cursor}</span>
    </footer>
  );
}
