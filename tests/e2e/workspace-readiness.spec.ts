import { expect, test, type Page } from "@playwright/test";
import path from "node:path";

const fixture = (rel: string) => path.resolve(process.cwd(), "fixtures", rel);

async function openWorkspace(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
  const trust = page.getByTestId("trust-prompt-trust");
  await trust.waitFor({ state: "visible", timeout: 60_000 }).catch(() => undefined);
  if (await trust.isVisible().catch(() => false)) await trust.click();
  await page.getByTestId("tool-windows").waitFor({ state: "visible", timeout: 90_000 });
}

// #330: the readiness band says one thing at a time. While restore/evaluation runs there is no
// recovery action; once it settles the action is there and enabled.
test("evaluation in flight shows one sentence and no recovery action", async ({ page }) => {
  await openWorkspace(page, fixture("OrderDesk/OrderDesk.sln"));
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 120_000 });
  await page.getByTestId("eval-run").click();
  await expect(page.getByTestId("eval-running")).toContainText("Finishing restore and evaluating projects");
  await expect(page.getByTestId("eval-stale-run")).toHaveCount(0);
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 120_000 });
  await expect(page.getByTestId("eval-run")).toBeEnabled();
  await expect(page.getByTestId("eval-blocked-reason")).toHaveCount(0);
});

// A blocked recovery action keeps its data-testid and shows the reason next to it.
test("blocked evaluation shows the reason next to the disabled action", async ({ page }) => {
  await openWorkspace(page, fixture("MissingSdk/MissingSdk.csproj"));
  await expect(page.getByTestId("eval-blocked-reason")).toContainText(
    "Trust the workspace and resolve the project SDK before evaluating.",
  );
  await expect(page.getByTestId("eval-run")).toBeDisabled();
  await page.getByTestId("toolband-problems").click();
  await expect(page.getByTestId("problems-partial")).toBeVisible();
  await expect(page.getByTestId("problems-restart-blocked")).toContainText("Resolve the project SDK before analysis.");
  await expect(page.getByTestId("problems-restart-analysis")).toBeDisabled();
});

// Partial coverage with analysis available offers the restart right next to the caveat.
test("partial coverage offers a working restart analysis", async ({ page }) => {
  await openWorkspace(page, fixture("BrokenImport/BrokenImport.csproj"));
  await page.getByTestId("toolband-problems").click();
  await expect(page.getByTestId("problems-partial")).toBeVisible({ timeout: 60_000 });
  await expect(page.getByTestId("problems-coverage")).toContainText("coverage partial");
  await expect(page.getByTestId("problems-restart-analysis")).toBeEnabled();
  await expect(page.getByTestId("problems-restart-blocked")).toHaveCount(0);
});
