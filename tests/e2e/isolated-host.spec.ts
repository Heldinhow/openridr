import { expect, test } from "@playwright/test";
import http from "node:http";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import type { AddressInfo } from "node:net";
import { startIsolatedHost, type IsolatedHost } from "./isolated-host";

// The helper is the only way a spec gets its own host, so this spec drives it directly: the port is
// the one the kernel granted, a foreign /health is refused, the owned one is accepted, and stop()
// leaves neither process nor home behind.

test.describe.configure({ mode: "serial", timeout: 360_000 });

let host: IsolatedHost | undefined;

test.afterAll(async () => {
  await host?.stop();
});

// Two sessions run this spec at the same time in different worktrees, and os.tmpdir() is shared:
// every prefix carries this worker's pid, or one run sees the other's homes.
const run = `${process.pid}-`;

function tempDirs(prefix: string) {
  return fs.readdirSync(os.tmpdir()).filter((name) => name.startsWith(prefix));
}

/** A child that prints the bridge's listening line for a port it does not own. */
function standIn(port: number) {
  const line = `openridr-host-http listening on http://127.0.0.1:${port}`;
  return {
    file: process.execPath,
    args: ["-e", `console.log(${JSON.stringify(line)}); setInterval(() => {}, 1000);`],
  };
}

test("refuses a /health that belongs to another process", async () => {
  const prefix = `openridr-isolated-decoy-${run}`;
  const decoy = http.createServer((_request, response) => {
    response.writeHead(200, { "content-type": "application/json" });
    response.end(JSON.stringify({ ok: true, pid: 999_999 }));
  });
  await new Promise<void>((resolve) => decoy.listen(0, "127.0.0.1", resolve));
  const port = (decoy.address() as AddressInfo).port;
  try {
    // The stand-in prints the decoy's port, so the helper reads a port whose /health is not the
    // child's: the pid check is the only thing between the spec and a foreign host.
    const error = await startIsolatedHost({ homePrefix: prefix, command: standIn(port) }).then(
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

test("accepts the host it started and reports the port the kernel granted", async () => {
  const prefix = `openridr-isolated-home-${run}`;
  host = await startIsolatedHost({ homePrefix: prefix });
  // No spec chose this port: it comes from the line the bridge printed after binding.
  expect(host.port).toBeGreaterThan(0);
  expect(host.base).toBe(`http://127.0.0.1:${host.port}`);
  const health = (await (await fetch(`${host.base}/health`)).json()) as { pid: number };
  expect(health.pid).toBe(host.pid);
  expect(fs.existsSync(host.home)).toBe(true);
  expect(tempDirs(prefix)).toEqual([path.basename(host.home)]);
});

test("a second host gets its own port", async () => {
  const other = await startIsolatedHost({ homePrefix: `openridr-isolated-second-${run}` });
  try {
    expect(other.port).not.toBe(host?.port);
    expect(other.pid).not.toBe(host?.pid);
  } finally {
    await other.stop();
  }
});

test("stop kills the host and removes its home", async () => {
  const started = host as IsolatedHost;
  await started.stop();
  await expect
    .poll(() => fetch(`${started.base}/health`).then(() => true, () => false))
    .toBe(false);
  expect(fs.existsSync(started.home)).toBe(false);
});

test("prepare seeds the home and its env before the host starts", async () => {
  // An existing path, because recent.list hides entries whose file is gone.
  const project = path.resolve("fixtures");
  host = await startIsolatedHost({
    homePrefix: `openridr-isolated-prepared-${run}`,
    prepare: (home) => {
      fs.writeFileSync(path.join(home, "recent.json"), JSON.stringify([{ path: project, openedAt: 1 }]));
      fs.writeFileSync(path.join(home, "trusted.json"), JSON.stringify({ [project]: "trusted" }));
      // The store off is how the host shows the env came from prepare, not from the caller.
      return { OPENRIDR_TRUST_STORE: "off" };
    },
  });
  const answer = (await (
    await fetch(`${host.base}/call`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ method: "recent.list", params: {} }),
    })
  ).json()) as { data: { projects: { path: string; decision: string | null }[] } };
  expect(answer.data.projects).toEqual([{ path: project, openedAt: 1, decision: null }]);
});

test("a prepare that throws leaves no home behind", async () => {
  const prefix = `openridr-isolated-prepare-fails-${run}`;
  const error = await startIsolatedHost({
    homePrefix: prefix,
    prepare: () => {
      throw new Error("seed failed");
    },
  }).then(
    () => null,
    (failure: Error) => failure,
  );
  expect(error?.message).toContain("seed failed");
  expect(tempDirs(prefix)).toEqual([]);
});
