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

echo "self-check: DMG assembly and verification"
stub="$work/OpenRidr.app"
mkdir -p "$stub/Contents/MacOS"
cp "$(command -v sh)" "$stub/Contents/MacOS/openridr"
stub_plist() {
  cat >"$stub/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>com.openridr.desktop</string>
  <key>CFBundleShortVersionString</key><string>$1</string>
</dict>
</plist>
EOF
}
stub_plist 9.9.9
mkdir -p "$work/run"
outputs="$work/run/github-output"
: >"$outputs"
if ! RUNNER_TEMP=$work/run GITHUB_OUTPUT=$outputs RELEASE_VERSION=9.9.9 \
  sh "$script" --dmg-only --app "$stub" >"$work/out" 2>"$work/err"; then
  cat "$work/err" >&2
  fail "the DMG stage failed on a stub app"
fi
dmg=$(sed -n 's/^dmg=//p' "$work/out")
[ -f "$dmg" ] || fail "no DMG at '$dmg'"
case $dmg in */OpenRidr_9.9.9_aarch64.dmg) ;; *) fail "unexpected DMG name: $dmg" ;; esac
grep -q 'DMG layout verified' "$work/err" || fail "the DMG layout was not verified"
expected_sha=$(shasum -a 256 "$dmg" | awk '{print $1}')
expected_bytes=$(stat -f%z "$dmg")
grep -q "^version=9.9.9$" "$outputs" || fail "GITHUB_OUTPUT has no version"
grep -q "^dmg=$dmg$" "$outputs" || fail "GITHUB_OUTPUT has no dmg"
grep -q "^sha256=$expected_sha$" "$outputs" || fail "GITHUB_OUTPUT has no sha256"
grep -q "^bytes=$expected_bytes$" "$outputs" || fail "GITHUB_OUTPUT has no bytes"

echo "self-check: DMG stage rejects incomplete or mismatched apps"
stub_plist 1.2.3
if RUNNER_TEMP=$work/run RELEASE_VERSION=9.9.9 sh "$script" --dmg-only --app "$stub" \
  >"$work/out" 2>"$work/err"; then
  fail "a wrong app version was accepted"
fi
grep -q "says version '1.2.3'" "$work/err" || fail "wrong version message is not specific: $(cat "$work/err")"
stub_plist 9.9.9
rm -f "$stub/Contents/MacOS/openridr"
if RUNNER_TEMP=$work/run RELEASE_VERSION=9.9.9 sh "$script" --dmg-only --app "$stub" \
  >"$work/out" 2>"$work/err"; then
  fail "an app without Contents/MacOS/openridr was accepted"
fi
grep -q "has no Contents/MacOS/openridr" "$work/err" ||
  fail "missing binary message is not specific: $(cat "$work/err")"
if RUNNER_TEMP=$work/run sh "$script" --dmg-only >"$work/out" 2>"$work/err"; then
  fail "--dmg-only without a version was accepted"
fi
grep -q "needs --tag" "$work/err" || fail "missing version message is not specific"

echo "self-check: credentials accept a keychain profile and require a Developer ID identity"
(
  credentials
  unset APPLE_API_KEY
  NOTARY_KEYCHAIN_PROFILE=notary
  export NOTARY_KEYCHAIN_PROFILE
  expect_ok --check --tag "v$VERSION"
)
(
  credentials
  APPLE_SIGNING_IDENTITY=-
  export APPLE_SIGNING_IDENTITY
  expect_fail "must be 'Developer ID Application:" --check --tag "v$VERSION"
)
(
  credentials
  APPLE_SIGNING_IDENTITY="Developer ID Application: X"
  export APPLE_SIGNING_IDENTITY
  expect_fail "must be 'Developer ID Application:" --check --tag "v$VERSION"
)

echo "self-check: Mach-O audit"
signed_app=/Applications/OpenRidr.app
if [ -d "$signed_app" ]; then
  if ! sh "$script" --audit "$signed_app" >"$work/out" 2>"$work/err"; then
    cat "$work/err" >&2
    fail "the audit rejected the installed Developer ID build"
  fi
  grep -Eq 'audit ok: [1-9][0-9]* Mach-O \([1-9][0-9]* with ' "$work/err" ||
    fail "the audit did not report the Mach-O it checked: $(cat "$work/err")"
  grep -q "createdump from Microsoft" "$work/err" ||
    fail "the audit did not account for createdump: $(cat "$work/err")"
  if APPLE_SIGNING_IDENTITY="Developer ID Application: Outra Pessoa (ZZZZZZZZZZ)" \
    sh "$script" --audit "$signed_app" >"$work/out" 2>"$work/err"; then
    fail "the audit accepted a different identity"
  fi
  grep -q "is not signed with 'Developer ID Application: Outra Pessoa (ZZZZZZZZZZ)'" "$work/err" ||
    fail "wrong identity message is not specific: $(cat "$work/err")"
else
  echo "self-check: not run: no Developer ID-signed app at $signed_app"
fi

echo "self-check: the signing path stops at the audit, before any notarization"
cp "$(command -v sh)" "$stub/Contents/MacOS/openridr"
stub_plist "$VERSION"
rt="$work/run-full"
mkdir -p "$rt"
if credentials >/dev/null 2>&1 && RUNNER_TEMP=$rt RELEASE_APP=$stub \
  APPLE_CERTIFICATE=$canary APPLE_CERTIFICATE_PASSWORD=$canary APPLE_API_KEY=$canary \
  APPLE_API_KEY_ID=$canary APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)" \
  sh "$script" --tag "v$VERSION" >"$work/out" 2>"$work/err"; then
  fail "the ad-hoc stub app passed the audit"
fi
grep -q "no hardened runtime" "$work/err" ||
  fail "the audit did not reject the ad-hoc app on hardening: $(cat "$work/err")"
leftovers=$(ls -A "$rt")
[ -z "$leftovers" ] || fail "the failed run left files in RUNNER_TEMP: $leftovers"

echo "ok: release self-check passed"
