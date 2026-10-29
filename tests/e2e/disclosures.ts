import { expect, type Locator } from "@playwright/test";

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
