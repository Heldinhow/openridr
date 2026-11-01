import { lastOpened, projectName, shortPath, type RecentProject } from "./recent";
import { RowMenu } from "./RowMenu";

type Props = {
  project: RecentProject;
  index: number;
  busy: boolean;
  error: string | undefined;
  onOpen: (path: string) => void;
  onTrust: (path: string, decision: "trusted" | "restricted") => void;
  onRemove: (path: string) => void;
};

const BADGE = {
  trusted: { text: "Trusted", tone: "success" },
  restricted: { text: "Restricted", tone: "warning" },
  none: { text: "Not decided", tone: "muted" },
} as const;

export function RecentRow({ project, index, busy, error, onOpen, onTrust, onRemove }: Props) {
  const name = projectName(project.path);
  const badge = BADGE[project.decision ?? "none"];
  const base = `welcome-recent-${index}`;
  return (
    <li className="recent-row">
      <div className="recent-line">
        <button
          type="button"
          className="recent-open"
          data-row-open=""
          data-testid={base}
          title={project.path}
          disabled={busy}
          aria-describedby={error ? `${base}-error` : undefined}
          onClick={() => onOpen(project.path)}
        >
          <span className="project-badge" aria-hidden="true">{name.slice(0, 2).toUpperCase()}</span>
          <span className="recent-text">
            <strong>{name}</strong>
            <span className="muted">{shortPath(project.path)}</span>
          </span>
          {project.openedAt !== undefined && (
            <time className="recent-time muted" dateTime={new Date(project.openedAt).toISOString()}>
              {lastOpened(project.openedAt)}
            </time>
          )}
          <span className={`trust-badge ${badge.tone}`} data-testid={`${base}-badge`}>{badge.text}</span>
        </button>
        <RowMenu
          testId={`${base}-menu`}
          name={name}
          decision={project.decision}
          disabled={busy}
          onTrust={(decision) => onTrust(project.path, decision)}
          onRemove={() => onRemove(project.path)}
        />
      </div>
      {error && <p className="row-error error" role="alert" id={`${base}-error`} data-testid={`${base}-error`}>{error}</p>}
    </li>
  );
}
