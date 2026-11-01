import {
  expect,
  request as playwrightRequest,
  test,
  type APIRequestContext,
  type Locator,
  type Page,
} from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

// Text of the Build/Run/Debug gates in src/executionGates.ts, asserted at real seams.
const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(180_000);

async function openTrustedRestored(page: Page) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-gates-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(copy, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", { timeout: 60_000 });
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });
  return base;
}

function createRestrictedWorkspace() {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-d13-"));
  const copy = path.join(base, "OrderDesk");
  fs.cpSync(fixture, copy, { recursive: true, filter: (s) => !/[\\/](bin|obj)([\\/]|$)/.test(s) });
  return { base, solution: path.join(copy, "OrderDesk.sln") };
}

async function openWorkspaceCandidate(page: Page, solution: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
  await expect(page.getByTestId("trust-prompt-dialog")).toBeVisible();
}

async function enterRestrictedWorkspace(page: Page, solution: string) {
  await openWorkspaceCandidate(page, solution);
  await page.getByTestId("trust-prompt-restricted").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");
  await expect(page.getByTestId("tree")).toBeVisible();
}

/** The host snapshot is the authority; /var and /private/var must compare by real path. */
async function activeWorkspacePath(request: APIRequestContext) {
  const response = await request.post(`${host}/call`, {
    data: { method: "workspace.state", params: {} },
  });
  const body = (await response.json()) as {
    ok: boolean;
    data?: { workspace?: { path?: string } | null };
    error?: unknown;
  };
  if (!body.ok || !body.data || !Object.hasOwn(body.data, "workspace")) {
    throw new Error(`workspace.state did not confirm the active workspace: ${JSON.stringify(body.error ?? body)}`);
  }
  const workspace = body.data.workspace;
  if (workspace === null) return null;
  if (typeof workspace?.path !== "string" || workspace.path === "") {
    throw new Error("workspace.state returned a workspace without a path");
  }
  return workspace.path;
}

function sameWorkspacePath(actual: string | null, expected: string) {
  if (!actual) return false;
  try {
    return fs.realpathSync.native(actual) === fs.realpathSync.native(expected);
  } catch {
    return false;
  }
}

async function closeOwnedWorkspace(request: APIRequestContext) {
  const response = await request.post(`${host}/call`, {
    data: { method: "workspace.close", params: {} },
  });
  const body = (await response.json()) as { ok: boolean; error?: unknown };
  if (!body.ok) {
    throw new Error(`workspace.close failed: ${JSON.stringify(body.error)}`);
  }
}

type CleanupResult = { removed: boolean; retained: boolean; reason: string | null };

/**
 * Snapshot -> ownership check -> close -> snapshot again -> remove. A failed/ambiguous ownership,
 * close, or state confirmation retains the copy and reports why; it never closes a foreign workspace.
 */
async function cleanupD13Workspace(
  base: string,
  request: APIRequestContext,
  solution: string,
): Promise<CleanupResult> {
  let active: string | null;
  try {
    active = await activeWorkspacePath(request);
  } catch (error) {
    return { removed: false, retained: true, reason: `workspace state unconfirmed: ${String(error)}` };
  }
  if (active !== null && !sameWorkspacePath(active, solution)) {
    return {
      removed: false,
      retained: true,
      reason: `active workspace ${active} is not the test copy ${solution}`,
    };
  }
  if (active !== null) {
    try {
      await closeOwnedWorkspace(request);
    } catch (error) {
      return { removed: false, retained: true, reason: `workspace.close failed: ${String(error)}` };
    }
    let after: string | null;
    try {
      after = await activeWorkspacePath(request);
    } catch (error) {
      return { removed: false, retained: true, reason: `post-close state unconfirmed: ${String(error)}` };
    }
    if (after !== null) {
      return {
        removed: false,
        retained: true,
        reason: `workspace.close returned ok but ${after} is still active`,
      };
    }
  }
  try {
    fs.rmSync(base, { recursive: true, force: true });
  } catch (error) {
    return { removed: false, retained: true, reason: `temp-copy removal failed: ${String(error)}` };
  }
  return { removed: true, retained: false, reason: null };
}

function reportRetention(reason: string, bodyFailed: boolean) {
  const text = `D13 cleanup retained the copy: ${reason}`;
  if (bodyFailed) console.error(text);
  else throw new Error(text);
}

