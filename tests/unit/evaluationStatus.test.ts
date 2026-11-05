import { describe, expect, test } from "bun:test";
import type { Snapshot } from "../../src/host/client";
import {
  evaluationStateLabel,
  evaluationStatusView,
  partialWorkspaceStatus,
} from "../../src/evaluationStatus";

function evaluation(overrides: Partial<Snapshot["evaluation"]> = {}): Snapshot["evaluation"] {
  return {
    state: "idle",
    context: { configuration: "Debug", targetFramework: "net8.0" },
    configurations: ["Debug"],
    availableTfms: ["net8.0"],
    projects: [],
    log: "",
    stale: false,
    staleReason: null,
    revision: 0,
    workspaceId: 1,
    contextId: 1,
    startedAt: null,
    finishedAt: null,
    ...overrides,
  };
}

describe("evaluationStatusView", () => {
  test("idle is the text outline, not an evaluation", () => {
    expect(evaluationStatusView(evaluation()).text).toBe("Text outline · not evaluated");
  });

  test("running is visible before the first project exists", () => {
    const view = evaluationStatusView(
      evaluation({ state: "running", context: { configuration: "Debug", targetFramework: null } }),
    );
    expect(view.text.startsWith("Evaluating…")).toBe(true);
    expect(view.text.includes("Text outline")).toBe(false);
    expect(view.tone).toBeUndefined();
  });

  test("failed and partial keep the red cue", () => {
    expect(evaluationStatusView(evaluation({ state: "failed" })).text.startsWith("Evaluation failed")).toBe(true);
    expect(evaluationStatusView(evaluation({ state: "failed" })).tone).toBe("error");
    expect(evaluationStatusView(evaluation({ state: "partial" })).text.startsWith("Evaluated with errors")).toBe(true);
    expect(evaluationStatusView(evaluation({ state: "partial" })).tone).toBe("error");
  });

  test("stale is written before the revision so the ellipsis cannot cut it", () => {
    const view = evaluationStatusView(evaluation({ state: "ok", stale: true, revision: 4 }));
    expect(view.text.startsWith("Evaluated · not current · Configuration Debug")).toBe(true);
    expect(view.text.indexOf("not current") < view.text.indexOf("revision 4")).toBe(true);
    expect(view.title.endsWith("revision 4 · not current")).toBe(true);
  });

  test("labels cover every state", () => {
    expect(evaluationStateLabel("running")).toBe("Evaluating…");
    expect(evaluationStateLabel("ok")).toBe("Evaluated");
    expect(evaluationStateLabel("partial")).toBe("Evaluated with errors");
    expect(evaluationStateLabel("failed")).toBe("Evaluation failed");
    expect(evaluationStateLabel("idle")).toBe("Not evaluated");
  });
});

describe("partialWorkspaceStatus", () => {
  test("a clean model has no warning", () => {
    expect(
      partialWorkspaceStatus([
        { state: "ok", restored: "restored" },
        { state: "ok", restored: "restored" },
      ]),
    ).toBe(null);
    expect(partialWorkspaceStatus([])).toBe(null);
  });

  test("failed or unrestored projects produce the discreet status warning", () => {
    const partial = partialWorkspaceStatus([
      { state: "ok", restored: "restored" },
      { state: "failed", restored: "restored" },
      { state: "ok", restored: "not_restored" },
    ]);
    expect(partial?.text).toBe("Partial workspace · 2/3");
    expect(partial?.title).toBe(
      "Partial workspace · 2 of 3 projects have incomplete context. This is not a clean solution.",
    );
  });
});
