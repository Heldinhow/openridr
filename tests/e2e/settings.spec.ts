/**
 * F8 — Settings: one global dialog with search and a page tree, applied live and persisted.
 * Appearance, Keymap, Editor, Build and Version Control are covered here; the Keymap dialog
 * itself keeps its own spec in actions.spec.ts.
 */
import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { e2eHome, host } from "./environment";

const solution = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
const programTab = "src/OrderDesk.Cli/Program.cs";
const globalJson = path.resolve("fixtures/OrderDesk/global.json");
const settingsFile = path.join(e2eHome, "settings.json");

async function openTextOnly(page: Page) {
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("open-text-only").click();
}

async function openTrusted(page: Page) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-checkbox").check();
  await expect(page.getByTestId("trust-grant")).toBeEnabled();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
  await page.getByTestId("open-text-only").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function openSettings(page: Page) {
  await page.getByTestId("toolbar-settings").click();
  await expect(page.getByTestId("settings-dialog")).toBeVisible();
}

async function selectPage(page: Page, id: string) {
  await page.getByTestId(`settings-page-${id}`).click();
  await expect(page.getByTestId(`settings-page-${id}`)).toHaveAttribute("aria-current", "page");
}

function savedSettings() {
  return JSON.parse(fs.readFileSync(settingsFile, "utf8"));
}

function resetSettings() {
  fs.mkdirSync(e2eHome, { recursive: true });
  fs.writeFileSync(settingsFile, "{}\n");
}

