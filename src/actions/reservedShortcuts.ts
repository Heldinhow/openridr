import { shortcut, type Shortcut } from "./keymap";

/**
 * Accelerators the native macOS menu owns. The webview never sees them: macOS resolves a menu key
 * equivalent before the keydown reaches the page, so a keymap default on one of these is dead.
 *
 * `Close Window` (⌘W) and the `Close All` AppKit injects next to it (⌥⌘W) were taken out of the
 * menu on purpose — `⌘W` belongs to `editor.closeTab` and `⌥⌘W` to `git.update`, and both used to
 * quit the app (issue #111). Undoing that in the shell makes those two entries live again, and this
 * list is where the next collision gets caught: `tests/unit/nativeMenuShortcuts.test.ts` fails
 * when a keymap default uses one of them.
 *
 * Mirrored by hand: the list is what the shell builds (`Menu::default` plus the Quit replacement in
 * `src-tauri/src/main.rs`), and it cannot be imported from Rust. Keep it in sync when the menu
 * changes; the macOS-only items are listed with the menu that owns them.
 */
export const reservedNativeShortcuts: Shortcut[] = [
  // App menu: Quit is replaced by the shell so the UI can ask about unsaved buffers first.
  shortcut("q", { primary: true }),
  shortcut("h", { primary: true }), // Hide OpenRidr
  shortcut("h", { primary: true, alt: true }), // Hide Others
  // Edit menu (predefined items), plus the two entries AppKit adds itself.
  shortcut("z", { primary: true }), // Undo
  shortcut("z", { primary: true, shift: true }), // Redo
  shortcut("x", { primary: true }), // Cut
  shortcut("c", { primary: true }), // Copy
  shortcut("v", { primary: true }), // Paste
  shortcut("a", { primary: true }), // Select All
  shortcut("space", { primary: true, control: true }), // Emoji & Symbols
  // View and Window menus: fullscreen is predefined, the window cycle is AppKit's.
  shortcut("f", { primary: true, control: true }), // Toggle Full Screen
  shortcut("m", { primary: true }), // Minimize
  shortcut("`", { primary: true }), // Cycle through windows
];
