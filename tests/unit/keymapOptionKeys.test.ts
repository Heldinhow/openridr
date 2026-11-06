import { describe, expect, test } from "bun:test";
import {
  matches,
  pressedKey,
  shortcut,
  shortcutFromEvent,
  shortcutsFor,
  typesACharacter,
  type ProfileId,
} from "../../src/actions/keymap";

// macOS composes a different character while Option is held: ⌥W types ∑ on a US layout, so a
// shortcut that carries `alt` never matched it and the advertised ⌥⌘W of Update Project did
// nothing (issue #111). These events are what the webview actually delivers, named by the physical
// key (`code`) the way the browser reports it.
function keyEvent(key: string, code: string, mods: Partial<KeyboardEvent> = {}): KeyboardEvent {
  return {
    key,
    code,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...mods,
  } as KeyboardEvent;
}

const profiles: ProfileId[] = ["rider-macos", "intellij", "visual-studio"];

describe("Option-composed keys", () => {
  test("the composed character still matches the shortcut the keymap stores", () => {
    const optionW = keyEvent("∑", "KeyW", { metaKey: true, altKey: true });
    expect(pressedKey(optionW)).toBe("w");
    expect(matches(optionW, shortcut("w", { primary: true, alt: true }), "macos")).toBe(true);
    for (const profile of profiles) {
      const update = shortcutsFor("git.update", { profile, overrides: {} }, "macos");
      expect(update.some((item) => matches(optionW, item, "macos"))).toBe(true);
    }
  });

  test("the physical key cannot rescue a different shortcut", () => {
    const optionW = keyEvent("∑", "KeyW", { metaKey: true, altKey: true });
    expect(matches(optionW, shortcut("t", { primary: true, alt: true }), "macos")).toBe(false);
    // Option is part of the binding, not decoration: ⌘W must not answer for ⌥⌘W.
    expect(matches(optionW, shortcut("w", { primary: true }), "macos")).toBe(false);
  });

  test("keys Option does not compose keep using event.key", () => {
    expect(matches(keyEvent("F5", "F5", { altKey: true }), shortcut("f5", { alt: true }), "macos"))
      .toBe(true);
    expect(matches(keyEvent("Enter", "Enter", { altKey: true }), shortcut("enter", { alt: true }), "macos"))
      .toBe(true);
    // Without Option the layout decides, so a query-localized key still matches by character.
    expect(matches(keyEvent("a", "KeyQ"), shortcut("a"), "macos")).toBe(true);
    expect(pressedKey(keyEvent("a", "KeyQ"))).toBe("a");
  });

  test("capture records the key that can be matched again", () => {
    // The Settings recorder used to store "∑" for ⌥W, which no default or override could match.
    expect(shortcutFromEvent(keyEvent("∑", "KeyW", { metaKey: true, altKey: true }), "macos"))
      .toEqual({ key: "w", primary: true, control: false, alt: true, shift: false });
    expect(shortcutFromEvent(keyEvent("¡", "Digit1", { altKey: true }), "macos"))
      .toEqual({ key: "1", primary: false, control: false, alt: true, shift: false });
  });

  test("an override saved with the composed character keeps working", () => {
    // Before the physical key, the recorder persisted "∑" for ⌥W. Those settings.json entries must
    // keep matching, otherwise the fix silently drops a binding the user already had.
    const legacy = { key: "∑", primary: true, alt: true };
    expect(matches(keyEvent("∑", "KeyW", { metaKey: true, altKey: true }), legacy, "macos"))
      .toBe(true);
  });

  test("an Option-only binding yields to typing in the editor", () => {
    // ⌥⇧U is a dead key on macOS; `tests.quickList` carries exactly that default, and swallowing the
    // keystroke in the editor would remove special-character input.
    const quickList = shortcutsFor("tests.quickList", { profile: "rider-macos", overrides: {} }, "macos")[0];
    expect(typesACharacter(keyEvent("Û", "KeyU", { altKey: true, shiftKey: true }), quickList))
      .toBe(true);
    expect(typesACharacter(keyEvent("Dead", "KeyU", { altKey: true, shiftKey: true }), quickList))
      .toBe(true);
    // A binding with ⌘ or ⌃ is a command, not a character; typing never wins over it.
    const optionW = keyEvent("∑", "KeyW", { metaKey: true, altKey: true });
    expect(typesACharacter(optionW, shortcut("w", { primary: true, alt: true }))).toBe(false);
    // Keys Option does not compose keep the same character, so they are commands.
    expect(typesACharacter(keyEvent("Enter", "Enter", { altKey: true }), shortcut("enter", { alt: true })))
      .toBe(false);
  });
});
