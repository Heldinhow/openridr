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
  return body.data.tests.discovery as { revision: number; state: string };
}

/** Clicks Discover and waits for the new host revision to settle: an automatic discovery may
 * already be current, so the old "Discovered" text is not proof that this click finished. */
export async function clickDiscover(page: Page) {
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
