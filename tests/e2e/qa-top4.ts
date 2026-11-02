import { expect, type APIRequestContext, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { host } from "./environment";

// Shared helpers for the QA top-4 specs (docs/SPEC-QA-TOP4.md). Real host, real fixtures.
export const orderDesk = path.resolve(process.cwd(), "fixtures/OrderDesk");
export const brokenImport = path.resolve(process.cwd(), "fixtures/BrokenImport");
export const program = "src/OrderDesk.Cli/Program.cs";
export const order = "src/OrderDesk.Core/Order.cs";

export function tempCopy(source: string, name: string) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-qa4-"));
  const copy = path.join(base, name);
  fs.cpSync(source, copy, {
    recursive: true,
    filter: (item) => !/[\\/](bin|obj)([\\/]|$)/.test(item),
  });
  return { base, copy };
}

export async function hostCall(request: APIRequestContext, method: string, params = {}) {
  const response = await request.post(`${host}/call`, { data: { method, params } });
  return response.json();
}

export async function hostState(request: APIRequestContext) {
  const body = await hostCall(request, "workspace.state");
  expect(body.ok, JSON.stringify(body.error)).toBe(true);
  return body.data;
}

export async function openTrusted(page: Page, target: string) {
  await page.goto("/");
  await page.getByTestId("workspace-path-input").fill(target);
  await page.getByTestId("workspace-open").click();
  await page.getByTestId("trust-prompt-trust").click();
  await expect(page.getByTestId("status-mode")).toHaveText("Trusted mode");
}

export async function openEvaluated(page: Page, copy: string) {
  await openTrusted(page, path.join(copy, "OrderDesk.sln"));
  await page.getByTestId("workspace-tools-toggle").click();
  await expect(page.getByTestId("eval-state")).toContainText("Evaluated", { timeout: 30_000 });
  await expect(page.getByTestId("restore-state")).toContainText("Restore succeeded", {
    timeout: 90_000,
  });
  await expect(page.getByTestId("eval-restore")).toHaveText("Restored", { timeout: 60_000 });
}


export async function cleanup(request: APIRequestContext) {
  await hostCall(request, "lsp.stop").catch(() => {});
  await hostCall(request, "workspace.close");
}
