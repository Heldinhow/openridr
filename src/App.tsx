import { useState } from "react";
import { SetupScreen } from "./SetupScreen";
import { WorkspaceView } from "./WorkspaceView";
import type { Snapshot } from "./host/client";

export default function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [stage, setStage] = useState<"setup" | "workspace">("setup");
  const [lastPath, setLastPath] = useState("");

  function handleOpened(opened: Snapshot) {
    setSnapshot(opened);
    if (opened.workspace) {
      setLastPath(opened.workspace.path);
    }
  }

  function handleClosed(closed: Snapshot) {
    setSnapshot(closed.workspace ? closed : null);
    setStage("setup");
  }

  if (stage === "workspace" && snapshot?.workspace) {
    return <WorkspaceView snapshot={snapshot} onClosed={handleClosed} />;
  }

  return (
    <SetupScreen
      key={lastPath}
      snapshot={snapshot}
      initialPath={lastPath}
      onOpened={handleOpened}
      onEnter={() => setStage("workspace")}
    />
  );
}
