import { createAudit, PLAIN } from "./lib.mjs";

const a = await createAudit("g");
const { page, shot, step, boot, bottom, finish } = a;

await step("17 evaluated tree", async () => {
  await boot({ target: PLAIN, trust: true, evaluate: true });
  for (const project of ["OrderDesk.Cli", "OrderDesk.Core", "OrderDesk.Tests"]) {
    const el = page.getByTestId(`tree-project-${project}`);
    if ((await el.count()) && (await el.getAttribute("aria-expanded")) !== "true") await el.click();
  }
  await page.waitForTimeout(800);
  await shot("17-tree-expanded", "Solution Explorer expanded after evaluation");
});

await step("06 restricted build", async () => {
  await page.evaluate(async () => {
    await fetch("/__host/call", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ method: "workspace.close", params: {} }),
    });
  });
  await page.goto("http://127.0.0.1:1420/");
  await page.getByTestId("workspace-path-input").fill(PLAIN);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-checkbox").waitFor();
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("tree").waitFor();
  await bottom("build");
  await page.waitForTimeout(600);
  await shot("06-restricted-build", "Build tool window in Restricted mode");
});

await finish("g");
