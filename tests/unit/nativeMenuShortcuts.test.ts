import { describe, expect, test } from "bun:test";
import {
  formatShortcut,
  keymapDefaults,
  shortcutsFor,
  signature,
  type ProfileId,
} from "../../src/actions/keymap";
import { reservedNativeShortcuts } from "../../src/actions/reservedShortcuts";

// The macOS menu resolves its key equivalents before the webview receives the keydown, so a keymap
// default that uses one of them never runs — it runs the menu item instead. The list lives in
// `src/actions/reservedShortcuts.ts`, next to the pointer to the shell that builds the menu.
const profiles: ProfileId[] = ["rider-macos", "intellij", "visual-studio"];

function macLabel(id: string, profile: ProfileId): string[] {
  return shortcutsFor(id, { profile, overrides: {} }, "macos").map((item) =>
    formatShortcut(item, "macos"),
  );
}

describe("keymap defaults against the native macOS menu", () => {
  test("no default uses an accelerator the native menu owns", () => {
    const reserved = new Set(reservedNativeShortcuts.map((item) => signature(item, "macos")));
    const collisions: string[] = [];
    for (const profile of profiles) {
      for (const id of Object.keys(keymapDefaults)) {
        for (const item of shortcutsFor(id, { profile, overrides: {} }, "macos")) {
          if (reserved.has(signature(item, "macos"))) {
            collisions.push(`${id} (${profile}) is bound to ${formatShortcut(item, "macos")}`);
          }
        }
      }
    }
    expect(collisions).toEqual([]);
  });

  test("⌘W and ⌥⌘W stay with the keymap, not with Close Window or Close All", () => {
    // The regression behind issue #111: both keys belonged to the native menu and quit the app
    // instead of closing a tab or opening Update.
    const reserved = reservedNativeShortcuts.map((item) => formatShortcut(item, "macos"));
    expect(reserved).not.toContain("⌘W");
    expect(reserved).not.toContain("⌥⌘W");
    for (const profile of profiles) {
      expect(macLabel("editor.closeTab", profile)).toEqual(["⌘W"]);
      expect(macLabel("git.update", profile)).toEqual(["⌥⌘W"]);
    }
  });
});
