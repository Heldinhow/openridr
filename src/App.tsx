import { useEffect, useRef, useState } from "react";
import { SetupScreen } from "./SetupScreen";
import { WorkspaceView } from "./WorkspaceView";
import { TrustPrompt } from "./TrustPrompt";
import { call, messageOf, type Snapshot } from "./host/client";
import { fetchRecent, type RecentProject } from "./welcome/recent";

export default function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [stage, setStage] = useState<"setup" | "workspace">("setup");
  const [lastPath, setLastPath] = useState("");
  // A workspace the host opened that still waits for its first Trust decision.
  const [prompt, setPrompt] = useState<Snapshot | null>(null);
  // Workspace that was open when the prompt appeared (Open recent inside the IDE); Cancel restores it.
  const previousPath = useRef<string | null>(null);
  const [cancelError, setCancelError] = useState<string | null>(null);
  const background = useRef<HTMLDivElement>(null);
  const [pendingFile, setPendingFile] = useState<string | null>(null);
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
  const recentSolutions = recentProjects.map((project) => project.path);
  // Path from Finder or the Dock while a workspace is mounted; WorkspaceView applies it with its dirty-buffer guard.
  const [externalOpen, setExternalOpen] = useState<{ path: string } | null>(null);
  // Same for the setup screen, which opens the path with its own form flow (candidates, errors, trust).
  const [setupOpen, setSetupOpen] = useState<{ path: string } | null>(null);
  // Only for a failed listener or invoke of the shell; open errors show where the open was started.
  const [externalError, setExternalError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    call<Snapshot>("app.state")
      .then((state) => {
        if (active) {
          setSnapshot(state);
        }
      })
      .catch(() => {
        // The toolchain section reports its own failures; the open form stays usable.
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    // The host records a project once it is decided; refetch when the workspace or stage changes.
    fetchRecent()
      .then(setRecentProjects)
      .catch(() => {});
  }, [snapshot?.workspace?.path, stage]);

  function handleSnapshot(next: Snapshot) {
    // Ignore a response produced by an older host context (including the initial
    // app.state racing a workspace.open). The host rejects late operation results;
    // this keeps the rendered snapshot from going backwards for the same reason.
    setSnapshot((current) =>
      current && next.contextId < current.contextId ? current : next,
    );
  }

  function handleOpened(opened: Snapshot) {
    handleSnapshot(opened);
    if (opened.workspace) {
      setLastPath(opened.workspace.path);
    }
    setPendingFile(null);
    // Undecided projects ask before entering the IDE; decided ones open straight in the stored mode.
    if (opened.trust.decision === null) {
      setCancelError(null);
      setPrompt(opened);
      setStage("setup");
    } else {
      setStage("workspace");
    }
  }

  useEffect(() => {
    // The page behind the modal is unreachable by keyboard and screen readers.
    background.current?.toggleAttribute("inert", prompt !== null);
  }, [prompt]);

  async function cancelPrompt() {
    setCancelError(null);
    try {
      // The host already switched workspaces on open: put the previous one back, or close.
      const previous = previousPath.current;
      const restored = previous
        ? await call<Snapshot>("workspace.open", { path: previous })
        : await call<Snapshot>("workspace.close");
      setPrompt(null);
      previousPath.current = null;
      if (previous) {
        handleOpened(restored);
      } else {
        handleClosed(restored);
      }
    } catch (caught) {
      setCancelError(messageOf(caught));
    }
  }

  function decided(next: Snapshot) {
    handleSnapshot(next);
    setPrompt(null);
    previousPath.current = null;
    setStage("workspace");
  }

  function handleClosed(closed: Snapshot) {
    // Keep the snapshot after closing: the setup screen still shows the neutral toolchain,
    // and any tracked process must stay visible with its pending decision.
    setSnapshot(closed);
    setStage("setup");
    setPendingFile(null);
  }

  async function openRecent(path: string) {
    // Opening another workspace resets trust in the host; the view remounts on the new id.
    // While a prompt is pending, the workspace to restore on Cancel is already in previousPath.
    if (!prompt) previousPath.current = snapshot?.workspace?.path ?? null;
    handleOpened(await call<Snapshot>("workspace.open", { path }));
  }

  function openExternal(path: string) {
    setExternalError(null);
    // A pending Trust prompt belongs to the project already on screen; replacing it would drop that decision.
    if (prompt) {
      setExternalError("Finish the Trust decision for the pending project before opening another one.");
      return;
    }
    if (stage === "workspace" && snapshot?.workspace) {
      setExternalOpen({ path });
      return;
    }
    setSetupOpen({ path });
  }

  // Read by the shell listener, which is registered once; the ref keeps the latest state.
  const openExternalRef = useRef(openExternal);
  openExternalRef.current = openExternal;

  useEffect(() => {
    // Finder, the Dock and the command line hand paths over only in the desktop shell.
    if (!window.__TAURI_INTERNALS__) return;
    let unmounted = false;
    let stop: (() => void) | undefined;
    void (async () => {
      const { invoke } = await import("@tauri-apps/api/core");
      const { listen } = await import("@tauri-apps/api/event");
      const takePending = async () => {
        const path = await invoke<string | null>("take_pending_open");
        if (path) openExternalRef.current(path);
      };
      const unlisten = await listen("openridr-open", () => void takePending());
      if (unmounted) {
        unlisten();
        return;
      }
      stop = unlisten;
      // A path that arrived before this listener (launch from Finder or argv) waits in the shell.
      await takePending();
    })().catch((caught) => setExternalError(messageOf(caught)));
    return () => {
      unmounted = true;
      stop?.();
    };
  }, []);

  function openGlobalJson() {
    setPendingFile(snapshot?.sdk.project.globalJsonPath ?? "global.json");
    setStage("workspace");
  }

  // Dismissable band for a path that could not be opened from outside the app.
  const externalBand = externalError ? (
    <div className="notice error external-open-error" role="alert" data-testid="external-open-error">
      {externalError}
      <button className="notice-dismiss" aria-label="Dismiss external open error" onClick={() => setExternalError(null)}>×</button>
    </div>
  ) : null;

  if (stage === "workspace" && snapshot?.workspace && !prompt) {
    return (
      <>
        {externalBand}
        <WorkspaceView
          key={snapshot.workspace.id}
          snapshot={snapshot}
          initialFile={pendingFile}
          recentSolutions={recentSolutions}
          externalOpen={externalOpen}
          onOpenRecent={openRecent}
          onSnapshot={handleSnapshot}
          onClosed={handleClosed}
        />
      </>
    );
  }

  return (
    <>
    {prompt && <TrustPrompt snapshot={prompt} onDecided={decided} onCancel={() => void cancelPrompt()} cancelError={cancelError} />}
    {externalBand}
    <div ref={background} style={{ display: "contents" }}>
    <SetupScreen
      key={lastPath}
      snapshot={snapshot}
      initialPath={lastPath}
      recent={recentProjects}
      onRecent={setRecentProjects}
      onOpened={handleOpened}
      onSnapshot={handleSnapshot}
      onOpenGlobalJson={openGlobalJson}
      externalOpen={setupOpen}
    />
    </div>
    </>
  );
}
