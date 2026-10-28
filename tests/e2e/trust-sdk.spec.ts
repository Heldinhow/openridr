import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
const solution = path.join(fixture, "OrderDesk.sln");
const program = path.join(fixture, "src/OrderDesk.Cli/Program.cs");
const programTab = "src/OrderDesk.Cli/Program.cs";
const missingFixture = path.resolve(process.cwd(), "fixtures/MissingSdk");
const missingProject = path.join(missingFixture, "MissingSdk.csproj");
const missingGlobal = path.join(missingFixture, "global.json");
const originalProgram = fs.readFileSync(program, "utf8");
const host = "http://127.0.0.1:4317";

test.beforeEach(() => {
  fs.writeFileSync(program, originalProgram);
});

test.afterEach(async ({ request }) => {
  await cleanupHost(request);
});

function hashOf(file: string) {
  return createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

async function cleanupHost(request: APIRequestContext) {
  const state = await request.post(`${host}/call`, {
    data: { method: "workspace.state", params: {} },
  });
  const body = await state.json().catch(() => null);
  for (const process of body?.data?.processes ?? []) {
    if (process.state === "running") {
      await request.post(`${host}/call`, {
        data: { method: "processes.terminate", params: { id: process.id } },
      });
    }
  }
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
  fs.writeFileSync(program, originalProgram);
}

async function openWorkspace(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
}

async function trustWorkspace(page: Page) {
  await page.getByTestId("trust-checkbox").check();
  await expect(page.getByTestId("trust-grant")).toBeEnabled();
  await page.getByTestId("trust-grant").click();
  await expect(page.getByTestId("mode-trusted")).toBeVisible();
}

async function enterWorkspace(page: Page) {
  await page.getByTestId("open-text-only").click();
  await expect(page.getByTestId("tree")).toBeVisible();
}

async function replaceEditorText(page: Page, text: string) {
  const editor = page.getByTestId("editor");
  await editor.click();
  await page.keyboard.press("Meta+A");
  await page.keyboard.insertText(text);
}

test("trust resolves the project SDK, revoke blocks calls and editing still works", async ({
  page,
  request,
}) => {
  await openWorkspace(page, solution);

  await expect(page.getByTestId("sdk-host")).toContainText("SDK 10.0.100");
  await expect(page.getByTestId("sdk-host")).toContainText("host analysis");
  await expect(page.getByTestId("sdk-project")).toContainText("Not inspected yet");
  await expect(page.getByTestId("sdk-project")).toContainText("8.0.408");
  await expect(page.getByTestId("sdk-project")).toContainText("rollForward disable");
  await expect(page.getByTestId("sdk-tfms")).toHaveText("net8.0 declared · not evaluated");
  await expect(page.getByTestId("sdk-debug-adapter")).toContainText(
    "Not checked in this slice",
  );
  if (process.arch === "arm64") {
    await expect(page.getByTestId("sdk-apple-silicon")).toContainText(
      "experimental · disabled",
    );
  }

  await trustWorkspace(page);
  await expect(page.getByTestId("sdk-project")).toContainText("SDK 8.0.408");
  await expect(page.getByTestId("sdk-project")).toContainText("resolved after trust");
  await expect(page.getByTestId("sdk-project")).toContainText("OpenRidr/sdks/8.0.408");
  await expect(page.getByTestId("sdk-project")).not.toContainText("10.0.100");
  await expect(page.getByTestId("trust-note")).toContainText("recorded in host memory");

  await enterWorkspace(page);
  await expect(page.getByTestId("mode-badge")).toHaveText("Trusted mode");
  await expect(page.getByTestId("status-sdk")).toHaveText(
    "Host SDK 10.0.100 · Project SDK 8.0.408",
  );

  await page.getByTestId(`tree-file-${programTab}`).click();
  await expect(page.getByTestId("editor")).toBeVisible();
  const trustedEdit = "// trusted edit\nConsole.WriteLine(\"trusted\");\n";
  await replaceEditorText(page, trustedEdit);
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+s");
  await expect(page.getByTestId("app-log")).toContainText(`Saved ${programTab}`);
  expect(fs.readFileSync(program, "utf8")).toBe(trustedEdit);

  await page.getByTestId("trust-revoke").click();
  await expect(page.getByTestId("mode-badge")).toHaveText("Restricted mode");
  await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");
  await expect(page.getByTestId("status-sdk")).toHaveText(
    "Host SDK 10.0.100 · Project SDK not inspected",
  );
  await expect(page.getByTestId("trust-banner")).toContainText("Blocked in Restricted mode");
  await expect(page.getByTestId("trust-checkbox")).not.toBeChecked();

  const denied = await request.post(`${host}/call`, {
    data: { method: "build.start", params: {} },
  });
  expect((await denied.json()).error.code).toBe("trust_required");
  const state = await request.post(`${host}/call`, {
    data: { method: "workspace.state", params: {} },
  });
  const stateBody = await state.json();
  expect(stateBody.data.trust.granted).toBe(false);
  expect(stateBody.data.workspace.mode).toBe("restricted");
  expect(stateBody.data.sdk.project.state).toBe("not_inspected");

  const revokedEdit = "// edit after revoke\nConsole.WriteLine(\"restricted\");\n";
  await replaceEditorText(page, revokedEdit);
  await page.getByTestId("editor").click();
  await page.keyboard.press("Meta+s");
  await expect(page.getByTestId("app-log")).toContainText(`Saved ${programTab}`);
  expect(fs.readFileSync(program, "utf8")).toBe(revokedEdit);

  await page.getByTestId("close-workspace").click();
  await expect(page.getByTestId("workspace-path-input")).toBeVisible();
  await expect(page.getByTestId("sdk-host")).toContainText("10.0.100");
  await expect(page.getByTestId("sdk-project")).toContainText("Open a workspace");
});

test("missing project SDK is distinct and global.json is never rewritten", async ({
  page,
  request,
}) => {
  const before = hashOf(missingGlobal);

  await openWorkspace(page, missingProject);
  await expect(page.getByTestId("sdk-host")).toContainText("SDK 10.0.100");

  await page.getByTestId("sdk-redetect").click();
  await expect(page.getByTestId("sdk-project")).toContainText("Not inspected yet");

  await trustWorkspace(page);
  await expect(page.getByTestId("sdk-project")).toContainText("Project SDK missing");
  await expect(page.getByTestId("sdk-project")).toContainText("8.0.999");
  await expect(page.getByTestId("sdk-project")).toContainText("rollForward disable");
  await expect(page.getByTestId("sdk-project")).not.toContainText("resolved after trust");
  await expect(
    page.getByTestId("sdk-project").getByRole("link", { name: "Official .NET downloads" }),
  ).toBeVisible();
  const state = await request.post(`${host}/call`, {
    data: { method: "workspace.state", params: {} },
  });
  const stateBody = await state.json();
  expect(stateBody.data.sdk.host.version).toBe("10.0.100");
  expect(stateBody.data.sdk.project.state).toBe("sdk_missing");
  expect(stateBody.data.sdk.project.resolvedVersion).toBeNull();
  await expect(page.getByTestId("sdk-open-global-json")).toBeVisible();
  expect(hashOf(missingGlobal)).toBe(before);

  await page.getByTestId("sdk-redetect").click();
  await expect(page.getByTestId("sdk-project")).toContainText("Project SDK missing");
  expect(hashOf(missingGlobal)).toBe(before);

  await page.getByTestId("sdk-open-global-json").click();
  await expect(page.getByTestId("tree")).toBeVisible();
  await expect(page.getByTestId("editor")).toContainText("8.0.999");
  await expect(page.getByTestId("mode-badge")).toHaveText("Trusted mode");
  expect(hashOf(missingGlobal)).toBe(before);

  expect(fs.existsSync(path.join(missingFixture, "bin"))).toBe(false);
  expect(fs.existsSync(path.join(missingFixture, "obj"))).toBe(false);

  await page.getByTestId("close-workspace").click();
  await expect(page.getByTestId("workspace-path-input")).toBeVisible();
  expect(hashOf(missingGlobal)).toBe(before);
});

test("revoked workspace shows the remaining process and terminate observes the exit", async ({
  page,
  request,
}) => {
  await openWorkspace(page, solution);
  await trustWorkspace(page);

  const probe = await request.post(`${host}/call`, {
    data: { method: "processes.probe", params: { kind: "sleep" } },
  });
  const probeBody = await probe.json();
  if (!probeBody.ok && /OPENRIDR_ENABLE_PROBE/.test(probeBody.error.message)) {
    await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
    test.skip(
      true,
      "host-http was reused without OPENRIDR_ENABLE_PROBE=1; restart it to run the probe test.",
    );
  }
  expect(probeBody.ok).toBe(true);
  const processInfo = probeBody.data.processes.at(-1);
  expect(processInfo.state).toBe("running");
  expect(processInfo.exit).toBeNull();

  await enterWorkspace(page);
  const row = page.getByTestId(`processes-row-${processInfo.id}`);
  await expect(row).toBeVisible();
  await expect(row).toContainText(`pid ${processInfo.pid}`);

  await page.getByTestId("trust-revoke").click();
  await expect(page.getByTestId("mode-badge")).toHaveText("Restricted mode");
  await expect(page.getByTestId(`processes-awaiting-${processInfo.id}`)).toBeVisible();
  await expect(page.getByTestId(`processes-state-${processInfo.id}`)).toContainText(
    "awaiting decision",
  );

  // Closing the workspace with an unsaved buffer must not hide the process state.
  await page.getByTestId(`tree-file-${programTab}`).click();
  await expect(page.getByTestId("editor")).toBeVisible();
  await replaceEditorText(page, "// unsaved when the workspace closes\n");
  await page.getByTestId("close-workspace").click();
  await expect(page.getByRole("dialog")).toContainText("Close workspace with unsaved changes?");
  await page.getByTestId("confirm-discard").click();
  await expect(page.getByTestId("workspace-path-input")).toBeVisible();
  await expect(page.getByTestId("processes-panel")).toBeVisible();
  await expect(page.getByTestId(`processes-awaiting-${processInfo.id}`)).toBeVisible();

  await page.getByTestId(`processes-terminate-${processInfo.id}`).click();
  await expect(page.getByTestId(`processes-state-${processInfo.id}`)).toContainText("exited");
  await expect(page.getByTestId(`processes-state-${processInfo.id}`)).toContainText(
    "exit unknown",
  );
  await expect(page.getByTestId(`processes-termination-${processInfo.id}`)).toContainText(
    "Observed: exited",
  );
  await expect(page.getByTestId(`processes-awaiting-${processInfo.id}`)).toHaveCount(0);
});
