import { createAudit, PLAIN } from "./lib.mjs";

const a = await createAudit("f");
const { page, shot, step, boot, bottom, finish } = a;

await step("95 build failure problems", async () => {
  await boot({ target: PLAIN, trust: true, evaluate: true, restore: true, lsp: true });
  await a.closeTools();
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+A");
  await page.keyboard.insertText('using OrderDesk.Core;\n\nvar order = new Orderr("A-104", 42m);\nConsole.WriteLine($"Order {order.Id}: {order.Total:C}");\n');
  await page.keyboard.press("Meta+s");
  await page.waitForTimeout(1500);
  await bottom("build");
  await page.getByTestId("build-action").click();
  await page.getByTestId("build-state").filter({ hasText: "failed" }).first().waitFor({ timeout: 180_000 });
  await shot("95-build-failed", "Build failed with a real compiler error");
});

await step("96 problems with error", async () => {
  await bottom("problems");
  await page.waitForTimeout(1200);
  await shot("96-problems-error", "Problems with a navigable compiler error");
  const row = page.locator('[data-testid="problems-list"] button').first();
  if (await row.count()) {
    await row.click();
    await page.waitForTimeout(600);
    await shot("97-problem-selected", "Selected problem with details");
    const nav = page.getByTestId("problem-navigate");
    if (await nav.count()) {
      await nav.click();
      await page.waitForTimeout(500);
      await shot("98-problem-navigated", "Problem navigated in the editor");
    }
  }
});

await finish("f");
