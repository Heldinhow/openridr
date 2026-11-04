import type { ContextMenuItem } from "./EditorContextMenu";

// Builders for the Solution tree context menus. Only actions with a real backend are listed;
// "Run Unit Test" stays visible but disabled, like in Rider. Gates arrive as ready-made reasons.
export interface ProjectMenu {
  name: string;
  projectFile: string;
  trusted: boolean;
  /** Run/Debug only exist for console projects; a library is buildable but not executable. */
  executable: boolean;
  /** Why Reload Project (re-evaluation) cannot run, or null. */
  reloadReason: string | null;
  buildReason: string | null;
  runReason: string | null;
  debugReason: string | null;
  shortcuts: { build?: string; run?: string; debug?: string };
  onTrust: () => void;
  onNuGet: () => void;
  onReload: () => void;
  onBuild: () => void;
  onRun: () => void;
  onDebug: () => void;
  onEditFile: () => void;
  onFind: () => void;
  onCopyPath: () => void;
  onReveal: () => void;
  onProperties: () => void;
  onNewFile: () => void;
}

export function projectMenuItems(m: ProjectMenu): ContextMenuItem[] {
  const items: ContextMenuItem[] = [];
  if (!m.trusted) {
    items.push({ id: "trust", label: "Trust this project", action: m.onTrust });
  }
  items.push(
    { id: "nuget", label: "Manage NuGet Packages", action: m.onNuGet, separatorBefore: !m.trusted },
    { id: "reload", label: "Reload Project", disabledReason: m.reloadReason, action: m.onReload },
    { id: "build", label: "Build Selected Projects", shortcut: m.shortcuts.build, disabledReason: m.buildReason, action: m.onBuild, separatorBefore: true },
  );
  if (m.executable) {
    items.push(
      { id: "run", label: `Run '${m.name}'`, shortcut: m.shortcuts.run, disabledReason: m.runReason, action: m.onRun },
      { id: "debug", label: `Debug '${m.name}'`, shortcut: m.shortcuts.debug, disabledReason: m.debugReason, action: m.onDebug },
    );
  }
  items.push(
    {
      id: "unit-test",
      label: "Run Unit Test",
      disabledReason: "Not available from the Solution tree; use the Tests tool window.",
      action: () => undefined,
    },
    {
      id: "edit",
      label: "Edit",
      action: () => undefined,
      separatorBefore: true,
      children: [
        { id: "edit-file", label: `Edit '${m.projectFile}'`, action: m.onEditFile },
        { id: "find", label: "Find in Files…", action: m.onFind, separatorBefore: true },
      ],
    },
    {
      id: "new-file",
      label: "New File…",
      action: m.onNewFile,
      separatorBefore: true,
    },
    // No Rename/Delete here: a project file belongs to the solution, so moving or deleting it
    // would leave the workspace pointing at a file that no longer matches.
    { id: "copy-path", label: "Copy Path", action: m.onCopyPath, separatorBefore: true },
    { id: "reveal", label: "Show in Finder", action: m.onReveal },
    { id: "properties", label: "Properties…", action: m.onProperties, separatorBefore: true },
  );
  return items;
}

export function fileMenuItems(m: {
  open?: () => void;
  onCopyPath: () => void;
  onFind: () => void;
  onReveal: () => void;
  newFile?: () => void;
  rename?: () => void;
  delete?: () => void;
}): ContextMenuItem[] {
  const items: ContextMenuItem[] = [];
  if (m.open) items.push({ id: "open", label: "Open", action: m.open });
  items.push(
    { id: "copy-path", label: "Copy Path", action: m.onCopyPath, separatorBefore: items.length > 0 },
    { id: "find", label: "Find in Files…", action: m.onFind },
    { id: "reveal", label: "Show in Finder", action: m.onReveal },
  );
  if (m.newFile) items.push({ id: "new-file", label: "New File…", action: m.newFile, separatorBefore: true });
  if (m.rename) items.push({ id: "rename", label: "Rename…", action: m.rename });
  if (m.delete) items.push({ id: "delete", label: "Delete", action: m.delete });
  return items;
}

/** The solution root: central build/restore commands plus the file actions that apply there. */
export function solutionMenuItems(m: {
  buildReason: string | null;
  restoreReason: string | null;
  reloadReason: string | null;
  buildShortcut?: string;
  onBuild: () => void;
  onRestore: () => void;
  onReload: () => void;
  onCopyPath: () => void;
  onFind: () => void;
  onReveal: () => void;
}): ContextMenuItem[] {
  return [
    { id: "build-solution", label: "Build Solution", shortcut: m.buildShortcut, disabledReason: m.buildReason, action: m.onBuild },
    { id: "restore-solution", label: "Restore Solution", disabledReason: m.restoreReason, action: m.onRestore },
    { id: "reload", label: "Reload Solution", disabledReason: m.reloadReason, action: m.onReload, separatorBefore: true },
    { id: "copy-path", label: "Copy Path", action: m.onCopyPath, separatorBefore: true },
    { id: "find", label: "Find in Files…", action: m.onFind },
    { id: "reveal", label: "Show in Finder", action: m.onReveal },
    {
      id: "add-project",
      label: "Add Project…",
      disabledReason: "Adding projects is not implemented.",
      action: () => undefined,
      separatorBefore: true,
    },
  ];
}
