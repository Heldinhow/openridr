#!/bin/sh
# BEGIN HELP
# Measure the first open of a downloaded (quarantined) OpenRidr.app on macOS.
#
# Per round it prints: exec (process exists), prompt (Gatekeeper dialog presented), click
# (dialog answered, only with --click), code (our code runs; the process stays suspended until
# the dialog is answered) and window (the OpenRidr window is listed as visible), plus the deltas.
#
# Two measured facts about macOS that this script relies on (docs/evidence/first-open.md):
#   1. A quarantined app is NOT started until the dialog is answered (launchInQuarantine), so the
#      "prompt" to "code" gap is the human click, not the app.
#   2. The system remembers the approval per path: repeating a round on the same copy in the same
#      directory shows no dialog. Each round therefore gets a fresh copy, a fresh path and a fresh
#      quarantine UUID. Clearing the syspolicyd cache itself needs root.
#
# Answering the dialog: --click X,Y clicks it automatically (physical click, display specific;
# take a screenshot while the dialog is up to find the Abrir button). Without --click the script
# waits for a human and reports the total; if nobody answers within --timeout it reports the
# prompt only. A click that does not start the app within 5 s fails the round.
#
# Interrupting a round can leave the app (and the dialog) on screen; quit it before measuring again.
# Killing the app does not dismiss the alert when the app never started: the alert belongs to
# CoreServicesUIAgent, so a failed round is followed by Escape (with --click) or a warning.
#
# Sampling runs at roughly 0,15-0,25 s per observation (pgrep/ps/perl/agent-desktop), so the
# printed hundredths are a lower bound of the real instant, not a microbenchmark.
#
# The script never mounts anything: it copies --app (default /Applications/OpenRidr.app, or
# $OPENRIDR_APP) into a temporary directory, so it reproduces the "app already on disk" condition,
# not the "open straight from the DMG" one.
#
# Requires macOS, a Developer ID + notarized bundle (--allow-unnotarized to measure another one)
# and agent-desktop with accessibility and screen recording granted.
# --check-only validates the arguments and preconditions and exits without launching anything.
# Usage: scripts/measure-first-open.sh [--app PATH] [--rounds N] [--click X,Y] [--timeout S] [--keep] [--check-only]
# END HELP
set -eu

APP=${OPENRIDR_APP:-/Applications/OpenRidr.app}
ROUNDS=3
CLICK=
TIMEOUT=120
KEEP=no
ALLOW_UNNOTARIZED=no
CHECK_ONLY=no

die() { printf 'measure-first-open: %s\n' "$*" >&2; exit 1; }

usage() {
  sed -n '/^# BEGIN HELP$/,/^# END HELP$/p' "$0" | sed '1d;$d;s/^# \{0,1\}//'
  exit 0
}

# A flag without its value used to die with "$2: unbound variable"; keep the message readable.
value() {
  [ "$2" -ge 2 ] || die "$1 needs a value"
}

while [ "$#" -gt 0 ]; do
  case $1 in
    --app) value "$1" "$#"; APP=$2; shift 2 ;;
    --rounds) value "$1" "$#"; ROUNDS=$2; shift 2 ;;
    --click) value "$1" "$#"; CLICK=$2; shift 2 ;;
    --timeout) value "$1" "$#"; TIMEOUT=$2; shift 2 ;;
    --keep) KEEP=yes; shift ;;
    --allow-unnotarized) ALLOW_UNNOTARIZED=yes; shift ;;
    --check-only) CHECK_ONLY=yes; shift ;;
    -h|--help) usage ;;
    *) die "unknown argument $1 (--help for usage)" ;;
  esac
done

[ "$(uname -s)" = Darwin ] || die "macOS only"
[ -d "$APP" ] || die "$APP is not an app bundle"
is_number() { case $1 in ''|*[!0-9]*) return 1 ;; *) return 0 ;; esac; }
is_number "$ROUNDS" || die "--rounds must be a number"
[ "$ROUNDS" -ge 1 ] && [ "$ROUNDS" -le 5 ] || die "--rounds must be 1..5"
is_number "$TIMEOUT" || die "--timeout must be a number of seconds"
[ "$TIMEOUT" -ge 5 ] || die "--timeout must be at least 5 seconds"
if [ -n "$CLICK" ]; then
  case $CLICK in *,*) ;; *) die "--click wants X,Y" ;; esac
fi

if [ "$ALLOW_UNNOTARIZED" = no ]; then
  codesign -dv --verbose=2 "$APP" 2>&1 | grep -q '^Authority=Developer ID Application' \
    || die "$APP is not Developer ID signed; numbers would not describe a distributed app (--allow-unnotarized)"
  spctl --assess --type execute "$APP" >/dev/null 2>&1 \
    || die "$APP is not accepted by Gatekeeper (not notarized?); --allow-unnotarized to measure anyway"
