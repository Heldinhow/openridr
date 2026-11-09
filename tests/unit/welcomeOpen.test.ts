import { describe, expect, test } from "bun:test";
import { openShortcutLabel, showsPathForm } from "../../src/welcome/openTarget";

describe("welcome open control", () => {
  test("the hint is the keymap's, so Linux reads Ctrl+O and macOS keeps ⌘O", () => {
    expect(openShortcutLabel("macos")).toBe("⌘O");
    expect(openShortcutLabel("linux")).toBe("Ctrl+O");
    expect(openShortcutLabel("windows")).toBe("Ctrl+O");
  });

  test("macOS keeps the field hidden until the chooser refuses", () => {
    expect(showsPathForm({ tauri: true, platform: "macos", pickerUnavailable: false })).toBe(false);
    expect(showsPathForm({ tauri: true, platform: "macos", pickerUnavailable: true })).toBe(true);
  });

  test("Linux and Windows always offer the typed path", () => {
    for (const platform of ["linux", "windows"]) {
      expect(showsPathForm({ tauri: true, platform, pickerUnavailable: false })).toBe(true);
    }
  });

  test("the browser build and the unknown first frame", () => {
    // Playwright cannot drive a native chooser, so the web surface always types the path.
    expect(showsPathForm({ tauri: false, platform: "macos", pickerUnavailable: false })).toBe(true);
    // Before the first snapshot the platform is unknown: showing nothing beats flashing the field
    // on macOS, where it must stay hidden.
    expect(showsPathForm({ tauri: true, platform: null, pickerUnavailable: false })).toBe(false);
    expect(showsPathForm({ tauri: true, platform: null, pickerUnavailable: true })).toBe(true);
  });
});
