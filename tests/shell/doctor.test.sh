#!/bin/sh
# Focused tests for the free-space check of .agents/skills/verify-openridr/scripts/doctor.sh. No disk is
# filled and no real tool is used: a stub `df` first on the PATH answers with the MiB the case asks for,
# and stubs for the rest of the environment (bun, cargo, dotnet, pgrep, lsof, agent-desktop) keep the exit
# code depending on the disk case alone. The script under test is copied into a minimal repo tree (with
# node_modules) because it derives the repo from its own path.
#
#   sh tests/shell/doctor.test.sh
#
#   DOCTOR_SCRIPT=<path> sh tests/shell/doctor.test.sh   # run the cases against another copy of the
#                                                        # script (the falha-antes proof)
#   DOCTOR_SHELL=dash sh tests/shell/doctor.test.sh      # run the script under dash, the runner's /bin/sh
#
# Covers: 4000 MiB free (`ok`, exit 0), 1500 MiB (`note`, exit 0), 900 MiB (`FAIL`, exit 1), a `df`
# without a number (`note`, exit 0, never a silent `ok`). Every case asserts the measured number and the
# measured directory in the line, and that nothing was deleted.
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
script=${DOCTOR_SCRIPT:-$repo/.agents/skills/verify-openridr/scripts/doctor.sh}

failures=0
pass() { echo "ok   - $*"; }
fail() { echo "FAIL - $*"; failures=$((failures + 1)); }

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-doctor-test.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

root=$scratch/repo
mkdir -p "$root/node_modules" "$root/.agents/skills/verify-openridr/scripts" \
  "$scratch/home" "$scratch/tmp" "$scratch/stubs"
cp "$script" "$root/.agents/skills/verify-openridr/scripts/doctor.sh"
printf '{"name":"fake","version":"0.0.0"}\n' >"$root/package.json"
# A file in the measured directory: the doctor measures, it never deletes.
: >"$scratch/tmp/sentinel"

# The stubs. `df` prints the layout of `df -Pk` and answers column 4 (`Available`, 1024-blocks) with
# $FAKE_DF_MIB; the others only satisfy the environment checks so they cannot move the exit code.
cat >"$scratch/stubs/df" <<'STUB'
#!/bin/sh
case "$FAKE_DF_MIB" in
  '' | *[!0-9]*) avail=- ;;
  *) avail=$((FAKE_DF_MIB * 1024)) ;;
  esac
printf '%s\n' 'Filesystem 1024-blocks Used Available Capacity Mounted on'
printf '%s\n' "/dev/disk9s1 100000000 0 $avail 0% /"
STUB
printf '#!/bin/sh\necho 1.3.0\n' >"$scratch/stubs/bun"
printf '#!/bin/sh\nexit 0\n' >"$scratch/stubs/cargo"
printf '#!/bin/sh\necho "8.0.408 [/usr/local/share/dotnet/sdk]"\n' >"$scratch/stubs/dotnet"
printf '#!/bin/sh\nexit 1\n' >"$scratch/stubs/pgrep"
printf '#!/bin/sh\nexit 1\n' >"$scratch/stubs/lsof"
printf '#!/bin/sh\necho %s\n' \
  "'{\"accessibility\":{\"state\":\"granted\"},\"screen_recording\":{\"state\":\"granted\"}}'" \
  >"$scratch/stubs/agent-desktop"
chmod +x "$scratch/stubs"/*

# run <MiB>: the doctor sees only this much free space. HOME is a scratch dir so the PATH the script
# prepends ($HOME/.bun/bin, $HOME/.dotnet) cannot shadow the stubs.
run() {
  set +e
  out=$(env HOME="$scratch/home" TMPDIR="$scratch/tmp" PATH="$scratch/stubs:$PATH" FAKE_DF_MIB="$1" \
    "${DOCTOR_SHELL:-sh}" "$root/.agents/skills/verify-openridr/scripts/doctor.sh" \
    "$scratch/none.app" 2>"$scratch/stderr")
  code=$?
  set -e
}

# case_line <MiB> <severity> <exit>: the exit code, then the one line, then the number in it.
case_line() {
  mib=$1
  sev=$2
  want=$3
  if [ "$code" -ne "$want" ]; then
    fail "$mib MiB: expected exit $want, got $code ($(cat "$scratch/stderr"))"
    return
  fi
  line=$(printf '%s\n' "$out" | grep -E "^$sev +disk $mib MiB free on $scratch/tmp" || true)
  if [ -n "$line" ]; then
    pass "$mib MiB: $sev and exit $want, number and directory in the line"
  else
    fail "$mib MiB: no '$sev  disk $mib MiB free on $scratch/tmp' line in: $out"
  fi
}

# --- above the floor: ok, exit 0 ---------------------------------------------------------------------
run 4000
case_line 4000 ok 0

# --- between the floors: note, exit 0 ----------------------------------------------------------------
run 1500
case_line 1500 note 0

# --- below the hard floor: FAIL, exit 1 --------------------------------------------------------------
run 900
case_line 900 FAIL 1

# --- a measurement with no number: note, never ok, exit 0 --------------------------------------------
run unreadable
if [ "$code" -eq 0 ] && printf '%s\n' "$out" | grep -qF "note  disk free space on $scratch/tmp unknown"; then
  pass "unreadable df: note and exit 0, not a silent ok"
else
  fail "unreadable df: expected 'note  disk free space on $scratch/tmp unknown' and exit 0, got exit $code: $out"
fi

# --- read-only: the measured directory is untouched --------------------------------------------------
if [ -f "$scratch/tmp/sentinel" ]; then
  pass "nothing was deleted from the measured directory"
else
  fail "the doctor deleted a file from the measured directory"
fi

echo
if [ "$failures" -eq 0 ]; then
  echo "all cases passed"
else
  echo "$failures failure(s)"
  exit 1
fi