fi

command -v agent-desktop >/dev/null 2>&1 \
  || die "needs agent-desktop on PATH (window detection; --click also clicks)"
status=$(agent-desktop status 2>/dev/null || true)
case $status in
  *'"accessibility":{"state":"granted"}'*) ;;
  *) die "accessibility is not granted to this terminal (agent-desktop permissions --guide)" ;;
esac
# The per-round screenshot only exists in --click mode; do not demand the permission otherwise.
if [ -n "$CLICK" ]; then
  case $status in
    *'"screen_recording":{"state":"granted"}'*) ;;
    *) die "screen recording is not granted to this terminal (needed for the per-round capture)" ;;
  esac
fi

# One instance at a time: a second launch lands on the single-instance plugin and measures nothing.
pgrep -x openridr >/dev/null 2>&1 && die "OpenRidr is already running; quit it before measuring"

if [ "$CHECK_ONLY" = yes ]; then
  printf 'measure-first-open: check ok: app=%s rounds=%s click=%s timeout=%ss\n' \
    "$APP" "$ROUNDS" "${CLICK:-none}" "$TIMEOUT"
  exit 0
fi

STATE=$(mktemp -d "${TMPDIR:-/tmp}/openridr-first-open.XXXXXX")
HOME_DIR="$STATE/home"
LOG="$STATE/uia.log"
LOG_PID=
APP_PID=
mkdir -p "$HOME_DIR"

# The app reads its state directory from the environment, and `open` does not forward ours.
PREV_HOME=$(launchctl getenv OPENRIDR_HOME || true)
launchctl setenv OPENRIDR_HOME "$HOME_DIR"
cleanup() {
  [ -n "$LOG_PID" ] && kill "$LOG_PID" 2>/dev/null || true
  # Only the instance this script launched: a real OpenRidr the user starts meanwhile is not ours.
  [ -n "$APP_PID" ] && kill "$APP_PID" 2>/dev/null || true
  if [ -n "$PREV_HOME" ]; then
    launchctl setenv OPENRIDR_HOME "$PREV_HOME"
  else
    launchctl unsetenv OPENRIDR_HOME 2>/dev/null || true
  fi
  if [ "$KEEP" = yes ]; then
    printf 'measure-first-open: state kept in %s\n' "$STATE"
  else
    rm -rf "$STATE"
  fi
}
trap cleanup EXIT
trap 'exit 130' INT TERM

now() { perl -MTime::HiRes=time -e 'printf "%.3f\n", time'; }
delta() { [ -n "$1" ] && [ -n "$2" ] && perl -e "printf '%.2f', $2 - $1" || printf -- '-'; }
since() { [ -n "$1" ] && perl -e "printf '%.2f', $1 - $T0" || printf -- '-'; }

# The dialog and the window are read from JSON text; accept both compact and spaced output.
has_window() {
  printf '%s' "$1" | grep -Eq '"app_name": *"OpenRidr"' && printf '%s' "$1" | grep -Eq '"visible": *true'
}

printf 'measure-first-open: app=%s rounds=%s%s\n' "$APP" "$ROUNDS" \
  "$([ "$KEEP" = yes ] && echo " state=$STATE" || echo "")"
printf '#%6s %6s %6s %6s %6s %6s %12s %13s\n' round exec prompt click code window \
  'click->code' 'click->window'

