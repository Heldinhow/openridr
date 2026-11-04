#!/bin/sh
# Read-only environment check for verify-openridr. Changes nothing, starts nothing.
# Usage: doctor.sh [bundle.app]. Exit 1 when a hard requirement of the web surface is missing.
repo=$(cd "$(dirname "$0")/../../../.." && pwd)
export PATH="$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH"
bad=0
ok() { printf 'ok    %s\n' "$1"; }
no() { printf 'FAIL  %s\n' "$1"; bad=1; }
note() { printf 'note  %s\n' "$1"; }

command -v bun >/dev/null && ok "bun $(bun --version)" || no "bun not found (export PATH=\"\$HOME/.bun/bin:\$PATH\")"
command -v cargo >/dev/null && ok "cargo present" || no "cargo not found"
[ -d "$repo/node_modules" ] && ok "node_modules present" || no "node_modules missing: run bun install (a new worktree has none)"
dotnet --list-sdks 2>/dev/null | grep -q '^8\.0\.408' && ok "dotnet SDK 8.0.408 (fixtures pin it, rollForward disabled)" || no "dotnet SDK 8.0.408 not found in PATH/~/.dotnet"
[ -d "/Applications/Google Chrome.app" ] && ok "Chrome (Playwright default channel)" || note "no Chrome: use PLAYWRIGHT_CHANNEL=chromium"
for port in 1420 4317 4318 1520 4417; do
  lsof -nP -iTCP:$port -sTCP:LISTEN >/dev/null 2>&1 && note "port $port is busy (another run, or the user's dev server): do not double-drive it"
done
version=$(sed -n 's/.*"version": "\(.*\)".*/\1/p' "$repo/package.json" | head -1)
app=${1:-${OPENRIDR_APP:-/Applications/OpenRidr.app}}
installed=$(/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" "$app/Contents/Info.plist" 2>/dev/null)
if [ -z "$installed" ]; then note "no app bundle at $app: build it (bun run tauri build --debug --bundles app) or bun run app:install"
elif [ "$installed" = "$version" ]; then ok "app $app $installed matches package.json $version"
else note "app $app $installed != package.json $version: rebuild before trusting desktop results"; fi
pgrep -x openridr >/dev/null && note "an OpenRidr app is running (maybe the user's): desktop.sh will refuse to start another"
if command -v agent-desktop >/dev/null; then
  status=$(agent-desktop status 2>/dev/null || true)
  acc=$(printf '%s\n' "$status" | sed -n 's/.*"accessibility":{"state":"\([a-z]*\)".*/\1/p')
  screen=$(printf '%s\n' "$status" | sed -n 's/.*"screen_recording":{"state":"\([a-z]*\)".*/\1/p')
  [ "$acc" = granted ] && ok "agent-desktop accessibility granted" || note "agent-desktop accessibility ${acc:-unknown}: agent-desktop permissions --guide"
  [ "$screen" = granted ] && ok "agent-desktop screen recording granted" || note "agent-desktop screen recording ${screen:-unknown}: screenshot needs it"
else note "agent-desktop not installed: desktop surface unavailable"; fi
note "desktop surface also needs the screen unlocked and awake"
exit $bad
