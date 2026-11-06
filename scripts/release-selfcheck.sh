#!/bin/sh
# Self-check for scripts/release-macos.sh --check: every preflight rule must reject on its own and
# the complete case must pass. Runs against a throwaway git checkout with planted version sources,
# so it never touches this repository, the keychain, the network or a real release.
# Usage: sh scripts/release-selfcheck.sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
script="$repo/scripts/release-macos.sh"
work=$(mktemp -d "${TMPDIR:-/tmp}/release-selfcheck.XXXXXX")
trap 'rm -rf "$work"' EXIT INT TERM

VERSION=0.3.0-alpha.11
fail() {
  printf 'self-check failed: %s\n' "$1" >&2
  exit 1
}

[ -f "$script" ] || fail "missing $script"

root="$work/repo"
mkdir -p "$root"
plant() {
  mkdir -p "$(dirname "$1")"
  printf '%s\n' "$2" >"$1"
}

add_version() {
  plant "$root/package.json" "{
  \"name\": \"openridr\",
  \"version\": \"$1\",
  \"private\": true
}"
  plant "$root/src-tauri/tauri.conf.json" "{
  \"productName\": \"OpenRidr\",
  \"version\": \"$1\"
}"
  for crate in src-tauri crates/host crates/host-http; do
    plant "$root/$crate/Cargo.toml" "[package]
name = \"openridr\"
version = \"$1\""
  done
  plant "$root/docs/RELEASE-ALPHA.md" "# OpenRidr $1 (pré-release)

Notas plantadas pelo self-check."
}

rewrite_version() {
  sed "s/$VERSION/9.9.9/" "$1" >"$work/tmp" && mv "$work/tmp" "$1"
}

git_() {
  git -C "$root" -c user.email=selfcheck@example.invalid -c user.name=selfcheck "$@"
}

add_version "$VERSION"
git -C "$root" init -q -b main
git_ add -A
git_ commit -qm "fixture"
git_ tag "v$VERSION"
git_ update-ref refs/remotes/origin/main HEAD

# Fictitious credentials: the canary must never show up in any output.
canary=canary-do-not-print
credentials() {
  APPLE_CERTIFICATE=$canary
  APPLE_CERTIFICATE_PASSWORD=$canary
  APPLE_API_KEY=$canary
  APPLE_API_KEY_ID=$canary
  APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)"
  export APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_API_KEY APPLE_API_KEY_ID \
    APPLE_SIGNING_IDENTITY
}
credentials

run() {
  RELEASE_ROOT=$root sh "$script" "$@"
}

expect_ok() {
  if ! run "$@" >"$work/out" 2>"$work/err"; then
    cat "$work/err" >&2
    fail "expected success: $*"
  fi
  if grep -q "$canary" "$work/out" "$work/err"; then fail "a credential value was printed"; fi
}

expect_fail() {
  expected=$1
  shift
  if run "$@" >"$work/out" 2>"$work/err"; then
    cat "$work/out" >&2
    fail "expected failure: $*"
  fi
  grep -q "$expected" "$work/err" || {
    cat "$work/err" >&2
    fail "failure message did not mention '$expected' for: $*"
  }
  if grep -q "$canary" "$work/out" "$work/err"; then fail "a credential value was printed"; fi
}

echo "self-check: the complete case passes"
expect_ok --check --tag "v$VERSION"

echo "self-check: tag shape"
expect_fail "not vX.Y.Z" --check --tag "$VERSION"
expect_fail "not vX.Y.Z" --check --tag "v01.2.3"
expect_fail "not vX.Y.Z" --check --tag "v1.2"

echo "self-check: version sources"
for source in package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml \
  crates/host/Cargo.toml crates/host-http/Cargo.toml; do
  rewrite_version "$root/$source"
  expect_fail "$source says '9.9.9'" --check --tag "v$VERSION"
  add_version "$VERSION"
done

echo "self-check: notes heading"
plant "$root/docs/RELEASE-ALPHA.md" "# OpenRidr 9.9.9 (pré-release)"
expect_fail "docs/RELEASE-ALPHA.md starts with" --check --tag "v$VERSION"
add_version "$VERSION"
expect_ok --check --tag "v$VERSION"

echo "self-check: the tag must exist, be on main and have origin/main fetched"
git_ checkout -q -b side
add_version 0.3.0-alpha.99
git_ add -A
git_ commit -qm "unmerged release commit"
git_ tag "v0.3.0-alpha.99"
expect_fail "is not on main" --check --tag "v0.3.0-alpha.99"
git_ checkout -q main
expect_fail "does not exist" --check --tag "v0.3.0-alpha.98"
git_ update-ref -d refs/remotes/origin/main
expect_fail "origin/main is not fetched" --check --tag "v$VERSION"
git_ update-ref refs/remotes/origin/main "$(git_ rev-parse main)"
expect_ok --check --tag "v$VERSION"

echo "self-check: credentials are named, never printed"
for name in APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_API_KEY APPLE_API_KEY_ID \
  APPLE_SIGNING_IDENTITY; do
  (
    credentials
    eval "unset $name"
    expect_fail "missing credentials" --check --tag "v$VERSION"
  )
done
(
  credentials
  APPLE_API_ISSUER=
  export APPLE_API_ISSUER
  expect_ok --check --tag "v$VERSION"
)

echo "self-check: tools and architecture"
bin="$work/bin"
mkdir -p "$bin"
for tool in sh sed head dirname git uname grep bun cargo xcrun codesign ditto shasum gh; do
  ln -s "$(command -v "$tool")" "$bin/$tool"
done
restricted() {
  PATH="$bin" RELEASE_ROOT=$root sh "$script" --check --tag "v$VERSION"
}
if restricted >"$work/out" 2>"$work/err"; then fail "missing hdiutil was accepted"; fi
grep -q "missing tools on PATH: hdiutil" "$work/err" ||
  fail "missing tool message is not specific: $(cat "$work/err")"
ln -s "$(command -v hdiutil)" "$bin/hdiutil"
rm -f "$bin/uname"
printf '#!/bin/sh\necho x86_64\n' >"$bin/uname"
chmod +x "$bin/uname"
if restricted >"$work/out" 2>"$work/err"; then fail "non-arm64 architecture was accepted"; fi
grep -q "Apple Silicon (arm64) only" "$work/err" ||
  fail "architecture message is not specific: $(cat "$work/err")"

echo "ok: release self-check passed"
