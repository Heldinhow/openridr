import { expect, test, type Page } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { hostPort } from "./environment";

// The welcome lists what the host stored, so this spec runs its own host with a real, isolated
// OPENRIDR_HOME (recent.json + trusted.json) and points the UI at it, like trust-prompt.spec.ts.
const port = hostPort + 2;
const fixture = path.resolve("fixtures/OrderDesk");
let home: string;
let server: ChildProcess;
const bases: string[] = [];

test.describe.configure({ mode: "serial" });

test.beforeAll(async () => {
  home = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-welcome-home-"));
  server = spawn("cargo", ["run", "-p", "openridr-host-http", "--", "--port", String(port)], {
    env: { ...process.env, OPENRIDR_HOME: home, OPENRIDR_TRUST_STORE: "on" },
    stdio: "ignore",
  });
  await expect.poll(async () => fetch(`http://127.0.0.1:${port}/health`).then((r) => r.ok, () => false), {
    timeout: 300_000,
  }).toBe(true);
});

test.afterAll(() => {
  server?.kill();
  fs.rmSync(home, { recursive: true, force: true });
  for (const base of bases) fs.rmSync(base, { recursive: true, force: true });
});

test.beforeEach(async () => {
  await hostCall("workspace.close");
  for (const file of ["recent.json", "trusted.json"]) fs.rmSync(path.join(home, file), { force: true });
});

async function hostCall(method: string, params = {}) {
  const response = await fetch(`http://127.0.0.1:${port}/call`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ method, params }),
  });
  return (await response.json()) as { data: any };
}

async function useIsolatedHost(page: Page) {
  await page.route(/\/__host\/(call|health)$/, async (route) => {
    const target = `http://127.0.0.1:${port}/${route.request().url().split("/").pop()}`;
    await route.fetch({ url: target }).then((response) => route.fulfill({ response }), () => {});
  });
}

