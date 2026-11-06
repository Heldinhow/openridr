import { expect, test } from "bun:test";
import { normalizeKeymap, shortcutsFor } from "../../src/actions/keymap";
import { firstActionFor, resolveActions } from "../../src/actions/registry";

test("Visual Studio profile survives settings normalization and changes shortcuts", () => {
  const visualStudio = normalizeKeymap(JSON.parse(JSON.stringify({ profile: "visual-studio" })));
  const rider = normalizeKeymap({ profile: "rider-macos" });
  expect(visualStudio.profile).toBe("visual-studio");
  expect(shortcutsFor("code.goToDeclaration", visualStudio, "macos")).toEqual([{ key: "f12" }]);
  expect(shortcutsFor("code.goToDeclaration", rider, "macos")).toEqual([
    { key: "f12" }, { key: "b", primary: true },
  ]);
  expect(shortcutsFor("debug.toggleBreakpoint", visualStudio, "macos")).toEqual([{ key: "f9" }]);
  expect(shortcutsFor("build.start", visualStudio, "macos")).toEqual([{ key: "b", primary: true, shift: true }]);
});

test("keyboard dispatch follows the selected profile", () => {
  const actions = ["debug.continue", "debug.toggleBreakpoint", "code.goToDeclaration"].map((id) => ({
    id, title: id, group: "Test", execute: () => {},
  }));
  const visualStudio = resolveActions(actions, normalizeKeymap({ profile: "visual-studio" }), "macos");
  const rider = resolveActions(actions, normalizeKeymap({ profile: "rider-macos" }), "macos");
  const event = (key: string, metaKey = false) => ({ key, metaKey, ctrlKey: false, altKey: false, shiftKey: false }) as KeyboardEvent;
  expect(firstActionFor(visualStudio, event("F9"), "macos")?.id).toBe("debug.toggleBreakpoint");
  expect(firstActionFor(visualStudio, event("F12"), "macos")?.id).toBe("code.goToDeclaration");
  expect(firstActionFor(rider, event("b", true), "macos")?.id).toBe("code.goToDeclaration");
});

test("Visual Studio Run uses Ctrl+F5 rather than macOS Command+F5", () => {
  const keymap = normalizeKeymap({ profile: "visual-studio" });
  expect(shortcutsFor("run.start", keymap, "macos")).toEqual([{ key: "f5", control: true }]);
  expect(shortcutsFor("run.start", keymap, "windows")).toEqual([{ key: "f5", primary: true }]);
  const actions = resolveActions([{ id: "run.start", title: "Run", group: "Run", execute: () => {} }], keymap, "macos");
  const event = (ctrlKey: boolean, metaKey: boolean) => ({ key: "F5", ctrlKey, metaKey, altKey: false, shiftKey: false }) as KeyboardEvent;
  expect(firstActionFor(actions, event(true, false), "macos")?.id).toBe("run.start");
  expect(firstActionFor(actions, event(false, true), "macos")).toBe(null);
});

test("Visual Studio F5 starts debugging or continues a paused session", () => {
  const keymap = normalizeKeymap({ profile: "visual-studio" });
  let executed = "";
  const actions = ["debug.start", "debug.continue"].map((id) => ({
    id, title: id, group: "Debug", execute: () => { executed = id; },
  }));
  const resolved = resolveActions(actions, keymap, "macos");
  const event = { key: "F5", metaKey: false, ctrlKey: false, altKey: false, shiftKey: false } as KeyboardEvent;
  expect(shortcutsFor("debug.continue", keymap, "macos")).toEqual([{ key: "f5" }]);
  firstActionFor(resolved, event, "macos", { profile: keymap.profile, debugPaused: false })?.execute();
  expect(executed).toBe("debug.start");
  firstActionFor(resolved, event, "macos", { profile: keymap.profile, debugPaused: true })?.execute();
  expect(executed).toBe("debug.continue");
  expect(resolved[0].conflicts).toEqual([]);
  const rider = resolveActions(actions, normalizeKeymap({ profile: "rider-macos" }), "macos");
  const riderEvent = { ...event, key: "F9" } as KeyboardEvent;
  expect(firstActionFor(rider, riderEvent, "macos", { profile: "rider-macos", debugPaused: true })?.id).toBe("debug.continue");
});