test.beforeEach(async ({ request }) => {
  resetSettings();
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test.afterEach(() => {
  resetSettings();
});

test("Settings opens from the bar and ⌘,, searches by name and explains the missing light theme", async ({
  page, request,
}) => {
  await page.setViewportSize({ width: 1100, height: 700 });
  await page.goto("/");
  await openTextOnly(page);
  await page.keyboard.press("Meta+,");
  await expect(page.getByTestId("settings-dialog")).toBeVisible();
  await expect(page.getByTestId("settings-search")).toBeFocused();
  await expect(page.getByTestId("settings-scope")).toContainText("Global settings");
  await expect(page.getByTestId("settings-scope")).toContainText("no per-project override");

  // Appearance states the fixed theme without presenting a misleading selector.
  await expect(page.getByTestId("settings-page-title")).toHaveText("Appearance");
  await expect(page.getByTestId("settings-theme")).toHaveText("Dark");
  await expect(page.getByTestId("settings-theme").locator("select")).toHaveCount(0);
  await expect(page.getByTestId("settings-theme-note")).toContainText("unavailable");

  // Searching a field name shows the page that owns it and highlights the field.
  await page.getByTestId("settings-search").fill("git path");
  await expect(page.getByTestId("settings-page-vcs")).toHaveAttribute("aria-current", "page");
  await expect(page.getByTestId("settings-page-title")).toHaveText("Version Control");
  await expect(page.getByTestId("settings-field-gitPath")).toHaveClass(/match/);
  await expect(page.getByTestId("settings-no-results")).toHaveCount(0);

  await page.getByTestId("settings-search").fill("font size");
  await expect(page.getByTestId("settings-page-editor")).toHaveAttribute("aria-current", "page");

  await page.getByTestId("settings-search").fill("no such setting");
  await expect(page.getByTestId("settings-no-results")).toBeVisible();
  await page.getByTestId("settings-search").fill("");

  // The tree switches pages independently of the search.
  await selectPage(page, "keymap");
  await expect(page.getByTestId("keymap-active-profile")).toContainText("Rider (macOS)");
  await expect(page.getByTestId("keymap-shortcut-build.start")).toHaveText("⌘F9");
  await expect(page.getByTestId("keymap-table").locator("thead th")).toHaveText([
    "Action", "Group / shortcut", "Actions",
  ]);
  await expect(page.getByTestId("keymap-row-run.start").getByRole("rowheader")).toHaveText("Run");
  await expect(page.getByTestId("keymap-row-run.start").getByRole("button", { name: "Change shortcut for Run" })).toBeVisible();
  await expect(page.getByTestId("keymap-row-run.start").getByRole("button", { name: "Reset shortcut for Run" })).toBeDisabled();
  await expect(page.getByTestId("settings-scope")).not.toContainText("settings.json");
  const table = page.getByTestId("keymap-table");
  const dialog = page.getByTestId("settings-dialog");
  const tableBox = await table.boundingBox();
  const dialogBox = await dialog.boundingBox();
  expect(tableBox).not.toBeNull();
  expect(dialogBox).not.toBeNull();
  if (tableBox && dialogBox) {
    expect(tableBox.x + tableBox.width).toBeLessThanOrEqual(dialogBox.x + dialogBox.width + 1);
  }

  // Restricted mode exposes the PATH-resolved executable, but querying its version remains gated.
  await selectPage(page, "vcs");
  await expect(page.getByTestId("settings-git-path")).toContainText("git");
  await expect(page.getByTestId("settings-git-version")).toContainText("Trust the workspace to query Git");
  const gitInfo = await request.post(`${host}/call`, { data: { method: "git.info", params: {} } });
  expect((await gitInfo.json()).error.code).toBe("trust_required");

  await selectPage(page, "appearance");

  await page.getByTestId("settings-cancel").click();
  await expect(page.getByTestId("settings-dialog")).toHaveCount(0);
});

test("editor font, size and indentation apply live, reflect in the status bar and persist", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);
  await page.getByTestId(`tree-file-${programTab}`).click();
  await openSettings(page);
  await selectPage(page, "editor");

  await page.getByTestId("settings-font-family").selectOption("Menlo");
  await page.getByTestId("settings-font-size").fill("16");
  await page.getByTestId("settings-indent-style").selectOption("tabs");
  await page.getByTestId("settings-indent-size").fill("2");

  const scroller = page.locator(".cm-scroller").first();
  await expect
    .poll(() => scroller.evaluate((element) => getComputedStyle(element).fontSize))
    .toBe("16px");
  expect(await scroller.evaluate((element) => getComputedStyle(element).fontFamily)).toContain(
    "Menlo",
  );
  await expect(page.getByTestId("status-indent")).toHaveText("Tabs (2)");

  await page.getByTestId("settings-save").click();
  await expect(page.getByTestId("settings-dialog")).toHaveCount(0);
  const saved = savedSettings();
  expect(saved.editor).toEqual({
    fontFamily: "Menlo",
    fontSize: 16,
    indentSize: 2,
    useTabs: true,
  });

  // The live indentation unit really reaches CodeMirror: a tab is one column, not four.
  await page.getByTestId("editor").click();
  await page.keyboard.press("Home");
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("status-cursor")).toContainText("Col 2");

  await page.reload();
  await openTextOnly(page);
  await expect(page.getByTestId("status-indent")).toHaveText("Tabs (2)");
  await openSettings(page);
  await selectPage(page, "editor");
  await expect(page.getByTestId("settings-font-family")).toHaveValue("Menlo");
  await expect(page.getByTestId("settings-font-size")).toHaveValue("16");
  await expect(page.getByTestId("settings-indent-size")).toHaveValue("2");
  await expect(page.getByTestId("settings-indent-style")).toHaveValue("tabs");
});

