/**
 * Ports the Chromium network stack refuses to connect to (`net::ERR_UNSAFE_PORT`), copied whole
 * from `net/base/port_util.cc` (kRestrictedPorts), checked 2026-11-07. Playwright drives Chrome,
 * so a harness port here fails at the first page.goto instead of at load.
 */
const RESTRICTED_PORTS = new Set([
  0, // Not in Fetch Spec.
  1, // tcpmux
  7, // echo
  9, // discard
  11, // systat
  13, // daytime
  15, // netstat
  17, // qotd
  19, // chargen
  20, // ftp data
  21, // ftp access
  22, // ssh
  23, // telnet
  25, // smtp
  37, // time
  42, // name
  43, // nicname
  53, // domain
  69, // tftp
  77, // priv-rjs
  79, // finger
  87, // ttylink
  95, // supdup
  101, // hostriame
  102, // iso-tsap
  103, // gppitnp
  104, // acr-nema
  109, // pop2
  110, // pop3
  111, // sunrpc
  113, // auth
  115, // sftp
  117, // uucp-path
  119, // nntp
  123, // NTP
  135, // loc-srv /epmap
  137, // netbios
  139, // netbios
  143, // imap2
  161, // snmp
  179, // BGP
  389, // ldap
  427, // SLP (Also used by Apple Filing Protocol)
  465, // smtp+ssl
  512, // print / exec
  513, // login
  514, // shell
  515, // printer
  526, // tempo
  530, // courier
  531, // chat
  532, // netnews
  540, // uucp
  548, // AFP (Apple Filing Protocol)
  554, // rtsp
  556, // remotefs
  563, // nntp+ssl
  587, // smtp (rfc6409)
  601, // syslog-conn (rfc3195)
  636, // ldap+ssl
  989, // ftps-data
  990, // ftps
  993, // ldap+ssl
  995, // pop3+ssl
  1719, // h323gatestat
  1720, // h323hostcall
  1723, // pptp
  2049, // nfs
  3659, // apple-sasl / PasswordServer
  4045, // lockd
  5060, // sip
  5061, // sips
  6000, // X11
  6566, // sane-port
  6665, // Alternate IRC [Apple addition]
  6666, // Alternate IRC [Apple addition]
  6667, // Standard IRC [Apple addition]
  6668, // Alternate IRC [Apple addition]
  6669, // Alternate IRC [Apple addition]
  6697, // IRC + TLS
  10080, // Amanda
]);

function chromeBlocked(value: number, where = ""): string {
  return `Invalid E2E port: ${value}${where} is blocked by Chrome (net::ERR_UNSAFE_PORT)`;
}

export function port(value: string | undefined, fallback: number): number {
  if (value === undefined) return fallback;
  if (!/^\d+$/.test(value) || Number(value) < 1 || Number(value) > 65535) {
    throw new Error(`Invalid E2E port: ${value}`);
  }
  if (RESTRICTED_PORTS.has(Number(value))) {
    throw new Error(`${chromeBlocked(Number(value))}; choose another port`);
  }
  return Number(value);
}

export const uiPort = port(process.env.OPENRIDR_E2E_UI_PORT, 1420);
export const hostPort = port(process.env.OPENRIDR_E2E_HOST_PORT, 4317);
if (uiPort === hostPort) throw new Error("E2E UI and host ports must differ");
// Specs that start their own host take hostPort + 1 .. + 7, and the browser reaches them directly.
for (let offset = 1; offset <= 7; offset += 1) {
  const derived = hostPort + offset;
  if (RESTRICTED_PORTS.has(derived)) {
    const where = ` (hostPort + ${offset} of ${uiPort}/${hostPort})`;
    throw new Error(`${chromeBlocked(derived, where)}; choose another port pair`);
  }
}
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
