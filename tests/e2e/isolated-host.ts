import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { test, type Page } from "@playwright/test";

/**
 * The only way an E2E spec gets a host of its own. It owns the child process and a temporary
 * OPENRIDR_HOME, asks the kernel for the port (`--port 0`) and reads the granted one from the line
 * the bridge prints, so no spec picks an auxiliary port. It also proves the process answering
 * /health is the one it started: a foreign /health fails here instead of the spec silently driving
 * a host it does not own.
 */
export interface IsolatedHostOptions {
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
  /**
   * Test seam for `isolated-host.spec.ts`: what to run instead of the bridge. The stand-in prints
   * the same listening line, which is how the spec points the helper at a /health that is not the
   * child's — the refusal the pid check exists for.
   */
  command?: { file: string; args: string[] };
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
export const START_TIMEOUT_MS = 300_000;
const POLL_INTERVAL_MS = 100;
const PROBE_TIMEOUT_MS = 2_000;
const KILL_TIMEOUT_MS = 10_000;
/** Enough of the child's stderr to name a bind failure or a compile error, without the whole log. */
const DIAGNOSTICS_LIMIT = 4_000;

const BRIDGE = { file: "cargo", args: ["run", "-p", "openridr-host-http", "--", "--port", "0"] };
const LISTENING = /^openridr-host-http listening on (http:\/\/\S+)$/;

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
  // A spawn that never produced a pid has nothing to kill and never emits "exit".
  if (server.pid === undefined || hasExited(server)) return Promise.resolve();
  return new Promise((resolve) => {
    // A host that ignores SIGTERM must not hang the suite; the timer dies with the process.
    const escalate = setTimeout(() => {
      server.kill("SIGKILL");
      resolve();
    }, KILL_TIMEOUT_MS);
    escalate.unref();
    server.once("exit", () => {
      clearTimeout(escalate);
      resolve();
    });
    server.kill();
  });
}

/**
 * The base URL from the line the child prints once it is bound. The kernel grants the port, so the
 * helper cannot know it in advance; a child that dies or never prints fails with its stderr.
 */
function readBase(child: ChildProcess, diagnostics: () => string): Promise<string> {
  return new Promise((resolve, reject) => {
    let buffer = "";
    let settled = false;
    const deadline = setTimeout(() => {
      if (settled) return;
      settled = true;
      reject(new Error(`no listening line within ${START_TIMEOUT_MS} ms`));
    }, START_TIMEOUT_MS);
    deadline.unref();
    const settle = (finish: () => void) => {
      if (settled) return;
      settled = true;
      clearTimeout(deadline);
      finish();
    };
    child.stdout?.setEncoding("utf8");
    // The listener stays attached after the port is read: an unread pipe would block the child.
    child.stdout?.on("data", (chunk: string) => {
      buffer += chunk;
      for (let newline = buffer.indexOf("\n"); newline >= 0; newline = buffer.indexOf("\n")) {
        const line = buffer.slice(0, newline).trim();
        buffer = buffer.slice(newline + 1);
        const listening = LISTENING.exec(line);
        if (listening) settle(() => resolve(listening[1]));
      }
    });
    child.once("error", (error) => settle(() => reject(new Error(error.message))));
    child.once("exit", (code, signal) =>
      settle(() =>
        reject(
          new Error(`child exited (${code ?? signal}) before printing its port${diagnostics()}`),
        ),
      ),
    );
  });
}

export async function startIsolatedHost({
  homePrefix,
  env = {},
  prepare,
  command = BRIDGE,
}: IsolatedHostOptions): Promise<IsolatedHost> {
  // The helper waits up to START_TIMEOUT_MS; the hook that awaits it has to outlast that budget, or
  // Playwright cuts the start and the child is left with nobody to stop it.
  test.setTimeout(START_TIMEOUT_MS + 30_000);

  const home = fs.mkdtempSync(path.join(os.tmpdir(), homePrefix));
  let server: ChildProcess | undefined;

  // Every failure path leaves no child and no home behind, so the next run starts clean.
  const fail = async (reason: string): Promise<never> => {
    if (server) await stopProcess(server);
    fs.rmSync(home, { recursive: true, force: true });
    throw new Error(`isolated host failed: ${reason}`);
  };

  let prepared: NodeJS.ProcessEnv;
  try {
    prepared = prepare?.(home) ?? {};
  } catch (error) {
    return fail(`prepare threw: ${error instanceof Error ? error.message : String(error)}`);
  }

  const child = spawn(command.file, command.args, {
    env: { ...process.env, ...env, ...prepared, OPENRIDR_HOME: home },
    stdio: ["ignore", "pipe", "pipe"],
  });
  server = child;
  // A bridge that dies before printing its port says why on stderr; keep the tail for the message.
  let diagnostics = "";
  child.stderr?.setEncoding("utf8");
  child.stderr?.on("data", (chunk: string) => {
    diagnostics = (diagnostics + chunk).slice(-DIAGNOSTICS_LIMIT);
  });
  const tail = () => (diagnostics.trim() ? `: ${diagnostics.trim()}` : "");

  const pid = child.pid;
  if (pid === undefined) return fail("the child never started");

  let base: string;
  try {
    base = await readBase(child, tail);
  } catch (error) {
    return fail(error instanceof Error ? error.message : String(error));
  }
  const port = Number(new URL(base).port);
  if (!Number.isInteger(port) || port < 1) return fail(`the child printed no port: ${base}`);

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
          await stopProcess(child);
          fs.rmSync(home, { recursive: true, force: true });
        },
      };
    }
    // Someone else owns the port: refuse instead of driving a foreign host.
    if (health.responded) {
      return fail(
        `port ${port} answered /health with pid ${health.pid ?? "unknown"}, expected child pid ${pid}`,
      );
    }
    if (hasExited(child)) {
      return fail(`child exited with code ${child.exitCode} before answering /health${tail()}`);
    }
    if (Date.now() > deadline) return fail(`no owned /health on port ${port} within ${START_TIMEOUT_MS} ms`);
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
  }
}
