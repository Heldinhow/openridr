import type { RecentProject } from "./recent";
import { RecentRow } from "./RecentRow";

type Props = {
  projects: RecentProject[];
  busy: boolean;
  rowErrors: Record<string, string>;
  onOpen: (path: string) => void;
  onTrust: (path: string, decision: "trusted" | "restricted") => void;
  onRemove: (path: string) => void;
};

// A plain list of buttons: ↑/↓ move focus between the rows' open buttons, Enter/click opens.
export function RecentList({ projects, busy, rowErrors, onOpen, onTrust, onRemove }: Props) {
  function onKeyDown(event: React.KeyboardEvent<HTMLUListElement>) {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const rows = [...event.currentTarget.querySelectorAll<HTMLElement>("[data-row-open]:not(:disabled)")];
    const index = rows.indexOf((event.target as HTMLElement).closest<HTMLElement>("[data-row-open]")!);
    if (index < 0) return;
    event.preventDefault();
    rows[Math.max(0, Math.min(rows.length - 1, index + (event.key === "ArrowDown" ? 1 : -1)))].focus();
  }

  return (
    <ul className="recent-list" data-testid="welcome-recent" aria-label="Recent projects" onKeyDown={onKeyDown}>
      {projects.map((project, index) => (
        <RecentRow
          key={project.path}
          project={project}
          index={index}
          busy={busy}
          error={rowErrors[project.path]}
          onOpen={onOpen}
          onTrust={onTrust}
          onRemove={onRemove}
        />
      ))}
    </ul>
  );
}
