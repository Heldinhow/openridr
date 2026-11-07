import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import type { Page } from "@playwright/test";

/**
 * The only way an E2E spec gets a host of its own. It owns the child process and a temporary
 * OPENRIDR_HOME, and it proves the process answering /health is the one it started: a port already
 * owned by another process fails here instead of the spec silently driving a foreign host.
 */
export interface IsolatedHostOptions {
  /** The port the spec picked (hostPort + N); the helper never chooses one. */
  port: number;
  /** Prefix of the temporary OPENRIDR_HOME the helper creates and deletes. */
  homePrefix: string;
  /** Host variables the spec needs (OPENRIDR_TRUST_STORE, OPENRIDR_ENABLE_PROBE, ...). */
  env?: NodeJS.ProcessEnv;
  /**
   * Seeds the home before the host starts (recent.json, a fake installer, the real debug adapter).
   * Variables it returns are merged over `env`, which is how a spec points OPENRIDR_TOOLS_MUXER or
   * OPENRIDR_BUNDLED_NETCOREDBG at a path inside the home it does not know yet.
   */
  prepare?: (home: string) => NodeJS.ProcessEnv | void;
}

export interface IsolatedHost {
  port: number;
  base: string;
  home: string;
  pid: number;
  /** Points the page's /__host proxy at this host instead of the shared one. */
  useFor(page: Page): Promise<void>;
  /** Kills the host and deletes the home. Idempotent. */
  stop(): Promise<void>;
}

/** The first `cargo run` in a fresh worktree compiles the host; the shared harness waits as long. */
const START_TIMEOUT_MS = 300_000;
const POLL_INTERVAL_MS = 100;
const PROBE_TIMEOUT_MS = 2_000;
const KILL_TIMEOUT_MS = 10_000;

type Health = { responded: false } | { responded: true; pid: number | null };

async function probeHealth(base: string): Promise<Health> {
  try {
    const response = await fetch(`${base}/health`, { signal: AbortSignal.timeout(PROBE_TIMEOUT_MS) });
    if (!response.ok) return { responded: false };
    const body = (await response.json()) as { pid?: unknown };
    return { responded: true, pid: typeof body.pid === "number" ? body.pid : null };
  } catch {
    return { responded: false };
  }
}

function hasExited(server: ChildProcess): boolean {
  return server.exitCode !== null || server.signalCode !== null;
}

function stopProcess(server: ChildProcess): Promise<void> {
  if (hasExited(server)) return Promise.resolve();
  return new Promise((resolve) => {
    server.once("exit", () => resolve());
    server.kill();
    // A host that ignores SIGTERM must not hang the suite.
    setTimeout(() => {
      server.kill("SIGKILL");
      resolve();
    }, KILL_TIMEOUT_MS).unref();
  });
}

export async function startIsolatedHost({
  port,
  homePrefix,
  env = {},
  prepare,
}: IsolatedHostOptions): Promise<IsolatedHost> {
  const base = `http://127.0.0.1:${port}`;
  const home = fs.mkdtempSync(path.join(os.tmpdir(), homePrefix));
  const prepared = prepare?.(home) ?? {};
  const server = spawn("cargo", ["run", "-p", "openridr-host-http", "--", "--port", String(port)], {
    env: { ...process.env, ...env, ...prepared, OPENRIDR_HOME: home },
    stdio: "ignore",
  });
  const spawnError = new Promise<Error>((resolve) => server.once("error", resolve));
  const pid = server.pid;

  // Every failure path leaves no child and no home behind, so the next run starts clean.
  const fail = async (reason: string): Promise<never> => {
    await stopProcess(server);
    fs.rmSync(home, { recursive: true, force: true });
    throw new Error(`isolated host on port ${port} failed: ${reason}`);
  };

  if (pid === undefined) return fail((await spawnError).message);

  const deadline = Date.now() + START_TIMEOUT_MS;
  for (;;) {
    const health = await probeHealth(base);
    if (health.responded && health.pid === pid) {
      return {
        port,
        base,
        home,
        pid,
        async useFor(page: Page) {
          await page.route(/\/__host\/(call|health)$/, async (route) => {
            const target = `${base}/${route.request().url().split("/").pop()}`;
            // A poll still in flight when the test ends fails with "Test ended"; that is not a failure.
            await route
              .fetch({ url: target })
              .then((response) => route.fulfill({ response }))
              .catch(() => {});
          });
        },
        async stop() {
          await stopProcess(server);
          fs.rmSync(home, { recursive: true, force: true });
        },
      };
    }
    // Someone else owns the port: refuse instead of driving a foreign host.
    if (health.responded) {
      return fail(
        `port answered /health with pid ${health.pid ?? "unknown"}, expected child pid ${pid}`,
      );
    }
    if (hasExited(server)) {
      return fail(`child exited with code ${server.exitCode} before answering /health`);
    }
    if (Date.now() > deadline) return fail(`no owned /health within ${START_TIMEOUT_MS} ms`);
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
  }
}
