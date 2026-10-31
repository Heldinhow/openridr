import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

test("empty editor scrolls locally in a constrained workspace (isolated DOM)", async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 700 });
  await page.setContent(`
    <div class="workspace-view" style="--output-height:294px">
      <header class="toolbar">Workspace</header>
      <div class="workspace-notices" style="height:168px">Workspace tools expanded</div>
      <section class="editor-pane" aria-label="Editor">
        <div class="empty-editor">
          <svg class="icon"></svg><h1>Your code, in focus.</h1>
          <p class="muted">Open a file from the solution explorer to start editing.</p>
          <dl class="shortcut-list">
            <dt>Save file</dt><dd><kbd>⌘S</kbd></dd>
            <dt>Go to definition</dt><dd><kbd>⌘B</kbd></dd>
            <dt>Rename symbol</dt><dd><kbd>Shift+F6</kbd></dd>
            <dt>Run saved build</dt><dd><kbd>⌘F9</kbd></dd>
          </dl>
          <p class="hint">Text editing is always available. Analysis and execution require trust.</p>
        </div>
      </section>
      <section class="output">Output</section><footer class="status">Status</footer>
    </div>`);
  await page.addStyleTag({ content: fs.readFileSync(path.resolve("src/styles.css"), "utf8") });
  const pane = page.getByRole("region", { name: "Editor" });
  expect((await pane.boundingBox())!.height).toBe(166);
  const empty = page.locator(".empty-editor");
  await expect(empty).toHaveCSS("overflow-y", "auto");
  expect(await empty.evaluate((element) => element.scrollHeight > element.clientHeight)).toBe(true);
  await empty.locator(".hint").scrollIntoViewIfNeeded();
  expect(await empty.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
  const hint = (await empty.locator(".hint").boundingBox())!;
  const bounds = (await pane.boundingBox())!;
  expect(hint.y).toBeGreaterThanOrEqual(bounds.y);
  expect(hint.y + hint.height).toBeLessThanOrEqual(bounds.y + bounds.height);
  await empty.locator("h1").scrollIntoViewIfNeeded();
  expect((await empty.locator("h1").boundingBox())!.y).toBeGreaterThanOrEqual(bounds.y);
  expect(await page.evaluate(() => document.documentElement.scrollHeight)).toBe(700);
});

test("editor action popups stay inside the editor and keep preview actions reachable", async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 700 });
  await page.setContent(`
    <section class="editor-pane" aria-label="Editor" style="width:700px;height:500px">
      <div class="rename-preview editor-popup">
        <h2>Rename preview</h2><div style="height:700px">Preview details</div>
        <div class="actions"><button>Apply</button><button>Cancel</button></div>
      </div>
      <div class="editor-body" style="min-height:100px">Editor</div>
    </section>`);
  await page.addStyleTag({ content: fs.readFileSync(path.resolve("src/styles.css"), "utf8") });
  const pane = page.getByRole("region", { name: "Editor" });
  const popup = page.locator(".editor-popup");
  const paneBox = (await pane.boundingBox())!;
  const popupBox = (await popup.boundingBox())!;
  expect(popupBox.x).toBeGreaterThanOrEqual(paneBox.x);
  expect(popupBox.y).toBeGreaterThanOrEqual(paneBox.y);
  expect(popupBox.x + popupBox.width).toBeLessThanOrEqual(paneBox.x + paneBox.width);
  expect(popupBox.y + popupBox.height).toBeLessThanOrEqual(paneBox.y + paneBox.height);
  const apply = popup.getByRole("button", { name: "Apply" });
  await expect(apply).toBeVisible();
  await apply.scrollIntoViewIfNeeded();
  const applyBox = (await apply.boundingBox())!;
  expect(applyBox.y + applyBox.height).toBeLessThanOrEqual(popupBox.y + popupBox.height);
  // Docked: the editor body sits above the popup and keeps its height.
  const bodyBox = (await page.locator(".editor-body").boundingBox())!;
  expect(bodyBox.y + bodyBox.height).toBeLessThanOrEqual(popupBox.y + 0.5);
  expect(bodyBox.height).toBeGreaterThanOrEqual(100);
});
