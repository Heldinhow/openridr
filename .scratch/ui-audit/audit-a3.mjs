import { createAudit, PLAIN } from "./lib.mjs";

const a = await createAudit("a3");
const { page, shot, step, click, waitText, boot, openTools, bottom, finish } = a;

await step("22 install/verify analysis tool", async () => {
  await boot({ target: PLAIN, trust: true, evaluate: true, restore: true });
  await openTools();
  await shot("22-tools-before-install", "Analysis tool reported as not installed");
  await click("install-plan-open");
  await page.getByTestId("install-plan").waitFor();
  await shot("22a-install-plan", "csharp-ls install plan with destination and integrity");
  await click("install-confirm");
  await page.getByTestId("install-ready").waitFor({ timeout: 240_000 });
  await shot("22b-install-verified", "Existing csharp-ls reused and verified");
});

await step("23 lsp ready", async () => {
  await click("lsp-start");
  await waitText("lsp-state", "Ready", 150_000);
  await shot("23-lsp-ready", "csharp-ls ready, capabilities listed");
  await a.closeTools();
});

await step("24 build success", async () => {
  await bottom("build");
  await page.getByTestId("build-action").click();
  await waitText("build-state", "succeeded", 180_000);
  await shot("24-build-success", "Build succeeded");
});

await step("25 run", async () => {
  await page.getByTestId("run-action").click();
  await waitText("run-state", "exited", 120_000);
  await shot("25-run-output", "Run output with command and exit");
});

await step("26 problems after build", async () => {
  await bottom("problems");
  await shot("26-problems", "Problems after a successful build");
});

await step("27 editor gutter", async () => {
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await page.waitForTimeout(1500);
  await shot("27-editor-gutter", "Editor with gutter icons and breadcrumb");
});

await finish("a3");
