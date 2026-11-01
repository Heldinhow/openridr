import { expect, type Locator, type Page } from "@playwright/test";

// Exercise the visible disclosure controls before interacting with their content.
export async function reveal(locator: Locator) {
  await expect(locator).toBeAttached();
  for (const details of await locator.locator("xpath=ancestor::details").all()) {
    if (await details.getAttribute("open") === null) {
      await details.locator(":scope > summary").click();
    }
    await expect(details).toHaveAttribute("open", "");
  }
}

// The status-bar Trust popover is not a <details>; open it from the status-mode button when closed.
export async function openTrustPopover(page: Page) {
  const popover = page.getByTestId("status-mode-popover");
  if (!(await popover.isVisible())) await page.getByTestId("status-mode").click();
  await expect(popover).toBeVisible();
}
