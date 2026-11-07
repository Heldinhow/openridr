#!/bin/sh
# Exercises every rule of scripts/ci-risk-tier.sh against synthetic git repositories, in the style of
# scripts/ci-pr-body-selfcheck.sh: one case per line of output, non-zero exit on the first mismatch.
#
# The script under test reads git, so each case is a real commit in a throwaway repository: nothing
# outside its own mktemp directory is touched.
#
# Usage: sh scripts/ci-risk-tier-selfcheck.sh
set -eu

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
tier="$here/ci-risk-tier.sh"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-risk-tier.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
repo="$scratch/repo"
mkdir -p "$repo"

git -C "$repo" init -q
git -C "$repo" config user.name fixture
git -C "$repo" config user.email fixture@example.com
git -C "$repo" config commit.gpgsign false

cases=0
failures=0

write() { # <path>; content on stdin
  path=$1
  mkdir -p "$repo/$(dirname "$path")"
  cat >"$repo/$path"
}

commit() { # <message>
  git -C "$repo" add -A
  git -C "$repo" commit -qm "$1"
}

check() { # <name> <expected tier>
  name=$1
  want=$2
  cases=$((cases + 1))
  set +e
  got=$(cd "$repo" && sh "$tier" HEAD~1 HEAD 2>"$scratch/$name.err")
  code=$?
  set -e
  if [ "$code" -ne 0 ] || [ "$got" != "$want" ]; then
    echo "FAIL $name: expected $want, got ${got:-<nothing>} (exit $code)"
    sed 's/^/    /' "$scratch/$name.err"
    failures=$((failures + 1))
  else
    printf 'ok   %-26s %s\n' "$name" "$got"
  fi
}

touch_case() { # <name> <expected> <path>...
  name=$1
  want=$2
  shift 2
  for path in "$@"; do
    printf 'touched\n' >>"$repo/$path"
  done
  commit "$name"
  check "$name" "$want"
}

write_case() { # <name> <expected> <path>; content on stdin
  name=$1
  want=$2
  path=$3
  write "$path"
  commit "$name"
  check "$name" "$want"
}

# --- seed: every file the cases modify exists before the first measured commit -----------------------

for path in \
  README.md AGENTS.md docs/SPEC.md docs/agents/issue-tracker.md \
  src/App.tsx src/app.css \
  crates/host/src/lib.rs crates/host/src/trusted.rs crates/host/tests/x.rs \
  tests/e2e/foo.spec.ts \
  .github/workflows/ci.yml .github/rulesets/main.json \
  scripts/release-macos.sh scripts/vendor-netcoredbg.sh scripts/ci-pr-body.sh \
  src-tauri/tauri.conf.json src-tauri/resources/dotnet.entitlements \
  tsconfig.json index.html rust-toolchain.toml Cargo.lock bun.lock \
  fixtures/OrderDesk/Program.cs; do
  printf 'seed %s\n' "$path" | write "$path"
done

write Cargo.toml <<'TOML'
[package]
name = "fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
TOML

write src-tauri/Cargo.toml <<'TOML'
[package]
name = "fixture-tauri"
version = "0.1.0"

[dependencies]
tauri = "2"
TOML

write package.json <<'JSON'
{
  "name": "fixture",
  "private": true,
  "scripts": {
    "build": "vite build"
  },
  "dependencies": {
    "react": "^18.3.1"
  }
}
JSON

commit seed

# --- one path per family ---------------------------------------------------------------------------

