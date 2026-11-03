import { useEffect, useRef, useState } from "react";
import { call, HostCallError, messageOf, type Candidate, type Snapshot } from "./host/client";
import { Icon } from "./Icon";
import { PathForm } from "./welcome/PathForm";
import { ProjectsView } from "./welcome/ProjectsView";
import { ToolchainView } from "./welcome/ToolchainView";
import { WelcomeNav, type WelcomeView } from "./welcome/WelcomeNav";
import type { RecentProject } from "./welcome/recent";

type Props = {
  snapshot: Snapshot | null;
  initialPath: string;
  recent: RecentProject[];
  onRecent: (projects: RecentProject[]) => void;
  onOpened: (snapshot: Snapshot) => void;
  onSnapshot: (snapshot: Snapshot) => void;
  onOpenGlobalJson: () => void;
  // Path from Finder, the Dock or the command line, opened through the same open() as the form.
  externalOpen: { path: string } | null;
};

export function SetupScreen({
  snapshot,
  initialPath,
  recent,
  onRecent,
  onOpened,
  onSnapshot,
  onOpenGlobalJson,
  externalOpen,
}: Props) {
  const [view, setView] = useState<WelcomeView>("projects");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [rowErrors, setRowErrors] = useState<Record<string, string>>({});
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const workspace = snapshot?.workspace ?? null;

  // `row` is the recent project the open was started from: its failure is shown on that row.
  async function open(target: string, row?: string) {
    if (!target.trim()) {
      setError("Enter an absolute path to a .sln, .slnx, .csproj or a directory.");
      return;
    }
    setBusy(true);
    setError(null);
    setRowErrors({});
    setCandidates([]);
    try {
      const opened = await call<Snapshot>("workspace.open", { path: target.trim() });
      onOpened(opened);
    } catch (caught) {
      if (row && !(caught instanceof HostCallError && caught.code === "ambiguous_workspace")) {
        setRowErrors({ [row]: messageOf(caught) });
      } else {
        setError(messageOf(caught));
      }
      if (caught instanceof HostCallError && caught.code === "ambiguous_workspace") {
        const details = caught.details as { candidates?: Candidate[] } | undefined;
        setCandidates(details?.candidates ?? []);
      }
    } finally {
      setBusy(false);
    }
  }

  async function pick() {
    try {
      const { path: picked } = await call<{ path: string | null }>("workspace.pick");
      if (picked) await open(picked);
    } catch (caught) {
      setError(messageOf(caught));
    }
  }

  // The first value belongs to the mount that already handled it; only a new request opens.
  const handledExternalOpen = useRef(externalOpen);
  useEffect(() => {
    if (!externalOpen || externalOpen === handledExternalOpen.current) return;
    handledExternalOpen.current = externalOpen;
    // Errors, candidates and the trust prompt show where the form's do, so the projects view must be visible.
    setView("projects");
    void open(externalOpen.path);
  }, [externalOpen]);

  return (
    <main className="setup-screen">
      <header className="chrome">
        <span className="brand"><Icon name="code" /> Welcome to OpenRidr</span>
        <span className="muted">{workspace?.name ?? ""}</span>
      </header>
      <div className="welcome-body">
        <WelcomeNav view={view} onView={setView} />
        <div className="setup" hidden={view !== "projects"} data-testid="welcome-projects">
          <ProjectsView
            active={view === "projects"}
            recent={recent}
            busy={busy}
            rowErrors={rowErrors}
            hostSdkMissing={snapshot?.sdk.host.state === "missing"}
            awaitingProcesses={(snapshot?.processes ?? []).filter((process) => process.awaitingDecision).length}
            onOpen={(path) => void open(path, path)}
            onPick={() => void pick()}
            onRecent={onRecent}
            onShowToolchain={() => setView("toolchain")}
          >
            {!window.__TAURI_INTERNALS__ && (
              <PathForm initialPath={initialPath} busy={busy} invalid={Boolean(error)} onOpen={(path) => void open(path)} />
            )}
            {error && (
              <div className="notice error" role="alert" data-testid="workspace-error">
                {error}
              </div>
            )}
            {candidates.length > 1 && (
              <div className="candidate-box" aria-labelledby="candidate-title" data-testid="workspace-candidates">
                <h2 id="candidate-title">Choose a workspace</h2>
                <p className="muted">
                  OpenRidr never picks one for you. Opening a workspace replaces the current one.
                </p>
                <div className="candidate-list">
                  {candidates.map((candidate, index) => (
                    <button
                      key={candidate.path}
                      data-testid={`workspace-candidate-${index}`}
                      onClick={() => void open(candidate.path)}
                      disabled={busy}
                    >
                      <span className="candidate-name">{candidate.name}</span>
                      <span className="candidate-path">{candidate.path}</span>
                    </button>
                  ))}
                </div>
              </div>
            )}
          </ProjectsView>
        </div>
        <div className="setup" hidden={view !== "toolchain"} data-testid="welcome-toolchain">
          <ToolchainView snapshot={snapshot} onSnapshot={onSnapshot} onOpenGlobalJson={onOpenGlobalJson} />
        </div>
      </div>
    </main>
  );
}
