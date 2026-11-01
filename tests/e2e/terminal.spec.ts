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

    // Trust opens the shell with a prompt by itself; there is no Start click.
    await page.getByTestId("bottom-tab-terminal").click();
    await expect(page.getByTestId("terminal-disabled-reason")).toHaveCount(0);
    await expect(page.getByTestId("terminal-state")).toHaveText("Running", { timeout: 30_000 });
    await expect(page.getByTestId("terminal-start")).toBeDisabled();
    await expect(page.getByTestId("terminal-shell")).toContainText(/(zsh|bash|sh)/);
    await expect(page.getByTestId("terminal-cwd")).toHaveText(root);
    await expect(page.getByTestId("terminal-size")).toContainText(/^\d+×\d+$/);

    const output = page.getByTestId("terminal-output");
    const input = page.getByTestId("terminal-input");
    expect((await output.boundingBox())!.height).toBeGreaterThanOrEqual(50);
    await page.setViewportSize({ width: 1100, height: 700 });
    await expect(page.getByTestId("terminal-close")).toBeVisible();
    await expect(page.getByTestId("terminal-interrupt")).toBeVisible();
    await expect(input).toBeVisible();
    expect((await output.boundingBox())!.height).toBeGreaterThanOrEqual(50);
    const details = page.getByRole("region", { name: "Terminal details" });
    expect(await details.evaluate((element) => element.scrollWidth > element.clientWidth)).toBe(true);
    await details.focus();
    await page.keyboard.press("ArrowRight");
    await expect.poll(() => details.evaluate((element) => element.scrollLeft)).toBeGreaterThan(0);
    await page.setViewportSize({ width: 1440, height: 900 });

    await input.fill("pwd");
    await input.press("Enter");
    await expect(output).toContainText(root);

    await input.fill("echo terminal-e2e-$((6*7))");
    await input.press("Enter");
    await expect(output).toContainText("terminal-e2e-42");

    // A carriage return overwrites from column 0 like a real terminal: the longer line leaves its tail.
    await input.fill("printf 'progress-before-CR\\rprogress-after-CR\\n'");
    await input.press("Enter");
    await expect(output).toContainText("progress-after-CR");
    const displayedLines = (await output.innerText()).split("\n").map((line) => line.trim());
    expect(displayedLines).toContain("progress-after-CRR");
    expect(displayedLines).not.toContain("progress-before-CR");
    const rawSnapshot = await page.request.post(`${host}/call`, {
      data: { method: "workspace.state", params: {} },
    });
    const rawTerminalLog = (await rawSnapshot.json()).data.terminal.log as string;
    expect(rawTerminalLog).toContain("progress-before-CR\rprogress-after-CR\n");
    await page.screenshot({ path: path.join(screenshots, "15-terminal-running.png") });

    // Typing goes straight to the terminal: keys, history, Tab completion, colors and resize.
    await output.click();
    await page.keyboard.type("echo keyed-$((6*7))");
    await page.keyboard.press("Enter");
    await expect(output).toContainText("keyed-42");
    await page.keyboard.type("echo history-marker");
    await page.keyboard.press("Enter");
    await expect(output).toContainText("history-marker");
    // Up arrow recalls the previous command onto the prompt line, the last row on screen.
    const promptLine = async () => ((await output.innerText()).split("\n").filter((line) => line.trim()).pop() ?? "");
    await page.keyboard.press("ArrowUp");
    await expect.poll(promptLine).toContain("echo history-marker");
    await page.keyboard.press("Control+u");
    await page.keyboard.type("ls src/OrderDesk.Cl");
    await page.keyboard.press("Tab");
    await expect(output).toContainText("src/OrderDesk.Cli/");
    await page.keyboard.press("Control+u");
    await page.keyboard.type("CLICOLOR_FORCE=1 ls -G src");
    await page.keyboard.press("Enter");
    await expect(output.locator('[class*="xterm-fg-"]').first()).toBeAttached();
    const size = ((await page.getByTestId("terminal-size").textContent()) ?? "").split("×");
    await page.keyboard.type("stty size");
    await page.keyboard.press("Enter");
    await expect(output).toContainText(`${size[1]} ${size[0]}`);

    // Fake Ctrl+C must interrupt the real child: `done-42` only prints after 30 s.
    await page.keyboard.type("sleep 30 && echo done-$((6*7))");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(700);
    await page.keyboard.press("Control+c");
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
    await expect(page.getByTestId("terminal-confirm-keep")).toHaveCSS("background-color", "rgb(84, 138, 247)");
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

test("Ctrl+D at an empty prompt ends the shell and the screen keeps its output", async ({ page }) => {
  const { base, copy } = tempCopy("Order Desk terminal eof");
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("trust-checkbox").check();
    await page.getByTestId("trust-grant").click();
    await page.getByTestId("open-text-only").click();
    await page.getByTestId("workspace-tools-toggle").click();
    await page.getByTestId("bottom-tab-terminal").click();
    await expect(page.getByTestId("terminal-state")).toHaveText("Running", { timeout: 30_000 });
    await page.getByTestId("terminal-output").click();
    await page.keyboard.type("echo before-eof");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("terminal-output")).toContainText("before-eof");
    await page.keyboard.press("Control+d");
    await expect(page.getByTestId("terminal-state")).not.toHaveText("Running", { timeout: 15_000 });
    await expect(page.getByTestId("terminal-output")).toContainText("before-eof");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test("a full-screen program runs in the terminal and q returns to the prompt", async ({ page }) => {
  const { base, copy } = tempCopy("Order Desk terminal less");
  try {
    await openWorkspace(page, path.join(copy, "OrderDesk.sln"));
    await page.getByTestId("trust-checkbox").check();
    await page.getByTestId("trust-grant").click();
    await page.getByTestId("open-text-only").click();
    await page.getByTestId("workspace-tools-toggle").click();
    await page.getByTestId("bottom-tab-terminal").click();
    await expect(page.getByTestId("terminal-state")).toHaveText("Running", { timeout: 30_000 });
    await page.getByTestId("terminal-output").click();
    await page.keyboard.type("less OrderDesk.sln");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("terminal-output")).toContainText("Visual Studio Version 17");
    await page.keyboard.press("q");
    await page.keyboard.type("echo back-at-prompt");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("terminal-output")).toContainText("back-at-prompt");
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
