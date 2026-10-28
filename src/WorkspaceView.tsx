import { useState } from "react";
import { call, HostCallError, type Snapshot, type OutlineProject } from "./host/client";
import { CodeEditor } from "./CodeEditor";

type Tab = {
  path: string;
  content: string;
  saved: string;
  hash: string;
  conflict: null | { content: string; hash: string };
};

type Confirmation = { kind: "tab"; path: string } | { kind: "workspace" };

type BottomTab = "build" | "problems" | "debug" | "terminal" | "tests";

type Props = {
  snapshot: Snapshot;
  onClosed: (snapshot: Snapshot) => void;
};

const bottomTabs: { id: BottomTab; label: string }[] = [
  { id: "build", label: "Build" },
  { id: "problems", label: "Problems" },
  { id: "debug", label: "Debug" },
  { id: "terminal", label: "Terminal" },
  { id: "tests", label: "Tests" },
];

function baseName(path: string) {
  return path.split("/").pop() ?? path;
}

function messageOf(error: unknown) {
  return error instanceof HostCallError ? error.message : String(error);
}

function projectFor(path: string, projects: OutlineProject[]) {
  return projects.find((project) => {
    const dir = project.path.replace(/\/[^/]+$/, "");
    return path.startsWith(`${dir}/`);
  });
}

