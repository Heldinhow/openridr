import { chromium } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";

const ROOT = "/Users/helder/Projetos/openridr";
const OUT = path.join(ROOT, "docs/evidence/ui-audit");
const TEXT = path.join(ROOT, ".scratch/ui-audit/text");
fs.mkdirSync(OUT, { recursive: true });
fs.mkdirSync(TEXT, { recursive: true });

const fixture = path.join(ROOT, "fixtures/OrderDesk/OrderDesk.sln");
const results = [];
const failures = [];
const started = Date.now();

function slug(s) { return s.replace(/[^a-z0-9-]+/gi, "-").toLowerCase(); }

const browser = await chromium.launch({
  channel: "chrome",
  headless: false,
  slowMo: 300,
  args: ["--window-position=30,30", "--window-size=1640,1080"],
});
const context = await browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1 });
const page = await context.newPage();
page.setDefaultTimeout(25_000);

async function reveal(locator) {
  for (const details of await locator.locator("xpath=ancestor::details").all()) {
    if ((await details.getAttribute("open")) === null) {
      await details.locator(":scope > summary").click();
    }
  }
}
async function tid(id) { return page.getByTestId(id); }
async function click(id, opts) { const el = await tid(id); await reveal(el); await el.click(opts); }
async function shot(name, label) {
  const metrics = await page.evaluate(() => {
    const vw = innerWidth, vh = innerHeight;
    const vis = (el) => {
      const r = el.getBoundingClientRect();
      const s = getComputedStyle(el);
      return r.width > 0 && r.height > 0 && s.visibility !== "hidden" && s.display !== "none" && Number(s.opacity || 1) > 0.05;
    };
    const clipped = [];
    const unlabeled = [];
    const small = [];
    for (const el of document.querySelectorAll("body *")) {
      if (!vis(el)) continue;
      const r = el.getBoundingClientRect();
      const s = getComputedStyle(el);
      const text = (el.textContent ?? "").trim();
      if (text && el.children.length === 0 && el.scrollWidth > el.clientWidth + 2 && /hidden|clip/.test(s.overflowX)) {
        clipped.push({ tag: el.tagName.toLowerCase(), cls: el.className?.toString().slice(0, 40), text: text.slice(0, 80), client: el.clientWidth, scroll: el.scrollWidth });
      }
      if (["BUTTON", "SUMMARY", "A", "INPUT", "SELECT"].includes(el.tagName) && (r.height < 22 || r.width < 22)) {
        small.push({ tag: el.tagName.toLowerCase(), text: (el.getAttribute("aria-label") ?? text).slice(0, 40), w: Math.round(r.width), h: Math.round(r.height) });
      }
      if (el.tagName === "BUTTON" && !text && !el.getAttribute("aria-label") && !el.getAttribute("title")) {
        unlabeled.push({ cls: el.className?.toString().slice(0, 40), testid: el.getAttribute("data-testid") });
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
      bodyScrollWidth: document.body.scrollWidth,
      toolbar: rect(".toolbar"),
      rail: rect(".toolrail"),
      side: rect(".side"),
      output: rect(".output"),
      status: rect(".status"),
      editor: rect(".editor-pane, .editor, .cm-editor"),
      fonts: {
        body: font("body"),
        toolbarButton: font(".toolbar button"),
        status: font(".status"),
        panelTitle: font(".panel-title"),
        tab: font(".tab"),
      },
      clipped: clipped.slice(0, 30),
      small: small.slice(0, 20),
      unlabeled: unlabeled.slice(0, 20),
    };
  }).catch((error) => ({ error: String(error) }));
  const file = path.join(OUT, `${name}.png`);
  await page.screenshot({ path: file });
  const text = await page.evaluate(() => document.body.innerText).catch(() => "");
  fs.writeFileSync(path.join(TEXT, `${name}.txt`), text);
  results.push({ name, label, metrics });
  console.log(`shot ${name} :: ${label}`);
}

async function step(label, fn) {
  try {
    await fn();
    console.log(`ok   ${label}`);
  } catch (error) {
    failures.push({ label, error: String(error).split("\n")[0] });
    console.log(`FAIL ${label} :: ${String(error).split("\n")[0]}`);
    await page.screenshot({ path: path.join(OUT, `zz-fail-${slug(label)}.png`) }).catch(() => {});
  }
}

async function waitText(testid, text, timeout = 30_000) {
  await page.getByTestId(testid).filter({ hasText: text }).first().waitFor({ timeout });
}

async function openPath(target) {
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
}

// ---------- A. First run, restricted, trust, shell ----------
await page.goto("http://127.0.0.1:1420/");
await page.getByTestId("workspace-path-input").waitFor();

await step("01 setup empty", async () => {
  await shot("01-setup-empty", "First run, no workspace");
});

await step("02 invalid path", async () => {
  await page.getByTestId("workspace-path-input").fill("/tmp/does-not-exist-openridr/No.sln");
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("workspace-error").waitFor();
  await shot("02-setup-invalid-path", "Invalid workspace path error");
});

await step("03 restricted toolchain", async () => {
  await openPath(fixture);
  await page.getByTestId("trust-checkbox").waitFor();
  await shot("03-toolchain-restricted", "Trust card, restricted mode, toolchain");
});

await step("04 toolchain expanded", async () => {
  for (const id of ["sdk-host", "sdk-project", "sdk-tfms"]) {
    const el = page.getByTestId(id);
    if (await el.count()) await el.click().catch(() => {});
  }
  const card = page.getByTestId("debug-adapter-card");
  if (await card.count()) await card.click().catch(() => {});
  await shot("04-toolchain-expanded", "Toolchain rows expanded");
});

await step("05 restricted workspace", async () => {
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("tree").waitFor();
  await shot("05-workspace-restricted", "Workspace in restricted mode");
});

await step("06 restricted build panel", async () => {
  await page.getByTestId("toolband-build").click();
  await shot("06-restricted-build", "Build tool window restricted");
});
await step("07 restricted problems", async () => {
  await page.getByTestId("toolband-problems").click();
  await shot("07-restricted-problems", "Problems tool window restricted");
});
await step("08 restricted debug", async () => {
  await page.getByTestId("toolband-debug").click();
  await shot("08-restricted-debug", "Debug tool window restricted");
});
await step("09 restricted terminal", async () => {
  await page.getByTestId("toolband-terminal").click();
  await shot("09-restricted-terminal", "Terminal tool window restricted");
});
await step("10 restricted tests", async () => {
  await page.getByTestId("toolband-tests").click();
  await shot("10-restricted-tests", "Tests tool window restricted");
});
await step("11 restricted search", async () => {
  await page.keyboard.press("Meta+t");
  await page.getByTestId("find-action").waitFor();
  await page.getByTestId("search-tab-classes").click();
  await page.getByTestId("find-action-input").fill("Order");
  await page.waitForTimeout(800);
  await shot("11-restricted-search-classes", "Search Everywhere classes without analysis");
  await page.keyboard.press("Escape");
});
await step("12 restricted settings vcs", async () => {
  await page.keyboard.press("Meta+,");
  await page.getByTestId("settings-dialog").waitFor();
  await page.getByTestId("settings-page-vcs").click();
  await shot("12-restricted-settings-vcs", "Settings Version Control untrusted");
  await page.keyboard.press("Escape");
});
await step("13 restricted git", async () => {
  await page.getByTestId("toolband-git").click();
  await shot("13-restricted-git", "Git log tool window untrusted");
});
await step("14 restricted commit", async () => {
  await page.getByTestId("toolband-commit").click();
  await shot("14-restricted-commit", "Commit tool window untrusted");
});
await step("15 restricted nuget", async () => {
  await page.getByTestId("toolband-nuget").click();
  await shot("15-restricted-nuget", "NuGet tool window untrusted");
});

await step("16 reopen and trust", async () => {
  await page.getByTestId("workspace-actions-menu").click();
  await page.getByTestId("close-workspace").click();
  await page.getByTestId("workspace-path-input").waitFor();
  await openPath(fixture);
  await page.getByTestId("trust-checkbox").check();
  await page.getByTestId("trust-grant").click();
  await page.getByTestId("mode-trusted").waitFor();
  await shot("16-toolchain-trusted", "Trusted toolchain before entering workspace");
  await page.getByTestId("open-text-only").click();
  await page.getByTestId("tree").waitFor();
  await waitText("eval-state", "Evaluated", 90_000);
  await shot("17-workspace-trusted", "Workspace after trust and evaluation");
});

await step("18 tree expanded", async () => {
  for (const project of ["OrderDesk.Cli", "OrderDesk.Core", "OrderDesk.Tests"]) {
    const el = page.getByTestId(`tree-project-${project}`);
    if (await el.count()) await el.click().catch(() => {});
  }
  await shot("18-tree-expanded", "Solution Explorer expanded");
});

await step("19 editor program", async () => {
  await page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs").click();
  await page.getByTestId("editor").waitFor();
  await shot("19-editor-program", "Program.cs open in the editor");
});

await step("20 project details", async () => {
  await page.getByTestId("tree-project-OrderDesk.Cli").click();
  await page.getByRole("button", { name: "Project details" }).click();
  await page.getByTestId("eval-details").waitFor();
  await shot("20-project-details", "Evaluated project details panel");
  await page.getByRole("button", { name: "Close project details" }).click();
});

await step("21 workspace tools", async () => {
  const toggle = page.getByTestId("workspace-tools-toggle");
  const open = await toggle.evaluate((el) => el.closest("details")?.hasAttribute("open"));
  if (!open) await toggle.click();
  await shot("21-workspace-tools", "Workspace tools: restore, LSP, adapter, installer");
});

await step("22 restore", async () => {
  await click("restore-action");
  await waitText("restore-state", "Restore succeeded", 180_000);
  await shot("22-restore-success", "Restore succeeded with command and log");
  await click("restore-reload");
  await waitText("eval-state", "revision 2", 90_000);
});

await step("23 lsp ready", async () => {
  await click("lsp-start");
  await waitText("lsp-state", "Ready", 120_000);
  await shot("23-lsp-ready", "csharp-ls ready, capabilities listed");
});

await step("24 build success", async () => {
  if ((await page.getByTestId("workspace-tools-toggle").evaluate((el) => el.closest("details")?.hasAttribute("open")))) {
    await page.getByTestId("workspace-tools-toggle").click();
  }
  await page.getByTestId("toolband-build").click();
  await page.getByTestId("build-action").click();
  await waitText("build-state", "succeeded", 180_000).catch(async () => {
    await waitText("build-state", "Build", 180_000);
  });
  await shot("24-build-success", "Build succeeded");
});

await step("25 run", async () => {
  await page.getByTestId("run-action").click();
  await page.getByTestId("run-state").filter({ hasText: "exited" }).first().waitFor({ timeout: 120_000 });
  await shot("25-run-output", "Run output with command and exit");
});

await step("26 problems after build", async () => {
  await page.getByTestId("toolband-problems").click();
  await shot("26-problems", "Problems after a successful build");
});

fs.writeFileSync(path.join(ROOT, ".scratch/ui-audit/results-a.json"), JSON.stringify({ results, failures, ms: Date.now() - started }, null, 2));
console.log("FAILURES", JSON.stringify(failures, null, 2));
await browser.close();
