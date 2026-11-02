import { useEffect, useState } from "react";
import { call, messageOf, type RunConfig, type RunConfigStore } from "./host/client";

// The per-solution run configuration store: read when the workspace changes, written by every commit.
// Moved out of WorkspaceView unchanged.
export function useRunConfigs(c: {
  workspacePath: string;
  setMessage: (message: { tone: "info" | "error"; text: string } | null) => void;
}) {
  const { workspacePath, setMessage } = c;
  const [runConfigStore, setRunConfigStore] = useState<RunConfigStore | null>(null);
  const [runConfigsLoaded, setRunConfigsLoaded] = useState(false);

  useEffect(() => {
    let alive = true;
    setRunConfigStore(null);
    setRunConfigsLoaded(false);
    call<RunConfigStore>("runConfigs.read", { solution: workspacePath })
      .then((store) => {
        if (alive) setRunConfigStore(store);
      })
      .catch((error) => {
        if (alive) {
          setMessage({
            tone: "error",
            text: `Run configurations could not be read: ${messageOf(error)}`,
          });
        }
      })
      .finally(() => {
        if (alive) setRunConfigsLoaded(true);
      });
    return () => {
      alive = false;
    };
  }, [workspacePath]);

  async function commitRunConfigs(configs: RunConfig[], activeId: string) {
    const store = await call<RunConfigStore>("runConfigs.write", {
      solution: workspacePath,
      activeId,
      configs,
    });
    setRunConfigStore(store);
    return store;
  }

  async function persistStartupProject(project: string) {
    const store = await call<RunConfigStore>("runConfigs.setStartup", {
      solution: workspacePath,
      startupProject: project,
    });
    setRunConfigStore(store);
    return store;
  }

  return { runConfigStore, runConfigsLoaded, commitRunConfigs, persistStartupProject };
}
