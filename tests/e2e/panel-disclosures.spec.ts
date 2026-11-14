import { expect, test } from "@playwright/test";
import path from "node:path";

const solution = path.resolve(process.cwd(), "fixtures/OrderDesk/OrderDesk.sln");

// #331: secondary panel controls live behind one labeled disclosure. The summary is the control:
// it is named, it takes focus, and Enter/Space opens the group without removing any data-testid.
test("panel disclosures keep secondary controls behind a keyboard-reachable summary", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  const trust = page.getByTestId("trust-prompt-trust");
  await trust.waitFor({ state: "visible", timeout: 60000 }).catch(() => undefined);
  if (await trust.isVisible().catch(() => false)) await trust.click();
  await page.getByTestId("tool-windows").waitFor({ state: "visible", timeout: 60000 });

  await page.getByTestId("toolband-tests").click();
  const testsSummary = page.getByTestId("tests-overflow").locator("summary");
  await expect(testsSummary).toHaveAttribute("aria-label", "More test actions");
  await expect(page.getByTestId("tests-expand-all")).toBeHidden();
  // #372: the counters and the filter moved inside the disclosure, so the summary is the last
  // control of the primary row. Tab reaches it from the controls before it, whichever of them is
  // enabled in this state (Run and Discover are gated until the model is evaluated).
  await expect(page.getByTestId("tests-filter")).toBeHidden();
  await page.getByTestId("tests-tfm").focus();
  for (let step = 0; step < 5; step += 1) {
    if (await testsSummary.evaluate((element) => element === document.activeElement)) break;
    await page.keyboard.press("Tab");
  }
  await expect(testsSummary).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("tests-expand-all")).toBeVisible();
  await expect(page.getByTestId("tests-export")).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("tests-expand-all")).toBeHidden();

  await page.getByTestId("toolband-nuget").click();
  const nugetSummary = page.getByTestId("nuget-filters").locator("summary");
  await expect(nugetSummary).toHaveAttribute("aria-label", "Package filters");
  await expect(page.getByTestId("nuget-scope")).toBeHidden();
  // The listing loads on mount and disables the project select while it does; focus needs it enabled.
  await expect(page.getByTestId("nuget-project")).toBeEnabled({ timeout: 60000 });
  await page.getByTestId("nuget-project").focus();
  await page.keyboard.press("Tab");
  await expect(nugetSummary).toBeFocused();
  await page.keyboard.press("Space");
  await expect(page.getByTestId("nuget-scope")).toBeVisible();
  await expect(page.getByTestId("nuget-source-select")).toBeVisible();
  await expect(page.getByTestId("nuget-prerelease")).toBeVisible();
});
