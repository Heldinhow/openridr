import { expect, test } from "@playwright/test";
import path from "node:path";
import fs from "node:fs";
import { host } from "./environment";

for (const viewport of [{ width: 1600, height: 1000 }, { width: 1440, height: 900 }, { width: 1100, height: 700 }]) {
  test(`IDE layout, resize and keyboard at ${viewport.width}x${viewport.height}`, async ({ page, request }) => {
    await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
    await page.setViewportSize(viewport);
    await page.goto("/");
    await expect(page.getByTestId("workspace-path-input")).toHaveValue("");
    await expect(page.getByTestId("workspace-error")).toHaveCount(0);
    await expect(page.getByTestId("sdk-host")).toBeAttached();
    const capture = async (name: string) => {
      const directory = process.env.OPENRIDR_SCREENSHOT_DIR;
      if (!directory) return;
      fs.mkdirSync(directory, { recursive: true });
      await page.screenshot({ path: path.join(directory, `${name}-${viewport.width}x${viewport.height}.png`) });
    };
    await capture("welcome");
    await page.getByTestId("workspace-path-input").fill(path.resolve("fixtures/OrderDesk/OrderDesk.sln"));
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("open-text-only").click();
    await expect(page.getByTestId("mode-badge")).toHaveText("Restricted mode");
    const tree = page.getByTestId("tree");
    const editor = page.getByRole("region", { name: "Editor", exact: true });
    const output = page.getByRole("region", { name: "Bottom panel" });
    const treeBox = (await tree.boundingBox())!;
    const editorBox = (await editor.boundingBox())!;
    const outputBox = (await output.boundingBox())!;
    expect(treeBox.width).toBe(280);
    expect(treeBox.y).toBe(44);
    expect(treeBox.y + treeBox.height).toBe(viewport.height - 24);
    expect(editorBox.height).toBeGreaterThan(200);
    expect(outputBox.height).toBe(220);
    for (const id of ["toolbar-build", "toolbar-run", "toolbar-debug", "mode-badge"]) {
      const box = (await page.getByTestId(id).boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
    }
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
    await expect(page.getByTestId("bottom-tab-problems")).toBeFocused();
    await expect(page.getByTestId("bottom-tab-problems")).toHaveAttribute("aria-selected", "true");
    await expect(page.getByTestId("toolbar-run")).toBeDisabled();
  });
}
