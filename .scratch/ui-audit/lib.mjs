import { chromium } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

export const ROOT = "/Users/helder/Projetos/openridr";
export const OUT = path.join(ROOT, "docs/evidence/ui-audit");
export const TEXT = path.join(ROOT, ".scratch/ui-audit/text");
export const BASE = "/var/folders/n4/bjx54c1d3_d7438735kj02sr0000gn/T/openridr-ui-audit";
export const PLAIN = path.join(BASE, "orderdesk/OrderDesk.sln");
export const GIT = path.join(BASE, "orderdesk-git/OrderDesk.sln");
export const URL = "http://127.0.0.1:1420/";

export async function createAudit(section) {
  fs.mkdirSync(OUT, { recursive: true });
  fs.mkdirSync(TEXT, { recursive: true });
  const results = [];
  const failures = [];
  const started = Date.now();
  const browser = await chromium.launch({
    channel: "chrome",
    headless: false,
    slowMo: 300,
    args: ["--window-position=30,30", "--window-size=1640,1080"],
  });
  const context = await browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1 });
  const page = await context.newPage();
  page.setDefaultTimeout(25_000);

  const slug = (s) => s.replace(/[^a-z0-9-]+/gi, "-").toLowerCase();

  async function reveal(locator) {
    for (const details of await locator.locator("xpath=ancestor::details").all()) {
      if ((await details.getAttribute("open")) === null) {
        await details.locator(":scope > summary").click();
      }
    }
  }
  async function click(id, opts) {
    const el = page.getByTestId(id);
    await reveal(el);
    await el.click(opts);
  }
  async function metrics() {
    return page.evaluate(() => {
      const vw = innerWidth, vh = innerHeight;
      const vis = (el) => {
        const r = el.getBoundingClientRect();
        const s = getComputedStyle(el);
        return r.width > 0 && r.height > 0 && s.visibility !== "hidden" && s.display !== "none" && Number(s.opacity || 1) > 0.05;
      };
      const clipped = [];
      const unlabeled = [];
      const small = [];
      const seen = new Set();
      for (const el of document.querySelectorAll("body *")) {
        if (!vis(el)) continue;
        const r = el.getBoundingClientRect();
        const s = getComputedStyle(el);
        const text = (el.textContent ?? "").trim();
        if (text && el.children.length === 0 && el.scrollWidth > el.clientWidth + 2 && /hidden|clip/.test(s.overflowX)) {
          const key = `${text}|${Math.round(r.x)}|${Math.round(r.y)}`;
          if (!seen.has(key)) {
            seen.add(key);
            clipped.push({ cls: String(el.className).slice(0, 30), text: text.slice(0, 60), w: el.clientWidth, sw: el.scrollWidth });
          }
        }
        if (["BUTTON", "SUMMARY", "A", "SELECT"].includes(el.tagName) && (r.height < 22 || r.width < 22)) {
          small.push({ tag: el.tagName.toLowerCase(), text: (el.getAttribute("aria-label") ?? text).slice(0, 40), w: Math.round(r.width), h: Math.round(r.height) });
        }
        if (el.tagName === "BUTTON" && !text && !el.getAttribute("aria-label") && !el.getAttribute("title")) {
          unlabeled.push({ cls: String(el.className).slice(0, 30), testid: el.getAttribute("data-testid") });
        }
      }
      const rect = (sel) => {
        const el = document.querySelector(sel);
        if (!el) return null;
        const r = el.getBoundingClientRect();
        return { w: Math.round(r.width), h: Math.round(r.height), x: Math.round(r.x), y: Math.round(r.y) };
      };
      const font = (sel) => {
        const el = document.querySelector(sel);
        if (!el) return null;
        const s = getComputedStyle(el);
        return `${s.fontSize}/${s.lineHeight} ${s.fontFamily.split(",")[0]}`;
      };
      return {
        viewport: { vw, vh },
        docScrollWidth: document.documentElement.scrollWidth,
        toolbar: rect(".toolbar"),
        rail: rect(".toolrail"),
        side: rect(".side"),
        output: rect(".output"),
        status: rect(".status"),
        fonts: { body: font("body"), toolbarButton: font(".toolbar button"), status: font(".status"), panelTitle: font(".panel-title"), tab: font(".tab") },
        clipped: clipped.slice(0, 30),
        small: [...new Map(small.map((s) => [s.tag + s.text + s.w + s.h, s])).values()].slice(0, 20),
        unlabeled: unlabeled.slice(0, 20),
      };
    }).catch((error) => ({ error: String(error) }));
  }
  async function shot(name, label, opts = {}) {
    const m = await metrics();
    await page.screenshot({ path: path.join(OUT, `${name}.png`), fullPage: Boolean(opts.fullPage) });
    const text = await page.evaluate(() => document.body.innerText).catch(() => "");
    fs.writeFileSync(path.join(TEXT, `${name}.txt`), text);
    results.push({ name, label, metrics: m });
    console.log(`shot ${name} :: ${label}`);
  }
  async function step(label, fn) {
    try {
      await fn();
      console.log(`ok   ${label}`);
    } catch (error) {
      const message = String(error).split("\n")[0];
      failures.push({ label, error: message });
      console.log(`FAIL ${label} :: ${message}`);
      await page.screenshot({ path: path.join(OUT, `zz-fail-${section}-${slug(label)}.png`) }).catch(() => {});
    }
  }
  async function waitText(testid, text, timeout = 30_000) {
    await page.getByTestId(testid).filter({ hasText: text }).first().waitFor({ timeout });
  }
  async function boot({ target = PLAIN, trust = true, evaluate = true, restore = false, lsp = false, closeBehind = true } = {}) {
    await page.goto(URL);
    await page.getByTestId("workspace-path-input").waitFor();
    await page.evaluate(() => localStorage.removeItem("openridr.layout.v1"));
    await page.reload();
    await page.getByTestId("workspace-path-input").waitFor();
    if (closeBehind) {
      // The host may still hold a workspace from a previous section.
      const state = await page.evaluate(async () => {
        try {
          const response = await fetch("/__host/call", {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({ method: "workspace.state", params: {} }),
          });
          const body = await response.json();
          return body?.data?.workspace?.id ?? null;
        } catch { return null; }
      });
      if (state !== null) {
        await page.evaluate(async () => {
          await fetch("/__host/call", {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({ method: "workspace.close", params: {} }),
          });
        });
        await page.reload();
        await page.getByTestId("workspace-path-input").waitFor();
      }
    }
    await page.getByTestId("workspace-path-input").fill(target);
    await page.getByTestId("workspace-open").click();
    await page.getByTestId("trust-checkbox").waitFor();
    if (trust) {
      await page.getByTestId("trust-checkbox").check();
      await page.getByTestId("trust-grant").click();
      await page.getByTestId("mode-trusted").waitFor({ timeout: 30_000 });
    }
    await page.getByTestId("open-text-only").click();
    await page.getByTestId("tree").waitFor();
    const band = page.getByTestId("toolband-solution");
    if ((await band.getAttribute("aria-pressed")) !== "true") await band.click();
    if (evaluate) await page.getByTestId("eval-state").waitFor({ timeout: 120_000 });
    if (restore) {
      await click("restore-action");
      await waitText("restore-state", "Restore succeeded", 180_000);
      await click("restore-reload");
      await waitText("eval-state", "revision 2", 120_000);
    }
    if (lsp) {
      await ensureAnalysisTool();
      await click("lsp-start");
      await waitText("lsp-state", "Ready", 150_000);
    }
  }
  async function ensureAnalysisTool() {
    if (await page.getByTestId("install-ready").count()) return;
    await openTools();
    if (await page.getByTestId("install-plan-open").count()) {
      await click("install-plan-open");
      await page.getByTestId("install-plan").waitFor();
      await click("install-confirm");
      await page.getByTestId("install-ready").waitFor({ timeout: 240_000 });
    }
  }
  async function openTools() {
    const toggle = page.getByTestId("workspace-tools-toggle");
    const open = await toggle.evaluate((el) => el.closest("details")?.hasAttribute("open"));
    if (!open) await toggle.click();
  }
  async function closeTools() {
    const toggle = page.getByTestId("workspace-tools-toggle");
    const open = await toggle.evaluate((el) => el.closest("details")?.hasAttribute("open"));
    if (open) await toggle.click();
  }
  async function bottom(id) {
    const band = page.getByTestId(`toolband-${id}`);
    if ((await band.getAttribute("aria-pressed")) !== "true") await band.click();
    const tab = page.getByTestId(`bottom-tab-${id}`);
    if ((await tab.count()) && (await tab.getAttribute("aria-selected")) !== "true") await tab.click();
  }
  async function left(id) {
    const band = page.getByTestId(`toolband-${id}`);
    if ((await band.getAttribute("aria-pressed")) !== "true") await band.click();
  }
  async function finish(name) {
    fs.writeFileSync(path.join(ROOT, `.scratch/ui-audit/results-${name}.json`), JSON.stringify({ results, failures, ms: Date.now() - started }, null, 2));
    console.log("FAILURES", JSON.stringify(failures, null, 2));
    await browser.close();
  }
  return { page, results, failures, shot, step, click, reveal, waitText, boot, openTools, closeTools, bottom, left, finish, metrics, ensureAnalysisTool };
}
