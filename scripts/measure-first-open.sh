#!/bin/sh
# Measure the first open of a downloaded (quarantined) OpenRidr.app on macOS.
#
# What it measures, per round:
#   exec    process exists (LaunchServices created it)
#   prompt  the Gatekeeper dialog was presented (CoreServicesUIAgent log line)
#   code    our code runs (RSS passes 20 MB; the process is suspended until the dialog is answered)
#   window  the OpenRidr window is listed as visible
# `decision->code` and `code->window` are derived when a decision was observed.
#
# Two facts about macOS that the script assumes, both measured (docs/evidence/first-open.md):
#   1. A quarantined app is NOT started until the dialog is answered (launchInQuarantine). The
#      "prompt" to "code" gap is the human click, not the app.
#   2. The system remembers the approval per path. Repeating a round on the same copy in the same
#      directory shows no dialog, so every round gets a fresh copy, a fresh path and a fresh
#      quarantine UUID. Clearing the syspolicyd cache itself needs root.
#
# Answering the dialog: pass --click X,Y to do it automatically (physical click, display
# specific; take a screenshot while the dialog is up to find the Abrir button). Without --click
# the script waits for a human to answer it and reports the total as well; if nobody answers
# within --timeout it reports the prompt only.
#
# Requires: macOS, `agent-desktop` (only for --click and for the window/prompt observation of
# --click mode is not needed otherwise) and accessibility permission for the terminal.
# Usage: scripts/measure-first-open.sh [--app PATH] [--rounds N] [--click X,Y] [--timeout S] [--keep]
set -eu

APP=${OPENRIDR_APP:-/Applications/OpenRidr.app}
ROUNDS=3
CLICK=
TIMEOUT=120
KEEP=no

usage() {
  sed -n '2,25p' "$0" | sed 's/^# \{0,1\}//'
  exit 1
}

while [ $# -gt 0 ]; do
  case $1 in
    --app) APP=$2; shift 2 ;;
    --rounds) ROUNDS=$2; shift 2 ;;
    --click) CLICK=$2; shift 2 ;;
    --timeout) TIMEOUT=$2; shift 2 ;;
    --keep) KEEP=yes; shift ;;
    -h|--help) usage ;;
    *) echo "measure-first-open: unknown argument $1" >&2; exit 1 ;;
  esac
done

[ -d "$APP" ] || { echo "measure-first-open: $APP is not an app bundle" >&2; exit 1; }
case $ROUNDS in ''|*[!0-9]*) echo "measure-first-open: --rounds must be a number" >&2; exit 1 ;; esac
[ "$ROUNDS" -ge 1 ] && [ "$ROUNDS" -le 5 ] || { echo "measure-first-open: --rounds must be 1..5" >&2; exit 1; }
command -v agent-desktop >/dev/null 2>&1 \
  || { echo "measure-first-open: needs agent-desktop on PATH (window detection; --click also clicks)" >&2; exit 1; }
agent-desktop status 2>/dev/null | grep -q '"accessibility":{"state":"granted"}' \
  || { echo "measure-first-open: accessibility is not granted to this terminal (agent-desktop permissions --guide)" >&2; exit 1; }

# One instance at a time: a second launch lands on the single-instance plugin and measures nothing.
if pgrep -x openridr >/dev/null 2>&1; then
  echo "measure-first-open: OpenRidr is already running; quit it before measuring" >&2
  exit 1
fi

STATE=$(mktemp -d "${TMPDIR:-/tmp}/openridr-first-open.XXXXXX")
HOME_DIR="$STATE/home"
LOG="$STATE/uia.log"
mkdir -p "$HOME_DIR"

PREV_HOME_SET=$(launchctl getenv OPENRIDR_HOME || true)
cleanup() {
  [ -n "${LOG_PID:-}" ] && kill "$LOG_PID" 2>/dev/null || true
  pkill -x openridr 2>/dev/null || true
  if [ -n "$PREV_HOME_SET" ]; then
    launchctl setenv OPENRIDR_HOME "$PREV_HOME_SET"
  else
    launchctl unsetenv OPENRIDR_HOME 2>/dev/null || true
  fi
  [ "$KEEP" = yes ] || rm -rf "$STATE"
}
trap cleanup EXIT INT TERM

