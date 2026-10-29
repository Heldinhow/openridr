import { useState } from "react";
import { call, HostCallError, type Candidate, type ProjectSdk, type Snapshot } from "./host/client";
import { TrustControls } from "./TrustControls";
import { ProcessPanel } from "./ProcessPanel";
import { DebugAdapterPanel } from "./ToolchainPanels";
import { Icon } from "./Icon";

type Props = {
  snapshot: Snapshot | null;
  initialPath: string;
  recent: string[];
  onOpened: (snapshot: Snapshot) => void;
  onSnapshot: (snapshot: Snapshot) => void;
  onEnter: () => void;
  onOpenGlobalJson: () => void;
};

function messageOf(error: unknown) {
  return error instanceof HostCallError ? error.message : String(error);
}

function ProjectSdkBody({
  project,
  onOpenGlobalJson,
}: {
  project: ProjectSdk;
  onOpenGlobalJson: () => void;
}) {
  const installLink = (
    <a href={project.instructionsUrl} target="_blank" rel="noreferrer noopener">
      Official .NET downloads
    </a>
  );
  const pin = project.requestedVersion
    ? `global.json requests ${project.requestedVersion} · rollForward ${
        project.rollForward ?? "patch"
      }`
    : null;

  switch (project.state) {
    case "resolved":
      return (
        <>
          <strong className="success">SDK {project.resolvedVersion}</strong>{" "}
          <span className="tag">resolved after trust · not a build</span>
          <p>
            {pin}
            {project.allowPrerelease ? " · prerelease allowed" : ""}.
          </p>
          <p className="muted">
            Resolved root: <code>{project.root}</code>
            <br />
            SDK path: <code>{project.sdkPath}</code>
          </p>
        </>
      );
    case "sdk_missing":
      return (
        <>
          <strong className="error">Project SDK missing</strong>
          <p data-testid="sdk-project-message">{project.message}</p>
          <p>{pin}. Install that SDK or change global.json yourself; this build does neither.</p>
          <p>{installLink}</p>
          <button type="button" data-testid="sdk-open-global-json" onClick={onOpenGlobalJson}>
            Open global.json
          </button>
        </>
      );
    case "invalid_global_json":
      return (
        <>
          <strong className="error">global.json cannot be used</strong>
          <p data-testid="sdk-project-message">{project.message}</p>
          <p>
            OpenRidr treats global.json as text and never rewrites it. {installLink}
          </p>
          <button type="button" data-testid="sdk-open-global-json" onClick={onOpenGlobalJson}>
            Open global.json
          </button>
        </>
      );
    case "not_pinned":
      return (
        <>
          <strong>Not pinned</strong>
          <p data-testid="sdk-project-message">{project.message}</p>
          <button type="button" data-testid="sdk-open-global-json" onClick={onOpenGlobalJson}>
            Open global.json
          </button>
        </>
      );
    case "not_inspected":
      return (
        <>
          <strong>Not inspected yet</strong>
          <p data-testid="sdk-project-message">
            {project.message} The project SDK is not resolved in Restricted mode, and the
            analysis host SDK is never used as its substitute.
          </p>
          {pin && <p className="muted">{pin} (read as text).</p>}
        </>
      );
    case "no_global_json":
      return (
        <>
          <strong>No global.json</strong>
          <p data-testid="sdk-project-message">{project.message}</p>
        </>
      );
    case "no_workspace":
      return <span className="muted">{project.message}</span>;
  }
}

