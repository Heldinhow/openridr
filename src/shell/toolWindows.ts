import type { IconName } from "../Icon";

export type ToolWindowSlot = "left" | "bottom";
export type BottomToolId = "build" | "problems" | "debug" | "terminal" | "tests";
export type ToolWindowId = "solution" | BottomToolId;

export type ToolWindow = {
  id: ToolWindowId;
  title: string;
  slot: ToolWindowSlot;
  icon: IconName;
};

// Registro único: faixas, abas e slots renderizam a partir daqui (F1).
export const toolWindows: ToolWindow[] = [
  { id: "solution", title: "Solution Explorer", slot: "left", icon: "project" },
  { id: "build", title: "Build", slot: "bottom", icon: "build" },
  { id: "problems", title: "Problems", slot: "bottom", icon: "problems" },
  { id: "debug", title: "Debug", slot: "bottom", icon: "debug" },
  { id: "terminal", title: "Terminal", slot: "bottom", icon: "terminal" },
  { id: "tests", title: "Tests", slot: "bottom", icon: "tests" },
];

export function toolWindowsIn(slot: ToolWindowSlot): ToolWindow[] {
  return toolWindows.filter((tool) => tool.slot === slot);
}

export type SlotLayout = { active: ToolWindowId; size: number; open: boolean };
export type ShellLayout = { left: SlotLayout; bottom: SlotLayout };

export const shellLayoutKey = "openridr.layout.v1";
export const leftSizeRange = { min: 220, max: 420 };
export const bottomSizeRange = { min: 120, max: 500 };

export const defaultShellLayout: ShellLayout = {
  left: { active: "solution", size: 280, open: true },
  bottom: { active: "build", size: 220, open: true },
};

function clampSize(value: unknown, range: { min: number; max: number }, fallback: number) {
  if (typeof value !== "number" || !Number.isFinite(value)) return fallback;
  return Math.min(range.max, Math.max(range.min, Math.round(value)));
}

function readSlot(raw: unknown, slot: ToolWindowSlot, fallback: SlotLayout): SlotLayout {
  const value = typeof raw === "object" && raw !== null ? (raw as Record<string, unknown>) : {};
  const ids = toolWindowsIn(slot).map((tool) => tool.id);
  const active = typeof value.active === "string" && ids.includes(value.active as ToolWindowId)
    ? (value.active as ToolWindowId)
    : fallback.active;
  const range = slot === "left" ? leftSizeRange : bottomSizeRange;
  return {
    active,
    size: clampSize(value.size, range, fallback.size),
    open: typeof value.open === "boolean" ? value.open : fallback.open,
  };
}

export function loadShellLayout(): ShellLayout {
  try {
    const raw = globalThis.localStorage?.getItem(shellLayoutKey);
    if (!raw) return defaultShellLayout;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    return {
      left: readSlot(parsed?.left, "left", defaultShellLayout.left),
      bottom: readSlot(parsed?.bottom, "bottom", defaultShellLayout.bottom),
    };
  } catch {
    return defaultShellLayout;
  }
}

export function saveShellLayout(layout: ShellLayout) {
  try {
    globalThis.localStorage?.setItem(shellLayoutKey, JSON.stringify(layout));
  } catch {
    // Private mode or a full quota only loses the layout; the app keeps working.
  }
}
