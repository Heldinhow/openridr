import { expect, test, type Locator, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

// Text of the Build/Run/Debug gates in src/executionGates.ts, asserted at real seams.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(180_000);

async function openTrustedRestored(page: Page) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-gates-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-checkbox").check();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", { timeout: 60_000 });
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });
  return base;
}

async function openRestrictedWorkspace(page: Page) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-d13-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("open-text-only").click();
  await expect(page.getByTestId("mode-badge")).toHaveText("Restricted mode");
  await expect(page.getByTestId("tree")).toBeVisible();
  return base;
}

const hostCallPattern = "**/__host/call";

/** Delay the real trust.grant bridge call so the UI's pending state is observable. */
async function delayNextTrustGrant(page: Page) {
  let release!: () => void;
  const blocked = new Promise<void>((resolve) => {
    release = resolve;
  });
  let finish!: () => void;
  const finished = new Promise<void>((resolve) => {
    finish = resolve;
  });
  let delayed = false;
  let released = false;
  await page.route(hostCallPattern, async (route) => {
    const body = route.request().postDataJSON() as { method?: string } | null;
    if (!delayed && body?.method === "trust.grant") {
      delayed = true;
      await blocked;
      try {
        await route.continue();
      } finally {
        finish();
      }
      return;
    }
    await route.continue();
  });
  return async () => {
    if (released) return;
    released = true;
    release();
    if (delayed) await finished;
    await page.unroute(hostCallPattern);
  };
}

/** Reject only the next trust.grant at the bridge boundary; all other calls stay real. */
async function rejectNextTrustGrant(page: Page) {
  let rejected = false;
  await page.route(hostCallPattern, async (route) => {
    const body = route.request().postDataJSON() as { method?: string } | null;
    if (!rejected && body?.method === "trust.grant") {
      rejected = true;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          ok: false,
          error: {
            code: "injected_failure",
            message: "Injected trust failure (UI boundary).",
          },
        }),
      });
      return;
    }
    await route.continue();
  });
  return async () => {
    await page.unroute(hostCallPattern);
  };
}

/** A point inside the target that is currently covered by the Trust popover. */
async function coveredTrustPoint(page: Page, target: Locator) {
  const popup = page.getByTestId("trust-banner");
  const popupBox = await popup.boundingBox();
  const targetBox = await target.boundingBox();
  if (!popupBox || !targetBox) throw new Error("The popup or target has no measurable box.");
  expect(
    popupBox.x < targetBox.x + targetBox.width &&
      popupBox.x + popupBox.width > targetBox.x &&
      popupBox.y < targetBox.y + targetBox.height &&
      popupBox.y + popupBox.height > targetBox.y,
  ).toBe(true);
  for (const xFraction of [0.3, 0.5, 0.7, 0.9]) {
    for (const yFraction of [0.3, 0.5, 0.7, 0.9]) {
      const point = {
        x: targetBox.x + targetBox.width * xFraction,
        y: targetBox.y + targetBox.height * yFraction,
      };
      const hit = await target.evaluate((element, { x, y }) => {
        const current = document.elementFromPoint(x, y);
        return {
          popup: Boolean(current?.closest('[data-testid="trust-banner"]')),
          target: Boolean(current && (current === element || element.contains(current))),
        };
      }, point);
      if (hit.popup && !hit.target) return point;
    }
  }
  throw new Error("No point of the target is covered by the Trust popover.");
}