export function WorkspaceView({ snapshot, onClosed }: Props) {
  const workspace = snapshot.workspace!;
  const projects = snapshot.outline?.projects ?? [];
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activePath, setActivePath] = useState<string | null>(null);
  const [message, setMessage] = useState<{ tone: "info" | "error"; text: string } | null>(null);
  const [confirm, setConfirm] = useState<Confirmation | null>(null);
  const [bottomTab, setBottomTab] = useState<BottomTab>("build");
  const [expanded, setExpanded] = useState<Set<string>>(
    () => new Set(projects.map((project) => project.path)),
  );

  const active = tabs.find((tab) => tab.path === activePath) ?? null;
  const activeProject = active ? projectFor(active.path, projects) : undefined;
  const dirtyCount = tabs.filter((tab) => tab.content !== tab.saved || tab.conflict).length;

  function toggleProject(path: string) {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }

  async function openFile(path: string) {
    if (tabs.some((tab) => tab.path === path)) {
      setActivePath(path);
      return;
    }
    try {
      const file = await call<{ path: string; content: string; hash: string }>("file.read", {
        path,
      });
      setTabs((current) => [
        ...current,
        { path: file.path, content: file.content, saved: file.content, hash: file.hash, conflict: null },
      ]);
      setActivePath(file.path);
      setMessage(null);
    } catch (error) {
      setMessage({ tone: "error", text: `Could not open ${path}: ${messageOf(error)}` });
    }
  }

  function updateContent(path: string, content: string) {
    setTabs((current) =>
      current.map((tab) => (tab.path === path ? { ...tab, content } : tab)),
    );
  }

  function applyConflict(path: string, error: HostCallError) {
    const details = error.details as { currentContent?: string; currentHash?: string } | undefined;
    if (details?.currentContent === undefined || !details.currentHash) {
      setMessage({ tone: "error", text: `Save failed: ${error.message}` });
      return;
    }
    setTabs((current) =>
      current.map((tab) =>
        tab.path === path
          ? { ...tab, conflict: { content: details.currentContent!, hash: details.currentHash! } }
          : tab,
      ),
    );
    setMessage({
      tone: "error",
      text: `${path} changed on disk. Your buffer was kept; choose Reload from disk or Overwrite disk.`,
    });
  }

  async function save() {
    const tab = active;
    if (!tab) return;
    if (tab.conflict) {
      setMessage({
        tone: "error",
        text: "A disk conflict is pending. Choose Reload from disk or Overwrite disk explicitly.",
      });
      return;
    }
    if (tab.content === tab.saved) {
      setMessage({ tone: "info", text: "No changes to save." });
      return;
    }
    const content = tab.content;
    try {
      const result = await call<{ path: string; hash: string }>("file.write", {
        path: tab.path,
        content,
        expected_hash: tab.hash,
      });
      setTabs((current) =>
        current.map((item) =>
          item.path === tab.path
            ? { ...item, saved: content, hash: result.hash, conflict: null }
            : item,
        ),
      );
      setMessage({ tone: "info", text: `Saved ${tab.path}` });
    } catch (error) {
      if (error instanceof HostCallError && error.code === "conflict") {
        applyConflict(tab.path, error);
      } else {
        setMessage({ tone: "error", text: `Save failed; buffer kept. ${messageOf(error)}` });
      }
    }
  }

  function reloadFromDisk() {
    if (!active?.conflict) return;
    const conflict = active.conflict;
    setTabs((current) =>
      current.map((tab) =>
        tab.path === active.path
          ? { ...tab, content: conflict.content, saved: conflict.content, hash: conflict.hash, conflict: null }
          : tab,
      ),
    );
    setMessage({
      tone: "info",
      text: `Reloaded ${active.path} from disk. Your edits were discarded.`,
    });
  }

  async function overwriteDisk() {
    const tab = active;
    if (!tab?.conflict) return;
    const conflict = tab.conflict;
    const content = tab.content;
    try {
      const result = await call<{ path: string; hash: string }>("file.write", {
        path: tab.path,
        content,
        expected_hash: conflict.hash,
      });
      setTabs((current) =>
        current.map((item) =>
          item.path === tab.path
            ? { ...item, saved: content, hash: result.hash, conflict: null }
            : item,
        ),
      );
      setMessage({ tone: "info", text: `Overwrote ${tab.path} with the buffer.` });
    } catch (error) {
      if (error instanceof HostCallError && error.code === "conflict") {
        applyConflict(tab.path, error);
      } else {
        setMessage({ tone: "error", text: `Overwrite failed; buffer kept. ${messageOf(error)}` });
      }
    }
  }

  function closeTab(path: string) {
    const remaining = tabs.filter((tab) => tab.path !== path);
    setTabs(remaining);
    if (activePath === path) {
      setActivePath(remaining.at(-1)?.path ?? null);
    }
  }

  function requestCloseTab(path: string) {
    const tab = tabs.find((item) => item.path === path);
    if (!tab) return;
    if (tab.content !== tab.saved || tab.conflict) {
      setConfirm({ kind: "tab", path });
      return;
    }
    closeTab(path);
  }

  async function closeWorkspace() {
    try {
      const closed = await call<Snapshot>("workspace.close");
      onClosed(closed);
    } catch (error) {
      setMessage({
        tone: "error",
        text: `Could not close the workspace: ${messageOf(error)}`,
      });
    }
  }

  function requestCloseWorkspace() {
    if (dirtyCount > 0) {
      setConfirm({ kind: "workspace" });
      return;
    }
    void closeWorkspace();
  }

  function discardConfirmed() {
    if (!confirm) return;
    const pending = confirm;
    setConfirm(null);
    if (pending.kind === "tab") {
      closeTab(pending.path);
      setMessage({ tone: "info", text: `Closed ${pending.path}; unsaved edits were discarded.` });
    } else {
      void closeWorkspace();
    }
  }

  function renderBottomContent() {
    switch (bottomTab) {
      case "build":
        return (
          <p className="muted">
            No build has run. Build is not available in this ticket; it arrives with ticket 11.
          </p>
        );
      case "problems":
        return (
          <p className="muted" data-testid="problems-empty">
            No analysis has run. Diagnostics from csharp-ls arrive with ticket 06.
          </p>
        );
      case "debug":
        return <p className="muted">Debug is not available in this build (ticket 14).</p>;
      case "terminal":
        return <p className="muted">Terminal is not available in this build (ticket 15).</p>;
      case "tests":
        return <p className="muted">Tests are not available in this build (ticket 17).</p>;
    }
  }

  return (
    <main className="workspace-view">
      <header className="chrome">
        <span className="brand">OpenRidr · {workspace.name}</span>
        <span className="warning">Restricted mode</span>
      </header>

      <div className="toolbar">
        <span>
          {workspace.kind === "solution" ? "Solution" : "Project"}:{" "}
          <strong>{workspace.name}</strong>
        </span>
        <span className="push" />
        <button disabled title="Build is not available in this ticket (ticket 11).">
          Build
        </button>
        <button disabled title="Run is not available in this ticket (ticket 12).">
          Run
        </button>
        <button disabled title="Debug is not available in this ticket (ticket 14).">
          Debug
        </button>
        <button data-testid="close-workspace" onClick={requestCloseWorkspace}>
          Close workspace
        </button>
      </div>

      <div className="notice warning">
        Executable actions are blocked: trust enforcement is added by ticket 02. MSBuild, restore,
        LSP, installation, terminal, build, run, debug and tests cannot start.
      </div>

      {message && (
        <div
          className={`notice ${message.tone === "error" ? "error" : ""}`}
          role={message.tone === "error" ? "alert" : "status"}
          data-testid="app-log"
        >
          {message.text}
        </div>
      )}

      <div className="workspace-grid">
        <nav className="side" aria-label="Solution explorer" data-testid="tree">
          <div className="panel-title">
            Solution Explorer <span className="tag">Text outline · not evaluated</span>
          </div>
          <div className="tree">
            <div className="tree-root">{workspace.name}</div>
            {projects.map((project) => {
              const open = expanded.has(project.path);
              return (
                <div key={project.path}>
                  <button
                    className="tree-row l1"
                    data-testid={`tree-project-${project.name}`}
                    aria-expanded={open}
                    onClick={() => toggleProject(project.path)}
                    title={project.path}
                  >
                    <span aria-hidden="true">{open ? "▾" : "▸"}</span>
                    <span>{project.name}</span>
                    <span className="tag">{project.kind}</span>
                    {project.tfms.map((tfm) => (
                      <span className="tag" key={tfm}>
                        {tfm}
                      </span>
                    ))}
                  </button>
                  {open && (
                    <>
                      {project.references.length > 0 && (
                        <div className="tree-note l2">References: {project.references.join(", ")}</div>
                      )}
                      {project.files.map((file) => (
                        <button
                          key={file}
                          className={`tree-row l2 ${activePath === file ? "selected" : ""}`}
                          data-testid={`tree-file-${file}`}
                          aria-current={activePath === file ? "page" : undefined}
                          onClick={() => void openFile(file)}
                          title={file}
                        >
                          {baseName(file)}
                        </button>
                      ))}
                      {project.files.length === 0 && (
                        <div className="tree-note l2">No .cs files found (bin/obj hidden).</div>
                      )}
                    </>
                  )}
                </div>
              );
            })}
          </div>
          <p className="tree-footnote">
            Read from project text only. MSBuild has not run and nothing is evaluated.
          </p>
        </nav>

        <section className="editor-pane" aria-label="Editor">
          <div className="tabbar">
            <div className="tabs" role="tablist" aria-label="Open files">
              {tabs.map((tab) => {
                const dirty = tab.content !== tab.saved || Boolean(tab.conflict);
                const selected = tab.path === activePath;
                return (
                  <div className={`tab ${selected ? "active" : ""}`} key={tab.path} data-testid={`tab-${tab.path}`}>
                    <button
                      role="tab"
                      aria-selected={selected}
                      className="tab-label"
                      onClick={() => setActivePath(tab.path)}
                      title={tab.path}
                      aria-label={`${baseName(tab.path)}${dirty ? ", modified" : ""}`}
                    >
                      {baseName(tab.path)}
                      {dirty && (
                        <span
                          className="dirty"
                          data-testid="dirty-marker"
                          aria-label="Modified"
                          title="Modified"
                        >
                          *
                        </span>
                      )}
                    </button>
                    <button
                      className="tab-close"
                      data-testid="close-buffer"
                      aria-label={`Close ${baseName(tab.path)}`}
                      title="Close file"
                      onClick={() => requestCloseTab(tab.path)}
                    >
                      ×
                    </button>
                  </div>
                );
              })}
            </div>
            <button
              className="save"
              data-testid="save-file"
              onClick={() => void save()}
              disabled={!active || (active.content === active.saved && !active.conflict)}
              title={active ? "Save with Cmd+S (macOS) or Ctrl+S (Windows)" : "No file open"}
            >
              Save
            </button>
          </div>

          {active ? (
            <>
              <div className="breadcrumb" data-testid="breadcrumb">
                {activeProject ? `${activeProject.name} / ` : ""}
                {baseName(active.path)}
                {activeProject ? ` / ${activeProject.tfms.join(", ") || "TFM not declared"}` : ""}
              </div>

              {active.conflict && (
                <div className="notice error" data-testid="conflict-banner" role="alert">
                  <strong>File changed on disk.</strong> {active.path} was modified outside
                  OpenRidr. Your buffer is preserved; nothing was overwritten.
                  <div className="actions">
                    <button data-testid="conflict-reload" onClick={reloadFromDisk}>
                      Reload from disk (discard my edits)
                    </button>
                    <button data-testid="conflict-overwrite" onClick={() => void overwriteDisk()}>
                      Overwrite disk
                    </button>
                  </div>
                </div>
              )}

              <div className="editor-body">
                <CodeEditor
                  key={active.path}
                  value={active.content}
                  onChange={(value) => updateContent(active.path, value)}
                  onSave={() => void save()}
                />
              </div>
            </>
          ) : (
            <div className="empty-editor">
              <p>Open a file from the text outline to edit it.</p>
              <p className="muted">Saving checks the disk first and never overwrites silently.</p>
            </div>
          )}
        </section>
      </div>

      <section className="output" aria-label="Bottom panel">
        <div className="tabs bottom-tabs" role="tablist" aria-label="Output panels">
          {bottomTabs.map((tab) => (
            <button
              key={tab.id}
              role="tab"
              aria-selected={bottomTab === tab.id}
              className={`tab-label ${bottomTab === tab.id ? "active" : ""}`}
              data-testid={`bottom-tab-${tab.id}`}
              onClick={() => setBottomTab(tab.id)}
            >
              {tab.label}
            </button>
          ))}
        </div>
        <div className="panel-body" role="tabpanel">
          {renderBottomContent()}
        </div>
      </section>

      <footer className="status">
        <span data-testid="status-workspace">{workspace.name}</span>
        <span className="warning" data-testid="status-mode">
          Restricted mode
        </span>
        <span>Text outline · not evaluated</span>
        {dirtyCount > 0 && (
          <span data-testid="status-dirty">
            {dirtyCount} modified buffer{dirtyCount === 1 ? "" : "s"}
          </span>
        )}
        <span className="push" data-testid="status-keymap">
          Keymap: IntelliJ / Rider
        </span>
      </footer>

      {confirm && (
        <div className="modal-backdrop">
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="confirm-title"
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                setConfirm(null);
              }
              if (event.key === "Tab") {
                const focusable = Array.from(
                  event.currentTarget.querySelectorAll<HTMLElement>("button"),
                );
                const first = focusable[0];
                const last = focusable[focusable.length - 1];
                if (!first || !last) return;
                if (event.shiftKey && document.activeElement === first) {
                  event.preventDefault();
                  last.focus();
                } else if (!event.shiftKey && document.activeElement === last) {
                  event.preventDefault();
                  first.focus();
                }
              }
            }}
          >
            <h2 id="confirm-title">
              {confirm.kind === "tab" ? "Close modified file?" : "Close workspace with unsaved changes?"}
            </h2>
            <p>
              {confirm.kind === "tab"
                ? `${confirm.path} has unsaved changes.`
                : `${dirtyCount} modified buffer${dirtyCount === 1 ? "" : "s"} will be lost.`}
            </p>
            <div className="actions">
              <button autoFocus data-testid="confirm-keep" onClick={() => setConfirm(null)}>
                Keep editing
              </button>
              <button
                className="danger"
                data-testid="confirm-discard"
                onClick={discardConfirmed}
              >
                {confirm.kind === "tab" ? "Discard and close" : "Discard and close workspace"}
              </button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}
