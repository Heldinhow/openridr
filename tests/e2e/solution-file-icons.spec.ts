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
  // The C# mark is a drawn vector in the C# token, not font-rendered text.
  await expect(row.locator("[data-icon] text")).toHaveCount(0);
  await expect(row.locator("[data-icon] svg path")).toHaveCount(1);
  await expect(row.locator("[data-icon]")).toHaveCSS("color", "rgb(157, 106, 251)");
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
    ["tree-file-OrderDesk.sln.DotSettings.user", "xml"],
    ["tree-file-src/OrderDesk.Core/Order.cs", "csharp"],
  ]) {
    await expect(glyph(testId)).toHaveAttribute("data-icon", icon);
    await expect(glyph(testId).locator("svg")).toHaveCount(1);
  }
  // The C# glyph is the vendored mark, not a Lucide icon or a symbol badge.
  await expect(glyph("tree-file-src/OrderDesk.Core/Order.cs").locator(".icon")).toHaveCount(0);
  // One glyph per row: a file never stacks two type icons.
  await expect(page.getByTestId("tree-file-README.md").locator("[data-icon]")).toHaveCount(1);
  // Fold the Dependencies groups for the evidence shot so the file rows fit the pane.
  for (const project of ["OrderDesk.Cli", "OrderDesk.Core", "OrderDesk.Tests"]) {
    await page.getByTestId(`tree-deps-${project}`).click();
  }
  await shot(page, "depois-glyphs");
});

test("folders have outline icons and an indent guide per ancestor", async ({ page }) => {
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

test("the two-tone document body keeps WCAG 1.4.11 contrast on every row background", async ({ page }) => {
  await open(page, tempCopy());
  await page.getByTestId("tree-show-all").click();
  const row = page.getByTestId("tree-file-LICENSE");
  await expect(row).toBeVisible();
  // Everything comes from the shipped CSS: the glyph token, the body opacity and the row backgrounds.
  const ratios = await row.evaluate((el) => {
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
    const opacity = Number(getComputedStyle(el.querySelector(".glyph-page")!).opacity);
    const fill = toRgb(getComputedStyle(el.querySelector(".file-icon")!).color);
    const over = (background: number[]) =>
      contrast(
        [0, 1, 2].map((i) => opacity * fill[i] + (1 - opacity) * background[i]),
        background,
      );
    return {
      opacity,
      panel: over(token("--panel")),
      raised: over(token("--raised")),
      select: over(token("--select")),
    };
  });
  expect(ratios.opacity).toBeGreaterThan(0.5);
  expect(ratios.panel).toBeGreaterThanOrEqual(3);
  expect(ratios.raised).toBeGreaterThanOrEqual(3);
  expect(ratios.select).toBeGreaterThanOrEqual(3);
  await shot(page, "depois-contraste");
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
