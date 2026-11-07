import { expect, test } from "@playwright/test";
import http from "node:http";
import fs from "node:fs";
import os from "node:os";
import { hostPort } from "./environment";
import { startIsolatedHost, type IsolatedHost } from "./isolated-host";

// The helper is the only way a spec gets its own host, so this spec drives it directly: a foreign
// /health is refused, the owned one is accepted, and stop() leaves neither process nor home behind.
const port = hostPort + 1;

test.describe.configure({ mode: "serial", timeout: 360_000 });

let host: IsolatedHost | undefined;

test.afterAll(async () => {
  await host?.stop();
});

function tempDirs(prefix: string) {
  return fs.readdirSync(os.tmpdir()).filter((name) => name.startsWith(prefix));
}

test("refuses a port whose /health belongs to another process", async () => {
  const prefix = "openridr-isolated-decoy-";
  const decoy = http.createServer((_request, response) => {
    response.writeHead(200, { "content-type": "application/json" });
    response.end(JSON.stringify({ ok: true, pid: 999_999 }));
  });
  await new Promise<void>((resolve) => decoy.listen(port, "127.0.0.1", resolve));
  try {
    const error = await startIsolatedHost({ port, homePrefix: prefix }).then(
      () => null,
      (failure: Error) => failure,
    );
    expect(error).toBeInstanceOf(Error);
    expect(error?.message).toContain(String(port));
    expect(error?.message).toContain("999999");
    // A failed start leaves no process and no home behind.
    expect(tempDirs(prefix)).toEqual([]);
  } finally {
    await new Promise<void>((resolve) => decoy.close(() => resolve()));
  }
});

test("accepts the host it started and reports its pid and home", async () => {
  host = await startIsolatedHost({ port, homePrefix: "openridr-isolated-home-" });
  expect(host.port).toBe(port);
  expect(host.base).toBe(`http://127.0.0.1:${port}`);
  const health = (await (await fetch(`${host.base}/health`)).json()) as { pid: number };
  expect(health.pid).toBe(host.pid);
  expect(fs.existsSync(host.home)).toBe(true);
});

test("stop kills the host and removes its home", async () => {
  const started = host as IsolatedHost;
  await started.stop();
  await expect
    .poll(() => fetch(`${started.base}/health`).then(() => true, () => false))
    .toBe(false);
  expect(fs.existsSync(started.home)).toBe(false);
});
