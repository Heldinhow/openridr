import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { hostPort, seedVerifiedTools } from "./environment";
import { startIsolatedHost, type IsolatedHost } from "./isolated-host";

// The shared E2E host runs with OPENRIDR_TRUST_STORE=off (every open asks). This spec needs the real
// per-project store, so it starts its own host on an isolated OPENRIDR_HOME and points the UI at it.
const port = hostPort + 1;
const fixture = path.resolve("fixtures/OrderDesk");
let home: string;
let host: IsolatedHost;

test.describe.configure({ mode: "serial" });

test.beforeAll(async () => {
  host = await startIsolatedHost({
    port,
    homePrefix: "openridr-trust-home-",
    env: { OPENRIDR_TRUST_STORE: "on" },
    prepare: (prepared) => ({ OPENRIDR_BUNDLED_NETCOREDBG: seedVerifiedTools(prepared) }),
  });
  home = host.home;
});

test.afterAll(async () => {
  await host?.stop();
});

function projectCopy() {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-trust-prompt-"));
  fs.cpSync(fixture, base, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  return path.join(base, "OrderDesk.sln");
}

async function openPath(page: Page, solution: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
}

async function hostCall(page: Page, method: string, params: object = {}) {
  const body = JSON.stringify({ method, params });
  return page.evaluate(async (payload) => {
    const response = await fetch("/__host/call", { method: "POST", headers: { "content-type": "application/json" }, body: payload });
    return (await response.json()).data;
  }, body);
}

async function openedWorkspace(page: Page) {
  const state = await page.evaluate(async () => {
    const response = await fetch("/__host/call", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ method: "app.state", params: {} }) });
    return response.json();
  });
  return state.data.workspace as { path: string } | null;
}

test("asks once for a new project, then reopening goes straight in", async ({ page }) => {
  await host.useFor(page);
  const solution = projectCopy();
  await openPath(page, solution);
  const dialog = page.getByTestId("trust-prompt-dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveAttribute("role", "dialog");
  await expect(dialog).toHaveAttribute("aria-modal", "true");
  await expect(page.getByTestId("trust-prompt-trust")).toBeFocused();
  // Nothing is entered before the decision.
  await expect(page.getByTestId("tree")).toHaveCount(0);

  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");

  await openPath(page, solution);
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
});

test("a Restricted decision is remembered and not asked again", async ({ page }) => {
  await host.useFor(page);
  const solution = projectCopy();
  await openPath(page, solution);
  await page.getByTestId("trust-prompt-restricted").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");

  await openPath(page, solution);
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(page.getByTestId("trust-prompt-dialog")).toHaveCount(0);
  await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");
});

test("Cancel and Esc return to the welcome screen without opening the project", async ({ page }) => {
  await host.useFor(page);
  const solution = projectCopy();
  await openPath(page, solution);
  await page.getByTestId("trust-prompt-cancel").click();
  await expect(page.getByTestId("trust-prompt-dialog")).toHaveCount(0);
  await expect(page.getByTestId("workspace-path-input")).toBeVisible();
  await expect(page.getByTestId("tree")).toHaveCount(0);
  expect(await openedWorkspace(page)).toBeNull();
  // Cancel leaves no trace: not in Recents, no stored decision.
  const key = fs.realpathSync(solution);
  const listed = (await hostCall(page, "recent.list")).projects as { path: string }[];
  expect(listed.map((project) => project.path)).not.toContain(key);
  expect((await hostCall(page, "trust.get", { path: key })).decision).toBeNull();

  // Cancel stores nothing, so the next open asks again; this time dismiss with Esc.
  await page.getByTestId("workspace-open").click();
  await expect(page.getByTestId("trust-prompt-dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("trust-prompt-dialog")).toHaveCount(0);
  await expect(page.getByTestId("tree")).toHaveCount(0);
  expect(await openedWorkspace(page)).toBeNull();
});

test("Cancel on a prompt raised from inside the IDE restores the previous workspace", async ({ page }) => {
  await host.useFor(page);
  const current = projectCopy();
  const other = projectCopy();
  // A legacy Recents entry with no decision: opening it from the IDE raises the prompt.
  fs.writeFileSync(path.join(home, "recent.json"), JSON.stringify([other]));
  await openPath(page, current);
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  await page.getByTestId("toolbar-project").click();
  await page.getByTestId("recent-solution-1").click();
  await expect(page.getByTestId("trust-prompt-dialog")).toBeVisible();
  await page.getByTestId("trust-prompt-cancel").click();
  await expect(page.getByTestId("trust-prompt-dialog")).toHaveCount(0);
  await expect(page.getByTestId("tree")).toBeVisible();
  expect((await openedWorkspace(page))?.path).toBe(fs.realpathSync(current));
});

test("the page behind the prompt is inert and a backdrop click keeps focus in the dialog", async ({ page }) => {
  await host.useFor(page);
  await openPath(page, projectCopy());
  await expect(page.getByTestId("trust-prompt-trust")).toBeFocused();
  await expect(page.locator("[inert]")).toHaveCount(1);
  await page.locator(".modal-backdrop").click({ position: { x: 4, y: 4 } });
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("trust-prompt-dialog").locator(":focus")).toHaveCount(1);
  await page.getByTestId("trust-prompt-cancel").click();
  await expect(page.locator("[inert]")).toHaveCount(0);
});

test("focus is trapped: Tab cycles Trust, Restricted, Cancel and Enter activates the focused button", async ({ page }) => {
  await host.useFor(page);
  await openPath(page, projectCopy());
  await expect(page.getByTestId("trust-prompt-trust")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("trust-prompt-restricted")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("trust-prompt-cancel")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("trust-prompt-trust")).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(page.getByTestId("trust-prompt-cancel")).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("trust-prompt-dialog")).toHaveCount(0);
  expect(await openedWorkspace(page)).toBeNull();
});
