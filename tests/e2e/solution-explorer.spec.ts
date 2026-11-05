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
  await expect(menu).toHaveAccessibleName("Project context menu");
  for (const id of ["build", "run", "debug", "reload"]) {
    await expect(page.getByTestId(`tree-menu-${id}`)).toBeDisabled();
  }
  // A console project has no tests: the item is absent, not disabled.
  await expect(page.getByTestId("tree-menu-unit-test")).toHaveCount(0);
  await expect(page.getByTestId("tree-menu-build")).toHaveAttribute("title", /Trust the workspace/);
  await expect(page.getByTestId("tree-menu-run")).toContainText("Run 'OrderDesk.Cli'");
  await expect(page.getByTestId("tree-menu-trust")).toBeVisible();
  // A test project keeps the item, gated by the same trust reason as the Tests panel.
  await page.keyboard.press("Escape");
  await page.getByTestId("tree-project-OrderDesk.Tests").click({ button: "right" });
  await expect(page.getByTestId("tree-menu-unit-test")).toBeDisabled();
  await expect(page.getByTestId("tree-menu-unit-test")).toHaveAttribute(
    "title",
    /Trust the workspace/,
  );
  await page.keyboard.press("Escape");
  await page.getByTestId("tree-project-OrderDesk.Cli").click({ button: "right" });
  // A project file is owned by the solution: no Rename/Delete from its menu.
  await expect(page.getByTestId("tree-menu-rename")).toHaveCount(0);
  await expect(page.getByTestId("tree-menu-delete")).toHaveCount(0);

  await page.getByTestId("tree-menu-edit").hover();
  await expect(page.getByTestId("tree-menu-edit-file")).toContainText("Edit 'OrderDesk.Cli.csproj'");
  await page.getByTestId("tree-menu-edit-file").click();
  await expect(page.getByTestId("tree-context-menu")).toHaveCount(0);
  await expect(page.getByTestId("editor")).toBeVisible();

  await page.getByTestId("tree-project-OrderDesk.Cli").click({ button: "right" });
  await page.getByTestId("tree-menu-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
});

test("external create, rename and delete refresh the evaluated tree", async ({ page }) => {
  test.setTimeout(180_000);
  const root = tempCopy();
  await open(page, root, true);
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });
  await page.getByTestId("tree-expand-all").click();
  const order = page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs");
  await expect(order).toBeVisible();

  // Collapse and select Core; the automatic refresh must keep both.
  await page.getByTestId("tree-project-OrderDesk.Core").click();
  await expect(order).toHaveCount(0);
  await expect(page.getByTestId("tree-project-OrderDesk.Core")).toHaveAttribute(
    "aria-current",
    "true",
  );

  // Create externally: the new .cs appears without clicking Re-evaluate.
  fs.writeFileSync(
    path.join(root, "src/OrderDesk.Cli/Extra.cs"),
    "namespace OrderDesk.Cli;\npublic static class Extra {}\n",
  );
  await expect(page.getByTestId("tree-file-src/OrderDesk.Cli/Extra.cs")).toBeVisible({
    timeout: 30_000,
  });
  await expect(order).toHaveCount(0); // the collapsed state survived the refresh
  await expect(page.getByTestId("tree-project-OrderDesk.Core")).toHaveAttribute(
    "aria-current",
    "true",
  );

  await page.getByTestId("tree-project-OrderDesk.Core").click();
  await expect(order).toBeVisible();

  // Rename externally: the old path goes and the new one arrives.
  fs.renameSync(
    path.join(root, "src/OrderDesk.Core/Order.cs"),
    path.join(root, "src/OrderDesk.Core/Renamed.cs"),
  );
  await expect(page.getByTestId("tree-file-src/OrderDesk.Core/Renamed.cs")).toBeVisible({
    timeout: 30_000,
  });
  await expect(order).toHaveCount(0);

  // Delete externally: the row disappears from the evaluated tree, which stays the model.
  fs.rmSync(path.join(root, "src/OrderDesk.Core/Renamed.cs"));
  await expect(page.getByTestId("tree-file-src/OrderDesk.Core/Renamed.cs")).toHaveCount(0, {
    timeout: 30_000,
  });
  await expect(page.getByTestId("tree-project-OrderDesk.Core")).toBeVisible();
  await expect(page.getByTestId("eval-stale")).toHaveCount(0);
});

