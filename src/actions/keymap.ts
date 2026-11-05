// Keymap model: shortcuts are key + modifiers, never a preformatted string. `primary` is
// Command on macOS and Control elsewhere, matching the approved Rider/IntelliJ table.

export type ProfileId = "rider-macos" | "intellij";

export type Shortcut = {
  key: string;
  primary?: boolean;
  control?: boolean;
  alt?: boolean;
  shift?: boolean;
};

type Platform = string;

export type Binding = { mac: Shortcut[]; windows: Shortcut[] };
export type Keymap = { profile: ProfileId; overrides: Record<string, Shortcut[]> };

export type Density = "comfortable" | "compact";
export type AppearanceSettings = { zoom: number; density: Density };
export type EditorSettings = {
  fontFamily: string;
  fontSize: number;
  indentSize: number;
  useTabs: boolean;
};
export type AppSettings = {
  keymap: Keymap;
  appearance: AppearanceSettings;
  editor: EditorSettings;
  [key: string]: unknown;
};

export const ZOOM_MIN = 80;
export const ZOOM_MAX = 150;
export const FONT_SIZE_MIN = 8;
export const FONT_SIZE_MAX = 32;
export const INDENT_MIN = 1;
export const INDENT_MAX = 12;

export const densityOptions: { id: Density; title: string }[] = [
  { id: "comfortable", title: "Comfortable" },
  { id: "compact", title: "Compact" },
];

export const fontFamilyOptions: { id: string; title: string }[] = [
  { id: "ui-monospace", title: "System Mono" },
  { id: "Menlo", title: "Menlo" },
  { id: "SF Mono", title: "SF Mono" },
  { id: "JetBrains Mono", title: "JetBrains Mono" },
  { id: "Courier New", title: "Courier New" },
];

export const appearanceDefaults: AppearanceSettings = {
  zoom: 100,
  density: "comfortable",
};
export const editorDefaults: EditorSettings = {
  fontFamily: "JetBrains Mono",
  fontSize: 13,
  indentSize: 4,
  useTabs: false,
};

export const profileOptions: { id: ProfileId; title: string }[] = [
  { id: "rider-macos", title: "Rider (macOS)" },
  { id: "intellij", title: "IntelliJ" },
];

export function profileTitle(profile: ProfileId) {
  return profileOptions.find((option) => option.id === profile)?.title ?? "Rider (macOS)";
}

export function defaultSettings(): AppSettings {
  return {
    keymap: { profile: "rider-macos", overrides: {} },
    appearance: { ...appearanceDefaults },
    editor: { ...editorDefaults },
  };
}

const names: Record<string, string> = {
  enter: "↩",
  escape: "Esc",
  arrowup: "↑",
  arrowdown: "↓",
  arrowleft: "←",
  arrowright: "→",
  backspace: "⌫",
  delete: "⌦",
  tab: "Tab",
  space: "Space",
  "/": "/",
  ",": ",",
};

const textNames: Record<string, string> = {
  enter: "Enter",
  escape: "Esc",
  arrowup: "Up",
  arrowdown: "Down",
  arrowleft: "Left",
  arrowright: "Right",
  backspace: "Backspace",
  delete: "Delete",
  tab: "Tab",
  space: "Space",
};

export function normalizeKey(raw: string): string {
  if (raw === " " || raw === "Spacebar") return "space";
  return raw.toLowerCase();
}

export function shortcut(key: string, mods: Omit<Shortcut, "key"> = {}): Shortcut {
  return { key: normalizeKey(key), ...mods };
}

function binding(mac: Shortcut[], windows: Shortcut[] = mac): Binding {
  return { mac, windows };
}

function both(shortcuts: Shortcut[]): Record<ProfileId, Binding> {
  return { "rider-macos": binding(shortcuts), intellij: binding(shortcuts) };
}

