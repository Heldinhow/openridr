import { useState } from "react";
import { call, HostCallError, type Snapshot } from "../host/client";
import { ProcessPanel } from "../ProcessPanel";
import { DebugAdapterPanel } from "../ToolchainPanels";
import { ProjectSdkBody } from "./ProjectSdkBody";

type Props = {
  snapshot: Snapshot | null;
  onSnapshot: (snapshot: Snapshot) => void;
  onOpenGlobalJson: () => void;
};

function messageOf(error: unknown) {
  return error instanceof HostCallError ? error.message : String(error);
}

export function ToolchainView({ snapshot, onSnapshot, onOpenGlobalJson }: Props) {
  const [redetecting, setRedetecting] = useState(false);
  const [toolchainError, setToolchainError] = useState<string | null>(null);
  const workspace = snapshot?.workspace ?? null;
  const outline = workspace ? snapshot?.outline : null;
  const tfms = [...new Set(outline?.projects.flatMap((project) => project.tfms) ?? [])];
  const sdk = snapshot?.sdk;

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
    <>
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
    </>
  );
}