export function SetupScreen({
  snapshot,
  initialPath,
  recent,
  onOpened,
  onSnapshot,
  onEnter,
  onOpenGlobalJson,
}: Props) {
  const [path, setPath] = useState(initialPath);
  const [busy, setBusy] = useState(false);
  const [redetecting, setRedetecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [toolchainError, setToolchainError] = useState<string | null>(null);
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const workspace = snapshot?.workspace ?? null;
  const outline = workspace ? snapshot?.outline : null;
  const tfms = [...new Set(outline?.projects.flatMap((project) => project.tfms) ?? [])];
  const sdk = snapshot?.sdk;
  const trusted = snapshot?.trust.granted ?? false;

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

  async function redetect() {
    setRedetecting(true);
    setToolchainError(null);
    try {
      onSnapshot(await call<Snapshot>("sdk.redetect"));
    } catch (caught) {
      setToolchainError(messageOf(caught));
    } finally {
      setRedetecting(false);
    }
  }

  return (
    <main className="setup-screen">
      <header className="chrome">
        <span className="brand"><Icon name="code" /> Welcome to OpenRidr</span>
        <span className="muted">{workspace?.name ?? ""}</span>
      </header>
      <div className="welcome-body">
        <nav className="welcome-nav" aria-label="Welcome">
          <div className="welcome-logo">
            <span className="welcome-mark"><Icon name="code" /></span>
            <div><strong>OpenRidr</strong><span className="muted">C# workspace</span></div>
          </div>
          <a className="welcome-link active" href="#projects">Projects</a>
          <a className="welcome-link" href="#toolchain-title">Toolchain</a>
        </nav>
        <div className="setup" id="projects">
        <form
          className="open-form"
          onSubmit={(event) => {
            event.preventDefault();
            void open(path);
          }}
        >
          <label htmlFor="workspace-path-input" className="sr-only">Workspace path (absolute)</label>
          <div className="open-row">
            <input
              id="workspace-path-input"
              data-testid="workspace-path-input"
              value={path}
              onChange={(event) => setPath(event.target.value)}
              placeholder="Path to a .sln, .csproj or folder"
              aria-describedby="workspace-path-help"
              aria-invalid={error ? true : undefined}
              autoFocus
              spellCheck={false}
              disabled={busy}
            />
            <button type="submit" className="welcome-button" data-testid="workspace-open" disabled={busy}>
              {busy ? "Opening…" : "Open"}
            </button>
          </div>
          <p className="hint" id="workspace-path-help">Absolute path. Opens in Restricted mode — no project code runs until you trust it.</p>
        </form>

        {recent.length > 0 && (
          <ul className="recent-list" data-testid="welcome-recent">
            {recent.map((solution, index) => {
              const name = (solution.split(/[\\/]/).pop() ?? solution).replace(/\.(sln|csproj)$/i, "");
              return (
                <li key={solution}>
                  <button type="button" data-testid={`welcome-recent-${index}`} title={solution} onClick={() => void open(solution)} disabled={busy}>
                    <span className="project-badge" aria-hidden="true">{name.slice(0, 2).toUpperCase()}</span>
                    <span className="recent-text"><strong>{name}</strong><span className="muted">{solution}</span></span>
                  </button>
                </li>
              );
            })}
          </ul>
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

        {workspace && snapshot && (
          <section className="trust-card" aria-labelledby="trust-title">
            <h2 id="trust-title">Review workspace trust</h2>
            <p data-testid="workspace-path">
              Workspace path: <code>{workspace.path}</code>
            </p>
            <p className="muted">
              {workspace.kind === "solution" ? "Solution" : "Project"} · {workspace.name}
            </p>
            <TrustControls snapshot={snapshot} onChange={onSnapshot} />
            <div className="actions">
              <button
                type="button"
                className="primary"
                data-testid="open-text-only"
                onClick={onEnter}
              >
                {trusted ? "Open workspace" : "Open text only"}
              </button>
            </div>
          </section>
        )}

        <section className="toolchain" aria-labelledby="toolchain-title" data-testid="toolchain">
          <h2 id="toolchain-title">Toolchain</h2>
          <p className="muted">Your local tools, checked without changing project files. Expand a tool for details.</p>
          {!sdk ? (
            <p className="muted">Detecting SDKs…</p>
          ) : (
            <div className="tool-rows">
              <details className="tool-row" open={workspace ? true : undefined}>
                <summary><span>Analysis host</span><span className={sdk.host.state === "found" ? "success" : "warning"}>{sdk.host.state === "found" ? `✓ SDK ${sdk.host.version}` : "! SDK 10 needed"}</span></summary>
                <div data-testid="sdk-host">
                  {sdk.host.state === "found" ? (
                    <>
                      <strong className="success">SDK {sdk.host.version}</strong>{" "}
                      <span className="tag">host analysis · separate from project</span>
                      <br />
                      <code>{sdk.host.sdkPath}</code>
                      <br />
                      <span className="muted">Root: {sdk.host.root}</span>
                    </>
                  ) : (
                    <>
                      <strong className="error">Analysis host missing</strong>
                      <p>{sdk.host.message}</p>
                      <a
                        href={sdk.host.instructionsUrl}
                        target="_blank"
                        rel="noreferrer noopener"
                        data-testid="sdk-host-instructions"
                      >
                        Official .NET downloads
                      </a>
                      <p className="muted">Install .NET 10, then use Redetect SDKs.</p>
                    </>
                  )}
                </div>
              </details>

              <details className="tool-row" open={workspace ? true : undefined}>
                <summary><span>Project SDK</span><span className={sdk.project.state === "resolved" ? "success" : "muted"}>{sdk.project.state === "resolved" ? `✓ ${sdk.project.resolvedVersion}` : workspace ? "Review required" : "Open a workspace first"}</span></summary>
                <div data-testid="sdk-project">
                  <ProjectSdkBody project={sdk.project} onOpenGlobalJson={onOpenGlobalJson} />
                </div>
              </details>

              <details className="tool-row">
                <summary><span>Project targets</span><span className="muted">{tfms.join(", ") || "Not inspected"}</span></summary>
                <div data-testid="sdk-tfms">
                  {tfms.length > 0
                    ? `${tfms.join(", ")} declared · not evaluated`
                    : "No TFM declared in project text · not evaluated"}
                </div>
              </details>

              {snapshot && (
                <details className="tool-row" data-testid="debug-adapter-card">
                  <summary><span>Debug adapter</span><span className={snapshot.debug.adapterState === "ready" ? "success" : "warning"}>{snapshot.debug.adapterState === "ready" ? "✓ Available" : "! Setup required"}</span></summary>
                  <DebugAdapterPanel snapshot={snapshot} onChange={onSnapshot} />
                </details>
              )}

              <details className="tool-row">
                <summary><span>SDK detection</span><span className="muted">{sdk.roots.length} roots checked</span></summary>
                <div>
                  <button
                    type="button"
                    data-testid="sdk-redetect"
                    onClick={() => void redetect()}
                    disabled={redetecting}
                  >
                    {redetecting ? "Detecting…" : "Redetect SDKs"}
                  </button>
                  <p className="hint">
                    Re-runs only the SDK inventory. It never evaluates, restores, installs an SDK
                    or touches project files.
                  </p>
                  <details>
                    <summary>Detected SDK roots ({sdk.roots.length})</summary>
                    <ul>
                      {sdk.roots.map((root) => (
                        <li key={`${root.source}:${root.root}`}>
                          <code>{root.root}</code>{" "}
                          <span className="tag">{root.source}</span>{" "}
                          <span className="muted">
                            {root.error ??
                              (root.versions.length > 0
                                ? root.versions.map((version) => version.version).join(", ")
                                : "no SDKs listed")}
                          </span>
                        </li>
                      ))}
                    </ul>
                  </details>
                </div>
              </details>
            </div>
          )}
          {toolchainError && (
            <p className="error" role="alert" data-testid="sdk-error">
              {toolchainError}
            </p>
          )}
        </section>

        {snapshot && <ProcessPanel snapshot={snapshot} onChange={onSnapshot} />}

        {workspace && outline && (
          <section className="outline-preview" aria-labelledby="outline-title">
            <h2 id="outline-title">
              Text outline <span className="tag">not evaluated</span>
            </h2>
            <p className="muted">
              Parsed from .sln, .csproj and global.json as text. No MSBuild, restore, LSP or
              project process has run.
            </p>
            <ul>
              {outline.projects.map((project) => (
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
        )}
        </div>
      </div>
    </main>
  );
}
