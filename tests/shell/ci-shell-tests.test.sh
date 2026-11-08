#!/bin/sh
# Focused tests for scripts/ci-shell-tests.sh, the runner behind the `shell` CI job. No network and no
# GitHub: every case is a scratch directory of fake *.test.sh files.
#
#   sh tests/shell/ci-shell-tests.test.sh
#   dash tests/shell/ci-shell-tests.test.sh   # the runner's own shell, as CI runs it
#
# Covers: a failing test next to a passing one (both reported, the passing one still ran), an all-green
# directory, a directory with no test (exit 2, never green), a missing shell (exit 2) and the proof that
# the fake tests ran under dash and not under this harness's own shell.
#
# SHELL_TESTS_DIR is always passed explicitly: the default is tests/shell, which contains this file, and
# the runner would call itself.
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
runner="$repo/scripts/ci-shell-tests.sh"

failures=0
pass() { echo "ok   - $*"; }
fail() { echo "FAIL - $*"; failures=$((failures + 1)); }

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-shell-tests.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

# run <dir> [env assignments...]: keeps exit code, stdout and stderr for the assertions.
run() {
  dir=$1
  shift
  set +e
  env SHELL_TESTS_DIR="$dir" "$@" sh "$runner" >"$scratch/stdout" 2>"$scratch/stderr"
  code=$?
  set -e
  out=$(cat "$scratch/stdout")
  err=$(cat "$scratch/stderr")
}

# expect <exit> <name> <dir> <fragment> [env assignments...]: the fragment is looked for in stdout and
# stderr together, because the runner reports per-test results on stdout and its own errors on stderr.
expect() {
  want=$1
  name=$2
  dir=$3
  fragment=$4
  shift 4
  run "$dir" "$@"
  combined="$out$err"
  if [ "$code" -ne "$want" ]; then
    fail "$name: expected exit $want, got $code ($combined)"
    return
  fi
  if [ -n "$fragment" ] && ! printf '%s' "$combined" | grep -qF "$fragment"; then
    fail "$name: output does not mention '$fragment' ($combined)"
    return
  fi
  pass "$name"
}

# Every fake test refuses to run under bash: BASH_VERSION is set by bash and absent in dash, so a runner
# that ignored SHELL_TESTS_SHELL and used the harness's own shell would fail here instead of passing.
bash_guard=$(cat <<'GUARD'
if [ -n "${BASH_VERSION:-}" ]; then
  echo "BASH_VERSION is set: this test did not run under dash" >&2
  exit 1
fi
GUARD
)

mixed="$scratch/mixed"
mkdir -p "$mixed"
cat >"$mixed/a-failing.test.sh" <<TEST
$bash_guard
echo "a-failing: about to fail"
exit 1
TEST
# The marker is the proof that the runner kept going after the failure above, not only that it printed the
# file name.
cat >"$mixed/b-passing.test.sh" <<TEST
$bash_guard
: >"$mixed/b-passing.ran"
echo "b-passing: fine"
TEST

# A failing test does not stop the suite, both are named and the exit code fails the job.
expect 1 mixed-exit-1 "$mixed" 'a-failing.test.sh'
if ! printf '%s' "$out" | grep -qF 'FAIL - '; then
  fail "mixed: no FAIL line ($out)"
fi
if ! printf '%s' "$out" | grep -qF "$mixed/b-passing.test.sh"; then
  fail "mixed: the passing test is not in the output ($out)"
fi
if [ ! -f "$mixed/b-passing.ran" ]; then
  fail "mixed: the passing test did not run after the failing one"
fi

green="$scratch/green"
mkdir -p "$green"
cat >"$green/c-passing.test.sh" <<TEST
$bash_guard
echo "c-passing: fine"
TEST
cat >"$green/d-passing.test.sh" <<TEST
$bash_guard
echo "d-passing: fine"
TEST

# All green is the only green, and the fake tests above prove it is dash that ran them.
expect 0 all-passing "$green" 'c-passing.test.sh'
expect 0 all-passing-summary "$green" '2 passed'

# A glob that matches nothing must never read as green: the job would run zero tests and pass.
empty="$scratch/empty"
mkdir -p "$empty"
expect 2 no-test-found "$empty" 'no tests/shell/*.test.sh'

# A file that does not match the glob and a directory that does: neither is a test.
mkdir -p "$empty/notes" "$empty/dir.test.sh"
printf 'exit 1\n' >"$empty/notes/not-a-test.sh"
expect 2 no-test-found-ignores-other-files "$empty" 'no tests/shell/*.test.sh'

# A missing shell is a broken job, not a passing one.
expect 2 missing-shell "$green" 'is not available' SHELL_TESTS_SHELL=openridr-no-such-shell

if [ "$failures" -ne 0 ]; then
  echo "ci-shell-tests tests: $failures failure(s)"
  exit 1
fi

echo "ci-shell-tests tests: all cases passed"