const hostCallPattern = "**/__host/call";

/** Delay the real trust.grant bridge call so the UI's pending state is observable. */
async function delayNextTrustGrant(page: Page) {
  let release!: () => void;
  const blocked = new Promise<void>((resolve) => {
    release = resolve;
  });
  let finish!: () => void;
  const finished = new Promise<void>((resolve) => {
    finish = resolve;
  });
  let delayed = false;
  let released = false;
  await page.route(hostCallPattern, async (route) => {
    const body = route.request().postDataJSON() as { method?: string } | null;
    if (!delayed && body?.method === "trust.grant") {
      delayed = true;
      await blocked;
      try {
        await route.continue();
      } finally {
        finish();
      }
      return;
    }
    await route.continue();
  });
  return async () => {
    if (released) return;
    released = true;
    release();
    if (delayed) await finished;
    await page.unroute(hostCallPattern);
  };
}

/** Reject only the next trust.grant at the bridge boundary; all other calls stay real. */
async function rejectNextTrustGrant(page: Page) {
  let rejected = false;
  await page.route(hostCallPattern, async (route) => {
    const body = route.request().postDataJSON() as { method?: string } | null;
    if (!rejected && body?.method === "trust.grant") {
      rejected = true;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          ok: false,
          error: {
            code: "injected_failure",
            message: "Injected trust failure (UI boundary).",
          },
        }),
      });
      return;
    }
    await route.continue();
  });
  return async () => {
    await page.unroute(hostCallPattern);
  };
}

/** A point inside the target that is currently covered by the Trust popover. */
async function coveredTrustPoint(page: Page, target: Locator) {
  const popup = page.getByTestId("status-mode-popover");
  const popupBox = await popup.boundingBox();
  const targetBox = await target.boundingBox();
  if (!popupBox || !targetBox) throw new Error("The popup or target has no measurable box.");
  expect(
    popupBox.x < targetBox.x + targetBox.width &&
      popupBox.x + popupBox.width > targetBox.x &&
      popupBox.y < targetBox.y + targetBox.height &&
      popupBox.y + popupBox.height > targetBox.y,
  ).toBe(true);
  for (const xFraction of [0.3, 0.5, 0.7, 0.9]) {
    for (const yFraction of [0.3, 0.5, 0.7, 0.9]) {
      const point = {
        x: targetBox.x + targetBox.width * xFraction,
        y: targetBox.y + targetBox.height * yFraction,
      };
      const hit = await target.evaluate((element, { x, y }) => {
        const current = document.elementFromPoint(x, y);
        return {
          popup: Boolean(current?.closest('[data-testid="status-mode-popover"]')),
          target: Boolean(current && (current === element || element.contains(current))),
        };
      }, point);
      if (hit.popup && !hit.target) return point;
    }
  }
  throw new Error("No point of the target is covered by the Trust popover.");
}

