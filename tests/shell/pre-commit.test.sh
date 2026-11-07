#!/bin/sh
# Focused tests for .githooks/pre-commit. The hook is the seam: every case makes a real `git commit` in
# a throwaway repository and reads the effect from outside (which commands ran, with what argv, the
# exit code, the line printed, whether the commit exists). `bun` and `cargo` are stubs first on the
# PATH that only log their argv, so nothing is compiled and the result depends on the staged paths
# alone.
#
#   sh tests/shell/pre-commit.test.sh
#
#   PRE_COMMIT_HOOK=<path> sh tests/shell/pre-commit.test.sh   # run the cases against another copy of
#                                                              # the hook (the falha-antes proof)
#
# Covers: docs only (nothing runs), TypeScript only (tsc, no cargo), Rust only (cargo, no tsc), both,
# an amend that only changes the message (nothing runs), TypeScript without node_modules (fails before
# bun, says `bun install`, never `command not found`), docs only without node_modules (passes), a
# `git commit --only` from scripts/commit-only.sh with Rust dirty in the work tree (the work tree is not
# classified), a failing check (the hook propagates the exit code and the commit is not created), and
# the hook run under dash, the runner's /bin/sh.
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
hook=${PRE_COMMIT_HOOK:-$repo/.githooks/pre-commit}
commit_only=$repo/scripts/commit-only.sh

failures=0
pass() { echo "ok   - $*"; }
fail() { echo "FAIL - $*"; failures=$((failures + 1)); }

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-pre-commit-test.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/stubs"

