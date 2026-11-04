import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import { revisionOf, settleRestore } from "./automatic";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");

test.use({ viewport: { width: 1440, height: 900 } });

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-tests-auto-"));
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return { base, copy };
}

function tempProject(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-tests-auto-"));
  const copy = path.join(base, name);
  fs.mkdirSync(copy, { recursive: true });
  fs.writeFileSync(
    path.join(copy, "global.json"),
    '{"sdk":{"version":"8.0.408","rollForward":"disable"}}\n',
  );
  fs.writeFileSync(
    path.join(copy, `${name}.csproj`),
    `<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <IsTestProject>true</IsTestProject>
  </PropertyGroup>
</Project>
`,
  );
  fs.writeFileSync(path.join(copy, "Code.cs"), "public sealed class C { }\n");
  return { base, copy };
}

/** A second test project with the same source; the two projects stay distinguishable by path. */
function addSecondTestProject(copy: string) {
  const source = path.join(copy, "tests/OrderDesk.Tests");
  const target = path.join(copy, "tests/OrderDesk.MoreTests");
  fs.cpSync(source, target, { recursive: true });
  fs.renameSync(
    path.join(target, "OrderDesk.Tests.csproj"),
    path.join(target, "OrderDesk.MoreTests.csproj"),
  );

  const guid = "{B7C1F1F2-3A4B-4C5D-8E9F-0A1B2C3D4E5F}";
  const slnPath = path.join(copy, "OrderDesk.sln");
  let sln = fs.readFileSync(slnPath, "utf8");
  sln = sln.replace(
    /(\r?\n)Global(\r?\n)/,
    `$1Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "OrderDesk.MoreTests", "tests\\OrderDesk.MoreTests\\OrderDesk.MoreTests.csproj", "${guid}"$1EndProject$1Global$2`,
  );
  sln = sln.replace(
    /(\tGlobalSection\(ProjectConfigurationPlatforms\) = postSolution\r?\n[\s\S]*?)(\tEndGlobalSection)/,
    `$1\t\t${guid}.Debug|Any CPU.ActiveCfg = Debug|Any CPU\r\n\t\t${guid}.Debug|Any CPU.Build.0 = Debug|Any CPU\r\n\t\t${guid}.Release|Any CPU.ActiveCfg = Release|Any CPU\r\n\t\t${guid}.Release|Any CPU.Build.0 = Release|Any CPU\r\n$2`,
  );
  fs.writeFileSync(slnPath, sln);
}

async function openTrusted(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });
  await settleRestore(page);
  await expect.poll(() => revisionOf(page)).toBeGreaterThanOrEqual(2);
}

async function workspaceState(request: APIRequestContext) {
  const response = await request.post(`${host}/call`, {
    data: { method: "workspace.state", params: {} },
  });
  const body = await response.json();
  expect(body.ok).toBe(true);
  return body.data;
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("discovery fills the tree without a click and an edit or a panel reopen starts nothing", async ({
  page,
  request,
}) => {
  test.setTimeout(240_000);
  const { base, copy } = tempCopy("Order Desk auto discovery");
  try {
    await openTrusted(page, path.join(copy, "OrderDesk.sln"));

    // The panel was never opened: the automatic discovery runs on the evaluation/restore boundary.
    await expect
      .poll(async () => (await workspaceState(request)).tests.discovery.state, {
        timeout: 120_000,
      })
      .toBe("completed");
    const discovered = await workspaceState(request);
    expect(discovered.tests.discovery.items).toHaveLength(4);
    const discoveryId = discovered.tests.discovery.discoveryId;
    const revision = discovered.tests.discovery.revision;
    expect(discoveryId).toMatch(/^discovery:\d+:\d+$/);
    expect(revision).toBeGreaterThan(0);

    await page.getByTestId("bottom-tab-tests").click();
    await expect(page.getByTestId("tests-items")).toBeVisible();
    await expect(page.getByTestId("tests-discover")).toHaveText("Refresh tests");
    await expect(page.getByTestId("test-identity-0")).toHaveText(
      "OrderDesk.Tests.OrderTests.Id_is_preserved",
    );

    // An edit does not touch the host's discovery record.
    await page.getByTestId("tree-file-tests/OrderDesk.Tests/OrderTests.cs").click();
    await page.getByTestId("editor").click();
    await page.keyboard.press("Meta+End");
    await page.keyboard.insertText("\n// automatic discovery must not run again\n");
    await expect(page.getByTestId("dirty-marker")).toBeVisible();
    await page.waitForTimeout(1_500);
    let after = await workspaceState(request);
    expect(after.tests.discovery.discoveryId).toBe(discoveryId);
    expect(after.tests.discovery.revision).toBe(revision);

    // Reopening the panel is not a trigger either.
    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("bottom-tab-tests").click();
    await page.waitForTimeout(500);
    after = await workspaceState(request);
    expect(after.tests.discovery.discoveryId).toBe(discoveryId);
    expect(after.tests.discovery.revision).toBe(revision);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("automatic discovery shows an adapter failure without pressing Discover", async ({
  page,
  request,
}) => {
  test.setTimeout(180_000);
  const { base, copy } = tempProject("AutoNoAdapter");
  try {
    await openTrusted(page, path.join(copy, "AutoNoAdapter.csproj"));

    await expect
      .poll(async () => (await workspaceState(request)).tests.discovery.state, {
        timeout: 60_000,
      })
      .toBe("adapter_missing");

    await page.getByTestId("bottom-tab-tests").click();
    await expect(page.getByTestId("tests-state")).toContainText("No VSTest adapter");
    await expect(page.getByTestId("tests-guidance")).toContainText("Microsoft.NET.Test.Sdk");
    await expect(page.getByTestId("tests-guidance")).toContainText(
      "does not add or install packages",
    );
    const state = await workspaceState(request);
    expect(state.tests.discovery.command).toEqual([]);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("selecting another test project runs its discovery", async ({ page, request }) => {
  test.setTimeout(240_000);
  const { base, copy } = tempCopy("Order Desk switch");
  addSecondTestProject(copy);
  try {
    await openTrusted(page, path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("bottom-tab-tests").click();
    await expect(page.getByTestId("tests-project")).toHaveValue("");

    const first = "tests/OrderDesk.Tests/OrderDesk.Tests.csproj";
    const second = "tests/OrderDesk.MoreTests/OrderDesk.MoreTests.csproj";
    await page.getByTestId("tests-project").selectOption(first);
    await expect
      .poll(
        async () => {
          const discovery = (await workspaceState(request)).tests.discovery;
          return `${discovery.project?.path}:${discovery.state}`;
        },
        { timeout: 120_000 },
      )
      .toBe(`${first}:completed`);
    const firstRevision = (await workspaceState(request)).tests.discovery.revision;

    await page.getByTestId("tests-project").selectOption(second);
    await expect
      .poll(
        async () => {
          const discovery = (await workspaceState(request)).tests.discovery;
          return discovery.project?.path === second &&
            discovery.state === "completed" &&
            discovery.revision > firstRevision
            ? "done"
            : `${discovery.project?.path}:${discovery.state}:${discovery.revision}`;
        },
        { timeout: 120_000 },
      )
      .toBe("done");
    await expect(page.getByTestId("tests-scope")).toContainText("OrderDesk.MoreTests");
    await expect(page.getByTestId("tests-state")).toContainText("Discovered 4 tests");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
