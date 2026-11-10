import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

// Material Icon Theme drawings in the Solution tree. Works on a temp copy of the fixture.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
/** The keymap's `primary` is Cmd on macOS and Ctrl elsewhere; the host reports the real platform. */
const PRIMARY = process.platform === "darwin" ? "Meta" : "Control";
// `OPENRIDR_FILE_ICONS_DPR=2` renders the same tree at 2× for the evidence capture; the assertions
// are resolution-independent.
test.use({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: Number(process.env.OPENRIDR_FILE_ICONS_DPR ?? 1) });

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
  // A plain `.md` next to `README.md`: the theme draws them differently.
  fs.mkdirSync(path.join(copy, "docs"), { recursive: true });
  fs.writeFileSync(path.join(copy, "docs/notes.md"), "# Notes\n");
  // Two folders a .NET solution has and the theme draws by name.
  fs.mkdirSync(path.join(copy, "Properties"), { recursive: true });
  fs.mkdirSync(path.join(copy, "wwwroot"), { recursive: true });
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

test("a C# row exposes its symbol kind to assistive tech without changing the name", async ({ page }) => {
  await open(page, tempCopy());
  const row = page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs");
  await expect(row).toBeVisible();
  // The kind is a description, not a visual badge or part of the accessible name.
  await expect(row).toHaveAccessibleDescription("C# class");
  await expect(row).toHaveAccessibleName(/Order\.cs$/);
  await expect(row.locator("[data-icon]")).toHaveAttribute("aria-hidden", "true");
  await expect(row.locator(".symbol-mark")).toHaveCount(0);
  // The C# glyph is the theme's own drawing: one image, no font and no Lucide icon.
  await expect(row.locator("[data-icon]")).toHaveAttribute("data-icon", "csharp");
  await expect(row.locator("[data-icon] img")).toHaveCount(1);
  await expect(row.locator("[data-icon] img")).toHaveAttribute("src", /\/file-icons\/csharp\.svg$/);
  await expect(row.locator("[data-icon] .icon")).toHaveCount(0);
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

test("the solution root and every project draw the theme's .sln/.csproj icon", async ({ page }) => {
  await open(page, tempCopy());
  // The theme has one Visual Studio drawing for both files; the kind stays text in the row.
  const root = page.getByTestId("tree-root").locator("[data-icon]");
  await expect(root).toHaveAttribute("data-icon", "visualstudio");
  await expect(root.locator("img")).toHaveAttribute("src", /\/file-icons\/visualstudio\.svg$/);
  for (const testId of ["tree-project-OrderDesk.Cli", "tree-project-OrderDesk.Core"]) {
    const icon = page.getByTestId(testId).locator(".tree-icon .project-icon");
    await expect(icon).toHaveAttribute("data-icon", "visualstudio");
    await expect(icon).toHaveAttribute("src", /\/file-icons\/visualstudio\.svg$/);
  }
  // The project kind is still readable as text, which is what the shared drawing cannot carry.
  await expect(page.getByTestId("tree-project-OrderDesk.Cli").locator(".tree-meta")).toHaveText("console · net8.0");
  await expect(page.getByTestId("tree-project-OrderDesk.Tests").locator(".tree-meta")).toHaveText("test · net8.0");
});

test("file kinds map to the theme's drawings", async ({ page }) => {
  await open(page, tempCopy());
  await page.getByTestId("tree-show-all").click();
  // A disk folder is not part of the solution, so "Expand all" leaves it closed.
  await page.getByTestId("tree-dir-docs").click();

  const glyph = (testId: string) => page.getByTestId(testId).locator("[data-icon]");
  for (const [testId, icon] of [
    ["tree-root", "visualstudio"],
    ["tree-project-OrderDesk.Core", "visualstudio"],
    ["tree-file-src/OrderDesk.Core/Order.cs", "csharp"],
    ["tree-file-appsettings.json", "json"],
    ["tree-file-global.json", "json"],
    ["tree-file-README.md", "readme"],
    ["tree-file-docs/notes.md", "markdown"],
    ["tree-file-Dockerfile", "docker"],
    ["tree-file-docker-compose.yml", "docker"],
    ["tree-file-OrderDesk.sln.DotSettings.user", "settings"],
    ["tree-file-LICENSE", "license"],
    ["tree-file-.gitignore", "git"],
  ]) {
    await expect(glyph(testId)).toHaveAttribute("data-icon", icon);
  }
  // One glyph per row, and it is always the theme's image — never a font character.
  for (const testId of ["tree-file-README.md", "tree-file-appsettings.json", "tree-file-src/OrderDesk.Core/Order.cs"]) {
    await expect(glyph(testId).locator("img")).toHaveCount(1);
    await expect(glyph(testId).locator("svg")).toHaveCount(0);
  }
  await expect(page.getByTestId("tree-file-README.md").locator("[data-icon]")).toHaveCount(1);
  // Fold the Dependencies groups for the evidence shot so the file rows fit the pane.
  for (const project of ["OrderDesk.Cli", "OrderDesk.Core", "OrderDesk.Tests"]) {
    await page.getByTestId(`tree-deps-${project}`).click();
  }
  await shot(page, "depois-glyphs");
});

test("folders draw the theme's icon for their name, in open and closed states", async ({ page }) => {
  const root = tempCopy();
  fs.mkdirSync(path.join(root, ".idea"), { recursive: true });
  fs.mkdirSync(path.join(root, "src/OrderDesk.Core/bin"), { recursive: true });
  await open(page, root);
  await page.getByTestId("tree-show-all").click();

  for (const testId of ["tree-dir-.idea", "tree-dir-src/OrderDesk.Core/bin", "tree-file-README.md"]) {
    await expect(page.getByTestId(testId)).toBeVisible();
  }
  // The folder occupies the same 16px slot as file glyphs, and it is an image.
  const folder = (testId: string) => page.getByTestId(testId).locator("[data-folder]");
  await expect(page.getByTestId("tree-dir-.idea").locator(".tree-icon img")).toHaveCount(1);
  await expect(page.getByTestId("tree-dir-.idea").locator("[data-icon]")).toHaveCount(0);
  await expect(folder("tree-dir-.idea")).toHaveAttribute("data-folder", "closed");
  await expect(folder("tree-dir-src/OrderDesk.Core/bin")).toHaveAttribute("data-folder", "closed");

  // The name decides the drawing: `docs`, `Properties`, `wwwroot` and `bin` have their own,
  // `.idea` is generic. (`src` and `tests` are ancestors of the projects, so the tree nests them
  // into the project rows and never draws a folder row for them; the unit test covers those names.)
  const iconOf = (testId: string) => page.getByTestId(testId).locator("[data-folder]").getAttribute("data-folder-icon");
  expect(await iconOf("tree-dir-docs")).toBe("folder-docs");
  expect(await iconOf("tree-dir-Properties")).toBe("folder-config");
  expect(await iconOf("tree-dir-wwwroot")).toBe("folder-public");
  expect(await iconOf("tree-dir-src/OrderDesk.Core/bin")).toBe("folder-dist");
  expect(await iconOf("tree-dir-.idea")).toBe("folder");

  // Open and closed are two different files of the same drawing.
  const closedDocs = await folder("tree-dir-docs").getAttribute("src");
  expect(closedDocs).toMatch(/\/folder-icons\/folder-docs\.svg$/);
  await page.getByTestId("tree-dir-docs").click();
  await expect(folder("tree-dir-docs")).toHaveAttribute("data-folder", "open");
  const openDocs = await folder("tree-dir-docs").getAttribute("src");
  expect(openDocs).toMatch(/\/folder-icons\/folder-docs-open\.svg$/);
  expect(openDocs).not.toBe(closedDocs);

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

test("every glyph is a 16 px drawing aligned with the row's text", async ({ page }) => {
  await open(page, tempCopy());
  await page.getByTestId("tree-show-all").click();
  const rows = [
    "tree-file-src/OrderDesk.Core/Order.cs",
    "tree-file-appsettings.json",
    "tree-file-README.md",
    "tree-file-.gitignore",
    "tree-file-OrderDesk.sln.DotSettings.user",
    "tree-dir-docs",
    "tree-dir-Properties",
    "tree-dir-wwwroot",
  ];
  for (const id of rows) {
    const row = page.getByTestId(id);
    const glyph = row.locator(".tree-icon img").first();
    await expect(glyph).toBeVisible();
    // The SVG actually loaded: a broken `src` renders an empty 16×16 box and would pass a size check.
    expect(`${id} loaded ${await glyph.evaluate((el) => (el as HTMLImageElement).naturalWidth > 0)}`).toBe(`${id} loaded true`);
    const box = (await glyph.boundingBox())!;
    // 16×16, not the SVG's own 32×32 grid and not a scaled-up or clipped drawing.
    expect(`${id} ${box.width}×${box.height}`).toBe(`${id} 16×16`);
    const rowBox = (await row.boundingBox())!;
    expect(`${id} top ${box.y >= rowBox.y}`).toBe(`${id} top true`);
    expect(`${id} bottom ${box.y + box.height <= rowBox.y + rowBox.height}`).toBe(`${id} bottom true`);
    // Centred on the label, not on the row box.
    const name = (await row.locator(".tree-name").boundingBox())!;
    const delta = Math.abs(box.y + box.height / 2 - (name.y + name.height / 2));
    expect(`${id} centred ${delta < 1.5}`).toBe(`${id} centred true`);
  }
  await shot(page, "depois-alinhamento");
});

test("the panel header has no chevron without a handler", async ({ page }) => {
  await open(page, tempCopy());
  // The title is text; the panel is toggled from its toolbar button, which has a handler.
  await expect(page.locator(".explorer-title .icon")).toHaveCount(0);
  await expect(page.getByTestId("tree-minimize")).toHaveCount(1);
});

test("Dependencies, packages and Frameworks draw the NuGet icon, not a gear", async ({ page }) => {
  await open(page, tempCopy());
  const deps = page.getByTestId("tree-deps-OrderDesk.Core").locator(".dependency-glyph");
  await expect(deps).toHaveCount(1);
  await expect(deps).toHaveAttribute("data-icon", "nuget");
  await expect(deps).toHaveAttribute("src", /\/file-icons\/nuget\.svg$/);

  const group = page.getByTestId("tree-deps-group-OrderDesk.Core-frameworks").locator(".runtime-glyph");
  await expect(group).toHaveCount(1);
  await expect(group).toHaveAttribute("src", /\/file-icons\/nuget\.svg$/);

  const framework = page.getByTestId("tree-framework-OrderDesk.Core-Microsoft.NETCore.App (net8.0)");
  await expect(framework.locator(".runtime-glyph")).toHaveCount(1);
  // The caller's icon is what the row draws: TreeRow no longer substitutes a settings gear.
  await expect(framework.locator(".icon-settings")).toHaveCount(0);
});

test("the same file keeps its glyph in the tree, the tab, Find in Files and Search Everywhere", async ({ page }) => {
  await open(page, tempCopy());
  const file = "src/OrderDesk.Core/Order.cs";
  const treeIcon = page.getByTestId(`tree-file-${file}`).locator("[data-icon]");
  await expect(treeIcon).toHaveAttribute("data-icon", "csharp");
  const treeSrc = await treeIcon.locator("img").getAttribute("src");

  await page.getByTestId(`tree-file-${file}`).click();
  const tabIcon = page.getByTestId(`tab-${file}`).locator("[data-icon]");
  await expect(tabIcon).toHaveAttribute("data-icon", "csharp");
  // One component, one drawing: the same file URL in both surfaces.
  expect(await tabIcon.locator("img").getAttribute("src")).toBe(treeSrc);

  await page.getByTestId("editor").click();
  await page.keyboard.press(`${PRIMARY}+Shift+F`);
  await page.getByTestId("find-in-files-input").fill("Order");
  const result = page
    .locator('[data-testid^="find-in-files-item-"]')
    .filter({ hasText: file })
    .first();
  await expect(result.locator("[data-icon]")).toHaveAttribute("data-icon", "csharp");
  expect(await result.locator("[data-icon] img").getAttribute("src")).toBe(treeSrc);
  await page.keyboard.press("Escape");

  await page.getByTestId("editor").click();
  await page.keyboard.press("Shift");
  await page.keyboard.press("Shift");
  await page.getByTestId("search-tab-files").click();
  await page.getByTestId("find-action-input").fill("Order.cs");
  const hit = page.locator('[data-testid^="search-item-"]').filter({ hasText: file }).first();
  await expect(hit.locator("[data-icon]")).toHaveAttribute("data-icon", "csharp");
  await shot(page, "depois-superficies");
});

test("a project row has a second layer and the tree indents 16 px per level", async ({ page }) => {
  await open(page, tempCopy());
  const row = page.getByTestId("tree-project-OrderDesk.Cli");
  // The kind is text, not only an aria-hidden glyph, and it is part of the row's accessible name.
  await expect(row.locator(".tree-meta")).toHaveText("console · net8.0");
  await expect(row).toHaveAccessibleName(/console/);
  // 16 px per level: the project is level 1, its files level 2.
  await expect(row).toHaveCSS("padding-left", "24px");
  const file = page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs");
  await expect(file).toHaveCSS("padding-left", "40px");
  // The density is untouched (36 px comfortable, 32 px compact — settings.spec.ts fixes it);
  // the project row is the two-layer row and still fits the comfortable height.
  const fileHeight = (await file.boundingBox())!.height;
  const projectHeight = (await row.boundingBox())!.height;
  expect(projectHeight).toBeLessThanOrEqual(44);
  expect(projectHeight).toBeGreaterThanOrEqual(fileHeight);
  await shot(page, "depois-projeto-duas-camadas");
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
