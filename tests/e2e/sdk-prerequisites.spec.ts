import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";
import { openToolchain } from "./disclosures";

// The UI must ask only for what this project needs: the SDK pinned by global.json or the
// .NET 10 runtime that runs csharp-ls. The host decides; these tests read the rendered answer.
const solution = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
const missingProject = path.resolve("fixtures/MissingSdk/MissingSdk.csproj");

// Mirrors sdk::AnalysisRuntime::from_runtimes(&[]) (crates/host/src/sdk.rs, guarded by a unit
// test). A real runtime-10 root cannot be hidden from the shared E2E host, so the snapshot is
// overlaid after the host response while every other field stays real.
const missingRuntime = {
  state: "missing",
  version: null,
  root: null,
  sdkPath: null,
  instructionsUrl: "https://dotnet.microsoft.com/download/dotnet/10.0",
  message:
    "No .NET 10 runtime (Microsoft.NETCore.App) was found in the detected roots. C# analysis needs that runtime; the project SDK is separate and is never used as a substitute.",
};

async function call(request: APIRequestContext, method: string, params: object = {}) {
  const response = await request.post(`${host}/call`, { data: { method, params } });
  return (await response.json()) as { ok: boolean; data: any };
}

async function openViaHost(page: Page, target: string) {
  await page.request.post(`${host}/call`, {
    data: { method: "workspace.open", params: { path: target } },
  });
  await page.goto("/");
}

async function trustViaHost(page: Page) {
  await page.request.post(`${host}/call`, {
    data: { method: "trust.grant", params: { remember: false } },
  });
  await page.reload();
}

async function overlayMissingRuntime(page: Page) {
  await page.route("**/__host/call", async (route) => {
    const method = (route.request().postDataJSON() as { method?: string } | null)?.method;
    if (method !== "app.state" && method !== "workspace.state" && method !== "sdk.redetect") {
      return route.continue();
    }
    const response = await route.fetch();
    const body = await response.json();
    if (body?.data?.sdk?.host) body.data.sdk.host = missingRuntime;
    await route.fulfill({
      status: response.status(),
      contentType: "application/json",
      body: JSON.stringify(body),
    });
  });
}

test.afterEach(async ({ request }) => {
  await call(request, "workspace.close");
});

test("a project pinned to SDK 8.0.408 with the .NET 10 runtime shows no prerequisite banner", async ({
  page,
  request,
}) => {
  await openViaHost(page, solution);
  const state = await call(request, "workspace.state");
  test.skip(
    state.data.sdk.host.state !== "found",
    "this machine has no .NET 10 runtime; the missing-runtime branch is covered below",
  );
  await trustViaHost(page);

  await expect(page.getByTestId("welcome-sdk-warning")).toHaveCount(0);
  await openToolchain(page);
  await expect(page.getByTestId("sdk-host")).toContainText(/\.NET 10\.\d+\.\d+ runtime/);
  await expect(page.getByTestId("sdk-host")).not.toContainText("runtime needed");
  await expect(page.getByTestId("sdk-host-instructions")).toHaveCount(0);
  await expect(page.getByTestId("sdk-project")).toContainText("SDK 8.0.408");
  await expect(page.getByTestId("sdk-project")).toContainText("global.json requests 8.0.408");
});

test("a missing project SDK names the pinned version and links the SDK page, never the runtime", async ({
  page,
  request,
}) => {
  await openViaHost(page, missingProject);
  await trustViaHost(page);
  await openToolchain(page);

  const project = page.getByTestId("sdk-project");
  await expect(project).toContainText("Project SDK missing");
  await expect(project).toContainText("global.json requests 8.0.999");
  await expect(project).not.toContainText("SDK 10");
  await expect(project).not.toContainText(".NET 10 runtime was found");
  const link = project.getByRole("link", { name: "Official .NET SDK downloads" });
  await expect(link).toHaveAttribute("href", "https://dotnet.microsoft.com/download/dotnet");

  const state = await call(request, "workspace.state");
  expect(state.data.sdk.project.state).toBe("sdk_missing");
  expect(state.data.sdk.project.requestedVersion).toBe("8.0.999");
});

test("a missing .NET 10 runtime asks for the runtime, not SDK 10", async ({ page, request }) => {
  await call(request, "workspace.open", { path: solution });
  await call(request, "trust.grant", { remember: false });
  await overlayMissingRuntime(page);
  await page.goto("/");

  const banner = page.getByTestId("welcome-sdk-warning");
  await expect(banner).toContainText("Analysis runtime missing");
  await expect(banner).toContainText("C# analysis needs the .NET 10 runtime");
  await expect(banner).not.toContainText("SDK 10");

  await openToolchain(page);
  const runtime = page.getByTestId("sdk-host");
  await expect(runtime).toContainText("Analysis runtime missing");
  await expect(runtime).toContainText(".NET 10 runtime");
  await expect(runtime).not.toContainText("SDK 10");
  await expect(runtime).toContainText("No environment variables");
  const link = page.getByTestId("sdk-host-instructions");
  await expect(link).toHaveAttribute("href", "https://dotnet.microsoft.com/download/dotnet/10.0");
  await expect(link).toHaveText("Get the .NET 10 runtime");
  // The project card keeps its own answer: the pinned SDK, not the analysis runtime.
  await expect(page.getByTestId("sdk-project")).toContainText("8.0.408");
});
