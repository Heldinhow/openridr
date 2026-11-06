#!/bin/sh
# Real Tauri app, isolated: runs the bundle binary directly with its own OPENRIDR_HOME and its own
# copy of fixtures/OrderDesk, passing the solution as argv (the same path Finder/Dock/CLI feed).
# Usage: desktop.sh up [bundle.app] | down. The Mac screen must be unlocked and agent-desktop needs
# Accessibility permission, otherwise `agent-desktop snapshot` fails with a permission error.
# VERIFY_OPENRIDR_DESKTOP_SOLUTION opens a prepared absolute .sln/.csproj instead of the fixture copy
# (the isolated OPENRIDR_HOME and the single-instance/PID checks stay the same).
set -eu

repo=$(cd "$(dirname "$0")/../../../.." && pwd)
tmp=${TMPDIR:-/tmp}; tmp=${tmp%/}
state=${VERIFY_OPENRIDR_DESKTOP_STATE:-$tmp/verify-openridr-$(id -u)-desktop}

case "${1:-}" in
up)
  app=${2:-${OPENRIDR_APP:-${OPENRIDR_APP_BUNDLE:-/Applications/OpenRidr.app}}}
  bin="$app/Contents/MacOS/openridr"
  [ -x "$bin" ] || { echo "desktop.sh: no executable at $bin" >&2; exit 1; }
  # The bundle under test may not be the checkout; say which version runs so proof is not attributed to HEAD.
  bundle_version=$(/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" "$app/Contents/Info.plist" 2>/dev/null || true)
  package_version=$(sed -n 's/.*"version": "\(.*\)".*/\1/p' "$repo/package.json" | head -1)
  echo "bundle $app ${bundle_version:-unknown}"
  if [ -n "$bundle_version" ] && [ -n "$package_version" ] && [ "$bundle_version" != "$package_version" ]; then
    echo "note: bundle is $bundle_version but package.json is $package_version; results describe the bundle, not this checkout" >&2
  fi
  # Same CFBundleIdentifier for installed and debug bundles: never start a second instance next to the user's.
  if pgrep -x openridr >/dev/null; then echo "desktop.sh: an OpenRidr is already running; not double-driving it" >&2; exit 1; fi
  # Validate the prepared workspace BEFORE any destructive step: an invalid override must leave the
  # previous state (log, trust store, fixture copy) untouched.
  # A prepared workspace (scripts/git-remote-fixture.sh and friends) replaces the fixture copy; the
  # default stays fixtures/OrderDesk. Only the solution path comes from outside; the home does not.
  prepared=${VERIFY_OPENRIDR_DESKTOP_SOLUTION:-}
  if [ -n "$prepared" ]; then
    case "$prepared" in
    /*/OrderDesk.sln | /*/*.sln | /*/*.csproj) ;;
    *) echo "desktop.sh: VERIFY_OPENRIDR_DESKTOP_SOLUTION must be an absolute .sln/.csproj path: $prepared" >&2; exit 2 ;;
    esac
    case "$prepared" in
    */../* | */..) echo "desktop.sh: VERIFY_OPENRIDR_DESKTOP_SOLUTION must not contain '..': $prepared" >&2; exit 2 ;;
    esac
    [ -f "$prepared" ] || { echo "desktop.sh: solution not found: $prepared" >&2; exit 2; }
    resolved=$(CDPATH= cd -- "$(dirname -- "$prepared")" && pwd -P) || {
      echo "desktop.sh: solution directory is not reachable: $prepared" >&2; exit 2;
    }
    prepared="$resolved/$(basename -- "$prepared")"
  fi
  # Rotate instead of deleting: the previous session's log, trust store and fixture copy stay in .prev.
  if [ -d "$state" ]; then rm -rf "$state.prev"; mv "$state" "$state.prev"; fi
  mkdir -p "$state/home" "$state/work"
  if [ -n "$prepared" ]; then
    solution=$prepared
    echo "workspace $solution (prepared; the fixture copy is skipped)"
  else
    cp -R "$repo/fixtures/OrderDesk" "$state/work/OrderDesk"
    find "$state/work" \( -name bin -o -name obj \) -type d -prune -exec rm -rf {} +
    solution=$state/work/OrderDesk/OrderDesk.sln
  fi
  OPENRIDR_HOME="$state/home" nohup "$bin" "$solution" >"$state/app.log" 2>&1 &
  pid=$!
  echo "$pid" >"$state/app.pid"
  sleep 4
  kill -0 "$pid" 2>/dev/null || { rm -f "$state/app.pid"; echo "desktop.sh: app exited; see $state/app.log" >&2; exit 1; }
  # The PID must still be openridr, and it must be the only instance (pgrep verifies, never kills).
  instances=$(pgrep -x openridr || true)
  if [ "$instances" != "$pid" ]; then
    rm -f "$state/app.pid"
    echo "desktop.sh: expected exactly one openridr (pid $pid) after launch; found: ${instances:-none}" >&2
    exit 1
  fi
  echo "app pid $pid"
  echo "state $state"
  echo "home $state/home; solution $solution"
  ;;
down)
  if [ -f "$state/app.pid" ]; then
    pid=$(cat "$state/app.pid")
    # A recycled PID must not be killed: only a live openridr recorded by `up` is ours.
    case "$(ps -p "$pid" -o comm= 2>/dev/null)" in
    */openridr | openridr)
      kill "$pid" 2>/dev/null || true
      i=0
      while kill -0 "$pid" 2>/dev/null && [ "$i" -lt 20 ]; do i=$((i + 1)); sleep 0.25; done
      ;;
    esac
    rm -f "$state/app.pid"
  fi
  left=$(pgrep -x openridr | tr '\n' ' ')
  if [ -n "$left" ]; then
    echo "desktop.sh: still running: $left(not started by this state; not killed)" >&2
    exit 1
  fi
  echo "stopped (state kept in $state)"
  ;;
*) echo "usage: desktop.sh up [bundle.app] | down" >&2; exit 2 ;;
esac
