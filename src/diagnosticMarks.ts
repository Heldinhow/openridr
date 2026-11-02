import type { Build, Diagnostics, Evaluation, Tests } from "./host/client";

/** A local edit newer than the publication makes its items awaiting before the next poll. */
export function withLocalEdits(
  diagnostics: Diagnostics,
  editedAt: Record<string, number>,
) {
  return diagnostics.snapshots.map((snapshot) =>
    snapshot.state === "current" &&
    snapshot.path !== null &&
    (editedAt[snapshot.path] ?? 0) > snapshot.receivedAt
      ? {
          ...snapshot,
          state: "awaiting" as const,
          staleReason:
            "The buffer was edited after this publication; awaiting diagnostics for the new revision.",
        }
      : snapshot,
  );
}

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

export type ProblemCounts = { errors: number; warnings: number; total: number };

/** One count for every origin: LSP, build, test discovery and failed evaluations. */
export function problemCounts(
  diagnostics: Diagnostics,
  build: Build,
  tests: Tests,
  evaluation: Evaluation,
  buildObsolete: boolean,
  editedAt: Record<string, number>,
): ProblemCounts {
  const severities = collectDiagnostics(diagnostics, build, buildObsolete, editedAt).map(
    (item) => item.severity,
  );
  // A stale or sealed discovery describes an older model; its problems are not current.
  if (!tests.discovery.sealed && !tests.discovery.stale) {
    severities.push(...tests.discovery.problems.map((problem) => problem.severityLabel));
  }
  const failedProjects = evaluation.projects.filter(
    (project) => project.state === "failed" && project.error,
  ).length;
  const errors = severities.filter((severity) => severity === "error").length + failedProjects;
  const warnings = severities.filter((severity) => severity === "warning").length;
  return { errors, warnings, total: errors + warnings };
}
