type EvaluationStatus = { text: string; title?: string; testId?: string };

export function StatusBar({ workspace, trusted, sdkSummary, evaluation, dirtyCount, breadcrumb, cursor, indent }: {
  workspace: { name: string; path: string };
  trusted: boolean;
  sdkSummary: string;
  evaluation: EvaluationStatus;
  dirtyCount: number;
  breadcrumb: string;
  cursor: string;
  indent: { label: string; title: string };
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
      {/* Indentation comes from Settings; no per-file detection is claimed. */}
      <span data-testid="status-indent" title={indent.title}>
        {indent.label}
      </span>
      <span data-testid="status-cursor">{cursor}</span>
    </footer>
  );
}
