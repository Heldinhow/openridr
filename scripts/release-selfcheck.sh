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
expect_fail "heading says '9.9.9'" --check --tag "v$VERSION"
# A version that only starts with the expected one is not the expected one.
plant "$root/docs/RELEASE-ALPHA.md" "# OpenRidr $VERSION-extra (pré-release)"
expect_fail "heading says '$VERSION-extra'" --check --tag "v$VERSION"
plant "$root/docs/RELEASE-ALPHA.md" "# Alguma coisa"
expect_fail "docs/RELEASE-ALPHA.md starts with" --check --tag "v$VERSION"
add_version "$VERSION"
expect_ok --check --tag "v$VERSION"

echo "self-check: the checked-out commit must be the tagged commit"
git_ checkout -q -b later
plant "$root/docs/RELEASE-ALPHA.md" "# OpenRidr $VERSION (pré-release)"
printf 'later\n' >"$root/later.txt"
git_ add -A
git_ commit -qm "commit after the tag"
expect_fail "HEAD is" --check --tag "v$VERSION"
git_ checkout -q main
git_ branch -qD later
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
  unset APPLE_API_KEY APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD
  NOTARY_KEYCHAIN_PROFILE=notary
  export NOTARY_KEYCHAIN_PROFILE
  expect_ok --check --tag "v$VERSION"
)
(
  credentials
  unset APPLE_CERTIFICATE
  expect_fail "missing credentials:.*APPLE_CERTIFICATE" --check --tag "v$VERSION"
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
if credentials >/dev/null 2>&1 && RELEASE_ROOT=$root RUNNER_TEMP=$rt RELEASE_APP=$stub \
  APPLE_CERTIFICATE=$canary APPLE_CERTIFICATE_PASSWORD=$canary APPLE_API_KEY=$canary \
  APPLE_API_KEY_ID=$canary APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)" \
  sh "$script" --tag "v$VERSION" >"$work/out" 2>"$work/err"; then
  fail "the ad-hoc stub app passed the audit"
fi
grep -q "no hardened runtime" "$work/err" ||
  fail "the audit did not reject the ad-hoc app on hardening: $(cat "$work/err")"
leftovers=$(ls -A "$rt")
[ -z "$leftovers" ] || fail "the failed run left files in RUNNER_TEMP: $leftovers"

echo "self-check: the audit verifies each signature, not only its metadata"
broken="$work/Broken.app"
mkdir -p "$broken/Contents/MacOS"
cp "$(command -v sh)" "$broken/Contents/MacOS/openridr"
printf 'x' >>"$broken/Contents/MacOS/openridr"
cp "$stub/Contents/Info.plist" "$broken/Contents/Info.plist"
if APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)" \
  sh "$script" --audit "$broken" >"$work/out" 2>"$work/err"; then
  fail "a binary with a broken signature passed the audit"
fi
grep -q "does not verify with codesign --verify --strict" "$work/err" ||
  fail "broken signature message is not specific: $(cat "$work/err")"

echo "self-check: --dry-run builds ad-hoc even when the environment has an identity"
bin3="$work/bin-build"
mkdir -p "$bin3"
built="$root/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/OpenRidr.app"
plist="$work/Info.plist"
cat >"$plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
</dict>
</plist>
PLIST
cat >"$bin3/bun" <<BUN
#!/bin/sh
printf '%s' "\${APPLE_SIGNING_IDENTITY-unset}" >"$work/build-identity"
mkdir -p "$built/Contents/MacOS"
cp "$(command -v sh)" "$built/Contents/MacOS/openridr"
cp "$plist" "$built/Contents/Info.plist"
exit 0
BUN
chmod +x "$bin3/bun"
mkdir -p "$work/run-dry"
if ! RELEASE_ROOT=$root RUNNER_TEMP=$work/run-dry \
  APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)" \
  PATH="$bin3:$PATH" sh "$script" --dry-run --tag "v$VERSION" >"$work/out" 2>"$work/err"; then
  cat "$work/err" >&2
  fail "the dry run failed against a stub build"
fi
[ -f "$work/build-identity" ] || fail "the stub build did not run"
[ -z "$(cat "$work/build-identity")" ] ||
  fail "the dry run passed APPLE_SIGNING_IDENTITY='$(cat "$work/build-identity")' to the build"
grep -q "signing identity cleared" "$work/err" || fail "the dry run does not say the identity was cleared"
grep -q '^dmg=' "$work/out" || fail "the dry run printed no DMG"
grep -q '^sha256=' "$work/out" || fail "the dry run printed no hash"

