import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const fixture = path.resolve(process.cwd(), "fixtures/OrderDesk");
import { host } from "./environment";
const screenshots = path.resolve(process.env.OPENRIDR_SCREENSHOT_DIR ?? "docs/evidence/screenshots");

test.use({ viewport: { width: 1440, height: 900 } });
test.setTimeout(240_000);

function tempCopy(name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-terminal-"));
  const copy = path.join(base, name);
  fs.cpSync(fixture, copy, {
    recursive: true,
    filter: (source) => !/[\\/](bin|obj)([\\/]|$)/.test(source),
  });
  return { base, copy, root: fs.realpathSync(copy) };
}

async function openWorkspace(page: Page, solution: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(solution);
  await page.getByTestId("workspace-open").click();
}

test.afterEach(async ({ request }) => {
  await request.post(`${host}/call`, { data: { method: "terminal.stop", params: { confirm: true } } });
  await request.post(`${host}/call`, { data: { method: "workspace.close", params: {} } });
});

test("a restricted workspace explains the terminal and starts no shell", async ({ page }) => {
  const { base, copy } = tempCopy("Order Desk restricted terminal");
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("open-text-only").click();
    await page.getByTestId("workspace-tools-toggle").click();
    await expect(page.getByTestId("tree")).toBeVisible();
    await page.getByTestId("bottom-tab-terminal").click();
    await expect(page.getByTestId("terminal-disabled-reason")).toContainText(
      "Trust the workspace before starting a terminal",
    );
    await expect(page.getByTestId("terminal-start")).toBeDisabled();
    await expect(page.getByTestId("terminal-state")).toHaveText("Not started");
    await expect(page.getByTestId("terminal-shell")).toHaveText("not started");
    await expect(page.getByTestId("terminal-input")).toBeDisabled();

    // A direct host call is refused before any shell exists, exactly like the UI.
    const response = await page.request.post(`${host}/call`, {
      data: { method: "terminal.start", params: {} },
    });
    const body = await response.json();
    expect(body.ok).toBe(false);
    expect(body.error.code).toBe("trust_required");

    const state = await page.request.post(`${host}/call`, {
      data: { method: "workspace.state", params: {} },
    });
    const snapshot = (await state.json()).data;
    expect(snapshot.terminal.state).toBe("idle");
    expect(
      snapshot.processes.filter(
        (process: { kind: string; state: string }) =>
          process.kind.startsWith("terminal:") && process.state === "running",
      ),
    ).toEqual([]);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a real shell runs in the workspace, interrupts a child with Ctrl+C and closes after confirmation", async ({
  page,
}) => {
  const { base, copy, root } = tempCopy("Order Desk terminal");
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("trust-checkbox").check();
    await expect(page.getByTestId("trust-grant")).toBeEnabled();
    await page.getByTestId("trust-grant").click();
    await expect(page.getByTestId("mode-trusted")).toBeVisible();
    await page.getByTestId("open-text-only").click();
    await page.getByTestId("workspace-tools-toggle").click();
    await expect(page.getByTestId("tree")).toBeVisible();
    fs.mkdirSync(screenshots, { recursive: true });

    await page.getByTestId("bottom-tab-terminal").click();
    await expect(page.getByTestId("terminal-disabled-reason")).toHaveCount(0);
    await page.getByTestId("terminal-start").click();
    await expect(page.getByTestId("terminal-state")).toHaveText("Running");
    await expect(page.getByTestId("terminal-shell")).toContainText(/(zsh|bash|sh)/);
    await expect(page.getByTestId("terminal-cwd")).toHaveText(root);
    await expect(page.getByTestId("terminal-size")).toContainText(/^\d+×\d+$/);

    const output = page.getByTestId("terminal-output");
    const input = page.getByTestId("terminal-input");

    await input.fill("pwd");
    await input.press("Enter");
    await expect(output).toContainText(root);

    await input.fill("echo terminal-e2e-$((6*7))");
    await input.press("Enter");
    await expect(output).toContainText("terminal-e2e-42");

    // The bounded UI log displays the final CR-delimited progress segment, while the host keeps raw bytes.
    await input.fill("printf 'progress-before-CR\\rprogress-after-CR\\n'");
    await input.press("Enter");
    await expect(output).toContainText("progress-after-CR");
    const displayedLines = (await output.textContent())!.split("\n").map((line) => line.trim());
    expect(displayedLines).toContain("progress-after-CR");
    expect(displayedLines).not.toContain("progress-before-CR");
    const rawSnapshot = await page.request.post(`${host}/call`, {
      data: { method: "workspace.state", params: {} },
    });
    const rawTerminalLog = (await rawSnapshot.json()).data.terminal.log as string;
    expect(rawTerminalLog).toContain("progress-before-CR\rprogress-after-CR\n");
    await page.screenshot({ path: path.join(screenshots, "15-terminal-running.png") });

    // Fake Ctrl+C must interrupt the real child: `done-42` only prints after 30 s.
    await input.fill("sleep 30 && echo done-$((6*7))");
    await input.press("Enter");
    await page.waitForTimeout(700);
    await input.press("Control+c");
    await expect(output).toContainText("^C", { timeout: 10_000 });
    await input.fill("echo prompt-back");
    await input.press("Enter");
    await expect(output).toContainText("prompt-back", { timeout: 10_000 });
    await expect(output).not.toContainText("done-42");
    await page.screenshot({ path: path.join(screenshots, "15-terminal-ctrl-c.png") });

    // A real dotnet invocation through the shell, with the fixture's pinned SDK.
    await input.fill(
      'SDK="$HOME/Library/Application Support/OpenRidr/sdks/8.0.408"; "$SDK/dotnet" --version',
    );
    await input.press("Enter");
    await expect(output).toContainText("8.0.408", { timeout: 60_000 });

    // Closing a running shell needs a decision; keeping it preserves the session.
    await page.getByTestId("terminal-close").click();
    await expect(page.getByTestId("terminal-confirm")).toBeVisible();
    await expect(page.getByTestId("terminal-confirm-keep")).toHaveCSS("border-color", "rgb(240, 160, 121)");
    await expect(page.getByTestId("terminal-confirm-stop")).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    await page.screenshot({ path: path.join(screenshots, "15-terminal-close-dialog.png") });
    await page.getByTestId("terminal-confirm-keep").click();
    await expect(page.getByTestId("terminal-confirm")).toHaveCount(0);
    await expect(page.getByTestId("terminal-state")).toHaveText("Running");
    await input.fill("echo still-alive");
    await input.press("Enter");
    await expect(output).toContainText("still-alive");

    await page.getByTestId("terminal-close").click();
    await page.getByTestId("terminal-confirm-stop").click();
    await expect(page.getByTestId("terminal-state")).toHaveText("Stopped");
    await expect(page.getByTestId("terminal-exit")).toContainText(/^Exit: (-?\d+|exit unknown)$/);
    await expect(page.getByTestId("terminal-termination")).toContainText(
      "Termination requested: SIGTERM",
    );
    await expect(page.getByTestId("terminal-termination")).toContainText("Observed: exited");
    await expect(output).toContainText("still-alive");
    await page.screenshot({ path: path.join(screenshots, "15-terminal-closed.png") });
    await expect(page.getByTestId("terminal-input")).toBeDisabled();

    // The UI shows the same exit the registry observed for the shell itself.
    const response = await page.request.post(`${host}/call`, {
      data: { method: "workspace.state", params: {} },
    });
    const snapshot = (await response.json()).data;
    expect(snapshot.terminal.state).toBe("stopped");
    expect(snapshot.terminal.sealed).toBe(false);
    expect(snapshot.terminal.termination.observed).toBe("exited");
    expect(snapshot.terminal.log).toContain("still-alive");
    const process = snapshot.processes.find(
      (item: { id: number }) => item.id === snapshot.terminal.processId,
    );
    expect(process.state).toBe("exited");
    expect(snapshot.terminal.exit).toBe(process.exit);
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