// Approved table in SPEC-MVP-RIDER "Ações e keymap". Actions without an entry have no
// default shortcut; every shortcut still comes from here and can be remapped.
export const keymapDefaults: Record<string, Partial<Record<ProfileId, Binding>>> = {
  "view.findAction": both([shortcut("a", { primary: true, shift: true })]),
  "settings.open": both([shortcut(",", { primary: true })]),
  "file.save": both([shortcut("s", { primary: true })]),
  "editor.closeTab": both([shortcut("w", { primary: true })]),
  "editor.nextTab": both([
    shortcut("]", { primary: true, shift: true }),
    shortcut("tab", { control: true }),
  ]),
  "editor.previousTab": both([
    shortcut("[", { primary: true, shift: true }),
    shortcut("tab", { control: true, shift: true }),
  ]),
  "toolWindow.hideActive": both([shortcut("escape", { shift: true })]),
  // ⌘1..⌘8 follow the band order in `shell/toolWindows.ts`; Git and NuGet stay unmapped.
  "toolWindow.solution": both([shortcut("1", { primary: true })]),
  "toolWindow.commit": both([shortcut("2", { primary: true })]),
  "toolWindow.build": both([shortcut("3", { primary: true })]),
  "toolWindow.run": both([shortcut("4", { primary: true })]),
  "toolWindow.problems": both([shortcut("5", { primary: true })]),
  "toolWindow.debug": both([shortcut("6", { primary: true })]),
  "toolWindow.terminal": both([shortcut("7", { primary: true })]),
  "toolWindow.tests": both([shortcut("8", { primary: true })]),
  "build.start": both([shortcut("f9", { primary: true })]),
  "build.solution": both([shortcut("f9", { primary: true, shift: true })]),
  "run.start": {
    "rider-macos": binding(
      [shortcut("f5", { control: true }), shortcut("r", { control: true })],
      [shortcut("f10", { shift: true })],
    ),
    intellij: binding(
      [shortcut("f5", { control: true }), shortcut("r", { control: true })],
      [shortcut("f10", { control: true, shift: true })],
    ),
  },
  "run.stop": both([shortcut("f5", { shift: true })]),
  "debug.start": {
    "rider-macos": binding(
      [shortcut("f5", { alt: true }), shortcut("d", { control: true })],
      [shortcut("f9", { shift: true })],
    ),
    intellij: binding(
      [shortcut("f5", { alt: true }), shortcut("d", { control: true })],
      [shortcut("f9", { shift: true })],
    ),
  },
  "debug.stepOver": both([shortcut("f8")]),
  "debug.continue": both([shortcut("f9")]),
  "debug.stop": both([shortcut("f2", { primary: true })]),
  "debug.stepOut": both([shortcut("f8", { shift: true })]),
  "debug.runToCursor": both([shortcut("f9", { alt: true })]),
  "debug.stepInto": both([shortcut("f7")]),
  "debug.toggleBreakpoint": both([shortcut("f8", { primary: true })]),
  "code.goToDeclaration": {
    "rider-macos": binding([shortcut("f12"), shortcut("b", { primary: true })]),
    intellij: binding([shortcut("b", { primary: true })]),
  },
  "code.findUsages": {
    "rider-macos": binding([shortcut("f12", { shift: true })]),
    intellij: binding([shortcut("f7", { alt: true })]),
  },
  "code.contextActions": both([shortcut("enter", { alt: true })]),
  "code.rename": both([shortcut("f6", { shift: true })]),
  // Double Shift is a gesture handled in WorkspaceView; ⌘T is the remappable binding.
  "search.everywhere": both([shortcut("t", { primary: true })]),
  "search.file": {
    "rider-macos": binding([shortcut("o", { primary: true, shift: true })]),
    intellij: binding([shortcut("o", { primary: true })]),
  },
  "search.recentFiles": both([shortcut("e", { primary: true })]),
  "search.symbol": {
    "rider-macos": binding([shortcut("t", { primary: true, alt: true, shift: true })]),
    intellij: binding([shortcut("o", { primary: true, alt: true })]),
  },
  "search.text": {
    "rider-macos": binding([shortcut("e", { primary: true, alt: true, shift: true })]),
    // Find in Files owns ⇧⌘F in the IntelliJ profile; Find in Path stays palette-only.
    intellij: binding([]),
  },
  "search.findInFiles": both([shortcut("f", { primary: true, shift: true })]),
  "search.line": {
    "rider-macos": binding([shortcut("g", { primary: true }), shortcut("l", { primary: true })]),
    intellij: binding([shortcut("l", { primary: true })]),
  },
  // Rider history navigation; the IntelliJ profile keeps its own table untouched.
  "navigate.back": {
    "rider-macos": binding([
      shortcut("[", { primary: true }),
      shortcut("arrowleft", { primary: true, alt: true }),
    ]),
    intellij: binding([]),
  },
  "navigate.forward": {
    "rider-macos": binding([
      shortcut("]", { primary: true }),
      shortcut("arrowright", { primary: true, alt: true }),
    ]),
    intellij: binding([]),
  },
  // Rider uses the approved two-key chords (⌘U then the key); only the IntelliJ profile has
  // single keys for the same actions. Chords are not remappable yet.
  "tests.runAll": {
    "rider-macos": binding([]),
    intellij: binding([shortcut("f10", { shift: true })], [shortcut("f10", { control: true, shift: true })]),
  },
  "tests.runSelected": {
    "rider-macos": binding([]),
    intellij: binding(
      [shortcut("f10", { control: true, shift: true })],
      [shortcut("f10", { alt: true, shift: true })],
    ),
  },
  "tests.debugSelected": {
    "rider-macos": binding([]),
    intellij: binding([shortcut("f9", { shift: true })], [shortcut("f9", { control: true, shift: true })]),
  },
  "tests.rerunFailed": {
    "rider-macos": binding([]),
    intellij: binding(
      [shortcut("r", { alt: true, shift: true })],
      [shortcut("r", { control: true, alt: true })],
    ),
  },
  "tests.quickList": both([shortcut("u", { alt: true, shift: true })]),
  "git.commit": {
    "rider-macos": binding([shortcut("k", { primary: true, alt: true, shift: true })]),
    intellij: binding([shortcut("k", { primary: true })]),
  },
  "git.push": both([shortcut("k", { primary: true, shift: true })]),
  "git.update": both([shortcut("w", { primary: true, alt: true })]),
  "git.branchCreate": both([shortcut("n", { primary: true, alt: true })]),
};

