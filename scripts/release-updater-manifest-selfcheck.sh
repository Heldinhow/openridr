#!/bin/sh
# Self-check for scripts/release-updater-manifest.sh: the manifest must carry the version, the
# notes, the archive URL and a real signature, and must refuse anything an installed app would
# reject later. Fixtures only: never reads the repository's release notes, never signs anything.
# Usage: sh scripts/release-updater-manifest-selfcheck.sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
script="$repo/scripts/release-updater-manifest.sh"
work=$(mktemp -d "${TMPDIR:-/tmp}/release-updater-manifest-selfcheck.XXXXXX")
trap 'rm -rf "$work"' EXIT INT TERM

version=0.3.0-alpha.12
url="https://github.com/Heldinhow/openridr/releases/download/updater-alpha/OpenRidr.app.tar.gz"
notes="$work/notes.md"
sig="$work/OpenRidr.app.tar.gz.sig"
fail() {
  printf 'self-check failed: %s\n' "$1" >&2
  exit 1
}

[ -f "$script" ] || fail "missing $script"

# A minisign signature box, base64 encoded the way `tauri signer sign` writes the `.sig`.
printf 'untrusted comment: signature from tauri secret key\nRWQfake\n' |
  openssl base64 -A >"$sig"
printf '# OpenRidr %s (pré-release)\n\nNotas plantadas.\n' "$version" >"$notes"

run() {
  sh "$script" "$@" >"$work/out" 2>"$work/err"
}

expect_ok() {
  if ! run "$@"; then
    cat "$work/err" >&2
    fail "expected success: $*"
  fi
}

expect_fail() {
  expected=$1
  shift
  if run "$@"; then
    cat "$work/out" >&2
    fail "expected failure: $*"
  fi
  grep -q "$expected" "$work/err" || {
    cat "$work/err" >&2
    fail "failure message did not mention '$expected' for: $*"
  }
}

echo "self-check: a valid manifest"
expect_ok "$version" "$url" "$sig" "$notes"
jq -e --arg version "$version" --arg url "$url" '
  .version == $version and
  .platforms["darwin-aarch64"].url == $url and
  (.platforms["darwin-aarch64"].signature | length > 0) and
  (.notes | contains("Notas plantadas.")) and
  (.pub_date | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$"))
' "$work/out" >/dev/null || fail "manifest is not the expected shape"
# The signature travels verbatim: the app base64-decodes this value and verifies it.
if [ "$(jq -r '.platforms["darwin-aarch64"].signature' "$work/out")" != "$(cat "$sig")" ]; then
  fail "the signature was not copied verbatim"
fi

echo "self-check: rejections"
expect_fail "is not semver" "v$version" "$url" "$sig" "$notes"
expect_fail "is not semver" "0.3" "$url" "$sig" "$notes"
expect_fail "is not https" "$version" "http://example.com/OpenRidr.app.tar.gz" "$sig" "$notes"
expect_fail "missing signature file" "$version" "$url" "$work/absent.sig" "$notes"
: >"$work/empty.sig"
expect_fail "is empty" "$version" "$url" "$work/empty.sig" "$notes"
printf 'not a signature\n' >"$work/plain.sig"
expect_fail "is not a base64 minisign signature" "$version" "$url" "$work/plain.sig" "$notes"
expect_fail "missing notes file" "$version" "$url" "$sig" "$work/absent.md"

echo "self-check: usage"
if run "$version" "$url"; then fail "two arguments exited 0"; fi
grep -q 'usage:' "$work/err" || fail "missing usage message"

echo "ok: release-updater-manifest self-check passed"
