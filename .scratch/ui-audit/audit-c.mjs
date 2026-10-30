import { createAudit, GIT } from "./lib.mjs";

const a = await createAudit("c");
const { page, shot, step, click, waitText, boot, bottom, finish } = a;

await step("50 boot git workspace", async () => {
  await boot({ target: GIT, trust: true, evaluate: true, restore: true });
  await a.closeTools();
  await a.left("commit");
  await page.getByTestId("commit-tool").waitFor();
  await page.waitForTimeout(800);
  await shot("50-git-commit", "Commit tool window with modified files");
});

await step("51 git diff", async () => {
  await page.getByTestId("commit-file-src/OrderDesk.Cli/Program.cs").click();
  await page.getByTestId("commit-diff").waitFor();
  await page.waitForTimeout(600);
  await shot("51-git-diff", "Inline diff inside the Commit tool window");
});

await step("52 git commit", async () => {
  await page.getByTestId("commit-message").fill("Audit commit");
  await page.getByTestId("commit-submit").click();
  await page.getByTestId("commit-state").waitFor({ timeout: 60_000 });
  await shot("52-git-committed", "Commit result with observed hash");
});

await step("53 git log", async () => {
  await bottom("git");
  await page.waitForTimeout(1200);
  await shot("53-git-log", "Git log tool window");
  const item = page.getByTestId(/git-log-item-/).first();
  if (await item.count()) await item.click();
  await page.waitForTimeout(500);
  await shot("54-git-log-files", "Selected commit with changed files");
});

await step("55 branch menu", async () => {
  await page.getByTestId("toolbar-branch").click();
  await page.getByTestId("branch-menu").waitFor();
  await shot("55-branch-menu", "Branch popup with create/update/push");
});

await step("56 branch created", async () => {
  await page.getByTestId("branch-create-open").click();
  await page.getByTestId("branch-name").fill("audit-branch");
  await page.getByTestId("branch-create-submit").click();
  await page.waitForTimeout(1200);
  await shot("56-branch-created", "Branch created from the current one");
  await page.keyboard.press("Escape");
});

await step("57 push dialog", async () => {
  await page.getByTestId("toolbar-branch").click();
  await page.getByTestId("branch-push").click();
  await page.waitForTimeout(600);
  await shot("57-git-push", "Push confirmation or disabled reason");
  const cancel = page.getByTestId("push-cancel");
  if (await cancel.count()) await cancel.click().catch(() => {});
  await page.keyboard.press("Escape");
});

await step("58 update dialog", async () => {
  await page.getByTestId("toolbar-branch").click();
  const update = page.getByTestId("branch-update");
  if (await update.isEnabled().catch(() => false)) {
    await update.click();
    await page.waitForTimeout(600);
    await shot("58-git-update", "Update project confirmation");
    const cancel = page.getByTestId("update-cancel");
    if (await cancel.count()) await cancel.click().catch(() => {});
  } else {
    await shot("58-git-update", "Update project disabled with reason");
  }
  await page.keyboard.press("Escape");
});

await step("59 nuget test project", async () => {
  await bottom("nuget");
  await page.getByTestId("nuget-project").waitFor();
  await page.getByTestId("nuget-project").selectOption({ label: "OrderDesk.Tests" });
  await page.waitForTimeout(800);
  const refresh = page.getByTestId("nuget-refresh");
  if (await refresh.count()) await refresh.click();
  await page.waitForTimeout(4000);
  await shot("59-nuget-packages", "NuGet installed and outdated for the test project");
});

await step("60 nuget no packages", async () => {
  await page.getByTestId("nuget-project").selectOption({ label: "OrderDesk.Cli" });
  await page.waitForTimeout(3000);
  await shot("60-nuget-no-packages", "NuGet for a project without package references");
});

await step("61 tests discover", async () => {
  await bottom("tests");
  await page.getByTestId("tests-project").waitFor();
  await page.getByTestId("tests-project").selectOption({ label: "OrderDesk.Tests" });
  await page.waitForTimeout(500);
  await page.getByTestId("tests-discover").click();
  await waitText("tests-state", "Discovered", 180_000);
  await shot("61-tests-discovered", "Discovered tests grouped by name");
});

await step("62 tests run all", async () => {
  await page.getByTestId("tests-run-all").click();
  await waitText("tests-run-state", "Run completed", 240_000);
  await shot("62-tests-results", "Test run results with failure details");
});

await step("63 tests failed detail", async () => {
  const failed = page.locator('[data-testid^="test-result-"]').filter({ hasText: "Failed" }).first();
  if (await failed.count()) await failed.click();
  await page.waitForTimeout(600);
  await shot("63-tests-failed", "Failed test details, stack and output");
});

await step("64 tests rerun failed", async () => {
  const rerun = page.getByTestId("tests-rerun-failed");
  if (await rerun.isEnabled().catch(() => false)) {
    await rerun.click();
    await waitText("tests-run-state", "Run completed", 240_000);
  }
  await shot("64-tests-rerun", "Rerun failed tests");
});

await step("65 tests quick list", async () => {
  await page.keyboard.press("Shift+Alt+u");
  await page.getByTestId("tests-quick-list").waitFor();
  await shot("65-tests-quick-list", "Test quick list");
  await page.keyboard.press("Escape");
});

await finish("c");
