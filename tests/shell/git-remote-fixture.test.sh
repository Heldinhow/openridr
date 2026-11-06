#!/bin/sh
# Focused tests for the Git acceptance fixture and the verification launcher's input validation.
# They only create and remove their own mktemp directories: nothing outside $test_root and the
# symlink target created by the test is touched.
#
#   sh tests/shell/git-remote-fixture.test.sh
#
# Covers: refusals that must never delete (existing dir with a sentinel, '/', $HOME, the checkout,
# a relative or '..' path, a symlink to an outside directory), the successful new-state run, two
# default runs producing distinct states, and the launcher validating its override before it
# rotates the previous state.
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
tmp=${TMPDIR:-/tmp}; tmp=${tmp%/}
tmp_real=$(CDPATH= cd -- "$tmp" && pwd -P)
script="$repo/scripts/git-remote-fixture.sh"
desktop="$repo/.agents/skills/verify-openridr/scripts/desktop.sh"

failures=0
pass() { echo "ok   - $*"; }
fail() { echo "FAIL - $*"; failures=$((failures + 1)); }

test_root=$(mktemp -d "$tmp_real/openridr-fixture-test.XXXXXX")
outside=$(mktemp -d "$tmp_real/openridr-fixture-outside.XXXXXX")
cleanup() { rm -rf "$test_root" "$outside"; }
trap cleanup EXIT INT TERM

# Runs the fixture once and keeps the exit code, stdout and stderr for the assertions.
run_fixture() {
  set +e
  "$@" >"$test_root/stdout" 2>"$test_root/stderr"
  code=$?
  set -e
  out=$(cat "$test_root/stdout")
  err=$(cat "$test_root/stderr")
}

# 1. An existing directory with a sentinel is refused and left intact.
sentinel_dir="$test_root/existing"
mkdir "$sentinel_dir"
printf 'keep me\n' >"$sentinel_dir/sentinel.txt"
run_fixture sh "$script" "$sentinel_dir"
if [ "$code" -ne 0 ] && [ "$(cat "$sentinel_dir/sentinel.txt")" = "keep me" ] \
  && [ ! -e "$sentinel_dir/origin.git" ] && [ ! -e "$sentinel_dir/user" ]; then
  pass "refuses an existing directory and keeps its sentinel"
else
  fail "existing directory: code=$code sentinel=$(cat "$sentinel_dir/sentinel.txt" 2>/dev/null || echo missing)"
fi

# 2. '/' is refused and untouched.
run_fixture sh "$script" /
if [ "$code" -ne 0 ] && [ -d / ] && [ ! -e /origin.git ]; then
  pass "refuses / without touching it"
else
  fail "root: code=$code"
fi

# 3. The home directory is refused and untouched.
run_fixture sh "$script" "$HOME"
if [ "$code" -ne 0 ] && [ -d "$HOME" ] && [ ! -e "$HOME/origin.git" ] && [ ! -e "$HOME/user/OrderDesk.sln" ]; then
  pass "refuses \$HOME without touching it"
else
  fail "home: code=$code"
fi

# 4. The checkout is refused and its working tree is unchanged.
before=$(git -C "$repo" status --porcelain)
run_fixture sh "$script" "$repo"
after=$(git -C "$repo" status --porcelain)
if [ "$code" -ne 0 ] && [ "$before" = "$after" ] && [ ! -e "$repo/origin.git" ]; then
  pass "refuses the checkout and leaves the working tree as it was"
else
  fail "checkout: code=$code status-changed=$([ "$before" = "$after" ] && echo no || echo yes)"
fi

# 5. Relative and '..' paths are refused.
run_fixture sh "$script" "relative/state"
code_rel=$code
run_fixture sh "$script" "$test_root/a/../b"
code_dotdot=$code
if [ "$code_rel" -ne 0 ] && [ "$code_dotdot" -ne 0 ] && [ ! -e "$test_root/b" ]; then
  pass "refuses a relative path and a '..' path"
else
  fail "relative/..: relative=$code_rel dotdot=$code_dotdot"
fi

