/**
 * F9 — NuGet. Real `dotnet list` needs a project SDK and a network for outdated data, neither
 * guaranteed on every machine; this spec uses the development-only `nuget.probe` listing
 * (OPENRIDR_ENABLE_PROBE=1 in playwright.config.ts) and never runs dotnet. The host unit tests
 * cover the parser, CPM refusal and the confirmed `dotnet add` plan.
 */
import { expect, test, type Page } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";

const solution = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
const cliProject = "src/OrderDesk.Cli/OrderDesk.Cli.csproj";

async function openTextOnly(page: Page, query = "") {
  await page.goto(`/${query}`);
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("open-text-only").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function openTrusted(page: Page, query = "") {
  await page.goto(`/${query}`);
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-checkbox").check();
  await expect(page.getByTestId("trust-grant")).toBeEnabled();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
  await page.getByTestId("open-text-only").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

test.beforeEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("NuGet refuses to read before trust and never shows an empty all-up-to-date list", async ({ page }) => {
  await openTextOnly(page);
  await page.getByTestId("bottom-tab-nuget").click();
  await expect(page.getByTestId("nuget-panel")).toBeVisible();
  await expect(page.getByTestId("nuget-disabled-reason")).toContainText("Trust the workspace");
  await expect(page.getByTestId("nuget-refresh")).toHaveCount(0);
});

test("NuGet groups installed and outdated packages by TFM with current, latest and source", async ({ page }) => {
  await openTrusted(page, "?nugetProbe=1");
  await page.getByTestId("bottom-tab-nuget").click();
  await expect(page.getByTestId("nuget-panel")).toBeVisible();
  await expect(page.getByTestId("nuget-probe")).toContainText("dotnet was not run");
  await expect(page.getByTestId("nuget-project")).toHaveValue(cliProject);
  await expect(page.getByTestId("nuget-sources")).toContainText("Enabled configured sources");
  await expect(page.getByTestId("nuget-sources")).toContainText("Package origin is not reported");

  // Two TFMs are grouped; the outdated row carries both versions and the consulted source.
  await expect(page.getByTestId("nuget-tfm-net8.0")).toBeVisible();
  await expect(page.getByTestId("nuget-tfm-netstandard2.0")).toBeVisible();
  const row = page.getByTestId("nuget-package-net8.0-Newtonsoft.Json");
  await expect(row).toContainText("13.0.1");
  await expect(page.getByTestId("nuget-latest-net8.0-Newtonsoft.Json")).toHaveText("13.0.3");
  await expect(page.getByTestId("nuget-source-net8.0-Newtonsoft.Json")).toHaveText("nuget.org (probe)");
  await expect(page.getByTestId("nuget-summary")).toContainText("1 outdated");
  await expect(page.getByTestId("nuget-update-net8.0-Newtonsoft.Json")).toBeEnabled();
  // A package with no newer version has no update action.
  await expect(page.getByTestId("nuget-update-net8.0-Serilog")).toHaveCount(0);

  // At the contract level, no confirmation means no process and no write. The exact
  // `dotnet add` plan, its revalidation and the update result are host unit tests.
  const unconfirmed = await page.request.post(`${host}/call`, {
    data: { method: "nuget.applyUpdate", params: { planId: "probe" } },
  });
  expect((await unconfirmed.json()).error.code).toBe("confirmation_required");
});

test("NuGet reports a project with no PackageReference without offering updates", async ({ page }) => {
  await openTrusted(page, "?nugetProbe=1&nugetProbeEmpty=1&nugetProbeSourcesFailed=1");
  await page.getByTestId("bottom-tab-nuget").click();
  await expect(page.getByTestId("nuget-project")).toHaveValue(cliProject);
  await expect(page.getByTestId("nuget-summary")).toContainText("No packages are referenced");
  await expect(page.getByTestId("nuget-no-packages")).toBeVisible();
  await expect(page.locator('[data-testid^="nuget-update-"]')).toHaveCount(0);
  await expect(page.getByTestId("nuget-installed-log")).toContainText("ok");
  await expect(page.getByTestId("nuget-outdated-log")).toContainText("ok");
  await expect(page.getByTestId("nuget-sources")).toContainText("source query failed");
  await expect(page.getByTestId("nuget-sources")).not.toHaveClass(/error/);
});

test("CPM lists normally and refuses the update with an explicit reason", async ({ page }) => {
  await openTrusted(page, "?nugetProbe=1&nugetProbeCpm=1");
  await page.getByTestId("bottom-tab-nuget").click();
  await expect(page.getByTestId("nuget-cpm")).toContainText("Central Package Management");
  await expect(page.getByTestId("nuget-package-net8.0-Newtonsoft.Json")).toContainText("13.0.1");
  const update = page.getByTestId("nuget-update-net8.0-Newtonsoft.Json");
  await expect(update).toBeDisabled();
  await expect(update).toHaveAttribute("title", /Central Package Management/);
  await expect(page.getByTestId("nuget-update-reason")).toContainText("Central Package Management");
  await expect(page.getByTestId("nuget-plan-dialog")).toHaveCount(0);
});
