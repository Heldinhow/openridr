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
