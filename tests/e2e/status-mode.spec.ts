import { expect, test } from "@playwright/test";
import path from "node:path";

async function openRestricted(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.resolve("fixtures/OrderDesk/OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-restricted").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");
}

test("the top bar has no trust indicator; the status bar owns the mode", async ({ page }) => {
  await openRestricted(page);
  for (const id of ["mode-badge", "trust-banner", "trust-prompt", "trust-prompt-open", "trust-prompt-dismiss"]) {
    await expect(page.getByTestId(id)).toHaveCount(0);
  }
  await expect(page.locator(".toolbar").getByText(/Restricted mode|Trusted mode/)).toHaveCount(0);
  await expect(page.locator(".status").getByTestId("status-mode")).toBeVisible();
});

test("status-mode toggles Restricted → Trusted → Restricted through its popover", async ({ page }) => {
  await openRestricted(page);
  const mode = page.getByTestId("status-mode");
  const popover = page.getByTestId("status-mode-popover");
  await expect(mode).toHaveAttribute("aria-haspopup", "dialog");
  await expect(mode).toHaveAttribute("aria-expanded", "false");

  await mode.click();
  await expect(mode).toHaveAttribute("aria-expanded", "true");
  await expect(popover).toContainText("Project code will not run");
  await page.getByTestId("trust-checkbox").check();
  await page.getByTestId("trust-grant").click();
  await expect(popover).toBeHidden();
  await expect(mode).toHaveText("Trusted mode");
  await expect(mode).toBeFocused();

  await mode.click();
  await page.getByTestId("trust-revoke").click();
  await expect(popover).toBeHidden();
  await expect(mode).toHaveText("Restricted mode");
  await expect(mode).toBeFocused();
});

test("Escape closes the popover and returns focus to status-mode", async ({ page }) => {
  await openRestricted(page);
  const mode = page.getByTestId("status-mode");
  await mode.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("status-mode-popover")).toBeVisible();
  await page.getByTestId("trust-checkbox").focus();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("status-mode-popover")).toBeHidden();
  await expect(mode).toHaveAttribute("aria-expanded", "false");
  await expect(mode).toBeFocused();
  await expect(mode).toHaveText("Restricted mode");
});