# The app reads its state directory from the environment, and `open` does not forward ours.
launchctl setenv OPENRIDR_HOME "$HOME_DIR"

# Phase clock: seconds with millisecond precision, without relying on coreutils' subsecond date.
now() { perl -MTime::HiRes=time -e 'printf "%.3f\n", time'; }

round=0
echo "measure-first-open: app=$APP rounds=$ROUNDS state=$STATE"
echo "# round   exec  prompt   code  window  decision->code  code->window  launch->window"
while [ "$round" -lt "$ROUNDS" ]; do
  round=$((round + 1))
  copy="$STATE/r$round/OpenRidr.app"
  mkdir -p "$STATE/r$round"
  ditto --noqtn "$APP" "$copy"
  xattr -d com.apple.quarantine "$copy" 2>/dev/null || true
  xattr -w com.apple.quarantine "0083;$(printf '%x' "$(date +%s)");Safari;$(uuidgen)" "$copy"
  rm -rf "$HOME_DIR"; mkdir -p "$HOME_DIR"
  : >"$LOG"

  log stream --style compact --predicate 'process == "CoreServicesUIAgent"' >"$LOG" 2>/dev/null &
  LOG_PID=$!
  sleep 1

  t0=$(now)
  open -n "$copy"

  t_exec=; t_prompt=; t_click=; t_code=; t_window=
  deadline=$(perl -e "print $t0 + $TIMEOUT")
  while perl -e "exit(( $deadline - $(now) ) > 0 ? 0 : 1)"; do
    if [ -z "$t_exec" ] && pid=$(pgrep -x openridr | head -n 1); then
      t_exec=$(now)
    fi
    if [ -z "$t_prompt" ] && grep -q "code-evaluation prompt" "$LOG" 2>/dev/null; then
      t_prompt=$(now)
      if [ -n "$CLICK" ]; then
        sleep 0.1
        agent-desktop mouse-click --xy "$CLICK" --headed >/dev/null 2>&1 || true
        t_click=$(now)
      fi
    fi
    if [ -z "$t_code" ] && [ -n "${pid:-}" ]; then
      rss=$(ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ' || true)
      [ -n "$rss" ] && [ "$rss" -gt 20000 ] && t_code=$(now)
    fi
    if [ -z "$t_window" ] && agent-desktop list-windows --app OpenRidr 2>/dev/null | grep -q '"app_name":"OpenRidr"'; then
      t_window=$(now)
    fi
    [ -n "$t_window" ] && break
    sleep 0.05
  done

  kill "$LOG_PID" 2>/dev/null || true; LOG_PID=
  # A round that timed out may still have the dialog on screen; killing the app dismisses it.
  while pgrep -x openridr >/dev/null 2>&1; do
    kill "$(pgrep -x openridr | head -n 1)" 2>/dev/null || true
    sleep 0.2
  done

  derive() { [ -n "$1" ] && [ -n "$2" ] && perl -e "printf '%.2f', $2 - $1" || printf '%6s' '-'; }
  dcode=$(derive "$t_click" "$t_code")
  dwin=$(derive "$t_click" "$t_window")
  lwin=$(derive "$t0" "$t_window")
  fmt() { [ -n "$1" ] && perl -e "printf '%6.2f', $1 - $t0" || printf '%6s' '-'; }
  printf '%7s %6s %6s %6s %6s %15s %13s %14s\n' "$round" "$(fmt "$t_exec")" "$(fmt "$t_prompt")" \
    "$(fmt "$t_code")" "$(fmt "$t_window")" "$dcode" "$dwin" "$lwin"
done
echo
echo "measure-first-open: prompt = system dialog cost; decision->code and code->window are the app's;"
echo "measure-first-open: launch->window includes the human click when --click is not given."