touch_case docs-only low docs/SPEC.md
touch_case root-readme low README.md
touch_case agents-md high AGENTS.md
touch_case stylesheet low src/app.css
touch_case src-tsx medium src/App.tsx
touch_case crate-source medium crates/host/src/lib.rs
touch_case crate-tests low crates/host/tests/x.rs
touch_case e2e-tests low tests/e2e/foo.spec.ts
touch_case workflow high .github/workflows/ci.yml
touch_case ruleset high .github/rulesets/main.json
touch_case release-script high scripts/release-macos.sh
touch_case vendor-script high scripts/vendor-netcoredbg.sh
touch_case ci-script high scripts/ci-pr-body.sh
touch_case entitlements high src-tauri/resources/dotnet.entitlements
touch_case tauri-conf high src-tauri/tauri.conf.json
touch_case trust-model high crates/host/src/trusted.rs
touch_case fixtures medium fixtures/OrderDesk/Program.cs
touch_case toolchain medium rust-toolchain.toml
touch_case unknown-tsconfig medium tsconfig.json
touch_case unknown-html medium index.html

# --- mixed pull requests take the highest tier ------------------------------------------------------

touch_case mixed-docs-src medium docs/SPEC.md src/App.tsx
touch_case mixed-src-workflow high src/App.tsx .github/workflows/ci.yml
touch_case mixed-low-tests low docs/SPEC.md tests/e2e/foo.spec.ts
touch_case mixed-agents-docs high AGENTS.md docs/SPEC.md

# The reasons on stderr are what a human reads when the gate says no.
cases=$((cases + 1))
if grep -qF '.github/workflows/ci.yml (CI workflow' "$scratch/mixed-src-workflow.err" &&
  grep -qF 'src/App.tsx (product code)' "$scratch/mixed-src-workflow.err"; then
  printf 'ok   %-26s %s\n' reasons-per-path 'named'
else
  echo 'FAIL reasons-per-path: the reasons do not name each path'
  sed 's/^/    /' "$scratch/mixed-src-workflow.err"
  failures=$((failures + 1))
fi

# --- dependency changes: lockfiles, dependency tables, and everything else --------------------------

touch_case lockfile-cargo high Cargo.lock
touch_case lockfile-bun high bun.lock

write_case manifest-added high crates/new/Cargo.toml <<'TOML'
[package]
name = "new"
version = "0.1.0"
TOML

write_case cargo-dep-add high Cargo.toml <<'TOML'
[package]
name = "fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
serde_json = "1"
TOML

write_case cargo-dep-remove high Cargo.toml <<'TOML'
[package]
name = "fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
TOML

write_case cargo-version-bump medium Cargo.toml <<'TOML'
[package]
name = "fixture"
version = "0.2.0"
edition = "2021"

[dependencies]
serde = "1"
TOML

write_case tauri-cargo-dep-add high src-tauri/Cargo.toml <<'TOML'
[package]
name = "fixture-tauri"
version = "0.1.0"

[dependencies]
tauri = "2"
tauri-plugin-updater = "2"
TOML

write_case pkg-dependency-add high package.json <<'JSON'
{
  "name": "fixture",
  "private": true,
  "scripts": {
    "build": "vite build"
  },
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  }
}
JSON

write_case pkg-dependency-reorder medium package.json <<'JSON'
{
  "name": "fixture",
  "private": true,
  "scripts": {
    "build": "vite build"
  },
  "dependencies": {
    "react-dom": "^18.3.1",
    "react": "^18.3.1"
  }
}
JSON

write_case pkg-scripts-only medium package.json <<'JSON'
{
  "name": "fixture",
  "private": true,
  "scripts": {
    "build": "vite build --mode production"
  },
  "dependencies": {
    "react-dom": "^18.3.1",
    "react": "^18.3.1"
  }
}
JSON

write_case pkg-dependency-version high package.json <<'JSON'
{
  "name": "fixture",
  "private": true,
  "scripts": {
    "build": "vite build --mode production"
  },
  "dependencies": {
    "react-dom": "^19.0.0",
    "react": "^18.3.1"
  }
}
JSON

# --- dependency tables the extractor must not mistake for "no change" --------------------------------

# A target-specific dependency subtable carries a dependency too: `[target.'cfg(...)'.dependencies.foo]`
# used to be dropped, so adding a dependency there read as "no dependency change".
write_case cargo-target-dep high Cargo.toml <<'TOML'
[package]
name = "fixture"
version = "0.2.0"
edition = "2021"

