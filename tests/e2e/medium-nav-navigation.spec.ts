import { expect, test, type Page } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";

const fixture = path.resolve("fixtures/OrderDesk");
const program = "src/OrderDesk.Cli/Program.cs";
const order = "src/OrderDesk.Core/Order.cs";
const orderTests = "tests/OrderDesk.Tests/OrderTests.cs";

async function openTrusted(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(fixture, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("tree-expand-all").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("Back and Forward traverse file opens by button and shortcut", async ({ page }) => {
  await openTrusted(page);
  await page.getByTestId(`tree-file-${program}`).click();
  await expect(page.getByTestId(`tab-${program}`)).toHaveClass(/active/);
  await page.getByTestId(`tree-file-${order}`).click();
  await expect(page.getByTestId(`tab-${order}`)).toHaveClass(/active/);

  const back = page.getByTestId("nav-back");
  const forward = page.getByTestId("nav-forward");
  await expect(back).toBeEnabled();
  await expect(forward).toBeDisabled();

  await page.keyboard.press("Meta+[");
  await expect(page.getByTestId(`tab-${program}`)).toHaveClass(/active/);
  await expect(forward).toBeEnabled();

  await page.keyboard.press("Meta+]");
  await expect(page.getByTestId(`tab-${order}`)).toHaveClass(/active/);

  // Rider's second history binding.
  await page.keyboard.press("Meta+Alt+ArrowLeft");
  await expect(page.getByTestId(`tab-${program}`)).toHaveClass(/active/);
  await page.keyboard.press("Meta+Alt+ArrowRight");
  await expect(page.getByTestId(`tab-${order}`)).toHaveClass(/active/);

  // The toolbar buttons run the same actions.
  await back.click();
  await expect(page.getByTestId(`tab-${program}`)).toHaveClass(/active/);
  await forward.click();
  await expect(page.getByTestId(`tab-${order}`)).toHaveClass(/active/);

  // A new navigation after going back discards the forward branch.
  await back.click();
  await expect(page.getByTestId(`tab-${program}`)).toHaveClass(/active/);
  await page.getByTestId(`tree-file-${orderTests}`).click();
  await expect(page.getByTestId(`tab-${orderTests}`)).toHaveClass(/active/);
  await expect(forward).toBeDisabled();
});

test("Go to Line opens from the editor and from outside it", async ({ page }) => {
  await openTrusted(page);
  await page.getByTestId(`tree-file-${program}`).click();
  await page.getByTestId("editor").click();

  const dialog = page.locator(".cm-goto-line");
  const input = dialog.locator('input[name="line"]');
  await page.keyboard.press("Meta+g");
  await expect(dialog).toBeVisible();
  await expect(input).toBeFocused();
  await input.fill("3");
  await input.press("Enter");
  await expect(page.getByTestId("status-cursor")).toHaveText("Ln 3, Col 1");
  await expect(page.getByTestId("editor-shell")).toHaveAttribute("data-cursor", "2:0");

  await page.keyboard.press("Meta+l");
  await expect(dialog).toBeVisible();
  await input.fill("4");
  await input.press("Enter");
  await expect(page.getByTestId("status-cursor")).toHaveText("Ln 4, Col 1");

  // Focus outside the editor: the action focuses the editor before opening the dialog.
  await page.getByTestId(`tab-${program}`).getByRole("tab").click();
  await page.keyboard.press("Meta+g");
  await expect(dialog).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
});

test("Go to Definition records history and Back returns to the previous position", async ({
  page,
  request,
}) => {
  test.setTimeout(180_000);
  await openTrusted(page);
  // tools.plan is inert. Only an already present launcher is reused; nothing is installed here.
  const planned = await (
    await request.post(`${host}/call`, {
      data: { method: "tools.plan", params: { tool: "csharp-ls" } },
    })
  ).json();
  const existing = planned.data.tools["csharp-ls"].plan.existing.state;
  test.skip(
    existing !== "present",
    `csharp-ls is not installed in this E2E home (existing: ${existing}); not verified, nothing is installed by this spec`,
  );
  const reused = await (
    await request.post(`${host}/call`, {
      data: {
        method: "tools.install",
        params: { planId: planned.data.tools["csharp-ls"].plan.planId, confirm: true },
      },
    })
  ).json();
  expect(reused.data.tools["csharp-ls"].reused, "the present launcher is reused, not reinstalled").toBe(true);
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 120_000 });
  // "partial" still answers definitions for the projects it loaded; only an idle server cannot.
  await expect
    .poll(
      async () => {
        const body = await (
          await request.post(`${host}/call`, {
            data: { method: "workspace.state", params: {} },
          })
        ).json();
        return body.data.lsp.state;
      },
      { timeout: 90_000 },
    )
    .toMatch(/^(ready|partial)$/);

  await page.getByTestId(`tree-file-${program}`).click();
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+ArrowUp");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Home");
  for (let index = 0; index < 20; index += 1) await page.keyboard.press("ArrowRight");
  await expect(page.getByTestId("editor-shell")).toHaveAttribute("data-cursor", "2:20");

  await page.keyboard.press("Meta+b");
  await expect(page.getByTestId(`tab-${order}`)).toHaveClass(/active/, { timeout: 30_000 });
  await expect(page.getByTestId("editor")).toContainText("public sealed class Order");

  await page.keyboard.press("Meta+[");
  await expect(page.getByTestId(`tab-${program}`)).toHaveClass(/active/);
  await expect(page.getByTestId("editor-shell")).toHaveAttribute("data-cursor", "2:20");
});
