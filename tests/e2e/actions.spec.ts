import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { e2eHome, host } from "./environment";

const solution = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
const programTab = "src/OrderDesk.Cli/Program.cs";
const settingsFile = path.join(e2eHome, "settings.json");

async function openTextOnly(page: Page) {
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-restricted").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

function resetSettings() {
  fs.mkdirSync(e2eHome, { recursive: true });
  fs.writeFileSync(settingsFile, "{}\n");
}

async function openKeymap(page: Page) {
  await page.getByRole("button", { name: "Workspace actions" }).click();
  await page.getByTestId("open-keymap").click();
  await expect(page.getByTestId("keymap-dialog")).toBeVisible();
}

test.beforeEach(async ({ request }) => {
  resetSettings();
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test.afterEach(() => {
  resetSettings();
});

test("Find Action filters by title, id and group, shows shortcuts and disabled reasons, and runs", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);
  await page.getByTestId(`tree-file-${programTab}`).click();
  await page.getByTestId("editor").click();

  // The editor has focus: ⇧⌘A is a global action, not a CodeMirror one.
  await page.keyboard.press("Meta+Shift+A");
  const dialog = page.getByTestId("find-action");
  await expect(dialog).toBeVisible();
  await expect(page.getByTestId("find-action-input")).toBeFocused();
  await expect(page.getByTestId("find-action-profile")).toContainText("Rider (macOS)");
  await expect(page.getByTestId("find-action-shortcut-view.findAction")).toHaveText("⇧⌘A");
  await expect(page.getByTestId("find-action-shortcut-build.start")).toHaveText("⌘F9");

  await page.getByTestId("find-action-input").fill("problems");
  await expect(page.getByTestId("find-action-item-toolWindow.problems")).toBeVisible();
  await expect(page.getByTestId("find-action-item-build.start")).toHaveCount(0);
  await page.getByTestId("find-action-input").press("Enter");
  await expect(dialog).toHaveCount(0);
  await expect(page.getByTestId("bottom-tab-problems")).toHaveAttribute("aria-selected", "true");

  // Id filter and a disabled action without trust: reason visible, Enter does not run it.
  await page.keyboard.press("Meta+Shift+A");
  await page.getByTestId("find-action-input").fill("git.commit");
  await expect(page.getByTestId("find-action-reason-git.commit")).toContainText(
    "Trust the workspace to use Git",
  );
  await expect(
    page.getByTestId("find-action-item-git.commit").getByRole("button"),
  ).toBeDisabled();
  await page.getByTestId("find-action-input").press("Enter");
  await expect(dialog).toBeVisible();
  await page.getByTestId("find-action-input").press("Escape");
  await expect(dialog).toHaveCount(0);

  // Group filter lists the tool windows registered in F1.
  await page.keyboard.press("Meta+Shift+A");
  await page.getByTestId("find-action-input").fill("Tool Windows");
  for (const id of ["solution", "build", "tests"]) {
    await expect(page.getByTestId(`find-action-item-toolWindow.${id}`)).toBeVisible();
  }
  await page.getByTestId("find-action-input").press("Escape");

  // The same registry drives the ••• menu, and an unavailable entry keeps its reason.
  await page.getByRole("button", { name: "Workspace actions" }).click();
  await expect(page.getByTestId("git-commit-action")).toBeDisabled();
  await expect(page.getByTestId("git-commit-action")).toHaveAttribute(
    "title",
    /Trust the workspace to use Git/,
  );
});

test("Rider is the default profile and IntelliJ from settings.json changes shortcuts on reload", async ({
  page,
}) => {
  fs.writeFileSync(settingsFile, JSON.stringify({ keymap: { profile: "rider-macos" } }));
  await page.goto("/");
  await openTextOnly(page);
  await page.keyboard.press("Meta+Shift+A");
  await expect(page.getByTestId("find-action-shortcut-search.file")).toHaveText("⇧⌘O");
  await expect(page.getByTestId("find-action-shortcut-search.everywhere")).toHaveText("⌘T");
  await expect(page.getByTestId("find-action-shortcut-run.start")).toContainText("⌃F5");
  await expect(page.getByTestId("find-action-shortcut-git.commit")).toContainText("⌥⇧⌘K");
  await page.getByTestId("find-action-input").press("Escape");

  fs.writeFileSync(settingsFile, JSON.stringify({ keymap: { profile: "intellij" } }));
  await page.reload();
  await openTextOnly(page);
  await page.keyboard.press("Meta+Shift+A");
  await expect(page.getByTestId("find-action-profile")).toContainText("IntelliJ");
  await expect(page.getByTestId("find-action-shortcut-search.file")).toHaveText("⌘O");
  await expect(page.getByTestId("find-action-shortcut-git.commit")).toHaveText("⌘K");
  await expect(page.getByTestId("find-action-shortcut-search.everywhere")).toHaveText(
    "⌘T",
  );
  await page.getByTestId("find-action-input").press("Escape");
});

test("keymap captures a key, persists in settings.json, warns on conflicts and resets", async ({
  page,
}) => {
  await page.goto("/");
  await openTextOnly(page);
  await openKeymap(page);
  await expect(page.getByTestId("keymap-profile")).toHaveValue("rider-macos");
  await expect(page.getByTestId("keymap-active-profile")).toContainText("Rider (macOS)");

  await page.getByTestId("keymap-change-build.start").click();
  await expect(page.getByTestId("keymap-change-build.start")).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await page.keyboard.press("Meta+Shift+F9");
  await expect(page.getByTestId("keymap-shortcut-build.start")).toHaveText("⇧⌘F9");
  let saved = JSON.parse(fs.readFileSync(settingsFile, "utf8"));
  expect(saved.keymap.overrides["build.start"]).toEqual([
    { key: "f9", primary: true, control: false, alt: false, shift: true },
  ]);
  // The user remap wins over a profile change too.
  await page.getByTestId("keymap-profile").selectOption("intellij");
  await expect(page.getByTestId("keymap-shortcut-build.start")).toHaveText("⇧⌘F9");

  // Both stay listed and remappable; the registry warns on each side.
  await page.getByTestId("keymap-change-run.start").click();
  await page.keyboard.press("Meta+Shift+F9");
  await expect(page.getByTestId("keymap-conflict-build.start")).toContainText("Run");
  await expect(page.getByTestId("keymap-conflict-run.start")).toContainText("Build");
  await expect(page.getByTestId("keymap-row-run.start")).toBeVisible();
  await expect(page.getByTestId("keymap-row-build.start")).toBeVisible();

  await page.getByTestId("keymap-reset-run.start").click();
  await expect(page.getByTestId("keymap-shortcut-run.start")).toContainText("⌃F5");
  // Build Solution's default is ⇧⌘F9, the key the test remapped Build onto.
  await expect(page.getByTestId("keymap-conflict-build.start")).toContainText("Build Solution");

  // Reload keeps the user remap, which wins over both profiles.
  await page.getByTestId("keymap-close").click();
  await page.reload();
  await openTextOnly(page);
  await openKeymap(page);
  await expect(page.getByTestId("keymap-shortcut-build.start")).toHaveText("⇧⌘F9");
  await page.getByTestId("keymap-reset-build.start").click();
  await expect(page.getByTestId("keymap-shortcut-build.start")).toHaveText("⌘F9");
  saved = JSON.parse(fs.readFileSync(settingsFile, "utf8"));
  expect(saved.keymap.overrides["build.start"]).toBeUndefined();
});

test("two actions on one shortcut stay listed as a warning and the first registered runs", async ({
  page,
}) => {
  fs.writeFileSync(
    settingsFile,
    JSON.stringify({
      keymap: {
        profile: "rider-macos",
        overrides: { "view.keymap": [{ key: "a", primary: true, shift: true }] },
      },
    }),
  );
  await page.goto("/");
  await openTextOnly(page);

  await page.keyboard.press("Meta+Shift+A");
  await expect(page.getByTestId("find-action")).toBeVisible();
  await expect(page.getByTestId("keymap-dialog")).toHaveCount(0);
  await expect(page.getByTestId("find-action-conflict-view.findAction")).toContainText(
    "Keymap: change shortcuts",
  );
  await expect(page.getByTestId("find-action-conflict-view.keymap")).toContainText(
    "Find Action",
  );

  // The losing action is still listed and runs from the palette.
  await page.getByTestId("find-action-input").fill("keymap: change");
  await page.getByTestId("find-action-input").press("Enter");
  await expect(page.getByTestId("keymap-dialog")).toBeVisible();
  await expect(page.getByTestId("keymap-conflict-view.findAction")).toBeVisible();
});

test("a remapped global shortcut outranks CodeMirror while ⌘F, arrows and Tab stay in the editor", async ({
  page,
}) => {
  // Build is remapped onto ⌘/; the app handler runs in capture and consumes it before the
  // CodeMirror comment key, so the buffer must not gain a comment.
  fs.writeFileSync(
    settingsFile,
    JSON.stringify({
      keymap: {
        profile: "rider-macos",
        overrides: { "build.start": [{ key: "/", primary: true }] },
      },
    }),
  );
  await page.goto("/");
  await openTextOnly(page);
  await page.getByTestId(`tree-file-${programTab}`).click();
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+A");
  await page.keyboard.type("class A {}");
  await page.keyboard.press("Meta+/");
  await expect(page.getByTestId("editor")).toHaveText("class A {}");
  await expect(page.getByTestId("app-log")).toContainText("Build unavailable: Trust the workspace");

  // Keys the app does not register still belong to CodeMirror.
  await page.keyboard.press("Meta+f");
  await expect(page.locator(".cm-search")).toBeVisible();
  await page.keyboard.press("Escape");

  await page.keyboard.press("Home");
  await page.keyboard.press("ArrowRight");
  await expect(page.getByTestId("status-cursor")).toContainText("Col 2");
  const before = (await page.getByTestId("status-cursor").textContent()) ?? "";
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("status-cursor")).not.toHaveText(before);
});

