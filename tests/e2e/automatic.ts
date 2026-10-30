import { expect, type Page } from "@playwright/test";

// Trust restores and evaluates by itself (docs/decisions.md); specs wait for that instead of clicking Restore.
export async function settleRestore(page: Page) {
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
    timeout: 60_000,
  });
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });
}

// Automatic restore and evaluation add revisions on their own, so waits are "at least revision N".
export async function revisionOf(page: Page) {
  const text = (await page.getByTestId("eval-state").textContent()) ?? "";
  return Number(/revision (\d+)/.exec(text)?.[1] ?? 0);
}