round=0
failed=0
started=0
while [ "$round" -lt "$ROUNDS" ]; do
  round=$((round + 1))
  copy="$STATE/r$round/OpenRidr.app"
  mkdir -p "$STATE/r$round"
  if ! ditto --noqtn "$APP" "$copy" 2>"$STATE/r$round/ditto.err"; then
    printf '%7s  failed: ditto refused (%s)\n' "$round" "$(cat "$STATE/r$round/ditto.err")"
    failed=$((failed + 1))
    continue
  fi
  # ditto --noqtn should not copy it, but the source may itself be inside a quarantined image.
  xattr -d com.apple.quarantine "$copy" 2>/dev/null || true
  xattr -w com.apple.quarantine "0083;$(printf '%x' "$(date +%s)");Safari;$(uuidgen)" "$copy"
  rm -rf "$HOME_DIR"; mkdir -p "$HOME_DIR"
  : >"$LOG"
  APP_PID=
  # Only a process that did not exist before this launch is ours; the user may open OpenRidr
  # mid-run and that instance must not be measured nor killed.
  before=$(pgrep -x openridr 2>/dev/null | tr '\n' ' ')

  log stream --style compact --predicate 'process == "CoreServicesUIAgent"' >"$LOG" 2>/dev/null &
  LOG_PID=$!
  sleep 1
  kill -0 "$LOG_PID" 2>/dev/null || die "log stream died; nothing would be observed"

  T0=$(now)
  if ! open -n "$copy" 2>"$STATE/r$round/open.err"; then
    kill "$LOG_PID" 2>/dev/null || true; LOG_PID=
    printf '%7s  failed: open refused (%s)\n' "$round" "$(cat "$STATE/r$round/open.err")"
    failed=$((failed + 1))
    continue
  fi

  t_exec=; t_prompt=; t_click=; t_code=; t_window=; pid=
  deadline=$(perl -e "print $T0 + $TIMEOUT")
  verdict=
  while [ "$(perl -e "print($(now) < $deadline ? 1 : 0)")" = 1 ]; do
    if [ -z "$t_exec" ]; then
      for candidate in $(pgrep -x openridr 2>/dev/null || true); do
        case " $before " in *" $candidate "*) continue ;; esac
        pid=$candidate; break
      done
      if [ -n "${pid:-}" ]; then t_exec=$(now); APP_PID=$pid; fi
    fi
    if [ -z "$t_prompt" ] && grep -q "code-evaluation prompt" "$LOG" 2>/dev/null; then
      t_prompt=$(now)
      if [ -n "$CLICK" ]; then
        sleep 0.1
        agent-desktop screenshot "$STATE/r$round/alert.png" --screen 0 >/dev/null 2>&1 || true
        agent-desktop mouse-click --xy "$CLICK" --headed >/dev/null 2>"$STATE/r$round/click.err" \
          || verdict="click failed: $(cat "$STATE/r$round/click.err")"
        t_click=$(now)
      fi
    fi
    if [ -z "$t_code" ] && [ -n "$pid" ]; then
      rss=$(ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ' || true)
      [ -n "$rss" ] && [ "$rss" -gt 20000 ] && t_code=$(now)
    fi
    if [ -z "$t_window" ]; then
      if has_window "$(agent-desktop list-windows --app OpenRidr 2>/dev/null || true)"; then
        t_window=$(now)
      fi
    fi
    # Fail closed instead of burning the whole timeout on a click that missed the button.
    if [ -n "$t_click" ] && [ -z "$t_code" ]; then
      if perl -e "exit($(now) - $t_click > 5 ? 0 : 1)"; then
        verdict="click at $CLICK did not dismiss the dialog (5 s)"
        break
      fi
    fi
    [ -n "$t_window" ] && break
    [ -n "$verdict" ] && break
    sleep 0.05
  done

  kill "$LOG_PID" 2>/dev/null || true; LOG_PID=
  # A round that never got `code` leaves the alert on screen even after the process is gone (the
  # alert belongs to CoreServicesUIAgent). Dismiss it, or the next round starts behind it.
  if [ -n "$t_prompt" ] && [ -z "$t_code" ]; then
    if [ -n "$CLICK" ]; then
      agent-desktop press escape --headed >/dev/null 2>&1 || true
    else
      printf 'measure-first-open: dismiss the Gatekeeper dialog on screen before the next round\n' >&2
    fi
  fi
  if [ -n "$APP_PID" ]; then
    kill "$APP_PID" 2>/dev/null || true
    waited=0
    while kill -0 "$APP_PID" 2>/dev/null && [ "$waited" -lt 40 ]; do sleep 0.1; waited=$((waited + 1)); done
    kill -9 "$APP_PID" 2>/dev/null || true
    APP_PID=
  fi

  printf '%7s %6s %6s %6s %6s %6s %12s %13s' "$round" "$(since "$t_exec")" "$(since "$t_prompt")" \
    "$(since "$t_click")" "$(since "$t_code")" "$(since "$t_window")" \
    "$(delta "$t_click" "$t_code")" "$(delta "$t_click" "$t_window")"
  if [ -n "$verdict" ]; then
    printf '  failed: %s\n' "$verdict"
    failed=$((failed + 1))
  elif [ -n "$t_window" ] && [ -z "$t_code" ]; then
    printf '  failed: window appeared but the RSS jump was never observed\n'
    failed=$((failed + 1))
  elif [ -n "$t_window" ]; then
    printf '\n'
    started=$((started + 1))
  else
    printf '  incomplete: no window within %ss\n' "$TIMEOUT"
    failed=$((failed + 1))
  fi
  rm -rf "$HOME_DIR"
done

printf '\nmeasure-first-open: %d of %d rounds reached a window\n' "$started" "$ROUNDS"
printf 'measure-first-open: prompt = system dialog cost; click->code and click->window are the app'"'"'s;\n'
printf 'measure-first-open: without --click the columns stop at prompt and the wait is the human.\n'
[ "$failed" -eq 0 ] || exit 1