test("settings.read and settings.write live in the app home, reject invalid payloads and need no trust", async ({
  request,
}) => {
  // The workspace is closed in beforeEach: no trust was granted.
  const written = await request.post(`${host}/call`, {
    data: {
      method: "settings.write",
      params: {
        settings: {
          keymap: {
            profile: "intellij",
            overrides: { "run.start": [{ key: "f5", control: true }] },
          },
        },
      },
    },
  });
  const body = await written.json();
  expect(body.ok, JSON.stringify(body.error)).toBe(true);
  const file = JSON.parse(fs.readFileSync(settingsFile, "utf8"));
  expect(file.keymap.profile).toBe("intellij");
  expect(file.keymap.overrides["run.start"]).toEqual([{ key: "f5", control: true }]);

  const read = await request.post(`${host}/call`, {
    data: { method: "settings.read", params: {} },
  });
  const readBody = await read.json();
  expect(readBody.ok).toBe(true);
  expect(readBody.data.path).toBe(settingsFile);
  expect(readBody.data.settings.keymap.profile).toBe("intellij");

  const invalid = await request.post(`${host}/call`, {
    data: { method: "settings.write", params: { settings: [] } },
  });
  const invalidBody = await invalid.json();
  expect(invalidBody.ok).toBe(false);
  expect(invalidBody.error.code).toBe("invalid_settings");
  expect(JSON.parse(fs.readFileSync(settingsFile, "utf8"))).toEqual(file);
});
