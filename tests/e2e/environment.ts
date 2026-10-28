export function port(value: string | undefined, fallback: number): number {
  if (value === undefined) return fallback;
  if (!/^\d+$/.test(value) || Number(value) < 1 || Number(value) > 65535) {
    throw new Error(`Invalid E2E port: ${value}`);
  }
  return Number(value);
}

export const uiPort = port(process.env.OPENRIDR_E2E_UI_PORT, 1420);
export const hostPort = port(process.env.OPENRIDR_E2E_HOST_PORT, 4317);
if (uiPort === hostPort) throw new Error("E2E UI and host ports must differ");
export const frontend = `http://127.0.0.1:${uiPort}`;
export const host = `http://127.0.0.1:${hostPort}`;
// Keep tool installation and the composite host root out of the real app home during E2E.
export const e2eHome = process.env.OPENRIDR_E2E_HOME ?? path.join(os.tmpdir(), "openridr-e2e-home");
import os from "node:os";
import path from "node:path";
