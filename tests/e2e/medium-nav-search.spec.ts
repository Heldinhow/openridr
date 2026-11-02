import { expect, test, type Page } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";

const fixture = path.resolve("fixtures/OrderDesk");
const solution = path.join(fixture, "OrderDesk.sln");

async function openRestricted(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-restricted").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

/** Ticket 03 removed ⌘T from the Rider profile; two clean Shift taps are the entry point. */
async function openSearch(page: Page) {
  await page.keyboard.press("Shift");
  await page.keyboard.press("Shift");
  await expect(page.getByTestId("find-action")).toBeVisible();
}

async function openAnalyzed(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  // tools.plan is inert. Only an already present launcher is reused; nothing is installed here.
  const planned = await (
    await page.request.post(`${host}/call`, {
      data: { method: "tools.plan", params: { tool: "csharp-ls" } },
    })
  ).json();
  const existing = planned.data.tools["csharp-ls"].plan.existing.state;
  test.skip(
    existing !== "present",
    `csharp-ls is not installed in this E2E home (existing: ${existing}); not verified, nothing is installed by this spec`,
  );
  const reused = await (
    await page.request.post(`${host}/call`, {
      data: {
        method: "tools.install",
        params: { planId: planned.data.tools["csharp-ls"].plan.planId, confirm: true },
      },
    })
  ).json();
  expect(reused.data.tools["csharp-ls"].reused, "the present launcher is reused").toBe(true);
  await expect
    .poll(
      async () => {
        const body = await (
          await page.request.post(`${host}/call`, {
            data: { method: "workspace.state", params: {} },
          })
        ).json();
        return body.data.lsp.state;
      },
      { timeout: 90_000 },
    )
    .toMatch(/^(ready|partial)$/);
}

function resultItems(page: Page) {
  return page.locator('[data-testid^="search-item-"]');
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("Text lists one entry per file and line", async ({ page }) => {
  await openRestricted(page);
  await openSearch(page);
  await page.getByTestId("search-tab-text").click();
  await page.getByTestId("find-action-input").fill("Order");

  await expect.poll(async () => resultItems(page).count()).toBeGreaterThan(0);
  const details = await resultItems(page).locator(".action-group").allTextContents();
  expect(details.length).toBeGreaterThan(0);
  expect(new Set(details).size, `duplicate path:line entries in ${JSON.stringify(details)}`).toBe(
    details.length,
  );
});

test("All ranks the exact file name first and keeps text out of the top", async ({ page }) => {
  await openRestricted(page);
  await openSearch(page);
  await expect(page.getByTestId("search-tab-all")).toHaveAttribute("aria-selected", "true");
  await page.getByTestId("find-action-input").fill("Order");

  await expect
    .poll(async () => resultItems(page).locator(".search-kind").allTextContents())
    .toContain("text");
  const titles = await resultItems(page).locator(".action-title").allTextContents();
  const kinds = await resultItems(page).locator(".search-kind").allTextContents();
  expect(titles[0]).toBe("Order.cs");
  expect(kinds[0]).toBe("file");
  const exact = titles.indexOf("Order.cs");
  const prefix = titles.indexOf("OrderDesk.sln");
  expect(prefix).toBeGreaterThan(exact);
  expect(kinds.indexOf("text")).toBeGreaterThan(prefix);
});

test("Symbols lists types and members with class/symbol labels; Classes keeps types", async ({
  page,
}) => {
  test.setTimeout(180_000);
  await openAnalyzed(page);
  await openSearch(page);
  await page.getByTestId("search-tab-symbols").click();
  const input = page.getByTestId("find-action-input");
  await input.fill("Order");

  const classItem = resultItems(page).filter({
    has: page.locator(".action-title", { hasText: /^Order$/ }),
  });
  await expect(classItem.first()).toBeVisible({ timeout: 60_000 });
  await expect(classItem.first().locator(".search-kind")).toHaveText("class");
  await expect(page.getByTestId("search-empty")).toHaveCount(0);
  await expect(page.getByTestId("search-suggest-classes")).toHaveCount(0);

  await page.getByTestId("search-tab-classes").click();
  await expect(classItem.first()).toBeVisible({ timeout: 60_000 });
  await expect(classItem.first().locator(".search-kind")).toHaveText("class");

  await page.getByTestId("search-tab-symbols").click();
  await input.fill("Id");
  // Wait for the debounced query to replace the previous "Order" results, then every hit is a member.
  await expect
    .poll(async () => {
      const kinds = await resultItems(page).locator(".search-kind").allTextContents();
      return kinds.length > 0 && kinds.every((kind) => kind === "symbol");
    })
    .toBe(true);
});

test("Reopening comes back to All with the previous query selected", async ({ page }) => {
  await openRestricted(page);
  await openSearch(page);
  await expect(page.getByTestId("search-tab-all")).toHaveAttribute("aria-selected", "true");
  const input = page.getByTestId("find-action-input");
  await input.fill("Order");
  await page.getByTestId("search-tab-text").click();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("find-action")).toHaveCount(0);

  await openSearch(page);
  await expect(page.getByTestId("search-tab-all")).toHaveAttribute("aria-selected", "true");
  await expect(input).toBeFocused();
  await expect(input).toHaveValue("Order");
  expect(
    await input.evaluate((el) => {
      const field = el as HTMLInputElement;
      return [field.selectionStart, field.selectionEnd];
    }),
  ).toEqual([0, 5]);
  await page.keyboard.type("x");
  await expect(input).toHaveValue("x");
});
