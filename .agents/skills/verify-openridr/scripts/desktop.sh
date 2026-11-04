#!/bin/sh
# Real Tauri app, isolated: runs the bundle binary directly with its own OPENRIDR_HOME and its own
# copy of fixtures/OrderDesk, passing the solution as argv (the same path Finder/Dock/CLI feed).
# Usage: desktop.sh up [bundle.app] | down. The Mac screen must be unlocked and agent-desktop needs
# Accessibility permission, otherwise `agent-desktop snapshot` fails with a permission error.
set -eu

repo=$(cd "$(dirname "$0")/../../../.." && pwd)
tmp=${TMPDIR:-/tmp}; tmp=${tmp%/}
state=${VERIFY_OPENRIDR_DESKTOP_STATE:-$tmp/verify-openridr-$(id -u)-desktop}

case "${1:-}" in
up)
  app=${2:-${OPENRIDR_APP:-${OPENRIDR_APP_BUNDLE:-/Applications/OpenRidr.app}}}
  bin="$app/Contents/MacOS/openridr"
  [ -x "$bin" ] || { echo "desktop.sh: no executable at $bin" >&2; exit 1; }
  # Same CFBundleIdentifier for installed and debug bundles: never start a second instance next to the user's.
  if pgrep -x openridr >/dev/null; then echo "desktop.sh: an OpenRidr is already running; not double-driving it" >&2; exit 1; fi
  # Rotate instead of deleting: the previous session's log, trust store and fixture copy stay in .prev.
  if [ -d "$state" ]; then rm -rf "$state.prev"; mv "$state" "$state.prev"; fi
  mkdir -p "$state/home" "$state/work"
  cp -R "$repo/fixtures/OrderDesk" "$state/work/OrderDesk"
  find "$state/work" \( -name bin -o -name obj \) -type d -prune -exec rm -rf {} +
  OPENRIDR_HOME="$state/home" nohup "$bin" "$state/work/OrderDesk/OrderDesk.sln" >"$state/app.log" 2>&1 &
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
  echo "home $state/home; solution $state/work/OrderDesk/OrderDesk.sln"
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
