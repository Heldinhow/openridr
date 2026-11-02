import type { Build, Diagnostics } from "./host/client";
import { withLocalEdits } from "./ProblemsPanel";

// Editor-ready diagnostics: current LSP publications plus the current build's problems, one list.
export type EditorDiagnostic = {
  path: string;
  line: number;
  character: number;
  endLine: number | null;
  endCharacter: number | null;
  severity: "error" | "warning" | "information" | "hint";
  message: string;
  code: string | null;
};

export function samePath(a: string, b: string) {
  const x = a.replace(/\\/g, "/");
  const y = b.replace(/\\/g, "/");
  return x === y || x.endsWith(`/${y}`) || y.endsWith(`/${x}`);
}

type Range = { start?: { line: number; character: number }; end?: { line: number; character: number } } | null;

export function collectDiagnostics(
  diagnostics: Diagnostics,
  build: Build,
  buildObsolete: boolean,
  editedAt: Record<string, number>,
): EditorDiagnostic[] {
  const out: EditorDiagnostic[] = [];
  for (const snapshot of withLocalEdits(diagnostics, editedAt)) {
    if (snapshot.state !== "current" || !snapshot.path) continue;
    for (const item of snapshot.items) {
      if (!item.position || item.severityLabel === "unknown") continue;
      const range = item.range as Range;
      out.push({
        path: snapshot.path,
        line: item.position.line,
        character: item.position.character,
        endLine: range?.end?.line ?? null,
        endCharacter: range?.end?.character ?? null,
        severity: item.severityLabel,
        message: item.message,
        code: item.code === null ? null : String(item.code),
      });
    }
  }
  // An obsolete, stale or sealed build says nothing about the buffer as it is now.
  if (!build.sealed && !build.stale && !buildObsolete) {
    for (const problem of build.problems) {
      const { position } = problem;
      if (!position) continue;
      if (out.some((d) => samePath(d.path, problem.path) && d.line === position.line && d.code === problem.code)) continue;
      out.push({
        path: problem.path,
        line: position.line,
        character: position.character,
        endLine: problem.range?.end.line ?? null,
        endCharacter: problem.range?.end.character ?? null,
        severity: problem.severityLabel,
        message: problem.message,
        code: problem.code,
      });
    }
  }
  return out;
}
