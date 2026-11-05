import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

// SPEC-SOLUTION-FILE-ICONS: the tree draws vendored file glyphs, folders are chevron + name and the
// current row is an outline. Works on a temp copy of the fixture, like the other tree specs.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
test.use({ viewport: { width: 1440, height: 900 } });

function tempCopy() {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-icons-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  // Files the reference shows that the fixture does not carry. The versioned fixture is untouched.
  fs.writeFileSync(path.join(copy, "README.md"), "# OrderDesk\n");
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
    ["tree-file-OrderDesk.sln.DotSettings.user", "doc"],
    ["tree-file-src/OrderDesk.Core/Order.cs", "csharp"],
  ]) {
    await expect(glyph(testId)).toHaveAttribute("data-icon", icon);
    await expect(glyph(testId).locator("svg")).toHaveCount(1);
  }
  // The C# glyph is the filled Simple Icons hexagon, not a stroke icon.
  await expect(glyph("tree-file-src/OrderDesk.Core/Order.cs").locator(".icon")).toHaveCount(0);
  // One glyph per row: a file never stacks two type icons.
  await expect(page.getByTestId("tree-file-README.md").locator("[data-icon]")).toHaveCount(1);
  // Fold the Dependencies groups for the evidence shot so the file rows fit the pane.
  for (const project of ["OrderDesk.Cli", "OrderDesk.Core", "OrderDesk.Tests"]) {
    await page.getByTestId(`tree-deps-${project}`).click();
  }
  await shot(page, "depois-glyphs");
});

test("folders are chevron and name only, with an indent guide per ancestor", async ({ page }) => {
  const root = tempCopy();
  fs.mkdirSync(path.join(root, ".idea"), { recursive: true });
  fs.mkdirSync(path.join(root, "src/OrderDesk.Core/bin"), { recursive: true });
  await open(page, root);
  await page.getByTestId("tree-show-all").click();

  for (const testId of ["tree-dir-.idea", "tree-dir-src/OrderDesk.Core/bin", "tree-file-README.md"]) {
    await expect(page.getByTestId(testId)).toBeVisible();
  }
  // Folders lost their icon; the empty 16px slot keeps the names aligned with file names. The
  // chevron is an svg too, so the assertion is on the icon slot itself.
  await expect(page.getByTestId("tree-dir-.idea").locator(".tree-icon")).toHaveText("");
  await expect(page.getByTestId("tree-dir-.idea").locator("svg")).toHaveCount(1);
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

test("the current row is an accent outline, not a filled block", async ({ page }) => {
  await open(page, tempCopy());
  const row = page.getByTestId("tree-file-src/OrderDesk.Cli/Program.cs");
  await expect(row).toBeVisible();
  const before = await row.evaluate((el) => getComputedStyle(el).boxShadow);
  expect(before).toBe("none");

  await row.click();
  await expect(row).toHaveAttribute("aria-current", "page");
  const after = await row.evaluate((el) => ({
    shadow: getComputedStyle(el).boxShadow,
    radius: getComputedStyle(el).borderRadius,
    height: el.getBoundingClientRect().height,
  }));
  expect(after.shadow).toContain("inset");
  expect(after.shadow).toContain("84, 138, 247"); // --accent
  expect(after.radius).not.toBe("0px");
  expect(after.height).toBeLessThan(36); // the outline never grows the row
  await shot(page, "depois-selecao");
});
