import type { Snapshot } from "./host/client";

export type EvaluationState = Snapshot["evaluation"]["state"];

export function evaluationStateLabel(state: EvaluationState): string {
  switch (state) {
    case "running":
      return "Evaluating…";
    case "ok":
      return "Evaluated";
    case "partial":
      return "Evaluated with errors";
    case "failed":
      return "Evaluation failed";
    default:
      return "Not evaluated";
  }
}

export type EvaluationStatusView = {
  testId: "status-evaluation";
  title: string;
  text: string;
  tone?: "error";
};

/** One source of truth for the status bar; `running`/`failed` are visible before any project exists. */
export function evaluationStatusView(evaluation: Snapshot["evaluation"]): EvaluationStatusView {
  if (evaluation.state === "idle") {
    return {
      testId: "status-evaluation",
      title: "Text outline · not evaluated",
      text: "Text outline · not evaluated",
    };
  }
  const label = evaluationStateLabel(evaluation.state);
  const tfm = evaluation.context.targetFramework ?? "not selected";
  const stale = evaluation.stale ? " · not current" : "";
  return {
    testId: "status-evaluation",
    tone: evaluation.state === "failed" || evaluation.state === "partial" ? "error" : undefined,
    title: `${label} · ${evaluation.context.configuration} · TFM ${tfm} · revision ${evaluation.revision}${stale}`,
    // Stale comes first so the ellipsis cannot cut it before the revision.
    text: `${label}${stale} · Configuration ${evaluation.context.configuration} · TFM ${tfm} · revision ${evaluation.revision}`,
  };
}

export type PartialWorkspaceStatus = { text: string; title: string };

/** Restores the old `tree-partial` warning in the status bar instead of a footnote in the tree. */
export function partialWorkspaceStatus(
  projects: Array<{ state: "ok" | "failed"; restored: "restored" | "not_restored" | "unknown" }>,
): PartialWorkspaceStatus | null {
  if (projects.length === 0) return null;
  const incomplete = projects.filter(
    (project) => project.state === "failed" || project.restored !== "restored",
  ).length;
  if (incomplete === 0) return null;
  return {
    text: `Partial workspace · ${incomplete}/${projects.length}`,
    title: `Partial workspace · ${incomplete} of ${projects.length} projects have incomplete context. This is not a clean solution.`,
  };
}