# The stubs. They record the argv the hook invoked and answer with the exit code the case asks for;
# they never run tsc or cargo.
cat >"$scratch/stubs/bun" <<'STUB'
#!/bin/sh
printf 'bun %s\n' "$*" >>"$FAKE_LOG"
exit "${FAKE_BUN_EXIT:-0}"
STUB
cat >"$scratch/stubs/cargo" <<'STUB'
#!/bin/sh
printf 'cargo %s\n' "$*" >>"$FAKE_LOG"
exit "${FAKE_CARGO_EXIT:-0}"
STUB
chmod +x "$scratch/stubs"/*

# git_env: git hermetic from the user's and the system's config (no gpgsign, no hooksPath leaking in),
# with the stubs first on the PATH and the argv log pointed at this case's file.
git_env() {
  env PATH="$scratch/stubs:$PATH" FAKE_LOG="$log" FAKE_BUN_EXIT="$bun_exit" \
    FAKE_CARGO_EXIT="$cargo_exit" GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null \
    git -C "$dir" "$@"
}

# new_repo <path>...: a throwaway repo whose initial commit holds the given files, with the hook under
# test installed in .githooks/ and enabled. The initial commit is made before core.hooksPath is set, so
# it never runs the hook.
n=0
new_repo() {
  n=$((n + 1))
  dir=$scratch/case$n
  log=$scratch/case$n.log
  out=''
  code=0
  bun_exit=0
  cargo_exit=0
  : >"$log"
  mkdir -p "$dir"
  git -C "$dir" init -q
  git_env config user.email test@example.com
  git_env config user.name Test
  git_env config commit.gpgsign false
  mkdir -p "$dir/docs"
  printf 'seed\n' >"$dir/docs/seed.md"
  for p in "$@"; do
    mkdir -p "$dir/$(dirname "$p")"
    printf 'seed\n' >"$dir/$p"
  done
  git_env add -A
  git_env commit -q -m init
  mkdir -p "$dir/.githooks"
  cp "$hook" "$dir/.githooks/pre-commit"
  chmod +x "$dir/.githooks/pre-commit"
  git_env config core.hooksPath .githooks
}

write_file() {
  mkdir -p "$dir/$(dirname "$1")"
  printf 'change\n' >"$dir/$1"
}

# stage <path>...: write the files and stage exactly them (never -A: .githooks/ and node_modules/ are
# untracked in the scratch repo and must not become part of the commit under test).
stage() {
  for p in "$@"; do write_file "$p"; done
  git_env add -- "$@"
}

# with_tsc: a node_modules that has the binary the hook gates on; no real TypeScript is installed.
with_tsc() {
  mkdir -p "$dir/node_modules/.bin"
  printf '#!/bin/sh\nexit 0\n' >"$dir/node_modules/.bin/tsc"
  chmod +x "$dir/node_modules/.bin/tsc"
}

commit() {
  set +e
  out=$(git_env commit "$@" 2>&1)
  code=$?
  set -e
}

expect_code() {
  if [ "$code" -eq "$2" ]; then
    pass "$1: exit $2"
  else
    fail "$1: expected exit $2, got $code: $out"
  fi
}

expect_log() {
  got=$(cat "$log")
  if [ "$got" = "$2" ]; then
    pass "$1: ran '$(printf '%s' "$2" | tr '\n' ' ')', nothing else"
  else
    fail "$1: expected '$(printf '%s' "$2" | tr '\n' ' ')', got '$(printf '%s' "$got" | tr '\n' ' ')'"
  fi
}

expect_out() {
  if printf '%s\n' "$out" | grep -qF -- "$2"; then
    pass "$1: '$2' in the output"
  else
    fail "$1: no '$2' in: $out"
  fi
}

expect_commits() {
  got=$(git_env rev-list --count HEAD)
  if [ "$got" -eq "$2" ]; then
    pass "$1: $2 commit(s)"
  else
    fail "$1: expected $2 commit(s), got $got"
  fi
}

# --- 1. docs only: nothing runs, the commit goes through ---------------------------------------------
new_repo
with_tsc
stage docs/guide.md README.md
commit -m 'docs: guide'
expect_code 'docs only' 0
expect_log 'docs only' ''
expect_out 'docs only' 'pre-commit: só documentação, nada a checar'
expect_commits 'docs only' 2

# --- 2. TypeScript only: tsc, no cargo ---------------------------------------------------------------
new_repo
with_tsc
stage src/app.tsx package.json
commit -m 'feat(ui): app'
expect_code 'ts only' 0
expect_log 'ts only' 'bun run tsc --noEmit'
expect_out 'ts only' 'pre-commit: tsc --noEmit'
expect_commits 'ts only' 2

# --- 3. Rust only: cargo, no tsc, no node_modules at all ---------------------------------------------
new_repo
stage crates/host/src/lib.rs Cargo.toml
commit -m 'feat(host): lib'
expect_code 'rust only' 0
expect_log 'rust only' 'cargo check --workspace --all-targets'
expect_out 'rust only' 'pre-commit: cargo check --workspace --all-targets'
expect_commits 'rust only' 2

# --- 4. both languages: tsc then cargo ---------------------------------------------------------------
new_repo
with_tsc
stage src/app.tsx crates/host/src/lib.rs
commit -m 'feat: both'
expect_code 'mixed' 0
expect_log 'mixed' 'bun run tsc --noEmit
cargo check --workspace --all-targets'
expect_commits 'mixed' 2

# --- 5. an amend that only changes the message: nothing runs -----------------------------------------
new_repo
with_tsc
stage docs/guide.md
commit -m 'docs: first'
: >"$log"
commit --amend -m 'docs: amended'
expect_code 'amend' 0
expect_log 'amend' ''
expect_commits 'amend' 2

# --- 6. TypeScript without node_modules: fails before bun, says bun install ---------------------------
new_repo
stage src/app.tsx
commit -m 'feat(ui): app'
expect_code 'no node_modules' 1
expect_log 'no node_modules' ''
expect_out 'no node_modules' 'bun install'
if printf '%s\n' "$out" | grep -q 'command not found'; then
  fail 'no node_modules: the output says command not found'
else
  pass 'no node_modules: never command not found'
fi
expect_commits 'no node_modules' 1

# --- 7. docs only without node_modules: passes -------------------------------------------------------
new_repo
stage docs/guide.md
commit -m 'docs: no node_modules'
expect_code 'docs only, no node_modules' 0
expect_log 'docs only, no node_modules' ''
expect_commits 'docs only, no node_modules' 2

# --- 8. scripts/commit-only.sh: Rust dirty in the work tree is not part of the commit ----------------
new_repo docs/guide.md crates/host/src/lib.rs
mkdir -p "$dir/scripts"
cp "$commit_only" "$dir/scripts/commit-only.sh"
chmod +x "$dir/scripts/commit-only.sh"
write_file docs/guide.md
write_file crates/host/src/lib.rs
set +e
out=$(cd "$dir" && env PATH="$scratch/stubs:$PATH" FAKE_LOG="$log" FAKE_BUN_EXIT=0 FAKE_CARGO_EXIT=0 \
  GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null \
  sh scripts/commit-only.sh 'docs: guide' docs/guide.md 2>&1)
code=$?
set -e
expect_code 'commit-only' 0
expect_log 'commit-only' ''
expect_commits 'commit-only' 2
committed=$(git_env show --name-only --format= HEAD)
if printf '%s\n' "$committed" | grep -qx 'docs/guide.md'; then
  pass 'commit-only: the docs file is in the commit'
else
  fail "commit-only: docs/guide.md missing from the commit: $committed"
fi
if printf '%s\n' "$committed" | grep -qx 'crates/host/src/lib.rs'; then
  fail 'commit-only: the dirty Rust file went into the commit'
else
  pass 'commit-only: the dirty Rust file stayed out of the commit'
fi

# --- 9. a failing check: the hook propagates the exit code and the commit is not created -------------
new_repo
stage crates/host/src/lib.rs
cargo_exit=3
commit -m 'feat(host): lib'
expect_log 'failing cargo' 'cargo check --workspace --all-targets'
if [ "$code" -ne 0 ]; then
  pass "failing cargo: exit $code, not 0"
else
  fail "failing cargo: expected a non-zero exit, got 0: $out"
fi
expect_commits 'failing cargo' 1

# --- 10. the hook under dash, the runner's /bin/sh: both checks still run -----------------------------
if ! command -v dash >/dev/null 2>&1; then
  fail 'dash: not on the PATH (brew install dash)'
else
  new_repo
  with_tsc
  stage src/app.tsx crates/host/src/lib.rs
  set +e
  out=$(cd "$dir" && env PATH="$scratch/stubs:$PATH" FAKE_LOG="$log" FAKE_BUN_EXIT=0 FAKE_CARGO_EXIT=0 \
    dash .githooks/pre-commit 2>&1)
  code=$?
  set -e
  expect_code 'dash' 0
  expect_log 'dash' 'bun run tsc --noEmit
cargo check --workspace --all-targets'
fi

echo
if [ "$failures" -eq 0 ]; then
  echo "all cases passed"
else
  echo "$failures failure(s)"
  exit 1
fi
