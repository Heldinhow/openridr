// Which control opens a workspace, and when the typed path is on screen. Kept out of the
// components so `tests/unit` can pin the platform rules without a DOM.
import { formatShortcut, shortcut, type Shortcut } from "../actions/keymap";

/** Open is `primary+O` — ⌘O on macOS, Ctrl+O elsewhere. The label comes from the keymap, never
 * from a literal in the JSX, so the hint cannot drift from the key that is handled. */
export const openShortcut: Shortcut = shortcut("o", { primary: true });

export function openShortcutLabel(platform: string): string {
  return formatShortcut(openShortcut, platform);
}

type Surfaces = {
  // Running inside the Tauri shell, the only surface where a native chooser can exist.
  tauri: boolean;
  // `null` until the first snapshot arrives: guessing would flash the field on macOS.
  platform: string | null;
  // The chooser answered `pick_unavailable` (or failed): the field is the way out.
  pickerUnavailable: boolean;
};

/** macOS keeps the verified look, so the field stays hidden until the chooser refuses. Everywhere
 * else it is always there, so opening a project never depends on a failure; the browser/E2E build
 * has no native chooser at all and always shows it. */
export function showsPathForm({ tauri, platform, pickerUnavailable }: Surfaces): boolean {
  if (pickerUnavailable) return true;
  if (!tauri) return true;
  if (platform === null) return false;
  return platform !== "macos";
}
