import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

// Rider-style tree glyphs and rounded selection. Works on a temp copy of the fixture.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
test.use({ viewport: { width: 1440, height: 900 } });

function tempCopy() {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-icons-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  // Files the reference shows that the fixture does not carry. The versioned fixture is untouched.
  fs.writeFileSync(path.join(copy, "README.md"), "# OrderDesk\n");
  fs.writeFileSync(path.join(copy, "LICENSE"), "Example\n");
  fs.writeFileSync(path.join(copy, "appsettings.json"), "{}\n");
  fs.writeFileSync(path.join(copy, "Dockerfile"), "FROM mcr.microsoft.com/dotnet/sdk:8.0\n");
  fs.writeFileSync(path.join(copy, "docker-compose.yml"), "services: {}\n");
  fs.writeFileSync(
    path.join(copy, "OrderDesk.sln.DotSettings.user"),
    '<wpf:ResourceDictionary xml:space="preserve" />\n',
  );
  return copy;
}

async function open(page: Page, root: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(root, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-restricted").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  await page.getByTestId("tree-expand-all").click();
}

test("a C# row exposes its symbol kind to assistive tech without changing the name", async ({ page }) => {
  await open(page, tempCopy());
  const row = page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs");
  await expect(row).toBeVisible();
  // The kind is a description, not a visual badge or part of the accessible name.
  await expect(row).toHaveAccessibleDescription("C# class");
  await expect(row).toHaveAccessibleName(/Order\.cs$/);
  await expect(row.locator("[data-icon]")).toHaveAttribute("aria-hidden", "true");
  await expect(row.locator(".symbol-mark")).toHaveCount(0);
  // The C# glyph is one character of the vendored Seti font, not a drawn vector or a badge.
  await expect(row.locator("[data-icon] .seti-glyph")).toHaveCount(1);
  await expect(row.locator("[data-icon] svg")).toHaveCount(0);
  const family = await row.locator("[data-icon] .seti-glyph").evaluate((el) => getComputedStyle(el).fontFamily);
  expect(family).toContain("seti");
  // Seti's `_c-sharp` blue family.
  await expect(row.locator("[data-icon]")).toHaveCSS("color", "rgb(81, 154, 186)");
});

test("the filter spells out the symbol kind without changing the accessible name", async ({ page }) => {
  await open(page, tempCopy());
  await page.getByTestId("tree-search").click();
  await page.getByTestId("tree-filter").fill("Order.cs");
  const row = page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs");
  await expect(row).toBeVisible();
  // The kind is text in the row (the scan follows the filter), and the badge is gone for good.
  await expect(row.locator(".tree-symbol-kind")).toHaveText("class");
  await expect(row.locator(".symbol-mark")).toHaveCount(0);
  // The text is aria-hidden: the name is still only the file name, the description still says the kind.
  await expect(row).toHaveAccessibleName(/Order\.cs$/);
  await expect(row).toHaveAccessibleDescription("C# class");
  await shot(page, "depois-filtro-tipo");
});

test("project rows draw their kind as a glyph in the kind color", async ({ page }) => {
  await open(page, tempCopy());
  for (const [project, kind, color] of [
    ["OrderDesk.Cli", "console", "rgb(124, 192, 255)"],
    ["OrderDesk.Core", "library", "rgb(142, 211, 154)"],
    ["OrderDesk.Tests", "test", "rgb(224, 183, 110)"],
  ]) {
    const icon = page.getByTestId(`tree-project-${project}`).locator(".tree-icon .project-icon");
    await expect(icon).toHaveClass(new RegExp(`project-${kind}`));
    await expect(icon.locator("text")).toHaveCount(0);
    await expect(icon).toHaveCSS("color", color);
  }
});

/** Evidence is opt-in: `OPENRIDR_FILE_ICONS_EVIDENCE=docs/evidence/solution-file-icons`. */
async function shot(page: Page, name: string) {
  const dir = process.env.OPENRIDR_FILE_ICONS_EVIDENCE;
  if (!dir) return;
  await page.screenshot({ path: path.join(dir, `${name}.png`), animations: "disabled" });
  await page.getByTestId("tree").screenshot({ path: path.join(dir, `${name}-arvore.png`) });
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("file kinds map to the reference glyphs", async ({ page }) => {
  await open(page, tempCopy());
  await page.getByTestId("tree-show-all").click();

  const glyph = (testId: string) => page.getByTestId(testId).locator("[data-icon]");
  for (const [testId, icon] of [
    ["tree-file-README.md", "markdown"],
    ["tree-file-appsettings.json", "json"],
    ["tree-file-global.json", "json"],
    ["tree-file-Dockerfile", "docker"],
    ["tree-file-docker-compose.yml", "docker"],
    ["tree-file-OrderDesk.sln.DotSettings.user", "text"],
    ["tree-file-src/OrderDesk.Core/Order.cs", "csharp"],
    ["tree-file-LICENSE", "license"],
  ]) {
    await expect(glyph(testId)).toHaveAttribute("data-icon", icon);
  }
  // The Seti-drawn kinds are font characters; only the four kinds Seti has no drawing for are SVG.
  for (const testId of ["tree-file-README.md", "tree-file-appsettings.json", "tree-file-Dockerfile", "tree-file-src/OrderDesk.Core/Order.cs"]) {
    await expect(glyph(testId).locator(".seti-glyph")).toHaveCount(1);
    await expect(glyph(testId).locator("svg")).toHaveCount(0);
  }
  for (const testId of ["tree-file-OrderDesk.sln.DotSettings.user"]) {
    await expect(glyph(testId).locator(".seti-glyph")).toHaveCount(0);
    await expect(glyph(testId).locator("svg")).toHaveCount(1);
  }
  // The C# glyph is the Seti character, not a Lucide icon or a symbol badge.
  await expect(glyph("tree-file-src/OrderDesk.Core/Order.cs").locator(".icon")).toHaveCount(0);
  // One glyph per row: a file never stacks two type icons.
  await expect(page.getByTestId("tree-file-README.md").locator("[data-icon]")).toHaveCount(1);
  // Fold the Dependencies groups for the evidence shot so the file rows fit the pane.
  for (const project of ["OrderDesk.Cli", "OrderDesk.Core", "OrderDesk.Tests"]) {
    await page.getByTestId(`tree-deps-${project}`).click();
  }
  await shot(page, "depois-glyphs");
});

test("folders have a filled glyph with open and closed states, and an indent guide per ancestor", async ({ page }) => {
  const root = tempCopy();
  fs.mkdirSync(path.join(root, ".idea"), { recursive: true });
  fs.mkdirSync(path.join(root, "src/OrderDesk.Core/bin"), { recursive: true });
  await open(page, root);
  await page.getByTestId("tree-show-all").click();

  for (const testId of ["tree-dir-.idea", "tree-dir-src/OrderDesk.Core/bin", "tree-file-README.md"]) {
    await expect(page.getByTestId(testId)).toBeVisible();
  }
  // The folder occupies the same 16px slot as file glyphs.
  await expect(page.getByTestId("tree-dir-.idea").locator(".tree-icon svg")).toHaveCount(1);
  await expect(page.getByTestId("tree-dir-.idea").locator("svg")).toHaveCount(2);
  await expect(page.getByTestId("tree-dir-src/OrderDesk.Core/bin").locator("[data-icon]")).toHaveCount(0);
  // Open and closed folders are two different drawings, and the folder name never changes the glyph.
  const folder = (testId: string) => page.getByTestId(testId).locator("[data-folder]");
  await expect(folder("tree-dir-.idea")).toHaveAttribute("data-folder", "closed");
  await expect(folder("tree-dir-src/OrderDesk.Core/bin")).toHaveAttribute("data-folder", "closed");
  const closedPath = await folder("tree-dir-.idea").locator("path").first().getAttribute("d");
  const otherClosedPath = await folder("tree-dir-src/OrderDesk.Core/bin").locator("path").first().getAttribute("d");
  expect(closedPath).toBe(otherClosedPath);
  await page.getByTestId("tree-dir-.idea").click();
  await expect(folder("tree-dir-.idea")).toHaveAttribute("data-folder", "open");
  const openPath = await folder("tree-dir-.idea").locator("path").first().getAttribute("d");
  expect(openPath).not.toBe(closedPath);
  const dirName = await page.getByTestId("tree-dir-.idea").locator(".tree-name").boundingBox();
  const fileName = await page.getByTestId("tree-file-README.md").locator(".tree-name").boundingBox();
  expect(Math.abs(dirName!.x - fileName!.x)).toBeLessThan(2);

  // Guides: one per expanded ancestor — the root row has none, its children one each.
  await expect(page.getByTestId("tree-root").locator(".tree-guide")).toHaveCount(0);
  await expect(page.getByTestId("tree-project-OrderDesk.Core").locator(".tree-guide")).toHaveCount(1);
  await expect(page.getByTestId("tree-dir-.idea").locator(".tree-guide")).toHaveCount(1);
  await expect(page.getByTestId("tree-dir-src/OrderDesk.Core/bin").locator(".tree-guide")).toHaveCount(2);
  await shot(page, "depois-pastas-e-guia");
});

test("every file glyph clears WCAG 1.4.11 contrast on the row surfaces", async ({ page }) => {
  await open(page, tempCopy());
  await page.getByTestId("tree-show-all").click();
  const rows = [
    "tree-file-src/OrderDesk.Core/Order.cs",
    "tree-file-appsettings.json",
    "tree-file-README.md",
    "tree-file-.gitignore",
    "tree-file-OrderDesk.sln.DotSettings.user",
  ];
  // Everything comes from the shipped CSS: the glyph's computed colour and the row surfaces.
  const measured = await page.evaluate((ids) => {
    // Tokens are `#rrggbb`; a computed color is `rgb(r, g, b)`.
    const toRgb = (value: string) =>
      value.trim().startsWith("#")
        ? [1, 3, 5].map((i) => Number.parseInt(value.trim().slice(i, i + 2), 16))
        : value.replace(/[^0-9.,]/g, "").split(",").slice(0, 3).map(Number);
    const channel = (c: number) => {
      const v = c / 255;
      return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
    };
    const luminance = ([r, g, b]: number[]) =>
      0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
    const contrast = (a: number[], b: number[]) => {
      const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
      return (hi + 0.05) / (lo + 0.05);
    };
    const root = getComputedStyle(document.documentElement);
    const token = (name: string) => toRgb(root.getPropertyValue(name));
    const surfaces: Record<string, number[]> = {
      panel: token("--panel"),
      raised: token("--raised"),
      select: token("--select"),
      focus: token("--select-row-focus"),
    };
    return ids.map((id) => {
      const icon = document.querySelector(`[data-testid="${id}"] [data-icon]`);
      if (!icon) return { id, ratios: null };
      const colour = toRgb(getComputedStyle(icon).color);
      const ratios: Record<string, number> = {};
      for (const [name, background] of Object.entries(surfaces)) {
        ratios[name] = contrast(colour, background);
      }
      return { id, ratios };
    });
  }, rows);
  for (const row of measured) {
    expect(row.ratios, `${row.id} has a glyph`).not.toBeNull();
    for (const [surface, ratio] of Object.entries(row.ratios!)) {
      expect(ratio, `${row.id} on ${surface}`).toBeGreaterThanOrEqual(3);
    }
  }
  await shot(page, "depois-contraste");
});

test("the panel header has no chevron without a handler", async ({ page }) => {
  await open(page, tempCopy());
  // The title is text; the panel is toggled from its toolbar button, which has a handler.
  await expect(page.locator(".explorer-title .icon")).toHaveCount(0);
  await expect(page.getByTestId("tree-minimize")).toHaveCount(1);
});

test("Dependencies and Frameworks draw their own glyphs, not a gear", async ({ page }) => {
  await open(page, tempCopy());
  await expect(page.getByTestId("tree-deps-OrderDesk.Core").locator(".dependency-glyph")).toHaveCount(1);
  await expect(page.getByTestId("tree-deps-group-OrderDesk.Core-frameworks").locator(".runtime-glyph")).toHaveCount(1);
  const framework = page.getByTestId("tree-framework-OrderDesk.Core-Microsoft.NETCore.App (net8.0)");
  await expect(framework.locator(".runtime-glyph")).toHaveCount(1);
  // The caller's icon is what the row draws: TreeRow no longer substitutes a settings gear.
  await expect(framework.locator(".icon-settings")).toHaveCount(0);
});

test("the current row has a rounded selection fill", async ({ page }) => {
  await open(page, tempCopy());
  const row = page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs");
  await expect(row).toBeVisible();
  const before = await row.evaluate((el) => getComputedStyle(el).boxShadow);
  expect(before).toBe("none");

  await row.click();
  await expect(row).toHaveAttribute("aria-current", "page");
  // The workspace selection is a neutral fill (--workspace-select-row-focus), not blue.
  await expect(row).toHaveCSS("background-color", "rgb(52, 52, 52)");
  const after = await row.evaluate((el) => ({
    shadow: getComputedStyle(el).boxShadow,
    radius: getComputedStyle(el).borderRadius,
    height: el.getBoundingClientRect().height,
  }));
  expect(after.shadow).toBe("none");
  expect(after.radius).not.toBe("0px");
  // One line: the comfortable row is 36 px; a two-layer row would be taller.
  expect(after.height).toBeLessThanOrEqual(36);
  await shot(page, "depois-selecao");
});
