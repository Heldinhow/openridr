import { createAudit, PLAIN } from "./lib.mjs";

const a = await createAudit("a");
const { page, shot, step, click, waitText, boot, openTools, bottom, finish } = a;

await step("01 setup empty", async () => {
  await page.goto("http://127.0.0.1:1420/");
  await page.getByTestId("workspace-path-input").waitFor();
  await shot("01-setup-empty", "First run, no workspace");
});

await step("02 invalid path", async () => {
  await page.getByTestId("workspace-path-input").fill("/tmp/does-not-exist-openridr/No.sln");
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("workspace-error").waitFor();
  await shot("02-setup-invalid-path", "Invalid workspace path error");
});

await step("03 restricted toolchain", async () => {
  await page.getByTestId("workspace-path-input").fill(PLAIN);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-checkbox").waitFor();
  await shot("03-toolchain-restricted", "Trust card and toolchain, restricted");
  await shot("03b-toolchain-restricted-full", "Trust card and toolchain full page", { fullPage: true });
});

await step("04 toolchain expanded", async () => {
  for (const id of ["sdk-host", "sdk-project", "sdk-tfms"]) {
    const el = page.getByTestId(id);
    if (await el.count()) await el.click().catch(() => {});
  }
  const card = page.getByTestId("debug-adapter-card");
  if (await card.count()) await card.click().catch(() => {});
  await shot("04-toolchain-expanded", "Toolchain rows expanded", { fullPage: true });
});

await step("05 restricted workspace", async () => {
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("tree").waitFor();
  await shot("05-workspace-restricted", "Workspace in restricted mode");
});

await step("06 restricted build", async () => {
  await bottom("build");
  await shot("06-restricted-build", "Build tool window restricted");
});
await step("07 restricted problems", async () => {
  await bottom("problems");
  await shot("07-restricted-problems", "Problems tool window restricted");
});
await step("08 restricted debug", async () => {
  await bottom("debug");
  await shot("08-restricted-debug", "Debug tool window restricted");
});
await step("09 restricted terminal", async () => {
  await bottom("terminal");
  await shot("09-restricted-terminal", "Terminal tool window restricted");
});
await step("10 restricted tests", async () => {
  await bottom("tests");
  await shot("10-restricted-tests", "Tests tool window restricted");
});
await step("11 restricted search", async () => {
  await page.keyboard.press("Meta+t");
  await page.getByTestId("find-action").waitFor();
  await page.getByTestId("search-tab-classes").click();
  await page.getByTestId("find-action-input").fill("Order");
  await page.waitForTimeout(1000);
  await shot("11-restricted-search-classes", "Search Everywhere classes without analysis");
  await page.keyboard.press("Escape");
});
await step("12 restricted settings vcs", async () => {
  await page.keyboard.press("Meta+,");
  await page.getByTestId("settings-dialog").waitFor();
  await page.getByTestId("settings-page-vcs").click();
  await shot("12-restricted-settings-vcs", "Settings Version Control untrusted");
  await page.keyboard.press("Escape");
});
await step("13 restricted git", async () => {
  await bottom("git");
  await shot("13-restricted-git", "Git log tool window untrusted");
});
await step("14 restricted commit", async () => {
  await a.left("commit");
  await shot("14-restricted-commit", "Commit tool window untrusted");
});
await step("15 restricted nuget", async () => {
  await bottom("nuget");
  await shot("15-restricted-nuget", "NuGet tool window untrusted");
});

await step("16 trusted workspace", async () => {
  await boot({ target: PLAIN, trust: true, evaluate: true });
  await shot("16-workspace-trusted", "Workspace after trust and evaluation");
});

await step("17 tree expanded", async () => {
  for (const project of ["OrderDesk.Cli", "OrderDesk.Core", "OrderDesk.Tests"]) {
    const el = page.getByTestId(`tree-project-${project}`);
    if ((await el.count()) && (await el.getAttribute("aria-expanded")) !== "true") await el.click().catch(() => {});
  }
  await shot("17-tree-expanded", "Solution Explorer expanded");
});

await step("18 editor program", async () => {
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await page.getByTestId("editor").waitFor();
  await shot("18-editor-program", "Program.cs open in the editor");
});

await step("19 project details", async () => {
  await page.getByTestId("tree-project-OrderDesk.Cli").click();
  await page.getByRole("button", { name: "Project details" }).click();
  await page.getByTestId("eval-details").waitFor();
  await shot("19-project-details", "Evaluated project details panel");
  await page.getByRole("button", { name: "Close project details" }).click();
});

await step("20 workspace tools", async () => {
  await openTools();
  await shot("20-workspace-tools", "Workspace tools: restore, LSP, adapter, installer");
});

await step("21 restore", async () => {
  await click("restore-action");
  await waitText("restore-state", "Restore succeeded", 180_000);
  await shot("21-restore-success", "Restore succeeded with command and log");
  await click("restore-reload");
  await waitText("eval-state", "revision 2", 120_000);
});

await step("22 lsp ready", async () => {
  await click("lsp-start");
  await waitText("lsp-state", "Ready", 150_000);
  await shot("22-lsp-ready", "csharp-ls ready, capabilities listed");
});

await step("23 build success", async () => {
  await a.closeTools();
  await bottom("build");
  await page.getByTestId("build-action").click();
  await waitText("build-state", "succeeded", 180_000);
  await shot("23-build-success", "Build succeeded");
});

await step("24 run", async () => {
  await page.getByTestId("run-action").click();
  await waitText("run-state", "exited", 120_000);
  await shot("24-run-output", "Run output with command and exit");
});

await step("25 problems after build", async () => {
  await bottom("problems");
  await shot("25-problems", "Problems after a successful build");
});

await step("26 editor gutter and breakpoint", async () => {
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await page.waitForTimeout(1200);
  await shot("26-editor-gutter", "Editor with gutter icons and breadcrumb");
});

await finish("a");
