import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { clickDiscover, revisionOf } from "./automatic";
import { host } from "./environment";

// The findings of docs/evidence/main-60e784f-acceptance.md, one test per defect: several of them
// are invisible from the snapshot alone (D1 is a file, D6 is the accessibility tree).
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
test.use({ viewport: { width: 1440, height: 900 } });

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-acc-"));
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return copy;
}

async function openTrusted(page: Page, root: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(root, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 60_000 });
  // Trust restores and evaluates by itself; the panels under test are not the Restore drawer, so
  // the finished restore is read from the host instead of opening that drawer. The evaluation is
  // then awaited too: a discovery started against the pre-restore model is refused as stale.
  await expect
    .poll(
      async () => {
        const response = await page.request.post(`${host}/call`, {
          data: { method: "workspace.state", params: {} },
        });
        const body = await response.json();
        const state = body.data;
        return state.restore.state === "ok" &&
          state.evaluation.state === "ok" &&
          state.evaluation.stale === false
          ? "ready"
          : `${state.restore.state}:${state.evaluation.state}:${state.evaluation.stale}`;
      },
      { timeout: 180_000 },
    )
    .toBe("ready");
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("D1+D3: single-TFM discovery names net8.0 and the export carries the real run", async ({
  page,
}) => {
  test.setTimeout(300_000);
  const root = tempCopy("Order Desk export");
  await openTrusted(page, root);

  await page.getByTestId("bottom-tab-tests").click();
  await expect(page.getByTestId("tests-panel")).toBeVisible();
  // D3: a project with one evaluated TFM is discovered on that TFM without an explicit choice,
  // so the panel never shows "TFM not selected".
  await expect(page.getByTestId("tests-tfm")).toHaveValue("net8.0");
  await clickDiscover(page);
  await expect(page.getByTestId("tests-state")).toContainText("Discovered 4 tests", {
    timeout: 120_000,
  });
  await expect(page.getByTestId("tests-scope")).toContainText("TFM net8.0");
  await expect(page.getByTestId("tests-command")).toContainText("-f net8.0");
  await expect(page.getByTestId("tests-panel")).not.toContainText("TFM not selected");

  await page.getByTestId("tests-run-all").click();
  await expect(page.getByTestId("tests-run-counts")).toContainText("1 failed", {
    timeout: 120_000,
  });

  // D1: the file the button writes is the panel, not an idle run.
  await page.getByTestId("tests-export").click();
  const exported = page.getByTestId("tests-export-path");
  await expect(exported).toContainText("Exported", { timeout: 30_000 });
  const file = (await exported.locator("code").textContent()) ?? "";
  expect(file).toMatch(/tests-testrun-\d+-\d+\.json$/);
  const payload = JSON.parse(fs.readFileSync(file, "utf8")) as {
    discovery: { revision: number; items: unknown[]; targetFramework: string | null };
    run: { revision: number; runId: string | null; counts: { total: number; failed: number }; command: string[] };
  };
  expect(payload.discovery.items).toHaveLength(4);
  expect(payload.discovery.targetFramework).toBe("net8.0");
  expect(payload.run.revision).toBeGreaterThan(0);
  expect(payload.run.runId).not.toBe(null);
  expect(payload.run.counts.total).toBe(4);
  expect(payload.run.counts.failed).toBe(1);
  expect(payload.run.command.length).toBeGreaterThan(0);
  // The old name was the discovery identity because the run id was null.
  expect(file).not.toContain("tests-discovery-");
});

test("D4: a dependency row's menu is the row owner's and selects that project", async ({ page }) => {
  test.setTimeout(300_000);
  const root = tempCopy("Order Desk menu");
  await openTrusted(page, root);

  // Select one project first: the bug was that the menu of another project's row inherited it.
  await page.getByTestId("tree-project-OrderDesk.Cli").click();
  await page.getByTestId("tree-expand-all").click();
  const packageRow = page.getByTestId("tree-package-OrderDesk.Tests-xunit");
  await expect(packageRow).toBeVisible();
  await packageRow.click({ button: "right" });
  const menu = page.getByTestId("tree-context-menu");
  await expect(menu).toBeVisible();
  // OrderDesk.Tests is a test project: no Run/Debug entry, and the unit-test entry names it.
  await expect(page.getByTestId("tree-menu-run")).toHaveCount(0);
  await expect(page.getByTestId("tree-menu-debug")).toHaveCount(0);
  await expect(page.getByTestId("tree-menu-unit-test")).toContainText("Run Unit Test");
  await expect(menu).not.toContainText("OrderDesk.Cli");

  // A framework row of a console project still runs that console project.
  await page.keyboard.press("Escape");
  await page
    .getByTestId("tree-framework-OrderDesk.Cli-Microsoft.NETCore.App (net8.0)")
    .click({ button: "right" });
  await expect(page.getByTestId("tree-menu-run")).toContainText("Run 'OrderDesk.Cli'");

  // Opening the menu of a row also selects its owner, so the inspector cannot disagree with it.
  await page.keyboard.press("Escape");
  await page.getByTestId("tree-more").click();
  await page.getByTestId("tree-more-details").click();
  await expect(page.getByTestId("eval-details")).toContainText("OrderDesk.Cli");
  await page
    .getByTestId("tree-package-OrderDesk.Tests-xunit")
    .click({ button: "right" });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("eval-details")).toContainText("OrderDesk.Tests");
});

