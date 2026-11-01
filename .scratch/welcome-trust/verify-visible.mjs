// Visible-Chrome walkthrough for welcome-trust. Usage (host on 4317 with a fresh OPENRIDR_HOME + OPENRIDR_TRUST_STORE=on, `bun run dev` on 1420):
//   node .scratch/welcome-trust/verify-visible.mjs
import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const out = path.resolve("docs/evidence/welcome-trust");
const ui = "http://localhost:1420";
const results = [];
const shot = (page, name) => page.screenshot({ path: path.join(out, name) });
const step = async (name, fn) => {
  try { await fn(); results.push(`ok      ${name}`); } catch (e) { results.push(`FAILED  ${name}: ${e.message.split("\n")[0]}`); }
};
const copy = (name) => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), `openridr-visible-${name}-`));
  fs.cpSync("fixtures/OrderDesk", base, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  const sln = path.join(base, `${name}.sln`);
  fs.renameSync(path.join(base, "OrderDesk.sln"), sln);
  return sln;
};
const A = copy("Alpha"), B = copy("Bravo");

const browser = await chromium.launch({ channel: "chrome", headless: false, slowMo: 150 });
const page = await (await browser.newContext({ viewport: { width: 1360, height: 860 } })).newPage();
const t = (id) => page.getByTestId(id);
const open = async (sln) => {
  await page.goto(ui);
  await t("workspace-path-input").fill(sln);
  await t("workspace-open").click();
};
const dialog = t("trust-prompt-dialog");

await step("1 welcome empty (first run)", async () => {
  await page.goto(ui);
  await t("welcome-empty").waitFor();
  assert.equal(await t("welcome-recent").count(), 0);
  await shot(page, "v01-welcome-empty.png");
});
await step("2 new project asks once (modal)", async () => {
  await open(A);
  await dialog.waitFor();
  assert.equal(await dialog.getAttribute("aria-modal"), "true");
  assert.equal(await t("tree").count(), 0);
  await shot(page, "v02-trust-prompt.png");
});
await step("3 Trust project: IDE opens, no Trusted/Restricted in top bar, status-mode + popover", async () => {
  await t("trust-prompt-trust").click();
  await t("tree").waitFor();
  assert.equal(await page.locator(".toolbar").getByText(/Restricted|Trusted/).count(), 0);
  assert.equal((await t("status-mode").textContent()).trim(), "Trusted mode");
  await shot(page, "v03-ide-trusted.png");
  await t("status-mode").click();
  await t("status-mode-popover").waitFor();
  await shot(page, "v03b-status-popover.png");
  await page.keyboard.press("Escape");
});
await step("4 reopen same project: no modal", async () => {
  await open(A);
  await t("tree").waitFor();
  assert.equal(await dialog.count(), 0);
  assert.equal((await t("status-mode").textContent()).trim(), "Trusted mode");
  await shot(page, "v04-reopen-no-prompt.png");
});
await step("5 second project Restricted; reopen does not ask", async () => {
  await open(B);
  await dialog.waitFor();
  await t("trust-prompt-restricted").click();
  await t("tree").waitFor();
  assert.equal((await t("status-mode").textContent()).trim(), "Restricted mode");
  await shot(page, "v05-ide-restricted.png");
  await open(B);
  await t("tree").waitFor();
  assert.equal(await dialog.count(), 0);
  assert.equal((await t("status-mode").textContent()).trim(), "Restricted mode");
});
const C = copy("Charlie");
await step("6 Cancel and Esc open nothing", async () => {
  await open(C);
  await dialog.waitFor();
  await t("trust-prompt-cancel").click();
  assert.equal(await dialog.count(), 0);
  assert.equal(await t("tree").count(), 0);
  await shot(page, "v06a-after-cancel.png");
  await t("workspace-open").click();
  await dialog.waitFor();
  await page.keyboard.press("Escape");
  assert.equal(await dialog.count(), 0);
  assert.equal(await t("tree").count(), 0);
  await shot(page, "v06b-after-esc.png");
});
await step("7 welcome with recents: badges, last opened, ⋯ menu (change trust without opening, remove)", async () => {
  await page.goto(ui);
  await t("welcome-count").waitFor();
  assert.equal((await t("welcome-count").textContent()).trim(), "3 projects");
  // Observation: Cancel (step 6) still registers Charlie in Recent, as "Not decided".
  const rows = await page.locator('[data-testid^="welcome-recent-"][data-testid$="-badge"]').allTextContents();
  assert.deepEqual(rows.sort(), ["Not decided", "Restricted", "Trusted"]);
  assert.match(await t("welcome-recent-0").textContent(), /just now|ago|min/);
  await shot(page, "v07a-welcome-recents.png");
  // Index 1 is Bravo (Restricted): flip it to Trusted from the menu.
  await t("welcome-recent-1-menu").click();
  await shot(page, "v07b-row-menu.png");
  await t("welcome-recent-1-menu-trust").click();
  await page.waitForFunction(() => document.querySelector('[data-testid="welcome-recent-1-badge"]')?.textContent === "Trusted");
  assert.equal(await t("tree").count(), 0, "menu must not open the project");
  await shot(page, "v07c-after-trust-change.png");
  await t("welcome-recent-0-menu").click();
  await t("welcome-recent-0-menu-remove").click();
  await page.waitForFunction(() => document.querySelector('[data-testid="welcome-count"]')?.textContent === "2 projects");
  await shot(page, "v07d-after-remove.png");
});

await browser.close();
console.log(results.join("\n"));
process.exit(results.some((r) => r.startsWith("FAILED")) ? 1 : 0);
