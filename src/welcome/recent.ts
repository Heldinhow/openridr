import { call } from "../host/client";

export type Decision = "trusted" | "restricted" | null;

// One `recent.list` entry; `openedAt` is epoch ms and absent for entries migrated from the old list format.
export type RecentProject = { path: string; openedAt?: number; decision: Decision };

export async function fetchRecent(): Promise<RecentProject[]> {
  return (await call<{ projects: RecentProject[] }>("recent.list")).projects;
}

export function projectName(path: string) {
  return (path.split(/[\\/]/).pop() ?? path).replace(/\.(sln|csproj)$/i, "");
}

// The browser cannot read the home folder, so the usual per-user roots stand in for it.
export function shortPath(path: string) {
  return path.replace(/^(\/Users\/|\/home\/|[A-Za-z]:\\Users\\)[^/\\]+/, "~");
}

const units: [Intl.RelativeTimeFormatUnit, number][] = [
  ["day", 86_400_000],
  ["hour", 3_600_000],
  ["minute", 60_000],
];
const relative = new Intl.RelativeTimeFormat("en", { numeric: "auto" });

export function lastOpened(openedAt: number, now = Date.now()) {
  const elapsed = now - openedAt;
  if (elapsed > 30 * 86_400_000) return new Date(openedAt).toLocaleDateString("en", { dateStyle: "medium" });
  for (const [unit, size] of units) {
    if (elapsed >= size) return relative.format(-Math.floor(elapsed / size), unit);
  }
  return "just now";
}