// A copy of the fixture whose solution is called `<name>.sln`, so rows are told apart by name.
function project(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-welcome-"));
  bases.push(base);
  fs.cpSync(fixture, base, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  const solution = path.join(base, `${name}.sln`);
  fs.renameSync(path.join(base, "OrderDesk.sln"), solution);
  return { base, solution };
}

// Registers a decided project in the recent list through the host (oldest first). Only decided
// projects are recorded: an undecided open leaves no trace.
async function seed(solution: string, decision: "trusted" | "restricted") {
  await hostCall("trust.set", { path: solution, decision });
  await hostCall("workspace.open", { path: solution });
  await hostCall("workspace.close");
}

async function storedDecision(solution: string) {
  return (await hostCall("trust.get", { path: solution })).data.decision as string | null;
}

test("first run: empty state with Open highlighted and no list", async ({ page }) => {
  await useIsolatedHost(page);
  await page.goto("/");
  await expect(page.getByTestId("welcome-empty")).toBeVisible();
  await expect(page.getByTestId("welcome-recent")).toHaveCount(0);
  await expect(page.getByTestId("welcome-count")).toHaveCount(0);
  await expect(page.getByTestId("workspace-pick")).toHaveClass(/primary/);
});

test("rows show name, short path, last opened and trust badge; search filters and counts", async ({ page }) => {
  await useIsolatedHost(page);
  const billing = project("Billing");
  const inventory = project("Inventory");
  await seed(billing.solution, "restricted");
  await seed(inventory.solution, "trusted");
  await page.goto("/");

  await expect(page.getByTestId("welcome-count")).toHaveText("2 projects");
  // Newest first.
  const first = page.getByTestId("welcome-recent-0");
  await expect(first).toContainText("Inventory");
  await expect(first).toContainText("just now");
  await expect(first).toHaveAttribute("title", fs.realpathSync(inventory.solution));
  await expect(page.getByTestId("welcome-recent-0-badge")).toHaveText("Trusted");
  await expect(page.getByTestId("welcome-recent-1-badge")).toHaveText("Restricted");
  if (process.env.OPENRIDR_SCREENSHOT_DIR) {
    await page.screenshot({ path: path.join(process.env.OPENRIDR_SCREENSHOT_DIR, "welcome-projects.png") });
  }

  const search = page.getByTestId("welcome-search");
  await search.fill("bill");
  await expect(page.getByTestId("welcome-count")).toHaveText("1 of 2 projects");
  await expect(page.getByTestId("welcome-recent-0")).toContainText("Billing");
  await expect(page.getByTestId("welcome-recent-1")).toHaveCount(0);

  await search.fill("zzz-nothing");
  await expect(page.getByTestId("welcome-count")).toHaveText("0 of 2 projects");
  await expect(page.getByTestId("welcome-no-match")).toBeVisible();

  await search.press("Escape");
  await expect(search).toHaveValue("");
  await expect(page.getByTestId("welcome-count")).toHaveText("2 projects");
});

test("the ⋯ menu changes the Trust decision without opening, and removes only the list entry", async ({ page }) => {
  await useIsolatedHost(page);
  const { solution } = project("Menu");
  await seed(solution, "restricted");
  await page.goto("/");
  await expect(page.getByTestId("welcome-recent-0-badge")).toHaveText("Restricted");

  await page.getByTestId("welcome-recent-0-menu").click();
  await expect(page.getByTestId("welcome-recent-0-menu-restrict")).toHaveCount(0);
  await page.getByTestId("welcome-recent-0-menu-trust").click();
  await expect(page.getByTestId("welcome-recent-0-badge")).toHaveText("Trusted");
  expect(await storedDecision(solution)).toBe("trusted");
  expect((await hostCall("app.state")).data.workspace).toBeNull();

  await page.getByTestId("welcome-recent-0-menu").click();
  await page.getByTestId("welcome-recent-0-menu-restrict").click();
  await expect(page.getByTestId("welcome-recent-0-badge")).toHaveText("Restricted");
  expect(await storedDecision(solution)).toBe("restricted");

  // Esc closes the menu and returns focus to its button.
  await page.getByTestId("welcome-recent-0-menu").click();
  await expect(page.getByRole("menu")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("menu")).toHaveCount(0);
  await expect(page.getByTestId("welcome-recent-0-menu")).toBeFocused();

  await page.getByTestId("welcome-recent-0-menu").click();
  await page.getByTestId("welcome-recent-0-menu-remove").click();
  await expect(page.getByTestId("welcome-empty")).toBeVisible();
  expect(await storedDecision(solution)).toBe("restricted");
  expect(fs.existsSync(solution)).toBe(true);
});

test("a project that cannot be opened shows the error on its own row", async ({ page }) => {
  await useIsolatedHost(page);
  const gone = project("Gone");
  const kept = project("Kept");
  await seed(gone.solution, "trusted");
  await seed(kept.solution, "trusted");
  await page.goto("/");
  await expect(page.getByTestId("welcome-recent-1")).toContainText("Gone");

  fs.rmSync(gone.base, { recursive: true, force: true });
  await page.getByTestId("welcome-recent-1").click();
  await expect(page.getByTestId("welcome-recent-1-error")).toBeVisible();
  await expect(page.getByTestId("welcome-recent-0-error")).toHaveCount(0);
  await expect(page.getByTestId("workspace-error")).toHaveCount(0);
  // The list stays usable and the broken entry can be removed.
  await page.getByTestId("welcome-recent-1-menu").click();
  await page.getByTestId("welcome-recent-1-menu-remove").click();
  await expect(page.getByTestId("welcome-recent-1")).toHaveCount(0);
});

test("keyboard: ↑/↓ move between rows, Enter opens, / focuses search, ⌘O asks to open a folder", async ({ page }) => {
  await useIsolatedHost(page);
  await seed(project("Alpha").solution, "trusted");
  await seed(project("Beta").solution, "trusted");
  // The real chooser is a native macOS dialog; answer it as "cancelled" and count the asks.
  const picks: string[] = [];
  await page.route(/\/__host\/call$/, async (route) => {
    if (!(route.request().postData() ?? "").includes("workspace.pick")) return route.fallback();
    picks.push("pick");
    await route.fulfill({ json: { ok: true, data: { path: null } } });
  });
  await page.goto("/");
  await expect(page.getByTestId("welcome-recent-1")).toBeVisible();

  await page.getByTestId("welcome-nav-projects").focus();
  await page.keyboard.press("/");
  await expect(page.getByTestId("welcome-search")).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(page.getByTestId("welcome-recent-0")).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(page.getByTestId("welcome-recent-1")).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await expect(page.getByTestId("welcome-recent-0")).toBeFocused();

  await page.keyboard.press("Control+o");
  await expect.poll(() => picks.length).toBeGreaterThan(0);

  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
});

test("Toolchain lives in the second view and Projects returns", async ({ page }) => {
  await useIsolatedHost(page);
  await page.goto("/");
  await expect(page.getByTestId("welcome-nav-projects")).toHaveAttribute("aria-current", "page");
  await expect(page.getByTestId("toolchain")).toBeHidden();
  await page.getByTestId("welcome-nav-toolchain").click();
  await expect(page.getByTestId("welcome-nav-toolchain")).toHaveAttribute("aria-current", "page");
  await expect(page.getByTestId("toolchain")).toBeVisible();
  await expect(page.getByTestId("sdk-host")).toBeAttached();
  await expect(page.getByTestId("sdk-redetect")).toBeAttached();
  await expect(page.getByTestId("workspace-pick")).toBeHidden();
  await page.getByTestId("welcome-nav-projects").click();
  await expect(page.getByTestId("workspace-pick")).toBeVisible();
  await expect(page.getByTestId("toolchain")).toBeHidden();
});
