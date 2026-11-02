import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";
import { settleRestore } from "./automatic";

// Isolated data: every spec works on a temp copy of the fixture; the host runs with the E2E app home.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
test.use({ viewport: { width: 1440, height: 900 } });

function tempCopy() {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-sol-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  return copy;
}

async function open(page: Page, root: string, trust = false) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(root, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId(trust ? "trust-prompt-trust" : "trust-prompt-restricted").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("Rider-style rows: counts, solution folders, nested sources and Dependencies", async ({ page }) => {
  await open(page, tempCopy());
  await expect(page.getByTestId("tree-root")).toContainText("3 projects");
  await expect(page.getByTestId("tree-project-OrderDesk.Cli")).toBeVisible();
  const row = await page.getByTestId("tree-project-OrderDesk.Cli").boundingBox();
  expect(row!.height).toBeLessThan(36); // one line, no two-layer row
  await page.getByTestId("tree-root").click();
  await expect(page.getByTestId("tree-project-OrderDesk.Cli")).toHaveCount(0);
  await page.getByTestId("tree-root").click();
  await expect(page.getByTestId("tree-project-OrderDesk.Cli")).toBeVisible();
});

test("toolbar: collapse/expand all, filter, minimize", async ({ page }) => {
  await open(page, tempCopy());
  await page.getByTestId("tree-collapse-all").click();
  await expect(page.getByTestId("tree-project-OrderDesk.Cli")).toBeVisible();
  await expect(page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs")).toHaveCount(0);
  await page.getByTestId("tree-expand-all").click();
  await expect(page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs")).toBeVisible();

  await page.getByTestId("tree-search").click();
  await page.getByTestId("tree-filter").fill("Program");
  await expect(page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs")).toBeVisible();
  await expect(page.getByTestId("tree-project-OrderDesk.Core")).toHaveCount(0);
  await page.getByTestId("tree-filter").press("Escape");
  await expect(page.getByTestId("tree-project-OrderDesk.Core")).toBeVisible();

  await page.getByTestId("tree-more").click();
  await expect(page.getByTestId("tree-more-reevaluate")).toBeDisabled(); // Restricted
  await page.keyboard.press("Escape");
  await page.getByTestId("tree-minimize").click();
  await expect(page.getByTestId("tree")).toHaveCount(0);
});

test("show all files lists bin/obj/dotfiles as no index and keeps sources indexed", async ({ page }) => {
  const root = tempCopy();
  fs.mkdirSync(path.join(root, "src/OrderDesk.Cli/bin"), { recursive: true });
  fs.mkdirSync(path.join(root, "src/OrderDesk.Cli/obj"), { recursive: true });
  fs.mkdirSync(path.join(root, ".idea"), { recursive: true });
  fs.writeFileSync(path.join(root, ".gitignore"), "bin/\n");
  await open(page, root);
  await expect(page.getByTestId("tree-dir-.idea")).toHaveCount(0);
  await page.getByTestId("tree-show-all").click();
  await expect(page.getByTestId("tree-dir-src/OrderDesk.Cli/bin")).toContainText("no index");
  await expect(page.getByTestId("tree-dir-src/OrderDesk.Cli/obj")).toContainText("no index");
  await expect(page.getByTestId("tree-dir-.idea")).toContainText("no index");
  await expect(page.getByTestId("tree-file-.gitignore")).toContainText("no index");
  await expect(page.getByTestId("tree-file-global.json")).toContainText("no index");
  await expect(page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs")).not.toContainText("no index");
  await page.getByTestId("tree-show-all").click();
  await expect(page.getByTestId("tree-dir-.idea")).toHaveCount(0);
});

test("Restricted project menu blocks execution, offers Trust and edits the csproj", async ({ page }) => {
  await open(page, tempCopy());
  await page.getByTestId("tree-project-OrderDesk.Cli").click({ button: "right" });
  const menu = page.getByTestId("tree-context-menu");
  await expect(menu).toBeVisible();
  for (const id of ["build", "run", "debug", "reload", "unit-test"]) {
    await expect(page.getByTestId(`tree-menu-${id}`)).toBeDisabled();
  }
  await expect(page.getByTestId("tree-menu-build")).toHaveAttribute("title", /Trust the workspace/);
  await expect(page.getByTestId("tree-menu-run")).toContainText("Run 'OrderDesk.Cli'");
  await expect(page.getByTestId("tree-menu-trust")).toBeVisible();

  await page.getByTestId("tree-menu-edit").hover();
  await expect(page.getByTestId("tree-menu-edit-file")).toContainText("Edit 'OrderDesk.Cli.csproj'");
  await page.getByTestId("tree-menu-edit-file").click();
  await expect(page.getByTestId("tree-context-menu")).toHaveCount(0);
  await expect(page.getByTestId("editor")).toBeVisible();

  await page.getByTestId("tree-project-OrderDesk.Cli").click({ button: "right" });
  await page.getByTestId("tree-menu-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
});

test("trusted menu: console can run, library builds; folders show counts", async ({ page }) => {
  test.setTimeout(180_000);
  await open(page, tempCopy(), true);
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });
  await page.getByTestId("workspace-tools-toggle").click();
  await settleRestore(page);
  await expect(page.getByTestId("tree-folder-src")).toContainText("2 projects");
  await expect(page.getByTestId("tree-folder-tests")).toContainText("1 project");
  await page.getByTestId("tree-expand-all").click();
  await expect(page.getByTestId("tree-deps-OrderDesk.Cli")).toBeVisible();

  await page.getByTestId("tree-project-OrderDesk.Core").click({ button: "right" });
  const build = page.getByTestId("tree-menu-build");
  await expect(build).toBeEnabled();
  await expect(build).not.toHaveAttribute("title", /console projects/);
  await expect(page.getByTestId("tree-menu-run")).toHaveCount(0);
  await expect(page.getByTestId("tree-menu-debug")).toHaveCount(0);
  await build.click();
  await expect(page.getByTestId("build-state")).toContainText("Build succeeded", { timeout: 60_000 });
  await expect(page.getByTestId("build-project")).toContainText("OrderDesk.Core");

  await page.getByTestId("tree-project-OrderDesk.Cli").click({ button: "right" });
  await expect(page.getByTestId("tree-menu-trust")).toHaveCount(0);
  await expect(page.getByTestId("tree-menu-reload")).toBeEnabled();
  await expect(page.getByTestId("tree-menu-run")).toBeEnabled();
  await expect(page.getByTestId("tree-menu-debug")).toBeEnabled();
  await expect(page.getByTestId("tree-menu-unit-test")).toBeDisabled();
});
