import { createAudit, PLAIN } from "./lib.mjs";

const a = await createAudit("e");
const { page, shot, step, boot, finish } = a;

await step("90 symbols retry", async () => {
  await boot({ target: PLAIN, trust: true, evaluate: true, restore: true, lsp: true });
  await a.closeTools();
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await page.waitForTimeout(6000);
  await page.keyboard.press("Meta+t");
  await page.getByTestId("find-action").waitFor();
  await page.getByTestId("search-tab-symbols").click();
  await page.getByTestId("find-action-input").fill("Order");
  await page.waitForTimeout(5000);
  await shot("90-search-symbols-retry", "Symbols tab after waiting for csharp-ls indexing");
  const items = await page.locator('[data-testid^="search-item-"]').count();
  console.log("symbol results:", items);
});

await step("91 classes retry", async () => {
  await page.getByTestId("search-tab-classes").click();
  await page.getByTestId("find-action-input").fill("Order");
  await page.waitForTimeout(4000);
  await shot("91-search-classes-retry", "Classes tab after waiting");
  const items = await page.locator('[data-testid^="search-item-"]').count();
  console.log("class results:", items);
});

await step("92 all retry", async () => {
  await page.getByTestId("search-tab-all").click();
  await page.getByTestId("find-action-input").fill("Order");
  await page.waitForTimeout(4000);
  await shot("92-search-all-retry", "All tab after waiting");
  const items = await page.locator('[data-testid^="search-item-"]').count();
  console.log("all results:", items);
  await page.keyboard.press("Escape");
});

await finish("e");