[dependencies]
serde = "1"

[target.'cfg(unix)'.dependencies.libc]
version = "0.2"
TOML

# A multiline value whose continuation starts with `[` is not a table header. An extractor that cannot
# tell them apart drops the rest of the table; the guard makes it fail closed instead.
write_case cargo-multiline-array high Cargo.toml <<'TOML'
[package]
name = "fixture"
version = "0.2.0"
edition = "2021"

[dependencies]
serde = "1"
libc = "0.2"
matrix = [
  ["unix", "windows"],
]
TOML

# --- the manifest is compared at the merge base, not at the base tip --------------------------------

# Both branches add the same dependency: against the base *tip* the two sides would agree and the change
# would read as "no dependency change". The pull request still introduced it, so it is high.
diverged_manifest() {
  name=$1
  want=$2
  base_ref=$3
  head_ref=$4
  cases=$((cases + 1))
  set +e
  got=$(cd "$repo" && sh "$tier" "$base_ref" "$head_ref" 2>"$scratch/$name.err")
  code=$?
  set -e
  if [ "$code" -ne 0 ] || [ "$got" != "$want" ]; then
    echo "FAIL $name: expected $want, got ${got:-<nothing>} (exit $code)"
    sed 's/^/    /' "$scratch/$name.err"
    failures=$((failures + 1))
  else
    printf 'ok   %-26s %s\n' "$name" "$got"
  fi
}

# Two branches that each add the same dependency, on top of the same ancestor.
git -C "$repo" checkout -q -b diverged-base
write Cargo.toml <<'TOML'
[package]
name = "fixture"
version = "0.2.0"
edition = "2021"

[dependencies]
serde = "1"
libc = "0.2"
rand = "0.8"
TOML
commit diverged-base-adds-rand
git -C "$repo" checkout -q -
write Cargo.toml <<'TOML'
[package]
name = "fixture"
version = "0.2.0"
edition = "2021"

[dependencies]
serde = "1"
libc = "0.2"
rand = "0.8"
TOML
commit pull-request-adds-rand
diverged_manifest cargo-same-dependency-both-sides high diverged-base HEAD

# A change that lives only inside a value the extractor cannot classify: identical extracted tables, so
# only the fail-closed guard catches it.
git -C "$repo" checkout -q -b unparseable-base
write Cargo.toml <<'TOML'
[package]
name = "fixture"
version = "0.2.0"
edition = "2021"

[dependencies]
serde = "1"
matrix = [
["a"],
]
TOML
commit unparseable-base
git -C "$repo" checkout -q -
write Cargo.toml <<'TOML'
[package]
name = "fixture"
version = "0.2.0"
edition = "2021"

[dependencies]
serde = "1"
matrix = [
["b"],
]
TOML
commit pull-request-changes-unparseable-value
diverged_manifest cargo-unparseable-value high unparseable-base HEAD

# --- errors: a caller bug is never "low" ------------------------------------------------------------

expect_exit() { # <name> <expected exit> <args>...
  name=$1
  want=$2
  shift 2
  cases=$((cases + 1))
  set +e
  (cd "$repo" && sh "$tier" "$@") >"$scratch/$name.out" 2>"$scratch/$name.err"
  code=$?
  set -e
  if [ "$code" -ne "$want" ]; then
    echo "FAIL $name: expected exit $want, got $code"
    sed 's/^/    /' "$scratch/$name.err"
    failures=$((failures + 1))
  else
    printf 'ok   %-26s exit %s\n' "$name" "$code"
  fi
}

expect_exit err-no-arguments 2
expect_exit err-unknown-base 2 no-such-ref HEAD
expect_exit err-empty-diff 2 HEAD HEAD

if [ "$failures" -ne 0 ]; then
  echo "ci-risk-tier self-check: $failures of $cases cases failed"
  exit 1
fi

echo "ci-risk-tier self-check: $cases cases passed"
