import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import path from "node:path";
import { host } from "./environment";
import { openToolchain } from "./disclosures";

// The UI must ask only for what this project needs: the SDK pinned by global.json or the
// .NET 10 runtime that runs csharp-ls. The host decides; these tests read the rendered answer.
const solution = path.resolve("fixtures/OrderDesk/OrderDesk.sln");
const missingProject = path.resolve("fixtures/MissingSdk/MissingSdk.csproj");
// A net10.0 project with no global.json: the TargetFramework is the SDK requirement.
const net10NoPin = path.resolve("fixtures/Net10NoPin/Net10NoPin.csproj");

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

// Mirrors ProjectSdk::missing(&request, resolve(..., installs = [])) for the MissingSdk pin
// (crates/host/src/sdk.rs, guarded by a unit test): no SDK in any root. A machine without an SDK
// cannot be produced here, so the snapshot is overlaid after the host response.
const noSdkInstalled = {
  detected: [],
  message:
    "No detected .NET SDK satisfies global.json version 8.0.999 with rollForward disable (detected: none). OpenRidr does not rewrite global.json and does not substitute the .NET 10 analysis runtime.",
};

// The installer extension the host appends for this platform.
const extension =
  process.platform === "darwin" ? "pkg" : process.platform === "win32" ? "exe" : "tar.gz";

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

async function setupState(page: Page) {
  const response = await page.request.post(`${host}/call`, {
    data: { method: "app.state", params: {} },
  });
  return ((await response.json()) as { data: { setup: { state: string } } }).data.setup.state;
}

