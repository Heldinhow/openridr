import { useState } from "react";
import { call, HostCallError, type Candidate, type Snapshot } from "./host/client";

type Props = {
  snapshot: Snapshot | null;
  initialPath: string;
  onOpened: (snapshot: Snapshot) => void;
  onEnter: () => void;
};

export function SetupScreen({ snapshot, initialPath, onOpened, onEnter }: Props) {
  const [path, setPath] = useState(initialPath);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const workspace = snapshot?.workspace ?? null;
  const outline = workspace ? snapshot?.outline : null;
  const tfms = [...new Set(outline?.projects.flatMap((project) => project.tfms) ?? [])];
  const globalJson = outline?.globalJson ?? null;

  async function open(target: string) {
    if (!target.trim()) {
      setError("Enter an absolute path to a .sln, .csproj or a directory.");
      return;
    }
    setBusy(true);
    setError(null);
    setCandidates([]);
    try {
      const opened = await call<Snapshot>("workspace.open", { path: target.trim() });
      onOpened(opened);
    } catch (caught) {
      if (caught instanceof HostCallError) {
        setError(caught.message);
        if (caught.code === "ambiguous_workspace") {
          const details = caught.details as { candidates?: Candidate[] } | undefined;
          setCandidates(details?.candidates ?? []);
        }
      } else {
        setError(String(caught));
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="setup-screen">
      <header className="chrome">
        <span className="brand">OpenRidr / Toolchain</span>
        <span className="muted">{workspace?.name ?? "No workspace open"}</span>
      </header>
      <div className="setup">
        <h1>OpenRidr</h1>
        <p className="muted">
          Open one .NET solution or project. OpenRidr starts in Restricted mode and only reads
          text in this build.
        </p>

        <form
          className="open-form"
          onSubmit={(event) => {
            event.preventDefault();
            void open(path);
          }}
        >
          <label htmlFor="workspace-path-input">Workspace path (absolute)</label>
          <div className="open-row">
            <input
              id="workspace-path-input"
              data-testid="workspace-path-input"
              value={path}
              onChange={(event) => setPath(event.target.value)}
              placeholder="/Users/you/src/OrderDesk/OrderDesk.sln"
              autoFocus
              spellCheck={false}
              disabled={busy}
            />
            <button type="submit" className="primary" data-testid="workspace-open" disabled={busy}>
              {busy ? "Opening…" : "Open workspace"}
            </button>
          </div>
          <p className="hint">Path may point to a .sln, a .csproj or a directory containing one.</p>
        </form>

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

        {workspace && (
          <>
            <section className="trust-card" aria-labelledby="trust-title">
              <h2 id="trust-title">Review workspace trust</h2>
              <p data-testid="workspace-path">
                Workspace path: <code>{workspace.path}</code>
              </p>
              <p className="muted">
                {workspace.kind === "solution" ? "Solution" : "Project"} · {workspace.name}
              </p>
              <div className="trust">
                <strong className="warning" data-testid="mode-restricted">
                  Restricted mode
                </strong>
                <p>Files can be read as text. Project code will not run.</p>
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    data-testid="trust-checkbox"
                    checked={false}
                    disabled
                    readOnly
                  />
                  I trust this repository to execute code
                </label>
                <p className="hint">
                  Trust enforcement is added by ticket 02. The checkbox and Trust button stay
                  disabled in this build.
                </p>
                <div className="actions">
                  <button
                    type="button"
                    className="primary"
                    data-testid="open-text-only"
                    onClick={onEnter}
                  >
                    Open text only
                  </button>
                  <button
                    type="button"
                    data-testid="trust-grant"
                    disabled
                    title="Trust enforcement is added by ticket 02."
                  >
                    Trust and inspect project
                  </button>
                </div>
                <p className="hint">
                  Trust and inspect project is disabled: trust enforcement is added by ticket 02.
                </p>
              </div>
            </section>

            <div className="tool-rows">
              <div className="tool-row">
                <strong>Project target</strong>
                <div>
                  {tfms.length > 0 ? tfms.join(", ") : "Not declared"} declared{" "}
                  <span className="tag">Text only · not evaluated</span>
                </div>
              </div>
              <div className="tool-row">
                <strong>global.json</strong>
                <div>
                  {globalJson ? (
                    <>
                      version {globalJson.version ?? "not declared"} · rollForward{" "}
                      {globalJson.rollForward ?? "not declared"}
                      <br />
                      <span className="muted">Read as text from {globalJson.path}.</span>
                    </>
                  ) : (
                    <span className="muted">No global.json at the workspace root.</span>
                  )}
                </div>
              </div>
              <div className="tool-row">
                <strong>Analysis host</strong>
                <div className="muted">
                  Not inspected in this ticket. SDK inventory and the host/project split arrive
                  with ticket 02.
                </div>
              </div>
              <div className="tool-row">
                <strong>Executable actions</strong>
                <div>
                  Blocked in Restricted mode: MSBuild evaluation, restore, language server, tool
                  installation, terminal, build, run, debug and tests cannot start.
                </div>
              </div>
            </div>

            <section className="outline-preview" aria-labelledby="outline-title">
              <h2 id="outline-title">
                Text outline <span className="tag">not evaluated</span>
              </h2>
              <p className="muted">
                Parsed from .sln, .csproj and global.json as text. No MSBuild, restore, LSP or
                process has run.
              </p>
              <ul>
                {outline?.projects.map((project) => (
                  <li key={project.path}>
                    <strong>{project.name}</strong> <span className="tag">{project.kind}</span>{" "}
                    <span className="tag">{project.tfms.join(", ") || "TFM not declared"}</span>{" "}
                    <span className="muted">
                      {project.files.length} C# file{project.files.length === 1 ? "" : "s"}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          </>
        )}
      </div>
    </main>
  );
}
