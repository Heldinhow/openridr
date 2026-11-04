import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

const fixture = path.resolve("fixtures/OrderDesk");
const program = "src/OrderDesk.Cli/Program.cs";
const order = "src/OrderDesk.Core/Order.cs";
const originalProgram = fs.readFileSync(path.join(fixture, program), "utf8");
const temporary: string[] = [];

test.use({ viewport: { width: 1440, height: 900 } });
test.describe.configure({ timeout: 180_000 });

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-editor-state-"));
  temporary.push(base);
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return { copy, solution: path.join(copy, "OrderDesk.sln") };
}

async function openWorkspace(page: Page, target: string, trusted: boolean) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId(trusted ? "trust-prompt-trust" : "trust-prompt-restricted").click();
  if (trusted) await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function hostCall(request: APIRequestContext, method: string, params = {}) {
  const response = await request.post(`${host}/call`, { data: { method, params } });
  return response.json();
}

async function closeWorkspace(page: Page, request: APIRequestContext) {
  const closed = await hostCall(request, "workspace.close");
  expect(closed.ok, JSON.stringify(closed.error)).toBe(true);
  await expect(page.getByTestId("workspace-path-input")).toBeVisible({ timeout: 15_000 });
}

async function replaceProgram(page: Page, text: string) {
  await page.getByTestId(`tree-file-${program}`).click();
  await expect(page.getByTestId("editor")).toBeVisible();
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+A");
  await page.keyboard.insertText(text);
  await page.keyboard.press("Meta+s");
  await expect(page.getByTestId("app-log")).toContainText(`Saved ${program}`);
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
  for (const directory of temporary.splice(0)) {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});

test("d13: a double click on a closed file opens one tab", async ({ page }) => {
  const { solution } = tempCopy("Order Desk dedupe");
  await openWorkspace(page, solution, false);
  await page.getByTestId(`tree-file-${program}`).dblclick();
  await expect(page.getByTestId(`tab-${program}`)).toBeVisible();
  await expect(page.getByTestId(`tab-${program}`)).toHaveCount(1);
  await expect(page.getByTestId("status-dirty")).toHaveCount(0);
});

test("b14: a clean buffer reloads from disk and a dirty one keeps the conflict", async ({ page }) => {
  const { copy, solution } = tempCopy("Order Desk reload");
  await openWorkspace(page, solution, false);
  await page.getByTestId(`tree-file-${program}`).click();
  await expect(page.getByTestId(`tab-${program}`)).toBeVisible();
  await page.getByTestId(`tree-file-${order}`).click();
  await expect(page.getByTestId(`tab-${order}`)).toBeVisible();

  fs.writeFileSync(path.join(copy, program), `// clean-disk-reload\n${originalProgram}`);
  await page.getByTestId(`tab-${program}`).click();
  await expect(page.getByTestId("editor")).toContainText("clean-disk-reload");
  await expect(page.getByTestId("conflict-banner")).toHaveCount(0);
  await expect(page.getByTestId("status-dirty")).toHaveCount(0);

  // A clean reload keeps the caret on its line and is not an undo step.
  await page.locator(".cm-line").nth(4).click();
  await expect(page.getByTestId("status-cursor")).toContainText("Ln 5");
  fs.writeFileSync(path.join(copy, program), `// second-clean-disk-reload\n${originalProgram}`);
  await page.getByTestId(`tab-${program}`).click();
  await expect(page.getByTestId("editor")).toContainText("second-clean-disk-reload");
  await expect(page.getByTestId("status-cursor")).toContainText("Ln 5");
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+Z");
  await expect(page.getByTestId("editor")).toContainText("second-clean-disk-reload");

  // A dirty buffer is never replaced silently; the existing conflict choice stays.
  await page.getByTestId("editor").click();
  await page.keyboard.press("Control+Home");
  await page.keyboard.insertText("// dirty-buffer-kept\n");
  fs.writeFileSync(path.join(copy, program), "// second-disk-version\n");
  await page.getByTestId(`tab-${order}`).click();
  await page.getByTestId(`tab-${program}`).click();
  await expect(page.getByTestId("conflict-banner")).toBeVisible();
  await expect(page.getByTestId("editor")).toContainText("dirty-buffer-kept");
  await expect(page.getByTestId("editor")).not.toContainText("second-disk-version");
});