async function trustViaHost(page: Page) {
  // A workspace opened through the host does not run the setup gate, and the direct grant is
  // refused until the host reports the verified tools ready.
  if ((await setupState(page)) !== "ready") {
    await page.request.post(`${host}/call`, { data: { method: "setup.start", params: {} } });
    await expect.poll(() => setupState(page), { timeout: 120_000 }).toBe("ready");
  }
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

// The reload lands on the welcome screen; the form's Open is what moves the app into the IDE, and
// a fresh open asks for the Trust decision again (the host grant is per open).
async function enterWorkspace(page: Page, target: string) {
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
  const prompt = page.getByTestId("trust-prompt-dialog");
  await prompt.or(page.getByTestId("tree")).first().waitFor();
  if (await prompt.isVisible()) await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function overlayNoSdk(page: Page) {
  await page.route("**/__host/call", async (route) => {
    const method = (route.request().postDataJSON() as { method?: string } | null)?.method;
    if (
      method !== "app.state" &&
      method !== "workspace.state" &&
      method !== "workspace.open" &&
      method !== "sdk.redetect"
    ) {
      return route.continue();
    }
    const response = await route.fetch();
    const body = await response.json();
    if (body?.data?.sdk?.project) {
      body.data.sdk.project = { ...body.data.sdk.project, ...noSdkInstalled };
    }
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

test("a net10 project without global.json resolves the installed SDK 10", async ({
  page,
  request,
}) => {
  await openViaHost(page, net10NoPin);
  const opened = await call(request, "workspace.state");
  const detected: string[] = opened.data.sdk.project.detected;
  test.skip(
    !detected.some((version) => version.startsWith("10.")),
    "this machine has no SDK 10; a net10.0 project cannot resolve here",
  );
  await trustViaHost(page);

  const state = await call(request, "workspace.state");
  const project = state.data.sdk.project;
  expect(project.state).toBe("resolved");
  expect(project.requirement.source).toBe("target_framework");
  expect(project.requirement.line).toBe("10.0");
  expect(project.requirement.version).toBeNull();
  expect(project.globalJsonPath).toBeNull();
  expect(project.requestedVersion).toBeNull();
  expect(project.resolvedVersion).toMatch(/^10\./);
  expect(project.detected).toContain(project.resolvedVersion);
  expect(project.message).toBeNull();

  await expect(page.getByTestId("welcome-sdk-warning")).toHaveCount(0);
  await openToolchain(page);
  const card = page.getByTestId("sdk-project");
  await expect(card).toContainText(`SDK ${project.resolvedVersion}`);
  await expect(card).toContainText("requires SDK 10.0 or newer");
  // Nothing is missing, so nothing complains about global.json.
  await expect(card).not.toContainText("No global.json");
  await expect(card).not.toContainText("Project SDK missing");
});

test("a missing project SDK names the pinned version and links the exact patch, never the runtime", async ({
  page,
  request,
}) => {
  await openViaHost(page, missingProject);
  await trustViaHost(page);
  await enterWorkspace(page, missingProject);

  const state = await call(request, "workspace.state");
  const sdk = state.data.sdk.project;
  expect(sdk.state).toBe("sdk_missing");
  expect(sdk.requestedVersion).toBe("8.0.999");
  // The pin names a version: the exact patch, with the line page as the way out of a dead patch.
  expect(sdk.download.label).toBe("Download SDK 8.0.999");
  expect(sdk.download.url).toContain("/Sdk/8.0.999/dotnet-sdk-8.0.999-");
  expect(sdk.download.url.endsWith(`.${extension}`)).toBe(true);
  expect(sdk.download.fallbackUrl).toBe("https://dotnet.microsoft.com/download/dotnet/8.0");
  const detected = sdk.detected.length > 0 ? `Detected SDKs: ${sdk.detected.join(", ")}` : "No SDK detected";

  // The tree note carries the requirement, the detected versions and both links.
  const reason = page.getByTestId("eval-disabled-reason");
  await expect(reason).toContainText("global.json requests 8.0.999");
  await expect(reason).toContainText("rollForward disable");
  await expect(reason).toContainText(detected);
  await expect(reason).not.toContainText("SDK 10");
  await expect(reason).not.toContainText(".NET 10 runtime was found");
  await expect(page.getByTestId("eval-sdk-download")).toHaveAttribute("href", sdk.download.url);
  await expect(page.getByTestId("sdk-download-fallback")).toHaveAttribute(
    "href",
    "https://dotnet.microsoft.com/download/dotnet/8.0",
  );
  // The bar is one line and holds no link: it names the requirement and the detected versions.
  await expect(page.getByTestId("status-sdk")).toContainText("global.json requests 8.0.999");
  await expect(page.getByTestId("status-sdk")).toContainText(
    `detected: ${sdk.detected.join(", ") || "none"}`,
  );

  // The Toolchain card lives in the welcome view; a reload lands there with the workspace still open.
  await page.reload();
  await openToolchain(page);
  const card = page.getByTestId("sdk-project");
  await expect(card).toContainText("Project SDK missing");
  await expect(card).toContainText("global.json requests 8.0.999");
  await expect(card).toContainText(detected);
  await expect(card).not.toContainText("SDK 10");
  await expect(card).not.toContainText(".NET 10 runtime was found");
  await expect(page.getByTestId("sdk-download")).toHaveAttribute("href", sdk.download.url);
  await expect(page.getByTestId("sdk-download-fallback")).toHaveAttribute(
    "href",
    "https://dotnet.microsoft.com/download/dotnet/8.0",
  );
});

test("with no SDK installed at all the block says so and still links the download", async ({
  page,
}) => {
  await openViaHost(page, missingProject);
  await overlayNoSdk(page);
  await trustViaHost(page);
  await enterWorkspace(page, missingProject);

  const reason = page.getByTestId("eval-disabled-reason");
  await expect(reason).toContainText("global.json requests 8.0.999");
  // "No SDK installed" and "the installed ones do not satisfy" differ by the message, not a state.
  await expect(reason).toContainText("No SDK detected");
  await expect(reason).toContainText("detected: none");
  await expect(page.getByTestId("eval-sdk-download")).toHaveAttribute(
    "href",
    /\/Sdk\/8\.0\.999\/dotnet-sdk-8\.0\.999-/,
  );
  await expect(page.getByTestId("sdk-download-fallback")).toHaveAttribute(
    "href",
    "https://dotnet.microsoft.com/download/dotnet/8.0",
  );
  await expect(page.getByTestId("status-sdk")).toContainText("detected: none");

  await page.reload();
  await openToolchain(page);
  const card = page.getByTestId("sdk-project");
  await expect(card).toContainText("Project SDK missing");
  await expect(card).toContainText("global.json requests 8.0.999");
  await expect(card).toContainText("No SDK detected");
  await expect(page.getByTestId("sdk-download")).toHaveAttribute(
    "href",
    /\/Sdk\/8\.0\.999\/dotnet-sdk-8\.0\.999-/,
  );
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
