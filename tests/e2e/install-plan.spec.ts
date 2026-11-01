import { expect, test } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";

// Ticket 10: the install plan is readable at 1440x900 and scrolls visibly at 1100x700.
const solution = path.resolve(process.cwd(), "fixtures/OrderDesk/OrderDesk.sln");

for (const viewport of [{ width: 1440, height: 900 }, { width: 1100, height: 700 }]) {
  test(`install plan layout at ${viewport.width}x${viewport.height}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(solution);
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-restricted").click();
    await page.getByTestId("workspace-tools-toggle").click();
    await page.getByTestId("install-plan-open").click();
    const plan = page.getByTestId("install-plan");
    await expect(plan).toBeVisible();
    // The plan lives in the Workspace tools drawer, which is the scroll container at every size.
    const drawer = page.getByTestId("workspace-tools");
    // Every material field is on screen at the same time as the confirm button.
    const ids = ["install-command", "install-destination", "install-staging", "install-integrity", "install-permissions"];
    const confirm = page.getByTestId("install-confirm");
    await expect(confirm).toBeVisible();
    const view = await drawer.evaluate((element) => ({
      scrolls: element.scrollHeight > element.clientHeight,
      overflowY: getComputedStyle(element).overflowY,
      top: element.getBoundingClientRect().top,
      bottom: element.getBoundingClientRect().bottom,
    }));
    const planBox = (await plan.boundingBox())!;
    if (viewport.height >= 900) {
      // 1440x900: the opened plan sits inside the drawer's visible area, no scrolling needed to read it.
      expect(planBox.y, "plan starts inside the drawer").toBeGreaterThanOrEqual(view.top - 1);
      for (const id of [...ids, "install-confirm"]) {
        const box = (await page.getByTestId(id).boundingBox())!;
        expect(box.y + box.height, `${id} is visible without scrolling`).toBeLessThanOrEqual(view.bottom);
      }
    } else {
      // 1100x700: the drawer scrolls through a visible, non-hidden container.
      expect(view.scrolls && view.overflowY !== "hidden", "the drawer scrolls visibly").toBe(true);
    }
    await page.screenshot({ path: `docs/evidence/screenshots/10-install-plan-${viewport.width}x${viewport.height}.png` });
    // The plan never scrolls inside itself: its rows read in the drawer's own scroll.
    expect(await plan.evaluate((element) => element.scrollHeight <= element.clientHeight)).toBe(true);
    // Confirm stays under the material facts it confirms.
    const confirmBox = (await confirm.boundingBox())!;
    for (const id of ids) {
      const box = (await page.getByTestId(id).boundingBox())!;
      expect(confirmBox.y, `confirm sits below ${id}`).toBeGreaterThan(box.y);
    }
  });
}

// Trusted first-install plan: Confirm and install is enabled and sits with the material fields. Nothing is clicked.
test("trusted first-install plan shows an active Confirm and install at 1440x900", async ({ page, request }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("workspace-tools-toggle").click();
  const state = await (await request.post(`${host}/call`, { data: { method: "workspace.state", params: {} } })).json();
  test.skip(state.data.tools["csharp-ls"].state === "ready", "csharp-ls already installed in this E2E home; run with a fresh OPENRIDR_E2E_HOME");
  await page.getByTestId("install-plan-open").click();
  const confirm = page.getByTestId("install-confirm");
  await expect(confirm).toBeVisible();
  await expect(confirm).toBeEnabled();
  await expect(confirm).toHaveText("Confirm and install");
  const drawer = await page.getByTestId("workspace-tools").boundingBox();
  const box = (await confirm.boundingBox())!;
  expect(box.y + box.height).toBeLessThanOrEqual(drawer!.y + drawer!.height);
  for (const id of ["install-command", "install-destination", "install-staging", "install-integrity", "install-permissions"]) {
    const field = (await page.getByTestId(id).boundingBox())!;
    expect(field.y + field.height, `${id} is visible with the active confirm`).toBeLessThanOrEqual(drawer!.y + drawer!.height);
    expect(box.y).toBeGreaterThan(field.y);
  }
  await page.screenshot({ path: "docs/evidence/screenshots/10-install-confirm-active-1440x900.png" });
});