test("tree menu creates, renames and deletes files with confirmation", async ({ page }) => {
  test.setTimeout(180_000);
  const root = tempCopy();
  await open(page, root, true);
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });
  await page.getByTestId("tree-expand-all").click();
  await expect(page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs")).toBeVisible();

  // Create an empty file from the project menu (its folder is the project directory).
  await page.getByTestId("tree-project-OrderDesk.Core").click({ button: "right" });
  await page.getByTestId("tree-menu-new-file").click();
  await expect(page.getByTestId("tree-name-dialog")).toBeVisible();
  await page.getByTestId("tree-name-input").fill("Extra.cs");
  await page.getByTestId("tree-name-confirm").click();
  const extra = page.getByTestId("tree-file-src/OrderDesk.Core/Extra.cs");
  await expect(extra).toBeVisible({ timeout: 30_000 });
  expect(fs.readFileSync(path.join(root, "src/OrderDesk.Core/Extra.cs"), "utf8")).toBe("");

  // Open it from the tree: the empty buffer is a normal tab.
  await extra.click();
  await expect(page.getByTestId("tab-src/OrderDesk.Core/Extra.cs")).toBeVisible();

  // Rename keeps the contents and the evaluated tree follows the new path.
  const order = page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs");
  await order.click({ button: "right" });
  // Files keep the destructive actions the project menu refuses.
  await expect(page.getByTestId("tree-menu-delete")).toBeVisible();
  await page.getByTestId("tree-menu-rename").click();
  await expect(page.getByTestId("tree-name-input")).toHaveValue("Order.cs");
  await page.getByTestId("tree-name-input").fill("Renamed.cs");
  await page.getByTestId("tree-name-confirm").click();
  const renamed = page.getByTestId("tree-file-src/OrderDesk.Core/Renamed.cs");
  await expect(renamed).toBeVisible({ timeout: 30_000 });
  await expect(order).toHaveCount(0);
  expect(fs.readFileSync(path.join(root, "src/OrderDesk.Core/Renamed.cs"), "utf8")).toContain(
    "class Order",
  );

  // Delete asks first; cancelling keeps the file.
  await renamed.click({ button: "right" });
  await page.getByTestId("tree-menu-delete").click();
  await expect(page.getByTestId("tree-delete-dialog")).toContainText("Delete file?");
  await page.getByTestId("tree-delete-cancel").click();
  await expect(renamed).toBeVisible();

  await renamed.click({ button: "right" });
  await page.getByTestId("tree-menu-delete").click();
  await page.getByTestId("tree-delete-confirm").click();
  await expect(renamed).toHaveCount(0, { timeout: 30_000 });
  expect(fs.existsSync(path.join(root, "src/OrderDesk.Core/Renamed.cs"))).toBe(false);
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
  // Neither a console project nor the library is a test project, so the item is absent.
  await expect(page.getByTestId("tree-menu-unit-test")).toHaveCount(0);
  await page.keyboard.press("Escape");

  await page.getByTestId("tree-project-OrderDesk.Tests").click({ button: "right" });
  await expect(page.getByTestId("tree-menu-unit-test")).toBeEnabled();
});

test("folder menu keeps the existing actions and adds Show in Finder", async ({ page }) => {
  const root = tempCopy();
  fs.mkdirSync(path.join(root, "docs"));
  await open(page, root);
  await page.getByTestId("tree-show-all").click();
  await page.getByTestId("tree-dir-docs").click({ button: "right" });
  await expect(page.getByTestId("tree-context-menu")).toBeVisible();
  for (const id of ["new-file", "copy-path", "find", "rename", "delete", "reveal"]) {
    await expect(page.getByTestId(`tree-menu-${id}`)).toBeVisible();
  }
  await expect(page.getByTestId("tree-menu-reveal")).toContainText("Show in Finder");
});

test("solution menu: Build, Restore, Copy Path, Find in Files and a disabled Add Project", async ({ page }) => {
  await open(page, tempCopy());
  await page.getByTestId("tree-root").click({ button: "right" });
  await expect(page.getByTestId("tree-context-menu")).toBeVisible();
  await expect(page.getByTestId("tree-menu-build-solution")).toBeDisabled();
  await expect(page.getByTestId("tree-menu-build-solution")).toHaveAttribute("title", /Trust the workspace/);
  await expect(page.getByTestId("tree-menu-restore-solution")).toBeDisabled();
  await expect(page.getByTestId("tree-menu-restore-solution")).toHaveAttribute("title", /Trust the workspace/);
  await expect(page.getByTestId("tree-menu-copy-path")).toBeEnabled();
  await expect(page.getByTestId("tree-menu-find")).toBeEnabled();
  await expect(page.getByTestId("tree-menu-reveal")).toBeEnabled();
  await expect(page.getByTestId("tree-menu-add-project")).toBeDisabled();
  await expect(page.getByTestId("tree-menu-add-project")).toHaveAttribute("title", "Adding projects is not implemented.");
});

test("Show in Finder calls workspace.reveal with the item path and shows a host refusal", async ({ page }) => {
  const root = tempCopy();
  fs.mkdirSync(path.join(root, "docs"));
  let revealed: string | undefined;
  await page.route("**/call", async (route) => {
    const body = route.request().postDataJSON() as { method: string; params?: { path?: string } };
    if (body.method !== "workspace.reveal") {
      await route.continue();
      return;
    }
    revealed = body.params?.path;
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        ok: false,
        error: { code: "outside_workspace", message: "Path is outside the workspace: ../outside" },
      }),
    });
  });
  await open(page, root);
  await page.getByTestId("tree-show-all").click();
  await page.getByTestId("tree-dir-docs").click({ button: "right" });
  await page.getByTestId("tree-menu-reveal").click();
  await expect(page.getByTestId("app-log")).toContainText("Show in Finder failed: Path is outside the workspace");
  expect(revealed).toBe("docs");
});

test("workspace.reveal refuses a path outside the workspace without touching the Finder", async ({ request }) => {
  const root = tempCopy();
  await request.post(`${host}/call`, { data: { method: "workspace.open", params: { path: path.join(root, "OrderDesk.sln") } } });
  const outside = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-outside-"));
  const outsideFile = path.join(outside, "Outside.cs");
  fs.writeFileSync(outsideFile, "class Outside {}\n");
  const response = await request.post(`${host}/call`, {
    data: { method: "workspace.reveal", params: { path: outsideFile } },
  });
  const body = await response.json();
  expect(body.ok).toBe(false);
  expect(body.error.code).toBe("outside_workspace");
});