# 6. A symlink pointing outside the temporary root is refused; its target is untouched.
printf 'target sentinel\n' >"$outside/sentinel.txt"
ln -s "$outside" "$test_root/link"
run_fixture sh "$script" "$test_root/link"
if [ "$code" -ne 0 ] && [ "$(cat "$outside/sentinel.txt")" = "target sentinel" ] \
  && [ ! -e "$outside/origin.git" ]; then
  pass "refuses a symlink to an outside directory and keeps the target"
else
  fail "symlink: code=$code"
fi

# 7. A new path under the temporary root succeeds and produces a working fixture.
new_state="$test_root/fresh-state"
run_fixture sh "$script" "$new_state"
baseline=$(sed -n 's/^baseline //p' "$test_root/stdout")
if [ "$code" -eq 0 ] && [ -d "$new_state/origin.git" ] && [ -f "$new_state/user/OrderDesk.sln" ] \
  && [ -d "$new_state/peer/.git" ] && [ -n "$baseline" ] \
  && [ "$baseline" = "$(git -C "$new_state/origin.git" rev-parse refs/heads/main)" ] \
  && [ "$baseline" = "$(git -C "$new_state/peer" rev-parse HEAD)" ] \
  && [ "$(git -C "$new_state/user" status --porcelain | wc -l | tr -d ' ')" = "0" ]; then
  pass "creates bare + user + peer at a new path with one shared baseline ($baseline)"
else
  fail "fresh state: code=$code baseline=${baseline:-none}"
fi

# 8. Two default runs create two distinct states and leave both intact.
run_fixture sh "$script"
first=$(sed -n 's/^state //p' "$test_root/stdout")
first_code=$code
run_fixture sh "$script"
second=$(sed -n 's/^state //p' "$test_root/stdout")
second_code=$code
if [ "$first_code" -eq 0 ] && [ "$second_code" -eq 0 ] && [ -n "$first" ] && [ -n "$second" ] \
  && [ "$first" != "$second" ] && [ -d "$first/origin.git" ] && [ -d "$second/origin.git" ]; then
  pass "default runs create unique states and keep the earlier one ($first, $second)"
  rm -rf "$first" "$second"
else
  fail "default runs: first=$first_code:$first second=$second_code:$second"
fi

# 9. The launcher validates its override before rotating any previous state.
bundle="$repo/src-tauri/target/debug/bundle/macos/OpenRidr.app"
if ! pgrep -x openridr >/dev/null 2>&1 && [ -x "$bundle/Contents/MacOS/openridr" ]; then
  launcher_state="$test_root/desktop-state"
  mkdir -p "$launcher_state/home"
  printf 'previous session\n' >"$launcher_state/app.log"

  try_override() {
    set +e
    VERIFY_OPENRIDR_DESKTOP_STATE="$launcher_state" VERIFY_OPENRIDR_DESKTOP_SOLUTION="$1" \
      sh "$desktop" up "$bundle" >/dev/null 2>"$test_root/stderr"
    code=$?
    set -e
  }

  try_override "relative.sln"; code_rel_sln=$code
  try_override "$tmp_real/../etc/OrderDesk.sln"; code_dotdot_sln=$code
  try_override "$test_root/missing/OrderDesk.sln"; code_missing=$code

  if [ "$code_rel_sln" -ne 0 ] && [ "$code_dotdot_sln" -ne 0 ] && [ "$code_missing" -ne 0 ] \
    && [ -f "$launcher_state/app.log" ] && [ "$(cat "$launcher_state/app.log")" = "previous session" ] \
    && [ ! -e "$launcher_state.prev" ]; then
    pass "launcher refuses an invalid override before rotating the previous state"
  else
    fail "launcher: rel=$code_rel_sln dotdot=$code_dotdot_sln missing=$code_missing rotated=$([ -e "$launcher_state.prev" ] && echo yes || echo no)"
  fi
else
  echo "skip - launcher case needs no running openridr and the debug bundle at $bundle"
fi

echo
if [ "$failures" -eq 0 ]; then
  echo "all fixture tests passed"
else
  echo "$failures fixture test(s) failed"
  exit 1
fi
