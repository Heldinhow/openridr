#!/bin/sh
# Isolated web surface for driving the OpenRidr UI: host-http (4417) + Vite (1520), own app home,
# own copy of fixtures/OrderDesk. Ports differ from `bun run test:e2e` (1420/4317/4318) so both can
# coexist. Usage: web.sh up | down | status. State lives in $VERIFY_OPENRIDR_STATE.
set -eu

repo=$(cd "$(dirname "$0")/../../../.." && pwd)
state=${VERIFY_OPENRIDR_STATE:-/tmp/verify-openridr}
host_port=${VERIFY_HOST_PORT:-4417}
ui_port=${VERIFY_UI_PORT:-1520}
export PATH="$HOME/.bun/bin:$HOME/.dotnet:$HOME/.dotnet/tools:$PATH"

alive() { [ -f "$state/$1.pid" ] && kill -0 "$(cat "$state/$1.pid")" 2>/dev/null; }

case "${1:-}" in
up)
  if alive host || alive ui; then echo "web.sh: already up (run 'web.sh down' first)" >&2; exit 1; fi
  for port in "$host_port" "$ui_port"; do
    if lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1; then
      echo "web.sh: port $port is owned by another process; refusing to double-drive it" >&2; exit 1
    fi
  done
  rm -rf "$state"; mkdir -p "$state/home" "$state/work"
  cp -R "$repo/fixtures/OrderDesk" "$state/work/OrderDesk"
  find "$state/work" \( -name bin -o -name obj \) -type d -prune -exec rm -rf {} +
  (cd "$repo" && cargo build -q -p openridr-host-http)
  # Trust store ON (the real per-project behaviour); session store ON as in the app.
  OPENRIDR_HOME="$state/home" OPENRIDR_TRUST_STORE=on DOTNET_ROOT="$HOME/.dotnet" DOTNET_CLI_UI_LANGUAGE=en \
    nohup "$repo/target/debug/openridr-host-http" --port "$host_port" >"$state/host.log" 2>&1 &
  echo $! >"$state/host.pid"
  # `exec` inside the subshell keeps $! equal to the server pid, so `down` kills exactly what `up` started.
  (cd "$repo" && OPENRIDR_E2E_UI_PORT="$ui_port" OPENRIDR_E2E_HOST_PORT="$host_port" VITE_HOST_URL=/__host \
    exec node node_modules/vite/bin/vite.js --config vite.e2e.config.ts) >"$state/ui.log" 2>&1 &
  echo $! >"$state/ui.pid"
  i=0
  until curl -fs "http://127.0.0.1:$host_port/health" >/dev/null && curl -fs "http://127.0.0.1:$ui_port/" >/dev/null; do
    i=$((i + 1)); [ "$i" -lt 60 ] || { echo "web.sh: not ready after 30s; see $state/*.log" >&2; exit 1; }
    sleep 0.5
  done
  echo "ready: UI http://127.0.0.1:$ui_port  host http://127.0.0.1:$host_port"
  echo "solution to open: $state/work/OrderDesk/OrderDesk.sln"
  echo "app home: $state/home"
  ;;
down)
  for name in ui host; do
    if alive "$name"; then kill "$(cat "$state/$name.pid")"; fi
  done
  echo "stopped (state kept in $state; evidence must be copied out before removing it)"
  ;;
status)
  for name in host ui; do
    if alive "$name"; then echo "$name: up (pid $(cat "$state/$name.pid"))"; else echo "$name: down"; fi
  done
  ;;
*) echo "usage: web.sh up|down|status" >&2; exit 2 ;;
esac