export function shortcutsFor(
  id: string,
  keymap: Keymap,
  platform: Platform,
): Shortcut[] {
  const override = keymap.overrides[id];
  if (override && override.length > 0) return override;
  const defaults = keymapDefaults[id];
  const selected = defaults?.[keymap.profile] ?? defaults?.["rider-macos"];
  if (!selected) return [];
  return platform === "macos" ? selected.mac : selected.windows;
}

/** Approved Rider chords: ⌘U then R (run selected), D (debug), L (run all), F (rerun failed).
 * The IntelliJ profile has no chords and uses each action's own key instead. */
const chordTable: Partial<
  Record<ProfileId, { prefix: Shortcut; keys: Record<string, string> }>
> = {
  "rider-macos": {
    prefix: shortcut("u", { primary: true }),
    keys: {
      r: "tests.runSelected",
      d: "tests.debugSelected",
      l: "tests.runAll",
      f: "tests.rerunFailed",
    },
  },
};

export function chordFor(id: string, profile: ProfileId): Shortcut[] {
  const table = chordTable[profile];
  if (!table) return [];
  const key = Object.keys(table.keys).find((candidate) => table.keys[candidate] === id);
  return key ? [table.prefix, shortcut(key)] : [];
}

export function chordLabelsFor(id: string, profile: ProfileId, platform: Platform): string[] {
  return chordFor(id, profile).map((item) => formatShortcut(item, platform));
}

export function chordPrefixFor(profile: ProfileId): Shortcut | null {
  return chordTable[profile]?.prefix ?? null;
}

export function chordTarget(profile: ProfileId, key: string): string | null {
  return chordTable[profile]?.keys[normalizeKey(key)] ?? null;
}

function keyLabel(key: string): string {
  return names[key] ?? key.toUpperCase();
}