test("Run builds the startup project when there is no build", async ({ page }) => {
  const base = await openTrustedRestored(page);
  try {
    await page.getByTestId("bottom-tab-build").click();
    // No build yet: Run is available and the host builds before launching.
    await expect(page.getByTestId("run-action")).toBeEnabled();
    await expect(page.getByTestId("run-disabled-reason")).toHaveCount(0);
    await page.getByTestId("run-action").click();
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-state")).toContainText(/Building|Running|Run exited/, {
      timeout: 30_000,
    });
    await page.getByTestId("bottom-tab-run").click();
    await expect(page.getByTestId("run-state")).toContainText("Run exited", { timeout: 60_000 });
    await page.getByTestId("bottom-tab-build").click();
    await expect(page.getByTestId("build-state")).toContainText("Build succeeded", {
      timeout: 60_000,
    });
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("Build and Run name Restricted mode as the reason", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(fixture, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-restricted").click();
  await page.getByTestId("bottom-tab-build").click();
  await expect(page.getByTestId("build-disabled-reason")).toContainText(
    "Trust the workspace before building; nothing runs in Restricted mode.",
  );
  await expect(page.getByTestId("run-disabled-reason")).toContainText(
    "Trust the workspace before running; nothing runs in Restricted mode.",
  );
});

// Ticket 36: the Restricted reason itself grants trust, with no checkbox in the warning.
test("Restricted warnings grant trust from the reason itself", async ({ page, request }) => {
  const { base, solution } = createRestrictedWorkspace();
  let bodyFailed = false;
  try {
    await enterRestrictedWorkspace(page, solution);
    await page.getByTestId("bottom-tab-build").click();
    await expect(page.getByTestId("build-disabled-reason")).toContainText("Restricted mode");
    await expect(page.getByTestId("build-trust")).toHaveText("Trust this project");
    await expect(page.getByTestId("run-trust")).toHaveText("Trust this project");
    expect(await page.getByTestId("build-disabled-reason").getByTestId("trust-checkbox").count()).toBe(0);

    await page.getByTestId("bottom-tab-debug").click();
    await expect(page.getByTestId("dap-disabled-reason")).toContainText("Restricted mode");
    await expect(page.getByTestId("dap-trust")).toHaveText("Trust this project");

    await page.getByTestId("bottom-tab-build").click();
    await page.getByTestId("build-trust").click();
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    await expect(page.getByTestId("build-trust")).toHaveCount(0);
    await expect(page.getByTestId("run-trust")).toHaveCount(0);
    await expect(page.getByTestId("build-disabled-reason")).not.toContainText("Restricted mode");
    await expect(page.getByTestId("run-disabled-reason")).not.toContainText("Restricted mode");

    await page.getByTestId("bottom-tab-debug").click();
    await expect(page.getByTestId("dap-trust")).toHaveCount(0);
    await expect(page.getByTestId("dap-disabled-reason")).not.toContainText("Restricted mode");
    await expect(page.getByTestId("trust-checkbox")).toHaveCount(0);
  } catch (error) {
    bodyFailed = true;
    throw error;
  } finally {
    const cleanup = await cleanupD13Workspace(base, request, solution);
    if (cleanup.retained) reportRetention(`${base}: ${cleanup.reason}`, bodyFailed);
  }
});

// D-10: the Trust popover closes once Revoke completes, so it never covers the tab strip.
test("Revoke closes the Trust popover and leaves the tabs clickable", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(path.join(fixture, "OrderDesk.sln"));
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
  await page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs").click();
  await expect(page.getByTestId("editor")).toBeVisible();
  await page.getByTestId("status-mode").click();
  await page.getByTestId("trust-revoke").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");
  await expect(page.getByTestId("status-mode-popover")).toBeHidden();
  await expect(page.getByTestId("status-mode")).toBeFocused();
  // The tab close button is reachable by a real click (nothing overlays the tab strip).
  await page.locator(".tab.active").getByTestId("close-buffer").click({ timeout: 5_000 });
  await expect(page.getByTestId("tab-src/OrderDesk.Core/Order.cs")).toHaveCount(0);
});

const trustViewports = [
  { name: "1440x900", width: 1440, height: 900 },
  { name: "1100x700", width: 1100, height: 700 },
];

