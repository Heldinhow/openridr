import {
  chordLabelsFor,
  formatShortcut,
  formatShortcutText,
  matches,
  shortcutsFor,
  signature,
  type Keymap,
  type Shortcut,
} from "./keymap";

/** One entry of the central registry. Menus, toolbar buttons, bands and Find Action read
 * only from this shape; a disabled action always carries why. */
export type Action = {
  id: string;
  title: string;
  group: string;
  execute: () => void;
  disabledReason?: string | null;
  /** Global commands may run while the CodeMirror editor has focus. */
  editor?: boolean;
};

export type ResolvedAction = Action & {
  shortcuts: Shortcut[];
  shortcutLabels: string[];
  shortcutTexts: string[];
  /** Two-key chords (⌘U R…) that are active but not remappable yet. */
  chordLabels: string[];
  conflicts: string[];
};

export function resolveActions(
  actions: Action[],
  keymap: Keymap,
  platform: string,
): ResolvedAction[] {
  const resolved = actions.map((action) => {
    const shortcuts = shortcutsFor(action.id, keymap, platform);
    return {
      ...action,
      shortcuts,
      shortcutLabels: shortcuts.map((shortcut) => formatShortcut(shortcut, platform)),
      shortcutTexts: shortcuts.map((shortcut) => formatShortcutText(shortcut, platform)),
      chordLabels: chordLabelsFor(action.id, keymap.profile, platform),
      conflicts: [] as string[],
    };
  });
  const owners = new Map<string, string[]>();
  for (const action of resolved) {
    for (const shortcut of action.shortcuts) {
      const key = signature(shortcut, platform);
      const ids = owners.get(key) ?? [];
      ids.push(action.id);
      owners.set(key, ids);
    }
  }
  for (const ids of owners.values()) {
    if (ids.length < 2) continue;
    for (const id of ids) {
      const action = resolved.find((item) => item.id === id);
      if (!action) continue;
      for (const other of ids) {
        if (other === id) continue;
        const title = resolved.find((item) => item.id === other)?.title ?? other;
        if (!action.conflicts.includes(title)) action.conflicts.push(title);
      }
    }
  }
  return resolved;
}

/** A fixed lookup a surface renders from; a missing id is a registry bug, not a runtime state. */
export function requireAction(
  actions: Map<string, ResolvedAction>,
  id: string,
): ResolvedAction {
  const action = actions.get(id);
  if (!action) throw new Error(`Registry action "${id}" is missing.`);
  return action;
}

/** First registered action wins a collision; availability never reorders the registry. */
export function firstActionFor(
  actions: ResolvedAction[],
  event: KeyboardEvent,
  platform: string,
): ResolvedAction | null {
  for (const action of actions) {
    if (action.shortcuts.some((shortcut) => matches(event, shortcut, platform))) {
      return action;
    }
  }
  return null;
}
