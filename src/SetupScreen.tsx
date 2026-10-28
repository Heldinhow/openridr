import { useState } from "react";
import { call, HostCallError, type Candidate, type ProjectSdk, type Snapshot } from "./host/client";
import { TrustControls } from "./TrustControls";
import { ProcessPanel } from "./ProcessPanel";

type Props = {
  snapshot: Snapshot | null;
  initialPath: string;
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
  const appleSilicon = snapshot?.app.platform === "macos" && snapshot?.app.arch === "aarch64";

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
        <span className="brand">OpenRidr / Toolchain</span>
        <span className="muted">{workspace?.name ?? "No workspace open"}</span>
      </header>
      <div className="setup">
        <h1>OpenRidr</h1>
        <p className="muted">
          Open one .NET solution or project. OpenRidr starts in Restricted mode; only text is
          read until you explicitly trust the workspace.
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
          <p className="muted">
            Detected in a neutral temporary directory. This is an inventory, not a loaded
            language service, and no project file is changed.
          </p>
          {!sdk ? (
            <p className="muted">Detecting SDKs…</p>
          ) : (
            <div className="tool-rows">
              <div className="tool-row">
                <strong>Analysis host</strong>
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
              </div>

              <div className="tool-row">
                <strong>Project SDK</strong>
                <div data-testid="sdk-project">
                  <ProjectSdkBody project={sdk.project} onOpenGlobalJson={onOpenGlobalJson} />
                </div>
              </div>

              <div className="tool-row">
                <strong>Project targets</strong>
                <div data-testid="sdk-tfms">
                  {tfms.length > 0
                    ? `${tfms.join(", ")} declared · not evaluated`
                    : "No TFM declared in project text · not evaluated"}
                </div>
              </div>

              <div className="tool-row">
                <strong>Debug adapter</strong>
                <div data-testid="sdk-debug-adapter">
                  Not checked in this slice.
                  <br />
                  <span className="muted">{snapshot?.debug.adapterMessage}</span>
                </div>
              </div>

              {appleSilicon && (
                <div className="tool-row">
                  <strong>Apple Silicon debugger</strong>
                  <div data-testid="sdk-apple-silicon">
                    <strong>experimental · disabled</strong> (OFF by default)
                    <br />
                    <span className="muted">
                      This slice does not install or enable the experimental debugger.
                    </span>
                  </div>
                </div>
              )}

              <div className="tool-row">
                <strong>Detection</strong>
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
              </div>
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
    </main>
  );
}