for (const viewport of trustViewports) {
  test(`D-13 grant stays open until host success and closes without blocking the tab at ${viewport.name}`, async ({
    page,
    request,
  }) => {
    const { base, solution } = createRestrictedWorkspace();
    const tab = page.getByTestId("tab-src/OrderDesk.Core/Order.cs");
    const target = tab.getByTestId("close-buffer");
    // The popover opens upward from the status bar and covers the bottom of the left tool window.
    const covered = page.getByRole("region", { name: "Bottom panel" });
    let bodyFailed = false;
    try {
      await enterRestrictedWorkspace(page, solution);
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.getByTestId("tree-file-src/OrderDesk.Core/Order.cs").click();
      await expect(page.getByTestId("editor")).toBeVisible();
      await page.getByTestId("status-mode").click();
      await expect(page.getByTestId("status-mode-popover")).toBeVisible();
      await page.getByTestId("trust-checkbox").check();
      // Checking the box is not a successful grant: the popover must stay open.
      await expect(page.getByTestId("status-mode-popover")).toBeVisible();
      const point = await coveredTrustPoint(page, covered);

      const release = await delayNextTrustGrant(page);
      try {
        await page.getByTestId("trust-grant").click();
        await expect(page.getByTestId("trust-grant")).toHaveText("Trusting…");
        await expect(page.getByTestId("trust-grant")).toBeDisabled();
        await expect(page.getByTestId("trust-checkbox")).toBeDisabled();
        await expect(page.getByTestId("status-mode-popover")).toBeVisible();
        await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");
        await expect(page.getByTestId("mode-trusted")).toHaveCount(0);
        const stillCovered = await covered.evaluate((element, { x, y }) => {
          const current = document.elementFromPoint(x, y);
          return {
            popup: Boolean(current?.closest('[data-testid="status-mode-popover"]')),
            target: Boolean(current && (current === element || element.contains(current))),
          };
        }, point);
        expect(stillCovered).toEqual({ popup: true, target: false });
        await release();
        await expect(page.getByTestId("status-mode-popover")).toBeHidden();
        await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
        await expect(page.getByTestId("status-mode")).toBeFocused();
      } finally {
        await release();
      }

      const nowReachable = await target.evaluate((element) => {
        const box = element.getBoundingClientRect();
        const current = document.elementFromPoint(
          box.left + box.width / 2,
          box.top + box.height / 2,
        );
        return {
          popup: Boolean(current?.closest('[data-testid="status-mode-popover"]')),
          target: Boolean(current && (current === element || element.contains(current))),
        };
      });
      expect(nowReachable).toEqual({ popup: false, target: true });

      // The focused summary must behave like real keyboard UI: Enter reopens, Escape closes and returns focus.
      await page.keyboard.press("Enter");
      await expect(page.getByTestId("status-mode-popover")).toBeVisible();
      await expect(page.getByTestId("mode-trusted")).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(page.getByTestId("status-mode-popover")).toBeHidden();
      await expect(page.getByTestId("status-mode")).toBeFocused();

      await target.click();
      await expect(tab).toHaveCount(0);
    } catch (error) {
      bodyFailed = true;
      throw error;
    } finally {
      const cleanup = await cleanupD13Workspace(base, request, solution);
      if (cleanup.retained) reportRetention(`${base}: ${cleanup.reason}`, bodyFailed);
    }
  });
}

// The rejection is injected at the UI bridge boundary, not a real host failure. The retry below uses the real host.
test("D-13 rejected grant keeps the popover open with an error and retry succeeds", async ({
  page,
  request,
}) => {
  const { base, solution } = createRestrictedWorkspace();
  let bodyFailed = false;
  try {
    await enterRestrictedWorkspace(page, solution);
    await page.getByTestId("status-mode").click();
    await page.getByTestId("trust-checkbox").check();
    const stopRejecting = await rejectNextTrustGrant(page);
    try {
      await page.getByTestId("trust-grant").click();
      await expect(page.getByTestId("trust-error")).toContainText(
        "Injected trust failure (UI boundary).",
      );
      await expect(page.getByTestId("status-mode-popover")).toBeVisible();
      await expect(page.getByTestId("status-mode")).toHaveText("Restricted mode");
      await expect(page.getByTestId("trust-grant")).toHaveText("Trust and inspect project");
      await expect(page.getByTestId("trust-grant")).toBeEnabled();
      await expect(page.getByTestId("trust-checkbox")).toBeEnabled();
      await expect(page.getByTestId("trust-checkbox")).toBeChecked();
    } finally {
      await stopRejecting();
    }

    await page.getByTestId("trust-grant").click();
    await expect(page.getByTestId("status-mode-popover")).toBeHidden();
    await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
    await expect(page.getByTestId("status-mode")).toBeFocused();
    await expect(page.getByTestId("trust-error")).toHaveCount(0);
  } catch (error) {
    bodyFailed = true;
    throw error;
  } finally {
    const cleanup = await cleanupD13Workspace(base, request, solution);
    if (cleanup.retained) reportRetention(`${base}: ${cleanup.reason}`, bodyFailed);
  }
});