export function formatShortcut(shortcut: Shortcut, platform: Platform): string {
  const key = keyLabel(shortcut.key);
  if (platform === "macos") {
    return `${shortcut.control ? "⌃" : ""}${shortcut.alt ? "⌥" : ""}${
      shortcut.shift ? "⇧" : ""
    }${shortcut.primary ? "⌘" : ""}${key}`;
  }
  const parts: string[] = [];
  if (shortcut.primary || shortcut.control) parts.push("Ctrl");
  if (shortcut.alt) parts.push("Alt");
  if (shortcut.shift) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

/** Plain text for tooltips, where symbols alone are ambiguous. */
export function formatShortcutText(shortcut: Shortcut, platform: Platform): string {
  const key = textNames[shortcut.key] ?? shortcut.key.toUpperCase();
  const parts: string[] = [];
  if (shortcut.primary) parts.push(platform === "macos" ? "Cmd" : "Ctrl");
  if (shortcut.control) parts.push("Ctrl");
  if (shortcut.alt) parts.push(platform === "macos" ? "Option" : "Alt");
  if (shortcut.shift) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

export function matches(event: KeyboardEvent, shortcut: Shortcut, platform: Platform) {
  if (normalizeKey(event.key) !== shortcut.key) return false;
  if (platform === "macos") {
    return (
      event.metaKey === !!shortcut.primary &&
      event.ctrlKey === !!shortcut.control &&
      event.altKey === !!shortcut.alt &&
      event.shiftKey === !!shortcut.shift
    );
  }
  return (
    event.ctrlKey === !!(shortcut.primary || shortcut.control) &&
    event.altKey === !!shortcut.alt &&
    event.shiftKey === !!shortcut.shift
  );
}

export function shortcutFromEvent(event: KeyboardEvent, platform: Platform): Shortcut | null {
  if (["Shift", "Control", "Alt", "Meta", "CapsLock"].includes(event.key)) return null;
  return {
    key: normalizeKey(event.key),
    primary: platform === "macos" ? event.metaKey : event.ctrlKey,
    control: platform === "macos" ? event.ctrlKey : false,
    alt: event.altKey,
    shift: event.shiftKey,
  };
}

export function signature(shortcut: Shortcut, platform: Platform): string {
  const meta = platform === "macos" && !!shortcut.primary;
  const ctrl =
    platform === "macos" ? !!shortcut.control : !!(shortcut.primary || shortcut.control);
  return `${shortcut.key}|${meta}|${ctrl}|${!!shortcut.alt}|${!!shortcut.shift}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parseShortcut(value: unknown): Shortcut | null {
  if (!isRecord(value)) return null;
  if (typeof value.key !== "string" || value.key.length === 0 || value.key.length > 24) {
    return null;
  }
  for (const flag of ["primary", "control", "alt", "shift"]) {
    if (value[flag] !== undefined && typeof value[flag] !== "boolean") return null;
  }
  return {
    key: normalizeKey(value.key),
    primary: value.primary === true,
    control: value.control === true,
    alt: value.alt === true,
    shift: value.shift === true,
  };
}

export function normalizeKeymap(raw: unknown): Keymap {
  const value = isRecord(raw) ? raw : {};
  const profile: ProfileId = value.profile === "intellij" ? "intellij" : "rider-macos";
  const overrides: Record<string, Shortcut[]> = {};
  if (isRecord(value.overrides)) {
    for (const [id, list] of Object.entries(value.overrides)) {
      if (!Array.isArray(list) || list.length === 0 || list.length > 4) continue;
      const parsed = list.map(parseShortcut);
      if (parsed.every((item): item is Shortcut => item !== null)) {
        overrides[id] = parsed;
      }
    }
  }
  return { profile, overrides };
}

function boundedInteger(value: unknown, min: number, max: number, fallback: number): number {
  return typeof value === "number" && Number.isInteger(value) && value >= min && value <= max
    ? value
    : fallback;
}

export function normalizeAppearance(raw: unknown): AppearanceSettings {
  const value = isRecord(raw) ? raw : {};
  return {
    zoom: boundedInteger(value.zoom, ZOOM_MIN, ZOOM_MAX, appearanceDefaults.zoom),
    density: value.density === "compact" ? "compact" : "comfortable",
  };
}

export function normalizeEditor(raw: unknown): EditorSettings {
  const value = isRecord(raw) ? raw : {};
  return {
    fontFamily:
      typeof value.fontFamily === "string" &&
      value.fontFamily.length > 0 &&
      value.fontFamily.length <= 80
        ? value.fontFamily
        : editorDefaults.fontFamily,
    fontSize: boundedInteger(value.fontSize, FONT_SIZE_MIN, FONT_SIZE_MAX, editorDefaults.fontSize),
    indentSize: boundedInteger(value.indentSize, INDENT_MIN, INDENT_MAX, editorDefaults.indentSize),
    useTabs: value.useTabs === true,
  };
}

export function normalizeSettings(raw: unknown): AppSettings {
  const base = isRecord(raw) ? raw : {};
  return {
    ...base,
    keymap: normalizeKeymap(base.keymap),
    appearance: normalizeAppearance(base.appearance),
    editor: normalizeEditor(base.editor),
  };
}