test("appearance applies zoom and density live; Cancel discards and invalid values never write", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);
  await openSettings(page);
  await selectPage(page, "appearance");

  await page.getByTestId("settings-zoom").fill("110");
  await page.getByTestId("settings-density").selectOption("compact");
  const workspace = page.locator(".workspace-view");
  await expect
    .poll(() => workspace.evaluate((element) => getComputedStyle(element).zoom))
    .toBe("1.1");
  await expect(workspace).toHaveAttribute("data-density", "compact");

  await page.getByTestId("settings-cancel").click();
  await expect
    .poll(() => workspace.evaluate((element) => getComputedStyle(element).zoom))
    .toBe("1");
  await expect(workspace).toHaveAttribute("data-density", "comfortable");
  expect(savedSettings()).toEqual({});

  await openSettings(page);
  await selectPage(page, "editor");
  await page.getByTestId("settings-font-size").fill("99");
  await expect(page.getByTestId("settings-error")).toContainText("between 8 and 32");
  await expect(page.getByTestId("settings-save")).toBeDisabled();
  expect(savedSettings()).toEqual({});

  await page.getByTestId("settings-font-size").fill("15");
  await selectPage(page, "appearance");
  await page.getByTestId("settings-zoom").fill("120");
  await page.getByTestId("settings-density").selectOption("compact");
  await page.getByTestId("settings-save").click();
  await expect(page.getByTestId("settings-dialog")).toHaveCount(0);
  const saved = savedSettings();
  expect(saved.appearance).toEqual({ zoom: 120, density: "compact" });
  expect(saved.editor.fontSize).toBe(15);

  await page.reload();
  await openTextOnly(page);
  await expect
    .poll(() => workspace.evaluate((element) => getComputedStyle(element).zoom))
    .toBe("1.2");
  await expect(workspace).toHaveAttribute("data-density", "compact");
  await openSettings(page);
  await selectPage(page, "editor");
  await expect(page.getByTestId("settings-font-size")).toHaveValue("15");
});

test("keymap changes made in Settings stay in the draft until Save, persist and reset", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);
  await openSettings(page);
  await selectPage(page, "keymap");
  await expect(page.getByTestId("keymap-active-profile")).toContainText("Rider (macOS)");

  await page.getByTestId("keymap-change-build.start").click();
  await page.keyboard.press("Meta+Shift+F9");
  await expect(page.getByTestId("keymap-shortcut-build.start")).toHaveText("⇧⌘F9");
  expect(savedSettings(), "Settings must not write before Save").toEqual({});

  // Cancel discards a remap applied to the draft.
  await page.getByTestId("settings-cancel").click();
  await openSettings(page);
  await selectPage(page, "keymap");
  await expect(page.getByTestId("keymap-shortcut-build.start")).toHaveText("⌘F9");

  await page.getByTestId("keymap-change-build.start").click();
  await page.keyboard.press("Meta+Shift+F9");
  await page.getByTestId("settings-save").click();
  await expect(page.getByTestId("settings-dialog")).toHaveCount(0);
  expect(savedSettings().keymap.overrides["build.start"]).toEqual([
    { key: "f9", primary: true, control: false, alt: false, shift: true },
  ]);

  await page.reload();
  await openTextOnly(page);
  await openSettings(page);
  await selectPage(page, "keymap");
  await expect(page.getByTestId("keymap-shortcut-build.start")).toHaveText("⇧⌘F9");
  await page.getByTestId("keymap-reset-build.start").click();
  await expect(page.getByTestId("keymap-shortcut-build.start")).toHaveText("⌘F9");
  await page.getByTestId("settings-save").click();
  expect(savedSettings().keymap.overrides["build.start"]).toBeUndefined();
});

test("Build lists detected SDKs with Redetect and Version Control shows the git in use read-only", async ({
  page,
}) => {
  const globalJsonBefore = fs.readFileSync(globalJson, "utf8");
  await openTrusted(page);
  await openSettings(page);
  await selectPage(page, "build");

  await expect(page.getByTestId("settings-sdk-list")).toContainText("Host SDK");
  await expect(page.getByTestId("settings-sdk-list")).toContainText("Project SDK");
  await expect(page.getByTestId("settings-analysis-host")).not.toBeEmpty();
  await expect(page.getByTestId("settings-host-root")).toBeAttached();

  await page.getByTestId("settings-redetect").click();
  await expect(page.getByTestId("settings-redetect-message")).toContainText(
    "SDK inventory refreshed",
  );
  expect(fs.readFileSync(globalJson, "utf8")).toBe(globalJsonBefore);

  await selectPage(page, "vcs");
  await expect(page.getByTestId("settings-git-path")).toContainText("git");
  await expect(page.getByTestId("settings-git-version")).toContainText("git version");
  await expect(page.getByTestId("settings-git-message")).toContainText("Read-only");
  await expect(page.getByTestId("settings-git-path").locator("input")).toHaveCount(0);
});
