import { useEffect, useState } from "react";
import { SetupScreen } from "./SetupScreen";
import { WorkspaceView } from "./WorkspaceView";
import { call, type Snapshot } from "./host/client";

export default function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [stage, setStage] = useState<"setup" | "workspace">("setup");
  const [lastPath, setLastPath] = useState("");
  const [pendingFile, setPendingFile] = useState<string | null>(null);
  const [recentSolutions, setRecentSolutions] = useState<string[]>([]);

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
    // The host records every successful open; refetch whenever the workspace changes.
    call<{ projects: string[] }>("recent.list")
      .then((result) => setRecentSolutions(result.projects))
      .catch(() => {});
  }, [snapshot?.workspace?.path]);

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
    handleOpened(await call<Snapshot>("workspace.open", { path }));
  }

  function openGlobalJson() {
    setPendingFile(snapshot?.sdk.project.globalJsonPath ?? "global.json");
    setStage("workspace");
  }

  if (stage === "workspace" && snapshot?.workspace) {
    return (
      <WorkspaceView
        key={snapshot.workspace.id}
        snapshot={snapshot}
        initialFile={pendingFile}
        recentSolutions={recentSolutions}
        onOpenRecent={openRecent}
        onSnapshot={handleSnapshot}
        onClosed={handleClosed}
      />
    );
  }

  return (
    <SetupScreen
      key={lastPath}
      snapshot={snapshot}
      initialPath={lastPath}
      recent={recentSolutions}
      onOpened={handleOpened}
      onSnapshot={handleSnapshot}
      onEnter={() => setStage("workspace")}
      onOpenGlobalJson={openGlobalJson}
    />
  );
}
