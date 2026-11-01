import { expect, test, type Locator, type Page } from "@playwright/test";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");

test.use({ viewport: { width: 1440, height: 900 } });

// Compares a disabled control with a live clone of itself made enabled in the same DOM context,
// so the check reads the real computed look that the user sees, not a hard-coded token value.
async function expectLooksDisabled(locator: Locator) {
  await expect(locator).toBeDisabled();
  const compared = await locator.evaluate((element) => {
    const enabled = element.cloneNode(true) as HTMLElement;
    enabled.removeAttribute("disabled");
    enabled.removeAttribute("aria-disabled");
    enabled.style.visibility = "hidden";
    enabled.style.position = "absolute";
    element.after(enabled);
    const look = (node: Element) => {
      const style = getComputedStyle(node);
      return {
        background: style.backgroundColor,
        color: style.color,
        border: style.borderColor,
        opacity: style.opacity,
        cursor: style.cursor,
      };
    };
    const result = { disabled: look(element), enabled: look(enabled) };
    enabled.remove();
    return result;
  });
  expect(compared.disabled.cursor).toBe("not-allowed");
  expect({ ...compared.disabled, cursor: "" }).not.toEqual({ ...compared.enabled, cursor: "" });
}

async function openWorkspace(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(fixture);
  await page.getByTestId("workspace-open").click();
}

test("disabled trust grant looks disabled and becomes enabled after the checkbox", async ({ page }) => {
  await openWorkspace(page);
  await page.getByTestId("trust-prompt-restricted").click();
  await page.getByTestId("mode-badge").click();
  await expectLooksDisabled(page.getByTestId("trust-grant"));

  await page.getByTestId("trust-checkbox").check();
  await expect(page.getByTestId("trust-grant")).toBeEnabled();
  // Buttons ease their background (120ms), so wait for the filled primary instead of reading mid-transition.
  await expect.poll(() => page.getByTestId("trust-grant").evaluate((button) => getComputedStyle(button).backgroundColor)).not.toBe("rgba(0, 0, 0, 0)");
});

test("disabled install confirmation looks disabled before trust", async ({ page }) => {
  await openWorkspace(page);
  await page.getByTestId("trust-prompt-restricted").click();
  await page.getByTestId("workspace-tools-toggle").click();
  await page.getByTestId("install-plan-open").click();
  await expect(page.getByTestId("install-plan")).toBeVisible();
  await expectLooksDisabled(page.getByTestId("install-confirm"));
});

test("disabled run configuration controls look disabled on the default configuration", async ({ page }) => {
  await openWorkspace(page);
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("mode-badge")).toHaveText("Trusted mode");
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });

  await page.getByTestId("toolbar-run-config").click();
  await page.getByTestId("run-config-edit").click();
  await expect(page.getByTestId("run-config-dialog")).toBeVisible();
  await expectLooksDisabled(page.getByTestId("run-config-save"));
  await expectLooksDisabled(page.getByTestId("run-config-run"));
  await expectLooksDisabled(page.getByTestId("run-config-name"));
  await expectLooksDisabled(page.getByTestId("run-config-configuration"));
});