echo "self-check: the audit sees every Mach-O magic (thin, fat, fat64) and nothing else"
magicapp="$work/Magic.app"
mkdir -p "$magicapp/Contents/MacOS" "$magicapp/Contents/Frameworks"
magicdir="$magicapp/Contents/MacOS"
# Octal escapes keep this portable between sh implementations.
printf '\317\372\355\376' >"$magicdir/thin64le"
printf '\376\355\372\317' >"$magicdir/thin64be"
printf '\316\372\355\376' >"$magicdir/thin32le"
printf '\376\355\372\316' >"$magicdir/thin32be"
printf '\312\376\272\276\000\000\000\002' >"$magicdir/fat32"
printf '\312\376\272\277\000\000\000\002' >"$magicdir/fat64"
printf '\276\272\376\312\000\000\000\002' >"$magicdir/fat32swapped"
printf '\277\272\376\312\000\000\000\002' >"$magicdir/fat64swapped"
# Java .class shares cafebabe but carries minor/major version (45+) in the second word.
printf '\312\376\272\276\000\000\000\075' >"$magicdir/Java.class"
printf '\000\001\002\003\004\005\006\007' >"$magicdir/data.bin"
binF="$work/bin-fake"
mkdir -p "$binF"
cat >"$binF/codesign" <<'FAKECODESIGN'
#!/bin/sh
case " $* " in
  *" --verify "*)
    if [ -n "${CODESIGN_LOG:-}" ]; then printf '%s\n' "$*" >>"$CODESIGN_LOG"; fi
    exit 0
    ;;
  *" -dv "*)
    printf 'flags=0x10000(runtime)\nAuthority=%s\nTimestamp=selftest\n' "${APPLE_SIGNING_IDENTITY}"
    exit 0
    ;;
esac
exit 0
FAKECODESIGN
chmod +x "$binF/codesign"
log="$work/codesign.log"
: >"$log"
CODESIGN_LOG=$log APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)" \
  PATH="$binF:$PATH" sh "$script" --audit "$magicapp" >"$work/out" 2>"$work/err" ||
  { cat "$work/err" >&2; fail "the audit rejected the synthetic Mach-O set"; }
for name in thin64le thin64be thin32le thin32be fat32 fat64 fat32swapped fat64swapped; do
  grep -q "/$name\$" "$log" || fail "the audit ignored $name (fat64 included)"
done
for name in Java.class data.bin; do
  if grep -q "/$name\$" "$log"; then fail "the audit treated $name as Mach-O"; fi
done
if grep -q "audit ok: 8 Mach-O" "$work/err"; then :; else
  fail "the audit did not report the eight synthetic binaries: $(cat "$work/err")"
fi

echo "self-check: the notary key never survives the run (success and rejection)"
cat >"$binF/xcrun" <<'FAKEXCRUN'
#!/bin/sh
# Fake xcrun: notarytool and stapler only.
case $1 in
  notarytool)
    case $2 in
      submit)
        if [ "${NOTARY_MODE:-ok}" = reject ]; then
          printf '{"id":"fake-1","status":"Invalid"}\n'
        else
          printf '{"id":"fake-1","status":"Accepted"}\n'
        fi
        ;;
      log) printf '{"id":"fake-1","status":"Invalid"}\n' ;;
    esac
    ;;
  stapler) : ;;
esac
exit 0
FAKEXCRUN
cat >"$binF/spctl" <<'FAKESPCTL'
#!/bin/sh
echo "selftest: accepted"
exit 0
FAKESPCTL
chmod +x "$binF/xcrun" "$binF/spctl"
cp "$(command -v sh)" "$stub/Contents/MacOS/openridr"
stub_plist "$VERSION"
release_key=$(printf 'selftest-notary-key' | openssl base64 -A)
# Anything the notarization leaves in RUNNER_TEMP: the key and the transport zip.
leftovers_in() {
  found=
  for candidate in "$1"/AuthKey_* "$1"/*notary*; do
    if [ -e "$candidate" ]; then found="$found $candidate"; fi
  done
  printf '%s' "$found"
}
run_signed() {
  mode=$1
  home=$2
  mkdir -p "$home"
  NOTARY_MODE=$mode RELEASE_ROOT=$root RUNNER_TEMP=$home RELEASE_APP=$stub \
    APPLE_CERTIFICATE=$canary APPLE_CERTIFICATE_PASSWORD=$canary \
    APPLE_API_KEY=$release_key APPLE_API_KEY_ID=SELFTEST123 \
    APPLE_SIGNING_IDENTITY="Developer ID Application: Helder Santos (K6M85697D4)" \
    PATH="$binF:$PATH" sh "$script" --tag "v$VERSION"
}
ok_home="$work/run-signed-ok"
if ! run_signed ok "$ok_home" >"$work/out" 2>"$work/err"; then
  cat "$work/err" >&2
  fail "the signed path failed with a fake notarytool"
fi
grep -q '^dmg=' "$work/out" || fail "the signed path produced no DMG"
grep -q "notarization accepted: OpenRidr-notary" "$work/err" ||
  fail "the app was not sent to the (fake) notary: $(cat "$work/err")"
leftovers=$(leftovers_in "$ok_home")
[ -z "$leftovers" ] || fail "after a successful notarization these stayed behind: $leftovers"

bad_home="$work/run-signed-reject"
if run_signed reject "$bad_home" >"$work/out" 2>"$work/err"; then
  fail "a rejected notarization was accepted"
fi
grep -q "Apple rejected" "$work/err" || fail "the rejection is not reported: $(cat "$work/err")"
leftovers=$(leftovers_in "$bad_home")
[ -z "$leftovers" ] || fail "after a rejection these stayed behind: $leftovers"

echo "ok: release self-check passed"