test("Run is gated until the startup project is built", async ({ page }) => {
  const base = await openTrustedRestored(page);
  try {
    await page.getByTestId("bottom-tab-build").click();
    await expect(page.getByTestId("run-disabled-reason")).toContainText(
      "Build the startup project first; Run never builds or restores implicitly.",
    );
    await expect(page.getByTestId("run-action")).toBeDisabled();
    await page.getByTestId("build-action").click();
    await expect(page.getByTestId("build-state")).toContainText("Build succeeded", { timeout: 60_000 });
    await expect(page.getByTestId("run-action")).toBeEnabled();
    await page.getByTestId("build-configuration").selectOption("Release");
    // Switching configuration re-evaluates the model, which stales the saved build.
    await expect(page.getByTestId("run-disabled-reason")).toContainText(
      "The saved build is stale: The project model was re-evaluated",
      { timeout: 30_000 },
    );
    await expect(page.getByTestId("run-action")).toBeDisabled();
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("Build and Run name Restricted mode as the reason", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(fixture, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("bottom-tab-build").click();
  await expect(page.getByTestId("build-disabled-reason")).toContainText(
    "Trust the workspace before building; nothing runs in Restricted mode.",
  );
  await expect(page.getByTestId("run-disabled-reason")).toContainText(
    "Trust the workspace before running; nothing runs in Restricted mode.",
  );
});

// D-10: the Trust popover closes once Revoke completes, so it never covers the tab strip.
test("Revoke closes the Trust popover and leaves the tabs clickable", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(fixture, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-checkbox").check();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs").click();
  await expect(page.getByTestId("editor")).toBeVisible();
  await page.getByTestId("mode-badge").click();
  await page.getByTestId("trust-revoke").click();
  await expect(page.getByTestId("mode-badge")).toHaveText("Restricted mode");
  await expect(page.getByTestId("trust-banner")).toBeHidden();
  await expect(page.getByTestId("mode-badge")).toBeFocused();
  // The tab close button is reachable by a real click (nothing overlays the tab strip).
  await page.locator(".tab.active").getByTestId("close-buffer").click({ timeout: 5_000 });
  await expect(page.getByTestId("tab-src/OrderDesk.Core/Order.cs")).toHaveCount(0);
});

const trustViewports = [
  { name: "1440x900", width: 1440, height: 900 },
  { name: "1100x700", width: 1100, height: 700 },
];

for (const viewport of trustViewports) {
  test(`D-13 grant stays open until host success and closes without blocking the tree at ${viewport.name}`, async ({ page }) => {
    const base = await openRestrictedWorkspace(page);
    const tab = page.getByTestId("tab-src/OrderDesk.Core/Order.cs");
    const target = tab.getByTestId("close-buffer");
    try {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs").click();
      await expect(page.getByTestId("editor")).toBeVisible();
      await page.getByTestId("mode-badge").click();
      await expect(page.getByTestId("trust-banner")).toBeVisible();
      await page.getByTestId("trust-checkbox").check();
      // Checking the box is not a successful grant: the popover must stay open.
      await expect(page.getByTestId("trust-banner")).toBeVisible();
      const point = await coveredTrustPoint(page, target);

      const release = await delayNextTrustGrant(page);
      try {
        await page.getByTestId("trust-grant").click();
        await expect(page.getByTestId("trust-grant")).toHaveText("Trusting…");
        await expect(page.getByTestId("trust-grant")).toBeDisabled();
        await expect(page.getByTestId("trust-checkbox")).toBeDisabled();
        await expect(page.getByTestId("trust-banner")).toBeVisible();
        await expect(page.getByTestId("mode-badge")).toHaveText("Restricted mode");
        await expect(page.getByTestId("mode-trusted")).toHaveCount(0);
        const stillCovered = await target.evaluate((element, { x, y }) => {
          const current = document.elementFromPoint(x, y);
          return {
            popup: Boolean(current?.closest('[data-testid="trust-banner"]')),
            target: Boolean(current && (current === element || element.contains(current))),
          };
        }, point);
        expect(stillCovered).toEqual({ popup: true, target: false });
        await release();
        await expect(page.getByTestId("trust-banner")).toBeHidden();
        await expect(page.getByTestId("mode-badge")).toHaveText("Trusted mode");
        await expect(page.getByTestId("mode-badge")).toBeFocused();
      } finally {
        await release();
      }

      const nowReachable = await target.evaluate((element) => {
        const box = element.getBoundingClientRect();
        const current = document.elementFromPoint(
          box.left + box.width / 2,
          box.top + box.height / 2,
        );
        return {
          popup: Boolean(current?.closest('[data-testid="trust-banner"]')),
          target: Boolean(current && (current === element || element.contains(current))),
        };
      });
      expect(nowReachable).toEqual({ popup: false, target: true });
      await target.click();
      await expect(tab).toHaveCount(0);
    } finally {
      fs.rmSync(base, { recursive: true, force: true });
    }
  });
}

// The rejection is injected at the UI bridge boundary, not a real host failure. The retry below uses the real host.
test("D-13 rejected grant keeps the popover open with an error and retry succeeds", async ({ page }) => {
  const base = await openRestrictedWorkspace(page);
  try {
    await page.getByTestId("mode-badge").click();
    await page.getByTestId("trust-checkbox").check();
    const stopRejecting = await rejectNextTrustGrant(page);
    try {
      await page.getByTestId("trust-grant").click();
      await expect(page.getByTestId("trust-error")).toContainText(
        "Injected trust failure (UI boundary).",
      );
      await expect(page.getByTestId("trust-banner")).toBeVisible();
      await expect(page.getByTestId("mode-badge")).toHaveText("Restricted mode");
      await expect(page.getByTestId("trust-grant")).toHaveText("Trust and inspect project");
      await expect(page.getByTestId("trust-grant")).toBeEnabled();
      await expect(page.getByTestId("trust-checkbox")).toBeEnabled();
      await expect(page.getByTestId("trust-checkbox")).toBeChecked();
    } finally {
      await stopRejecting();
    }

    await page.getByTestId("trust-grant").click();
    await expect(page.getByTestId("trust-banner")).toBeHidden();
    await expect(page.getByTestId("mode-badge")).toHaveText("Trusted mode");
    await expect(page.getByTestId("mode-badge")).toBeFocused();
    await expect(page.getByTestId("trust-error")).toHaveCount(0);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
