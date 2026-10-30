import { createAudit, PLAIN } from "./lib.mjs";

const a = await createAudit("d");
const { page, shot, step, click, waitText, boot, bottom, finish } = a;

await step("70 boot with analysis", async () => {
  await boot({ target: PLAIN, trust: true, evaluate: true, restore: true, lsp: true });
  await a.closeTools();
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await page.getByTestId("editor").waitFor();
  await page.waitForTimeout(800);
});

await step("71 editor context menu", async () => {
  await page.getByTestId("editor").click({ button: "right" });
  await page.getByTestId("editor-context-menu").waitFor();
  await shot("71-editor-context-menu", "Editor context menu");
  await page.keyboard.press("Escape");
});

async function cursorLine3Char20() {
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+ArrowUp");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Home");
  for (let i = 0; i < 20; i += 1) await page.keyboard.press("ArrowRight");
}

await step("72 code actions", async () => {
  await cursorLine3Char20();
  await page.keyboard.press("Alt+Enter");
  await page.waitForTimeout(1500);
  await shot("72-code-actions", "Context actions popup");
  await page.keyboard.press("Escape");
});

await step("73 rename inline", async () => {
  await cursorLine3Char20();
  await page.keyboard.press("Shift+F6");
  await page.waitForTimeout(600);
  await shot("73-rename-inline", "Inline rename field over the identifier");
  const input = page.getByTestId("rename-input");
  if (await input.count()) {
    await input.fill("order2");
    await page.getByTestId("rename-submit").click();
    await page.getByTestId("rename-preview").waitFor({ timeout: 30_000 });
    await shot("74-rename-preview", "Rename preview with per-file diffs");
    await page.getByTestId("rename-cancel-preview").click();
  }
});

await step("75 find usages", async () => {
  await cursorLine3Char20();
  await page.keyboard.press("Shift+F12");
  await page.waitForTimeout(1500);
  await shot("75-find-usages", "Find Usages list");
  await page.keyboard.press("Escape");
});

await step("76 gutter", async () => {
  await page.getByTestId("editor").click();
  await page.waitForTimeout(1200);
  await shot("76-editor-gutter", "Editor gutter with run/test icons");
});

await step("77 debug flag and plan", async () => {
  await a.openTools();
  await shot("77-debug-adapter", "Debug adapter card with experimental flag");
  const checkbox = page.getByTestId("debug-flag-checkbox");
  if (await checkbox.count()) await checkbox.check();
  const enable = page.getByTestId("debug-flag-enable");
  if (await enable.count()) await enable.click();
  await page.waitForTimeout(800);
  const plan = page.getByTestId("debug-plan-open");
  if (await plan.count()) await plan.click();
  await page.waitForTimeout(800);
  await shot("78-debug-plan", "Debug adapter plan after enabling the flag");
  const install = page.getByTestId("debug-install");
  if (await install.isEnabled().catch(() => false)) {
    await install.click();
    await page.waitForTimeout(4000);
  }
  await shot("79-debug-installed", "Debug adapter state after verification");
  await a.closeTools();
});

await step("80 build for debug", async () => {
  await bottom("build");
  const build = page.getByTestId("build-action");
  if (await build.isEnabled().catch(() => false)) {
    await build.click();
    await waitText("build-state", "succeeded", 180_000);
  }
  await shot("80-build-for-debug", "Debug build before launch");
});

await step("81 breakpoint and launch", async () => {
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+ArrowUp");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Meta+F8");
  await page.waitForTimeout(500);
  await bottom("debug");
  await page.getByTestId("debug-launch").click();
  await waitText("dap-state", "paused", 120_000);
  await shot("81-debug-paused", "Debug session paused with frames and locals");
});

await step("82 step and continue", async () => {
  await page.getByTestId("debug-step-over").click();
  await page.waitForTimeout(1200);
  await shot("82-debug-stepped", "Stepped one line");
  const cont = page.getByTestId("debug-continue");
  if (await cont.isEnabled().catch(() => false)) await cont.click();
  await page.waitForTimeout(2500);
  await shot("83-debug-ended", "Debug session ended with observed exit");
});

await step("84 terminal", async () => {
  await bottom("terminal");
  await page.getByTestId("terminal-start").click();
  await waitText("terminal-state", "Running", 60_000);
  await page.waitForTimeout(1500);
  await page.getByTestId("terminal-input").fill("pwd");
  await page.getByTestId("terminal-send").click();
  await page.waitForTimeout(1200);
  await page.getByTestId("terminal-input").fill("dotnet --version");
  await page.getByTestId("terminal-send").click();
  await page.waitForTimeout(2000);
  await shot("84-terminal", "Terminal shell with real cwd and output");
});

await step("85 terminal close dialog", async () => {
  await page.getByTestId("terminal-close").click();
  await page.getByTestId("terminal-confirm").waitFor();
  await shot("85-terminal-close", "Terminal close confirmation");
  await page.getByTestId("terminal-confirm-keep").click();
});

await step("86 dirty close dialog", async () => {
  await page.getByTestId("tab-src/OrderDesk.Cli/Program.cs").click();
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+A");
  await page.keyboard.insertText("using OrderDesk.Core;\n\nvar order = new Order(\"A-104\", 42m);\nConsole.WriteLine($\"Total: {order.Total:C}\");\n// dirty\n");
  await page.getByTestId("close-buffer").click();
  await page.getByTestId("confirm-save").waitFor();
  await shot("86-close-dirty", "Close unsaved buffer dialog");
  await page.getByTestId("confirm-keep").click();
  await page.waitForTimeout(500);
});

await step("87 layout 1440", async () => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.waitForTimeout(800);
  await shot("87-layout-1440", "Workspace at 1440x900");
});

await step("88 layout 1100", async () => {
  await page.setViewportSize({ width: 1100, height: 700 });
  await page.waitForTimeout(800);
  await shot("88-layout-1100", "Workspace at 1100x700");
});

await step("89 settings at 1100", async () => {
  await page.keyboard.press("Meta+,");
  await page.getByTestId("settings-dialog").waitFor();
  await page.getByTestId("settings-page-keymap").click();
  await shot("89-settings-1100", "Settings at 1100x700");
  await page.getByTestId("settings-cancel").click();
});

await finish("d");
