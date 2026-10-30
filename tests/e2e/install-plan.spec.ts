import { expect, test } from "@playwright/test";
import path from "node:path";

// Ticket 10: the install plan is readable at 1440x900 and scrolls visibly at 1100x700.
const solution = path.resolve(process.cwd(), "fixtures/OrderDesk/OrderDesk.sln");

for (const viewport of [{ width: 1440, height: 900 }, { width: 1100, height: 700 }]) {
  test(`install plan layout at ${viewport.width}x${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(solution);
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("open-text-only").click();
    await page.getByTestId("workspace-tools-toggle").click();
    await page.getByTestId("install-plan-open").click();
    const plan = page.getByTestId("install-plan");
    await expect(plan).toBeVisible();
    // The plan lives in the Workspace tools drawer, which is the scroll container at every size.
    const drawer = page.getByTestId("workspace-tools");
    const drawerScrolls = await drawer.evaluate((element) => {
      element.scrollTop = 0;
      return element.scrollHeight > element.clientHeight && getComputedStyle(element).overflowY !== "hidden";
    });
    expect(drawerScrolls, "the drawer scrolls with a visible scroll container").toBe(true);
    // The plan never scrolls inside itself: its rows read in the drawer's own scroll.
    expect(await plan.evaluate((element) => element.scrollHeight <= element.clientHeight)).toBe(true);
    // Confirm stays under the material facts it confirms.
    const confirm = page.getByTestId("install-confirm");
    if (await confirm.count()) {
      const confirmBox = (await confirm.boundingBox())!;
      const destinationBox = (await page.getByTestId("install-destination").boundingBox())!;
      expect(confirmBox.y).toBeGreaterThan(destinationBox.y);
    }
  });
}
