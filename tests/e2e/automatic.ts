import { expect, type Page } from "@playwright/test";
import { host } from "./environment";

// Trust restores and evaluates by itself (docs/decisions.md); specs wait for that instead of clicking Restore.
export async function settleRestore(page: Page) {
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
    timeout: 60_000,
  });
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });
}

// Project details live in the Solution ⋮ menu; the inspector itself is unchanged.
export async function openProjectDetails(page: Page) {
  await page.getByTestId("tree-more").click();
  await page.getByTestId("tree-more-details").click();
}

// Automatic restore and evaluation add revisions on their own, so waits are "at least revision N".
export async function revisionOf(page: Page) {
  const text = (await page.getByTestId("eval-state").textContent()) ?? "";
  return Number(/revision (\d+)/.exec(text)?.[1] ?? 0);
}

async function discoverySnapshot(page: Page) {
  const response = await page.request.post(`${host}/call`, {
    data: { method: "workspace.state", params: {} },
  });
  const body = await response.json();
  const data = body.data;
  const discovery = data.tests.discovery as { revision: number; state: string };
  // A workspace with no evaluated test project never discovers: the automatic discovery is only
  // started when exactly one test project is available. Fixtures like CrashApp are settled from
  // the start, so the poll must not wait for a revision that will never come.
  const evaluation = data.evaluation as { state: string; projects: { kind: string; state: string }[] };
  const evaluated = ["ok", "partial"].includes(evaluation.state);
  const testProject = (evaluation.projects ?? []).some(
    (project) => project.kind === "test" && project.state === "ok",
  );
  return { ...discovery, evaluated, testProject };
}

/** Waits for the automatic discovery to finish: it builds the test project, so a build started
 * while it runs can collide on the same output file (#251). A workspace with no test project
 * never discovers; it is settled as soon as the evaluation says so. */
export async function settleDiscovery(page: Page) {
  await expect
    .poll(
      async () => {
        const discovery = await discoverySnapshot(page);
        if (!discovery.evaluated) return `evaluating:${discovery.state}`;
        if (!discovery.testProject) return "settled";
        return discovery.revision >= 1 && discovery.state !== "discovering"
          ? "settled"
          : `${discovery.state}:${discovery.revision}`;
      },
      { timeout: 180_000 },
    )
    .toBe("settled");
  // A discovery that failed is the app's own automation failing: say so here instead of letting
  // the build run over a state the spec never checked.
  const { state } = await discoverySnapshot(page);
  expect(state, "the automatic test discovery did not finish").not.toMatch(
    /^(failed|cancelled)$/,
  );
}

/** Clicks Discover and waits for the new host revision to settle: an automatic discovery may
 * already be current, so the old "Discovered" text is not proof that this click finished. */
export async function clickDiscover(page: Page) {
  // A source write can trigger an automatic re-evaluation; discovery over the stale model would
  // return the old test list at the same revision. Wait for the model before clicking.
  await expect(page.getByTestId("eval-state")).not.toContainText("Evaluating");
  await expect(page.getByTestId("eval-state")).not.toContainText("not current");
  await expect
    .poll(async () => (await discoverySnapshot(page)).state, { timeout: 180_000 })
    .not.toBe("discovering");
  await expect(page.getByTestId("tests-discover")).toBeEnabled({ timeout: 120_000 });
  const before = (await discoverySnapshot(page)).revision;
  await page.getByTestId("tests-discover").click();
  await expect
    .poll(
      async () => {
        const discovery = await discoverySnapshot(page);
        return discovery.revision > before && discovery.state !== "discovering"
          ? "settled"
          : `${discovery.state}:${discovery.revision}`;
      },
      { timeout: 180_000 },
    )
    .toBe("settled");
}
