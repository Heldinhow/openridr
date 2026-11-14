import { expect, test } from "@playwright/test";
import path from "node:path";
import fs from "node:fs";
import os from "node:os";
import { openProjectDetails } from "./automatic";
import { host, requestedScreenshotsDir } from "./environment";

const temporaryWorkspaces: string[] = [];

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
  for (const directory of temporaryWorkspaces.splice(0)) {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});

for (const viewport of [{ width: 1600, height: 1000 }, { width: 1440, height: 900 }, { width: 1100, height: 700 }]) {
  test(`IDE layout, resize and keyboard at ${viewport.width}x${viewport.height}`, async ({ page, request }) => {
    await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
    await page.setViewportSize(viewport);
    await page.goto("/");
    await expect(page.getByTestId("workspace-path-input")).toHaveValue("");
    await expect(page.getByTestId("workspace-error")).toHaveCount(0);
    await expect(page.getByTestId("sdk-host")).toBeAttached();
    const capture = async (name: string) => {
      const directory = requestedScreenshotsDir;
      if (!directory) return;
      fs.mkdirSync(directory, { recursive: true });
      await page.screenshot({ path: path.join(directory, `${name}-${viewport.width}x${viewport.height}.png`) });
    };
    await capture("welcome");
    // One palette for the whole app: Welcome and the workspace read the same tokens.
    const palette = (selector: string) =>
      page.locator(selector).evaluate((element) => {
        const style = getComputedStyle(element);
        return {
          bg: style.getPropertyValue("--bg").trim(),
          chrome: style.getPropertyValue("--chrome").trim(),
        };
      });
    expect(await palette(".setup-screen")).toEqual({ bg: "#090909", chrome: "#050505" });
    await page.getByTestId("workspace-path-input").fill(path.resolve("fixtures/OrderDesk/OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-restricted").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");
    expect(await palette(".workspace-view")).toEqual({ bg: "#090909", chrome: "#050505" });
    const tree = page.getByTestId("tree");
    const editor = page.getByRole("region", { name: "Editor", exact: true });
    const output = page.getByRole("region", { name: "Bottom panel" });
    const treeBox = (await tree.boundingBox())!;
    const editorBox = (await editor.boundingBox())!;
    const outputBox = (await output.boundingBox())!;
    expect(treeBox.width).toBe(280);
    expect(treeBox.y).toBe(56);
    // The Solution Explorer remains alongside both the editor and bottom tool window.
    expect(treeBox.y + treeBox.height).toBeGreaterThanOrEqual(outputBox.y + outputBox.height);
    expect(editorBox.height).toBeGreaterThan(200);
    expect(outputBox.height).toBe(220);
    expect(outputBox.x).toBe(editorBox.x);
    expect(outputBox.x + outputBox.width).toBe(editorBox.x + editorBox.width);
    for (const id of ["toolbar-build", "toolbar-run", "toolbar-debug", "toolbar-settings", "toolbar-run-config", "status-mode"]) {
      const box = (await page.getByTestId(id).boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
    }
    // The trust state lives in the status bar only: nothing in the top bar announces it.
    for (const id of ["mode-badge", "trust-banner", "trust-prompt", "trust-prompt-open", "trust-prompt-dismiss"]) {
      await expect(page.getByTestId(id)).toHaveCount(0);
    }
    await expect(page.locator(".toolbar").getByText(/Restricted mode|Trusted mode/)).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(viewport.width);
    await capture("workspace-empty");
    if (viewport.height === 700) {
      await page.getByTestId("workspace-tools-toggle").click();
      const height = page.getByRole("separator", { name: "Output panel height" });
      await height.focus();
      await page.keyboard.press("End");
      await expect(height).toHaveAttribute("aria-valuenow", "294");
      const empty = page.locator(".empty-editor");
      expect(await empty.evaluate((element) => element.scrollHeight > element.clientHeight)).toBe(true);
      await empty.locator(".hint").scrollIntoViewIfNeeded();
      const hintBox = (await empty.locator(".hint").boundingBox())!;
      const paneBox = (await editor.boundingBox())!;
      expect(hintBox.y).toBeGreaterThanOrEqual(paneBox.y);
      expect(hintBox.y + hintBox.height).toBeLessThanOrEqual(paneBox.y + paneBox.height);
      await expect(empty.getByText("Run saved build", { exact: true })).toBeVisible();
      await page.getByTestId("workspace-tools-toggle").click();
      const handle = (await height.boundingBox())!;
      await page.mouse.move(handle.x + 40, handle.y + 2);
      await page.mouse.down();
      await page.mouse.move(handle.x + 40, handle.y + 76);
      await page.mouse.up();
      await expect(height).toHaveAttribute("aria-valuenow", "220");
    }
    await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
    await expect(page.getByTestId("editor")).toBeVisible();
    await page.getByTestId("editor").click();
    await page.keyboard.press("Control+Home");
    await expect(page.getByTestId("status-cursor")).toContainText("Ln");
    await capture("workspace");

    const width = page.getByRole("separator", { name: "Solution explorer width" });
    await width.focus();
    await page.keyboard.press("ArrowRight");
    await expect(width).toHaveAttribute("aria-valuenow", "296");
    await page.keyboard.press("Home");
    await expect(width).toHaveAttribute("aria-valuenow", "220");
    const handle = (await width.boundingBox())!;
    await page.mouse.move(handle.x + 2, handle.y + 40);
    await page.mouse.down();
    await page.mouse.move(handle.x + 62, handle.y + 40);
    await page.mouse.up();
    await expect(width).toHaveAttribute("aria-valuenow", "280");

    const height = page.getByRole("separator", { name: "Output panel height" });
    await height.focus();
    await page.keyboard.press("ArrowUp");
    await expect(height).toHaveAttribute("aria-valuenow", "236");
    await page.getByTestId("bottom-tab-build").focus();
    await page.keyboard.press("ArrowRight");
    await expect(page.getByTestId("bottom-tab-run")).toBeFocused();
    await expect(page.getByTestId("bottom-tab-run")).toHaveAttribute("aria-selected", "true");
    await expect(page.getByTestId("toolbar-run")).toBeDisabled();

  });
}

test("evaluated project details keep paths intact and Close with the heading", async ({ page, request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-project-details-"));
  temporaryWorkspaces.push(base);
  const copy = path.join(base, "fixture");
  fs.cpSync(path.resolve("fixtures/OrderDesk"), copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("workspace-tools-toggle").click();
  await page.getByTestId("eval-run").click();
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });
  const project = page.getByTestId("tree-project-OrderDesk.Cli");
  await project.click();
  // Clicking a project row also toggles it; make sure its files are visible for the next step.
  if ((await project.getAttribute("aria-expanded")) === "false") await project.click();
  await openProjectDetails(page);

  const details = page.getByTestId("eval-details");
  await expect(details).toBeVisible();
  const projectPath = details.locator("code.detail-path").first();
  await expect(details.getByText("Path", { exact: true })).toBeVisible();
  await expect(projectPath).toHaveText("src/OrderDesk.Cli/OrderDesk.Cli.csproj");
  await expect(projectPath).toHaveCSS("white-space", "nowrap");
  await expect(projectPath).toHaveAttribute("title", "src/OrderDesk.Cli/OrderDesk.Cli.csproj");
  await expect(page.getByTestId("status-workspace")).toHaveAttribute("title", fs.realpathSync(path.join(copy, "OrderDesk.sln")));
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await expect(page.getByTestId("editor")).toBeVisible();
  await expect(page.getByTestId("status-breadcrumb")).not.toContainText("OrderDesk.sln");
  if (requestedScreenshotsDir) {
    fs.mkdirSync(requestedScreenshotsDir, { recursive: true });
    await page.screenshot({ path: path.join(requestedScreenshotsDir, "workspace-trusted-1440x900.png") });
  }
  const close = page.getByRole("button", { name: "Close project details" });
  await expect(close.locator("..")).toContainText("OrderDesk.Cli");
  await close.click();
  await expect(details).toBeHidden();
  await page.getByTestId("workspace-tools-close").click();
  await expect(page.getByTestId("toolbar-build")).toBeEnabled({ timeout: 60_000 });
  if (requestedScreenshotsDir) {
    fs.mkdirSync(requestedScreenshotsDir, { recursive: true });
    await page.screenshot({ path: path.join(requestedScreenshotsDir, "workspace-trusted-1440x900.png") });
  }
});

for (const viewport of [{ width: 1600, height: 1000 }, { width: 1440, height: 900 }, { width: 1100, height: 700 }]) {
  test(`toolbar labels truncate with full titles at ${viewport.width}x${viewport.height}`, async ({ page, request }) => {
    await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
    const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-layout-"));
    temporaryWorkspaces.push(base);
    const copy = path.join(base, "fixture");
    fs.cpSync(path.resolve("fixtures/OrderDesk"), copy, {
      recursive: true,
      filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
    });
    const solution = path.join(copy, "OrderDesk.sln");
    const stored = await request.post(`${host}/call`, {
      data: {
        method: "runConfigs.write",
        params: {
          solution,
          activeId: "layout-long",
          configs: [{
            id: "layout-long",
            name: "Run configuration with a deliberately long visible name",
            project: "src/OrderDesk.Cli/OrderDesk.Cli.csproj",
            configuration: "Debug",
            targetFramework: "net8.0",
            args: [],
            workingDirectory: "src/OrderDesk.Cli",
            env: {},
            envFiles: [],
          }],
        },
      },
    });
    const body = await stored.json();
    expect(body.ok, JSON.stringify(body.error)).toBe(true);
    await page.setViewportSize(viewport);
    await page.goto("/");
    await page.getByTestId("workspace-path-input").fill(solution);
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-prompt-trust").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    await page.getByTestId("workspace-tools-toggle").click();
    await page.getByTestId("eval-run").click();
    await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });

    const activeConfig = page.getByTestId("run-config-active");
    await expect(activeConfig).toHaveText("Run configuration with a deliberately long visible name");
    // Middle truncation: the head shrinks and the distinctive tail stays whole.
    expect(await activeConfig.locator(".mid-head").evaluate((element) => element.scrollWidth > element.clientWidth)).toBe(true);
    for (const id of ["toolbar-build", "toolbar-run", "toolbar-debug"]) {
      const box = (await page.getByTestId(id).boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
    }
    expect(await page.locator(".toolbar").evaluate((element) => element.scrollWidth)).toBeLessThanOrEqual(
      await page.locator(".toolbar").evaluate((element) => element.clientWidth),
    );
    const evaluationLabel = page.getByTestId("status-evaluation");
    await expect(evaluationLabel).toHaveAttribute("title", /^Evaluated · Debug · TFM .* · revision \d+$/);
    // #365: readiness is never the item that yields; the whole summary fits at 1440 px.
    expect(await evaluationLabel.evaluate((element) => element.scrollWidth > element.clientWidth)).toBe(false);
  });
}
