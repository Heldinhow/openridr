#!/bin/sh
# Self-check for scripts/release-notes.sh: the release body must carry the human notes plus the
# artifact's real hash, and must reject anything that would publish a stale or malformed block.
# Fixtures only: never reads the repository's release notes and never talks to GitHub.
# Usage: sh scripts/release-notes-selfcheck.sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
script="$repo/scripts/release-notes.sh"
work=$(mktemp -d "${TMPDIR:-/tmp}/release-notes-selfcheck.XXXXXX")
trap 'rm -rf "$work"' EXIT INT TERM

version=0.3.0-alpha.11
hash=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
notes="$work/notes.md"
fail() {
  printf 'self-check failed: %s\n' "$1" >&2
  exit 1
}

[ -f "$script" ] || fail "missing $script"

write_notes() {
  printf '# OpenRidr %s (pré-release)\n\n%s\n\n## Instalar\n\n1. Abra o DMG.\n' "$1" "${2:-Notas plantadas.}" >"$notes"
}
write_notes "$version"

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

echo "self-check: notes and generated block"
expect_ok "$version" "$hash" 12345 "$notes"
grep -q 'Notas plantadas.' "$work/out" || fail "human notes are missing from the body"
grep -q '^## Verificação$' "$work/out" || fail "generated block is missing"
grep -q -- "- DMG: \`OpenRidr_${version}_aarch64.dmg\` — 12345 bytes" "$work/out" ||
  fail "DMG line is not the expected one"
grep -q -- "- SHA-256: \`$hash\`" "$work/out" || fail "hash line is not the expected one"
grep -q "^$hash  OpenRidr_${version}_aarch64.dmg$" "$work/out" ||
  fail "shasum-compatible line is missing"
grep -q 'shasum -a 256' "$work/out" || fail "verification command is missing"
if grep -q 'codesign:' "$work/out"; then fail "verification lines printed without results"; fi

echo "self-check: verification results appear only when provided"
RELEASE_VERIFY_CODESIGN='app e DMG ok' RELEASE_VERIFY_SPCTL='Notarized Developer ID' \
  RELEASE_VERIFY_STAPLER='app e DMG validados' \
  run "$version" "$hash" 12345 "$notes" || fail "expected success with verification results"
grep -q -- '- codesign: app e DMG ok' "$work/out" || fail "codesign result missing"
grep -q -- '- spctl: Notarized Developer ID' "$work/out" || fail "spctl result missing"
grep -q -- '- stapler: app e DMG validados' "$work/out" || fail "stapler result missing"

echo "self-check: rejections"
write_notes 9.9.9
expect_fail "expected '$version' (exact)" "$version" "$hash" 12345 "$notes"
# A longer prerelease that starts with the expected version is not the same version.
write_notes "$version-extra"
expect_fail "heading says '$version-extra'" "$version" "$hash" 12345 "$notes"
write_notes "$version"
expect_fail "not 64 lowercase hex" "$version" abc 12345 "$notes"
expect_fail "not 64 lowercase hex" "$version" "$(printf '%s' "$hash" | tr 'a-f' 'A-F')" 12345 "$notes"
expect_fail "not a positive integer" "$version" "$hash" 0 "$notes"
expect_fail "not a positive integer" "$version" "$hash" 12.5 "$notes"
expect_fail "missing notes file" "$version" "$hash" 12345 "$work/absent.md"
write_notes "$version" "Hash copiado à mão: $hash"
expect_fail "already contains a SHA-256" "$version" "$hash" 12345 "$notes"
# Uppercase hex is still a hand-written hash.
write_notes "$version" "Hash copiado à mão: $(printf '%s' "$hash" | tr 'a-f' 'A-F')"
expect_fail "already contains a SHA-256" "$version" "$hash" 12345 "$notes"
write_notes "$version"

echo "self-check: usage"
if run "$version" "$hash"; then fail "two arguments exited 0"; fi
grep -q 'usage:' "$work/err" || fail "missing usage message"

echo "ok: release-notes self-check passed"
