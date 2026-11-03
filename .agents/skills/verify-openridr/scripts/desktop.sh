#!/bin/sh
# Real Tauri app, isolated: runs the bundle binary directly with its own OPENRIDR_HOME and its own
# copy of fixtures/OrderDesk, passing the solution as argv (the same path Finder/Dock/CLI feed).
# Usage: desktop.sh up [bundle.app] | down. The Mac screen must be unlocked and agent-desktop needs
# Accessibility permission, otherwise `agent-desktop snapshot` answers PERM_DENIED.
set -eu

repo=$(cd "$(dirname "$0")/../../../.." && pwd)
state=${VERIFY_OPENRIDR_DESKTOP_STATE:-/tmp/verify-openridr-desktop}

case "${1:-}" in
up)
  app=${2:-${OPENRIDR_APP_BUNDLE:-/Applications/OpenRidr.app}}
  bin="$app/Contents/MacOS/openridr"
  [ -x "$bin" ] || { echo "desktop.sh: no executable at $bin" >&2; exit 1; }
  # Same CFBundleIdentifier for installed and debug bundles: never start a second instance next to the user's.
  if pgrep -x openridr >/dev/null; then echo "desktop.sh: an OpenRidr is already running; not double-driving it" >&2; exit 1; fi
  rm -rf "$state"; mkdir -p "$state/home" "$state/work"
  cp -R "$repo/fixtures/OrderDesk" "$state/work/OrderDesk"
  find "$state/work" \( -name bin -o -name obj \) -type d -prune -exec rm -rf {} +
  OPENRIDR_HOME="$state/home" nohup "$bin" "$state/work/OrderDesk/OrderDesk.sln" >"$state/app.log" 2>&1 &
  echo $! >"$state/app.pid"
  sleep 4
  kill -0 "$(cat "$state/app.pid")" || { echo "desktop.sh: app exited; see $state/app.log" >&2; exit 1; }
  echo "app pid $(cat "$state/app.pid"); home $state/home; solution $state/work/OrderDesk/OrderDesk.sln"
  ;;
down)
  [ -f "$state/app.pid" ] && kill "$(cat "$state/app.pid")" 2>/dev/null || true
  echo "stopped (state kept in $state)"
  ;;
*) echo "usage: desktop.sh up [bundle.app] | down" >&2; exit 2 ;;
esac