test("D-13 partial-open cleanup closes the test-owned workspace before removing the copy", async ({
  page,
  request,
}) => {
  const { base, solution } = createRestrictedWorkspace();
  let originalError: Error | null = null;
  let bodyFailed = false;
  let cleanup: CleanupResult | null = null;
  try {
    await openWorkspaceCandidate(page, solution);
    expect(sameWorkspacePath(await activeWorkspacePath(request), solution)).toBe(true);
    // The real host opened the workspace, then the helper fails before open-text-only completes.
    throw new Error("controlled partial-open failure");
  } catch (error) {
    bodyFailed = true;
    originalError = error as Error;
  }
  try {
    cleanup = await cleanupD13Workspace(base, request, solution);
    expect(originalError?.message).toBe("controlled partial-open failure");
    expect(cleanup).toEqual({ removed: true, retained: false, reason: null });
    expect(await activeWorkspacePath(request)).toBeNull();
    expect(fs.existsSync(base)).toBe(false);
  } finally {
    if (cleanup?.retained) {
      const recovery = await cleanupD13Workspace(base, request, solution);
      if (recovery.retained) {
        reportRetention(`partial-open recovery retained ${base}: ${recovery.reason}`, bodyFailed);
      }
    }
  }
});

test("D-13 cleanup retains the target copy when another test workspace is active", async ({
  page,
  request,
}) => {
  const target = createRestrictedWorkspace();
  const foreign = createRestrictedWorkspace();
  let bodyFailed = false;
  try {
    await openWorkspaceCandidate(page, foreign.solution);
    expect(sameWorkspacePath(await activeWorkspacePath(request), foreign.solution)).toBe(true);
    const cleanup = await cleanupD13Workspace(target.base, request, target.solution);
    expect(cleanup.retained).toBe(true);
    expect(cleanup.reason).toContain("is not the test copy");
    expect(fs.existsSync(target.base)).toBe(true);
    expect(fs.existsSync(path.join(target.base, "OrderDesk", "OrderDesk.sln"))).toBe(true);
    expect(fs.existsSync(foreign.base)).toBe(true);
    expect(sameWorkspacePath(await activeWorkspacePath(request), foreign.solution)).toBe(true);
  } catch (error) {
    bodyFailed = true;
    throw error;
  } finally {
    let reason: string | null = null;
    let active: string | null;
    try {
      active = await activeWorkspacePath(request);
    } catch (error) {
      active = null;
      reason = `retained cleanup snapshot unconfirmed: ${String(error)}`;
    }
    if (reason === null && active !== null) {
      if (!sameWorkspacePath(active, foreign.solution)) {
        reason = `retained cleanup found unexpected active workspace ${active}`;
      } else {
        try {
          await closeOwnedWorkspace(request);
        } catch (error) {
          reason = `retained cleanup could not close the foreign sentinel: ${String(error)}`;
        }
        if (reason === null) {
          let after: string | null;
          try {
            after = await activeWorkspacePath(request);
          } catch (error) {
            after = active;
            reason = `retained cleanup post-close state unconfirmed: ${String(error)}`;
          }
          if (reason === null && after !== null) {
            reason = `retained cleanup saw ${after} still active after close`;
          }
        }
      }
    }
    if (reason !== null) {
      reportRetention(reason, bodyFailed);
    } else {
      fs.rmSync(target.base, { recursive: true, force: true });
      fs.rmSync(foreign.base, { recursive: true, force: true });
    }
  }
});

test("D-13 cleanup retains the copy when workspace state is unavailable", async ({
  page,
  request,
}) => {
  const { base, solution } = createRestrictedWorkspace();
  let bodyFailed = false;
  try {
    await openWorkspaceCandidate(page, solution);
    expect(sameWorkspacePath(await activeWorkspacePath(request), solution)).toBe(true);
    const unavailable = await playwrightRequest.newContext();
    await unavailable.dispose();
    const retained = await cleanupD13Workspace(base, unavailable, solution);
    expect(retained.retained).toBe(true);
    expect(retained.reason).toContain("workspace state unconfirmed");
    expect(fs.existsSync(base)).toBe(true);
    expect(sameWorkspacePath(await activeWorkspacePath(request), solution)).toBe(true);

    const recovered = await cleanupD13Workspace(base, request, solution);
    expect(recovered).toEqual({ removed: true, retained: false, reason: null });
    expect(await activeWorkspacePath(request)).toBeNull();
    expect(fs.existsSync(base)).toBe(false);
  } catch (error) {
    bodyFailed = true;
    throw error;
  } finally {
    const recovery = await cleanupD13Workspace(base, request, solution);
    if (recovery.retained) {
      reportRetention(`snapshot-unavailable recovery retained ${base}: ${recovery.reason}`, bodyFailed);
    }
  }
});