test("D5: the batch bar is rendered once", async ({ page }) => {
  const root = tempCopy("Order Desk nuget");
  await openTrusted(page, root);
  await page.getByTestId("bottom-tab-nuget").click();
  await expect(page.getByTestId("nuget-panel")).toBeVisible();
  await expect(page.getByTestId("nuget-selected-count")).toHaveCount(1);
});

test("D6: the status bar, the test details and the discovery details are named regions", async ({
  page,
}) => {
  test.setTimeout(300_000);
  const root = tempCopy("Order Desk a11y");
  await openTrusted(page, root);

  await expect(page.getByRole("contentinfo", { name: "Status bar" })).toContainText(
    "Trusted mode",
  );

  await page.getByTestId("bottom-tab-tests").click();
  await expect(page.getByRole("region", { name: "Test details" })).toBeVisible();
  await expect(page.getByRole("group", { name: "Discovery details" })).toBeVisible();

  // The panel keeps its name and announces the new selection.
  await clickDiscover(page);
  await expect(page.getByTestId("tests-state")).toContainText("Discovered 4 tests", {
    timeout: 120_000,
  });
  await page.getByTestId("test-item-0").click();
  const details = page.getByRole("region", { name: "Test details" });
  await expect(details).toContainText("OrderDesk.Tests.OrderTests");
  await expect(details).toHaveAttribute("aria-live", "polite");
  await expect.poll(() => revisionOf(page)).toBeGreaterThan(0);
});

test("D3b: the current evaluation wins over the TFM of an earlier discovery", async ({ page }) => {
  test.setTimeout(300_000);
  // One test project with two TFMs: the discovery runs on net8.0, then the model is re-evaluated
  // on netstandard2.0. The selector follows the model; the old discovery cannot hold it back.
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-acc-multi-"));
  const root = path.join(base, "MultiScope");
  fs.mkdirSync(root, { recursive: true });
  fs.writeFileSync(
    path.join(root, "global.json"),
    '{"sdk":{"version":"8.0.408","rollForward":"disable"}}\n',
  );
  fs.writeFileSync(
    path.join(root, "MultiScope.csproj"),
    `<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFrameworks>net8.0;netstandard2.0</TargetFrameworks>
    <IsTestProject>true</IsTestProject>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="Microsoft.NET.Test.Sdk" Version="17.11.1" />
    <PackageReference Include="xunit" Version="2.9.2" />
    <PackageReference Include="xunit.runner.visualstudio" Version="2.8.2" />
  </ItemGroup>
</Project>
`,
  );
  fs.writeFileSync(
    path.join(root, "ScopeTests.cs"),
    "using Xunit;\nnamespace MultiScope;\npublic sealed class ScopeTests { [Fact] public void One() { } }\n",
  );

  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(root, "MultiScope.csproj"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await expect
    .poll(
      async () => {
        const response = await page.request.post(`${host}/call`, {
          data: { method: "workspace.state", params: {} },
        });
        const state = (await response.json()).data;
        return state.restore.state === "ok" && state.evaluation.state === "ok" && !state.evaluation.stale
          ? "ready"
          : `${state.restore.state}:${state.evaluation.state}:${state.evaluation.stale}`;
      },
      { timeout: 180_000 },
    )
    .toBe("ready");

  await page.getByTestId("bottom-tab-tests").click();
  await page.getByTestId("tests-tfm").selectOption("net8.0");
  await expect.poll(() => revisionOf(page)).toBeGreaterThan(0);
  await clickDiscover(page);
  await expect(page.getByTestId("tests-state")).toContainText("Discovered", { timeout: 180_000 });
  const discovered = await page.request.post(`${host}/call`, {
    data: { method: "workspace.state", params: {} },
  });
  expect((await discovered.json()).data.tests.discovery.targetFramework).toBe("net8.0");

  // The model moves to the other TFM: the panel must show the model's TFM, not the discovery's.
  await page.getByTestId("tests-tfm").selectOption("netstandard2.0");
  await expect
    .poll(
      async () => {
        const response = await page.request.post(`${host}/call`, {
          data: { method: "workspace.state", params: {} },
        });
        return (await response.json()).data.evaluation.context.targetFramework;
      },
      { timeout: 180_000 },
    )
    .toBe("netstandard2.0");
  await expect(page.getByTestId("tests-tfm")).toHaveValue("netstandard2.0");
});
