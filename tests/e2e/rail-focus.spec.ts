import { expect, test, type Page } from "@playwright/test";
import path from "node:path";

const solution = path.resolve(process.cwd(), "fixtures/OrderDesk/OrderDesk.sln");

// #333: the rail is 34 px wide and scrolls, so the global 2 px ring at offset 2 was clipped on
// both sides. The ring is inset instead: still 2 px, entirely inside the button and the rail.
type Ring = { offset: string; width: string; style: string; inside: boolean; label: string | null };

// Measure while the button is focused: :focus-visible only paints on the active element.
async function railRingsByTab(page: Page) {
  await page.locator("body").click({ position: { x: 600, y: 300 } });
  const seen = new Map<string, Ring>();
  for (let i = 0; i < 120; i++) {
    await page.keyboard.press("Tab");
    const ring = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      const id = el?.getAttribute("data-testid");
      if (!el || !id || !(id.startsWith("toolband-") || id === "workspace-tools-toggle")) return null;
      const rail = el.closest(".toolrail") as HTMLElement;
      const s = getComputedStyle(el);
      const r = el.getBoundingClientRect();
      const b = rail.getBoundingClientRect();
      return {
        id,
        offset: s.outlineOffset,
        width: s.outlineWidth,
        style: s.outlineStyle,
        inside: r.left >= b.left && r.right <= b.right && r.top >= b.top && r.bottom <= b.bottom,
        label: el.getAttribute("aria-label"),
      };
    });
    if (ring) {
      const { id, ...rest } = ring;
      seen.set(id, rest);
    }
  }
  return seen;
}

test("every rail item reached by Tab has an inset ring inside the rail at 1440 and 1100", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  const trust = page.getByTestId("trust-prompt-trust");
  await trust.waitFor({ state: "visible", timeout: 60_000 }).catch(() => undefined);
  if (await trust.isVisible().catch(() => false)) await trust.click();
  await page.getByTestId("tool-windows").waitFor({ state: "visible", timeout: 90_000 });

  for (const viewport of [{ width: 1440, height: 900 }, { width: 1100, height: 760 }]) {
    await page.setViewportSize(viewport);
    const rings = await railRingsByTab(page);
    expect(rings.size, `rail items reached at ${viewport.width}`).toBeGreaterThanOrEqual(11);

    for (const [id, ring] of rings) {
      expect(ring.style, `${id} at ${viewport.width}`).toBe("solid");
      expect(ring.width, `${id} at ${viewport.width}`).toBe("2px");
      expect(ring.offset, `${id} at ${viewport.width}`).toBe("-2px");
      expect(ring.inside, `${id} at ${viewport.width}`).toBe(true);
      expect(ring.label, `${id} at ${viewport.width}`).toBeTruthy();
    }
  }

  // Every icon-only button in the rail carries an accessible name, including the disabled ones.
  const unnamed = await page.evaluate(() =>
    [...document.querySelectorAll(".toolrail button")]
      .filter((el) => !el.textContent?.trim() && !el.getAttribute("aria-label"))
      .map((el) => el.getAttribute("data-testid") ?? el.textContent),
  );
  expect(unnamed).toEqual([]);
});
