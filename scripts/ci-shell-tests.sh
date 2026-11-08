#!/bin/sh
# Every tests/shell/*.test.sh, run under dash — the `/bin/sh` of the ubuntu CI runner, which is not the
# shell a developer's machine uses. The macOS `/bin/sh` is bash, so a test that leans on `local`, `[[ ]]`
# or `10#` passes locally and fails on the runner; running them under dash here makes the local result and
# the CI result the same shell.
#
# The list is the glob, never a hand-written one: a new test starts running without editing the workflow
# or this file. A glob that matches nothing exits 2, so a job that runs zero tests can never be green.
#
# Usage: sh scripts/ci-shell-tests.sh
#   SHELL_TESTS_SHELL  shell to run the tests with (default: dash)
#   SHELL_TESTS_DIR    directory to read (default: tests/shell next to this script)
#   Both exist for tests/shell/ci-shell-tests.test.sh; CI sets neither.
# Exit: 0 every test passed, 1 at least one failed (named on stdout), 2 the shell is missing or no test
#       was found.
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
dir=${SHELL_TESTS_DIR:-$repo/tests/shell}
shell=${SHELL_TESTS_SHELL:-dash}

command -v "$shell" >/dev/null 2>&1 || {
  echo "shell tests: $shell is not available; install it or set SHELL_TESTS_SHELL" >&2
  exit 2
}

echo "shell tests: $dir/*.test.sh under $shell"

found=0
failed=0
started=$(date +%s)
for file in "$dir"/*.test.sh; do
  # A glob that matches nothing stays literal, and a directory named *.test.sh is not a test.
  [ -f "$file" ] || continue
  found=$((found + 1))
  begin=$(date +%s)
  # `set +e` because the exit code is the result being recorded, not an error to abort on: the suite keeps
  # going so one run names every broken test.
  set +e
  "$shell" "$file"
  code=$?
  set -e
  elapsed=$(($(date +%s) - begin))
  if [ "$code" -eq 0 ]; then
    echo "ok   - $file (${elapsed}s)"
  else
    echo "FAIL - $file (exit $code, ${elapsed}s)"
    failed=$((failed + 1))
  fi
done

if [ "$found" -eq 0 ]; then
  echo "shell tests: no tests/shell/*.test.sh under $dir; the job would run nothing" >&2
  exit 2
fi

total=$(($(date +%s) - started))
echo "shell tests: $((found - failed)) passed, $failed failed under $shell (${total}s)"
[ "$failed" -eq 0 ] || exit 1
