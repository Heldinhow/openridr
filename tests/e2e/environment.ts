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
/**
 * Keep tool installation and the composite host root out of the real app home during E2E.
 * The default is keyed by the UI port so two sessions with different ports never share a home
 * (each run rewrites the verified tools it finds there); OPENRIDR_E2E_HOME still wins.
 */
export function e2eHomeFor(uiPort: number, override: string | undefined): string {
  return override ?? path.join(os.tmpdir(), `openridr-e2e-home-${uiPort}`);
}

export const e2eHome = e2eHomeFor(uiPort, process.env.OPENRIDR_E2E_HOME);
/**
 * Keep evidence captures out of the repository by default: a spec that fails must not rewrite the
 * versioned PNGs. The default is keyed by the UI port like the E2E app home; OPENRIDR_SCREENSHOT_DIR
 * still wins, and a relative override stays resolved against the current directory.
 */
export function screenshotsDirFor(uiPort: number, override: string | undefined): string {
  return path.resolve(override ?? path.join(os.tmpdir(), `openridr-e2e-screenshots-${uiPort}`));
}

export const screenshotsDir = screenshotsDirFor(uiPort, process.env.OPENRIDR_SCREENSHOT_DIR);
/**
 * The capture directory only when OPENRIDR_SCREENSHOT_DIR asked for one. Specs that capture on
 * request stay silent without it; specs that always capture take `screenshotsDir`.
 */
export const requestedScreenshotsDir = process.env.OPENRIDR_SCREENSHOT_DIR
  ? screenshotsDir
  : undefined;

/** Same pinned csharp-ls package SHA-512 the host verifies before installing. */
const PACKAGE_SHA512 =
  "a138b9c8f9de9dd6343e9b0d0c8f4d51637bba7e38606b7c2b08b1d91e8eefa39f12123057f6625f425812b4361e52841bd2856704ec127ff50144ce05170a99";

/**
 * The pinned csharp-ls package. Setup is mandatory now, so the shared host and specs that need
 * the post-setup state start from a real verified launcher; the fake launcher would break LSP.
 */
const PACKAGE_URL =
  "https://api.nuget.org/v3-flatcontainer/csharp-ls/0.28.0/csharp-ls.0.28.0.nupkg";

function ensureRealCsharpLs(cacheHome: string): string {
  const destination = path.join(cacheHome, "real-tools/csharp-ls/0.28");
  const launcher = path.join(destination, "csharp-ls");
  if (fs.existsSync(launcher)) return destination;
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "openridr-e2e-real-tool-"));
  try {
    const packagePath = path.join(temp, "csharp-ls.0.28.0.nupkg");
    // globalSetup is async; this helper is called from it and from sync beforeAll blocks, so
    // the download uses a synchronous worker via execFileSync("curl") only when missing.
    execFileSync("curl", ["-fsSL", PACKAGE_URL, "-o", packagePath], { stdio: "ignore" });
    const digest = crypto.createHash("sha512").update(fs.readFileSync(packagePath)).digest("hex");
    if (digest !== PACKAGE_SHA512) {
      throw new Error(`csharp-ls package SHA-512 mismatch: ${digest}`);
    }
    const feed = path.join(temp, "feed");
    fs.mkdirSync(feed, { recursive: true });
    fs.copyFileSync(packagePath, path.join(feed, "csharp-ls.0.28.0.nupkg"));
    const config = path.join(temp, "offline.nuget.config");
    fs.writeFileSync(
      config,
      "<configuration><packageSources><clear /></packageSources></configuration>",
    );
    const staging = path.join(temp, "staging");
    const home = path.join(temp, "home");
    fs.mkdirSync(home, { recursive: true });
    execFileSync(
      "dotnet",
      [
        "tool",
        "install",
        "csharp-ls",
        "--version",
        "0.28.0",
        "--tool-path",
        staging,
        "--add-source",
        feed,
        "--configfile",
        config,
        "--no-cache",
      ],
      {
        stdio: "ignore",
        env: {
          ...process.env,
          HOME: home,
          DOTNET_NOLOGO: "1",
          DOTNET_CLI_UI_LANGUAGE: "en",
          DOTNET_CLI_TELEMETRY_OPTOUT: "1",
          NUGET_PACKAGES: path.join(temp, "packages"),
        },
      },
    );
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.rmSync(destination, { recursive: true, force: true });
    fs.cpSync(staging, destination, { recursive: true });
    return destination;
  } finally {
    fs.rmSync(temp, { recursive: true, force: true });
  }
}

/**
 * Seeds a real verified csharp-ls launcher (copied from a per-run cache) and a bundled
 * netcoredbg. Returns the root to pass as OPENRIDR_BUNDLED_NETCOREDBG.
 */
export function seedVerifiedTools(home: string): string {
  const real = ensureRealCsharpLs(e2eHome);
  const destination = path.join(home, "tools/csharp-ls/0.28");
  fs.rmSync(destination, { recursive: true, force: true });
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.cpSync(real, destination, { recursive: true });
  const launcher = path.join(destination, "csharp-ls");
  const launcherSha = crypto.createHash("sha256").update(fs.readFileSync(launcher)).digest("hex");
  fs.writeFileSync(
    path.join(destination, ".openridr-integrity"),
    `${PACKAGE_SHA512}\n${launcherSha}\n`,
  );
  const bundled = path.join(home, "bundled");
  fs.mkdirSync(bundled, { recursive: true });
  const adapter = path.join(bundled, "netcoredbg");
  fs.writeFileSync(adapter, "#!/bin/sh\nprintf 'netcoredbg 3.2.0-1092\\n'\n");
  fs.chmodSync(adapter, 0o755);
  return bundled;
}
import os from "node:os";
import path from "node:path";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