test("b16: saving preserves CRLF and UTF-16 encodings", async ({ page }) => {
  const { copy, solution } = tempCopy("Order Desk encodings");
  const crlfFile = path.join(copy, program);
  const utf16File = path.join(copy, order);
  fs.writeFileSync(crlfFile, "// crlf one\r\n// crlf two\r\n");
  fs.writeFileSync(
    utf16File,
    Buffer.concat([
      Buffer.from([0xff, 0xfe]),
      Buffer.from("// utf16 one\r\n// utf16 two\r\n", "utf16le"),
    ]),
  );
  await openWorkspace(page, solution, false);

  await replaceProgram(page, "// crlf saved\n");
  expect(fs.readFileSync(crlfFile, "utf8")).toBe("// crlf saved\r\n");

  await page.getByTestId(`tree-file-${order}`).click();
  await expect(page.getByTestId("editor")).toContainText("// utf16 one");
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+A");
  await page.keyboard.insertText("// utf16 saved\n");
  await page.keyboard.press("Meta+S");
  await expect(page.getByTestId("app-log")).toContainText(`Saved ${order}`);
  const saved = fs.readFileSync(utf16File);
  expect([...saved.subarray(0, 2)]).toEqual([0xff, 0xfe]);
  expect(saved.subarray(2).toString("utf16le")).toBe("// utf16 saved\r\n");
});

test("b15: small windows leave the editor visible and do not overflow", async ({ page }) => {
  const { solution } = tempCopy("Order Desk small");
  for (const viewport of [
    { width: 800, height: 520 },
    { width: 560, height: 420 },
  ]) {
    await page.setViewportSize(viewport);
    await openWorkspace(page, solution, false);
    await page.getByTestId(`tree-file-${program}`).click();
    await expect(page.getByTestId("editor")).toBeVisible();

    const editor = (await page.getByRole("region", { name: "Editor", exact: true }).boundingBox())!;
    expect(editor.height).toBeGreaterThanOrEqual(170);
    const tab = page.getByTestId(`tab-${program}`);
    await expect(tab).toBeVisible();
    const tabBox = (await tab.boundingBox())!;
    expect(tabBox.width).toBeGreaterThan(40);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(viewport.width);
    await closeWorkspace(page, page.request);
  }
});

test("d19: Problems counts build errors once and the default panel is tall enough", async ({ page }) => {
  const { solution } = tempCopy("Order Desk problems");
  await openWorkspace(page, solution, true);
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", { timeout: 60_000 });
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });

  await replaceProgram(page, `${originalProgram}\nnew Missing();\n`);
  await page.getByTestId("bottom-tab-build").click();
  await page.getByTestId("build-action").click();
  await expect(page.getByTestId("build-state")).toContainText("Build failed", { timeout: 60_000 });
  await page.getByTestId("bottom-tab-problems").click();

  await expect(page.getByTestId("problems-counts")).toContainText("1 error");
  await expect(page.getByTestId("status-diagnostics")).toContainText("1 error");
  await expect(page.getByTestId("bottom-tab-problems")).toContainText("Problems (1)");
  await expect(page.getByTestId("problems-panel")).not.toContainText(
    "No analysis session has published diagnostics",
  );

  // Without a stored layout, the real default is a panel with room for several rows.
  await page.evaluate(() => localStorage.removeItem("openridr.layout.v1"));
  await openWorkspace(page, solution, true);
  await page.getByTestId("toolband-problems").click();
  const output = (await page.getByRole("region", { name: "Bottom panel" }).boundingBox())!;
  expect(output.height).toBe(320);
});

test("b13: a missing project SDK says what to install and offers actions", async ({ page, request }) => {
  const target = path.resolve("fixtures/MissingSdk/MissingSdk.csproj");
  await openWorkspace(page, target, true);
  await page.getByTestId("workspace-tools-toggle").click();

  const reason = page.getByTestId("eval-disabled-reason");
  await expect(reason).toContainText("global.json requests 8.0.999");
  await expect(reason).toContainText("never installs");
  await expect(reason).not.toContainText("SDK 10");
  const download = page.getByTestId("eval-sdk-download");
  await expect(download).toHaveAttribute("href", "https://dotnet.microsoft.com/download/dotnet");
  await expect(page.getByTestId("eval-open-global-json")).toBeVisible();

  await page.getByTestId("eval-redetect").click();
  const state = await hostCall(request, "workspace.state");
  expect(state.data.sdk.project.state).toBe("sdk_missing");
});

test("b10: opening the Terminal tab focuses the shell", async ({ page }) => {
  const { solution } = tempCopy("Order Desk terminal focus");
  await openWorkspace(page, solution, true);
  await page.getByTestId("bottom-tab-terminal").click();
  await expect(page.getByTestId("terminal-state")).toHaveText("Running", { timeout: 30_000 });
  await expect(page.locator(".xterm-helper-textarea")).toBeFocused();
  await page.keyboard.type("echo focus-ok");
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("terminal-output")).toContainText("focus-ok");
});
