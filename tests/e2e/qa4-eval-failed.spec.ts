import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { cleanup, brokenImport, openTrusted, tempCopy } from "./qa-top4";

test.use({ viewport: { width: 1440, height: 900 } });
test.describe.configure({ timeout: 240_000 });

test.afterEach(async ({ request }) => cleanup(request));

test("4 - a project whose evaluation failed still lists its files and the error in Problems", async ({
  page,
}) => {
  const { base, copy } = tempCopy(brokenImport, "Broken Import qa4");
  try {
    await openTrusted(page, path.join(copy, "BrokenImport.csproj"));
    await page.getByTestId("workspace-tools-toggle").click();
    await expect(page.getByTestId("eval-state")).toContainText("Evaluation failed", {
      timeout: 30_000,
    });
    await expect(page.getByTestId("status-evaluation")).toHaveClass(/error/);
    await expect(page.getByTestId("eval-state")).toHaveClass(/error/);
    await expect(page.getByTestId("tree-partial")).toContainText("Partial workspace");

    await expect(page.getByTestId("tree-file-Program.cs")).toBeVisible();
    await expect(page.getByTestId("tree-file-BrokenImport.csproj")).toBeVisible();

    await page.getByTestId("bottom-tab-problems").click();
    const problems = page.getByTestId("eval-problems");
    await expect(problems).toContainText("MSB4019");
    await problems.getByTestId("eval-problem-row-0").click();
    await expect(page.getByTestId("tab-BrokenImport.csproj")).toBeVisible();
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
