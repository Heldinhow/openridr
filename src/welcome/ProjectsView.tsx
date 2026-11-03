import { useEffect, useRef, useState, type ReactNode } from "react";
import { call, messageOf } from "../host/client";
import { Icon } from "../Icon";
import { fetchRecent, type RecentProject } from "./recent";
import { RecentList } from "./RecentList";

type Props = {
  // Shortcuts (`/`, ⌘O) only work while the Projects view is the visible one.
  active: boolean;
  recent: RecentProject[];
  busy: boolean;
  // Errors of opening a project, keyed by path; shown on that row.
  rowErrors: Record<string, string>;
  hostSdkMissing: boolean;
  // Tracked processes of a closed workspace that still wait for a keep/terminate decision.
  awaitingProcesses: number;
  onOpen: (path: string) => void;
  onPick: () => void;
  onRecent: (projects: RecentProject[]) => void;
  onShowToolchain: () => void;
  // Browser/E2E path form, workspace error and candidate chooser, rendered under the search row.
  children?: ReactNode;
};

export function ProjectsView({ active, recent, busy, rowErrors, hostSdkMissing, awaitingProcesses, onOpen, onPick, onRecent, onShowToolchain, children }: Props) {
  const [filter, setFilter] = useState("");
  const [actionErrors, setActionErrors] = useState<Record<string, string>>({});
  const search = useRef<HTMLInputElement>(null);
  const needle = filter.trim().toLowerCase();
  const shown = recent.filter((project) => project.path.toLowerCase().includes(needle));

  useEffect(() => {
    if (!active) return;
    function onKeyDown(event: KeyboardEvent) {
      const target = event.target as HTMLElement;
      if (target.closest("[aria-modal]")) return;
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "o") {
        event.preventDefault();
        onPick();
      } else if (event.key === "/" && !event.metaKey && !event.ctrlKey && !/^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName)) {
        event.preventDefault();
        search.current?.focus();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [active, onPick]);

  function failRow(path: string, caught: unknown) {
    setActionErrors((current) => ({ ...current, [path]: messageOf(caught) }));
  }

  async function setTrust(path: string, decision: "trusted" | "restricted") {
    setActionErrors(({ [path]: _gone, ...rest }) => rest);
    try {
      // Changes the stored decision only; the project is not opened.
      await call("trust.set", { path, decision });
      onRecent(await fetchRecent());
    } catch (caught) {
      failRow(path, caught);
    }
  }

  async function remove(path: string) {
    try {
      onRecent((await call<{ projects: RecentProject[] }>("recent.remove", { path })).projects);
    } catch (caught) {
      failRow(path, caught);
    }
  }

  function onSearchKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Escape") setFilter("");
    else if (event.key === "ArrowDown") {
      event.preventDefault();
      document.querySelector<HTMLElement>("[data-row-open]:not(:disabled)")?.focus();
    }
  }

  const empty = recent.length === 0;
  const count = needle ? `${shown.length} of ${recent.length}` : String(recent.length);

  return (
    <>
      <div className="projects-bar">
        <div className="search-box">
          <Icon name="search" />
          <input
            ref={search}
            data-testid="welcome-search"
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            onKeyDown={onSearchKeyDown}
            placeholder="Search projects"
            aria-label="Search projects"
            spellCheck={false}
          />
        </div>
        <button type="button" className={`welcome-button${empty ? " primary" : ""}`} data-testid="workspace-pick" disabled={busy} onClick={onPick}>
          {busy ? "Opening…" : "Open"}
        </button>
      </div>
      {children}
      {hostSdkMissing && (
        <p className="sdk-warning warning" role="status" data-testid="welcome-sdk-warning">
          Analysis host SDK missing: C# analysis needs .NET 10.{" "}
          <button type="button" className="link-button" onClick={onShowToolchain}>Open Toolchain</button>
        </p>
      )}
      {awaitingProcesses > 0 && (
        <p className="sdk-warning warning" role="status" data-testid="welcome-process-warning">
          {awaitingProcesses === 1 ? "1 process" : `${awaitingProcesses} processes`} from the closed workspace still need a decision.{" "}
          <button type="button" className="link-button" onClick={onShowToolchain}>Open Toolchain</button>
        </p>
      )}
      {!empty && (
        <div className="projects-head">
          <h2>Recent projects</h2>
          <span className="muted" role="status" data-testid="welcome-count">
            {count} {recent.length === 1 ? "project" : "projects"}
          </span>
        </div>
      )}
      {empty && (
        <div className="projects-empty" data-testid="welcome-empty">
          <strong>No recent projects yet</strong>
          <p className="muted">Open a .sln, .slnx, .csproj or folder to start. OpenRidr asks once per project whether to trust it; until then no project code runs.</p>
        </div>
      )}
      {!empty && shown.length === 0 && (
        <p className="muted projects-none" data-testid="welcome-no-match">No projects match “{filter.trim()}”.</p>
      )}
      {shown.length > 0 && (
        <RecentList
          projects={shown}
          busy={busy}
          rowErrors={{ ...actionErrors, ...rowErrors }}
          onOpen={onOpen}
          onTrust={(path, decision) => void setTrust(path, decision)}
          onRemove={(path) => void remove(path)}
        />
      )}
      <p className="welcome-keys muted">
        <span><kbd>↑</kbd> <kbd>↓</kbd> select</span>
        <span><kbd>Enter</kbd> open</span>
        <span><kbd>/</kbd> search</span>
        <span><kbd>⌘O</kbd> open folder</span>
      </p>
    </>
  );
}
