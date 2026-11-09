import { expect, test, type Page } from "@playwright/test";
import path from "node:path";

const solution = path.resolve(process.cwd(), "fixtures/OrderDesk/OrderDesk.sln");

// #333: the rail is 34 px wide and scrolls, so the global 2 px ring at offset 2 was clipped on
// both sides. The ring is inset instead: still 2 px, entirely inside the button.
async function focusFirstRailButton(page: Page) {
  await page.locator("body").click({ position: { x: 600, y: 300 } });
  for (let i = 0; i < 80; i++) {
    await page.keyboard.press("Tab");
    const id = await page.evaluate(() => document.activeElement?.getAttribute("data-testid"));
    if (id?.startsWith("toolband-")) return id;
  }
  throw new Error("Tab never reached a tool band button");
}

test("rail focus ring is inset and complete at 1440 and 1100", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  const trust = page.getByTestId("trust-prompt-trust");
  await trust.waitFor({ state: "visible", timeout: 60_000 }).catch(() => undefined);
  if (await trust.isVisible().catch(() => false)) await trust.click();
  await page.getByTestId("tool-windows").waitFor({ state: "visible", timeout: 90_000 });

  for (const viewport of [{ width: 1440, height: 900 }, { width: 1100, height: 760 }]) {
    await page.setViewportSize(viewport);
    const id = await focusFirstRailButton(page);
    const ring = await page.evaluate((id) => {
      const el = document.querySelector(`[data-testid="${id}"]`) as HTMLElement;
      const rail = el.closest(".toolrail") as HTMLElement;
      const s = getComputedStyle(el);
      const r = el.getBoundingClientRect();
      const b = rail.getBoundingClientRect();
      return {
        offset: s.outlineOffset,
        width: s.outlineWidth,
        style: s.outlineStyle,
        inside: r.left >= b.left && r.right <= b.right,
        named: Boolean(el.getAttribute("aria-label") || el.textContent?.trim()),
      };
    }, id);
    expect(ring.style, `${id} at ${viewport.width}`).toBe("solid");
    expect(ring.width, `${id} at ${viewport.width}`).toBe("2px");
    expect(ring.offset, `${id} at ${viewport.width}`).toBe("-2px");
    expect(ring.inside, `${id} at ${viewport.width}`).toBe(true);
    expect(ring.named, `${id} at ${viewport.width}`).toBe(true);
  }
});
