import { useEffect, useRef, useState, type ReactNode } from "react";
import { SetupScreen } from "./SetupScreen";
import { SetupGate } from "./SetupGate";
import { WorkspaceView } from "./WorkspaceView";
import { TrustPrompt } from "./TrustPrompt";
import { UpdateDialog } from "./UpdateDialog";
import { call, messageOf, type Snapshot } from "./host/client";
import { fetchRecent, type RecentProject } from "./welcome/recent";

export default function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [stage, setStage] = useState<"setup" | "gate" | "workspace">("setup");
  const [lastPath, setLastPath] = useState("");
  // A workspace the host opened that still waits for its first Trust decision.
  const [prompt, setPrompt] = useState<Snapshot | null>(null);
  // Workspace that was open when the prompt appeared (Open recent inside the IDE); Cancel restores it.
  const previousPath = useRef<string | null>(null);
  const [cancelError, setCancelError] = useState<string | null>(null);
  const background = useRef<HTMLDivElement>(null);
  const [pendingFile, setPendingFile] = useState<string | null>(null);
  const [gateError, setGateError] = useState<string | null>(null);
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
  const recentSolutions = recentProjects.map((project) => project.path);
  // Path from Finder or the Dock while a workspace is mounted; WorkspaceView applies it with its dirty-buffer guard.
  const [externalOpen, setExternalOpen] = useState<{ path: string } | null>(null);
  // Increments on every Cmd+Q or app-menu Quit; WorkspaceView guards it with the unsaved-changes dialog.
  const [quitRequest, setQuitRequest] = useState(0);
  // Set by "Restart now" in the update dialog: the same guard runs, and the exit restarts instead.
  const [restartAfterQuit, setRestartAfterQuit] = useState(false);
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
    if (snapshot?.tools["csharp-ls"].state !== "verifying" || stage !== "setup") return;
    let active = true;
    const timer = setInterval(() => {
      call<Snapshot>("app.state").then((next) => {
        if (active) handleSnapshot(next);
      }).catch(() => {});
    }, 400);
    return () => { active = false; clearInterval(timer); };
  }, [snapshot?.tools["csharp-ls"].state, stage]);

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

  async function handleOpened(opened: Snapshot) {
    let next = opened;
    // Both tools are fixed project setup; ask the host to verify/install them before the
    // workspace is usable. A ready installation is reused synchronously, so no gate appears.
    if (next.workspace && next.setup.state !== "ready") {
      setGateError(null);
      try {
        next = await call<Snapshot>("setup.start");
      } catch (caught) {
        setGateError(messageOf(caught));
      }
    }
    handleSnapshot(next);
    if (next.workspace) {
      setLastPath(next.workspace.path);
    }
    setPendingFile(null);
    if (!next.workspace) {
      setStage("setup");
      return;
    }
    if (next.setup.state !== "ready") {
      setPrompt(null);
      setStage("gate");
      return;
    }
    // Undecided projects ask before entering the IDE; decided ones open straight in the stored mode.
    if (next.trust.decision === null) {
      setCancelError(null);
      setPrompt(next);
      setStage("setup");
    } else {
      setStage("workspace");
    }
  }

  useEffect(() => {
    // The gate polls through SetupGate; once the host reports ready, continue to trust/workspace.
    if (stage !== "gate" || !snapshot?.workspace || snapshot.setup.state !== "ready") return;
    setGateError(null);
    if (snapshot.trust.decision === null) {
      setPrompt(snapshot);
      setStage("setup");
    } else {
      setStage("workspace");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [stage, snapshot]);

  async function retrySetup() {
    setGateError(null);
    try {
      handleSnapshot(await call<Snapshot>("setup.start"));
    } catch (caught) {
      setGateError(messageOf(caught));
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
        void handleOpened(restored);
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
    await handleOpened(await call<Snapshot>("workspace.open", { path }));
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
    if (stage === "gate") {
      setExternalError("Finish the project setup before opening another project.");
      return;
    }
    setSetupOpen({ path });
  }

  // Read by the shell listener, which is registered once; the ref keeps the latest state.
  const openExternalRef = useRef(openExternal);
  openExternalRef.current = openExternal;

  async function quitNow(restart: boolean) {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke(restart ? "restart_app" : "quit_app");
    } catch (error) {
      setExternalError(`Could not quit OpenRidr: ${messageOf(error)}`);
    }
  }

  function requestQuit(restart = false) {
    // Every request states its own intent, so a cancelled restart cannot turn a later Quit into one.
    setRestartAfterQuit(restart);
    // Only the mounted workspace owns buffers; anywhere else there is nothing to save.
    if (stage === "workspace" && snapshot?.workspace && !prompt) {
      setQuitRequest((current) => current + 1);
      return;
    }
    void quitNow(restart);
  }

  const requestQuitRef = useRef(requestQuit);
  requestQuitRef.current = requestQuit;

  useEffect(() => {
    // Finder, the Dock and the command line hand paths over only in the desktop shell;
    // Cmd+Q and the app menu Quit arrive here too, after the shell stopped the native quit.
    if (!window.__TAURI_INTERNALS__) return;
    let unmounted = false;
    let stop: (() => void) | undefined;
    void (async () => {
      const { invoke } = await import("@tauri-apps/api/core");
      const { listen } = await import("@tauri-apps/api/event");
      const takePending = async () => {
        try {
          const path = await invoke<string | null>("take_pending_open");
          if (path) openExternalRef.current(path);
        } catch (error) {
          setExternalError(`Could not open the requested file: ${messageOf(error)}`);
        }
      };
      const takeQuit = async () => {
        try {
          if (await invoke<boolean>("take_pending_quit")) requestQuitRef.current();
        } catch (error) {
          setExternalError(`Could not quit OpenRidr: ${messageOf(error)}`);
        }
      };
      const stopOpen = await listen("openridr-open", () => void takePending());
      const stopQuit = await listen("openridr-quit", () => void takeQuit());
      if (unmounted) {
        stopOpen();
        stopQuit();
        return;
      }
      stop = () => {
        stopOpen();
        stopQuit();
      };
      // Requests that arrived before these listeners (launch from Finder or a quit during first paint) wait in the shell.
      await takePending();
      await takeQuit();
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

  let body: ReactNode;
  if (stage === "gate" && snapshot?.workspace) {
    body = (
      <>
        {externalBand}
        <SetupGate
          snapshot={snapshot}
          error={gateError}
          onSnapshot={handleSnapshot}
          onRetry={() => void retrySetup()}
          onCancel={() => {
            setGateError(null);
            void call<Snapshot>("workspace.close")
              .then(handleClosed)
              .catch((caught) => setGateError(messageOf(caught)));
          }}
        />
      </>
    );
  } else if (stage === "workspace" && snapshot?.workspace && !prompt) {
    body = (
      <>
        {externalBand}
        <WorkspaceView
          key={snapshot.workspace.id}
          snapshot={snapshot}
          initialFile={pendingFile}
          recentSolutions={recentSolutions}
          externalOpen={externalOpen}
          quitRequest={quitRequest}
          restartOnQuit={restartAfterQuit}
          onOpenRecent={openRecent}
          onSnapshot={handleSnapshot}
          onClosed={handleClosed}
        />
      </>
    );
  } else {
    body = (
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

  return (
    <>
      {body}
      <UpdateDialog onRestart={() => requestQuit(true)} />
    </>
  );
}
