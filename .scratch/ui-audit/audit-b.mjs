import { createAudit, PLAIN } from "./lib.mjs";

const a = await createAudit("b");
const { page, shot, step, click, waitText, boot, bottom, finish } = a;

await step("30 boot with analysis", async () => {
  await boot({ target: PLAIN, trust: true, evaluate: true, restore: true, lsp: true });
  await a.closeTools();
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await page.getByTestId("editor").waitFor();
  await page.waitForTimeout(800);
});

await step("31 search everywhere all", async () => {
  await page.keyboard.press("Meta+t");
  await page.getByTestId("find-action").waitFor();
  await page.getByTestId("find-action-input").fill("Order");
  await page.waitForTimeout(1200);
  await shot("31-search-all", "Search Everywhere, All tab");
});

await step("32 search files and preview", async () => {
  await page.getByTestId("search-tab-files").click();
  await page.getByTestId("find-action-input").fill("Program");
  await page.waitForTimeout(1200);
  await shot("32-search-files", "Search Everywhere, Files tab with preview");
});

await step("33 search symbols", async () => {
  await page.getByTestId("search-tab-symbols").click();
  await page.getByTestId("find-action-input").fill("Order");
  await page.waitForTimeout(1500);
  await shot("33-search-symbols", "Search Everywhere, Symbols tab");
});

await step("34 search text", async () => {
  await page.getByTestId("search-tab-text").click();
  await page.getByTestId("find-action-input").fill("Total");
  await page.waitForTimeout(1500);
  await shot("34-search-text", "Search Everywhere, Text tab with preview");
});

await step("35 search actions", async () => {
  await page.getByTestId("search-tab-actions").click();
  await page.getByTestId("find-action-input").fill("git");
  await page.waitForTimeout(800);
  await shot("35-search-actions", "Search Everywhere, Actions tab");
  await page.keyboard.press("Escape");
});

await step("36 find action disabled reason", async () => {
  await page.keyboard.press("Shift+Meta+a");
  await page.getByTestId("find-action").waitFor();
  await page.getByTestId("find-action-input").fill("git.commit");
  await page.waitForTimeout(800);
  await shot("36-find-action", "Find Action with unavailable action reason");
  await page.keyboard.press("Escape");
});

await step("37 workspace actions menu", async () => {
  await page.getByTestId("workspace-actions-menu").click();
  await shot("37-workspace-menu", "Workspace actions menu");
  await page.keyboard.press("Escape");
});

await step("38 run config dialog default", async () => {
  await page.getByTestId("toolbar-run-config").click();
  await page.getByTestId("run-config-edit").click();
  await page.getByTestId("run-config-dialog").waitFor();
  await shot("38-run-config-default", "Run configurations, derived default read-only");
});

await step("39 run config validation", async () => {
  await page.getByTestId("run-config-duplicate").click();
  await page.getByTestId("run-config-name").fill("");
  await page.getByTestId("run-config-save").click();
  await page.waitForTimeout(600);
  await shot("39-run-config-validation", "Run configuration field validation");
});

await step("40 run config saved", async () => {
  await page.getByTestId("run-config-name").fill("Audit run");
  await page.getByTestId("run-config-args").fill("--audit");
  await page.getByTestId("run-config-save").click();
  await page.getByTestId("run-config-dialog").waitFor({ state: "hidden" });
  await page.waitForTimeout(800);
  await shot("40-run-config-saved", "Saved run configuration selected in the toolbar");
});

await step("41 settings appearance", async () => {
  await page.keyboard.press("Meta+,");
  await page.getByTestId("settings-dialog").waitFor();
  await shot("41-settings-appearance", "Settings, Appearance page");
});

await step("42 settings editor", async () => {
  await page.getByTestId("settings-page-editor").click();
  await shot("42-settings-editor", "Settings, Editor page");
});

await step("43 settings build", async () => {
  await page.getByTestId("settings-page-build").click();
  await shot("43-settings-build", "Settings, Build page");
});

await step("44 settings keymap", async () => {
  await page.getByTestId("settings-page-keymap").click();
  await shot("44-settings-keymap", "Settings, Keymap page");
});

await step("45 keymap conflict", async () => {
  await page.getByTestId("keymap-change-run.start").click();
  await page.keyboard.press("Meta+F9");
  await page.waitForTimeout(800);
  await shot("45-keymap-conflict", "Keymap remap conflict warning");
  const reset = page.getByTestId("keymap-reset-run.start");
  if (await reset.count()) await reset.click().catch(() => {});
  await page.keyboard.press("Escape");
  await page.getByTestId("settings-cancel").click();
});

await step("46 search no results", async () => {
  await page.keyboard.press("Meta+t");
  await page.getByTestId("find-action").waitFor();
  await page.getByTestId("search-tab-files").click();
  await page.getByTestId("find-action-input").fill("zzzz-no-such-file");
  await page.waitForTimeout(1000);
  await shot("46-search-empty", "Search Everywhere empty state");
  await page.keyboard.press("Escape");
});

await finish("b");
