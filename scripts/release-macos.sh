#!/bin/sh
# Release pipeline for OpenRidr (macOS arm64): one script for both the manual release and the
# GitHub Actions workflow, so the published artifact is the same code path either way.
#
#   sh scripts/release-macos.sh --check [--tag vX.Y.Z]   preflight only, no side effects
#   sh scripts/release-macos.sh --dry-run                build + DMG, stop before signing
#   sh scripts/release-macos.sh                          full path: build, sign, notarize, staple
#
# Credentials come from the environment and are never printed, written to the repo or passed as
# command-line values (the notary API key is passed as a path):
#   APPLE_SIGNING_IDENTITY (repo variable), APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD,
#   APPLE_API_KEY (base64 of the .p8), APPLE_API_KEY_ID, APPLE_API_ISSUER (Team API keys only).
# Temporary work stays in $RUNNER_TEMP (default: ${TMPDIR:-/tmp}).
set -eu

ROOT=${RELEASE_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)}
WORK=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
PRODUCT=OpenRidr
BUNDLE_ID=com.openridr.desktop

MODE=full
TAG=

# Names only: the preflight reports what is missing, never a value.
REQUIRED_CREDENTIALS="APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_API_KEY APPLE_API_KEY_ID"
REQUIRED_IDENTITY=APPLE_SIGNING_IDENTITY
REQUIRED_TOOLS="bun cargo xcrun codesign hdiutil ditto shasum gh"

usage() {
  cat >&2 <<'EOF'
usage: sh scripts/release-macos.sh [--check | --dry-run] [--tag vX.Y.Z]

  --check     preflight only: tag, versions, notes heading, credentials, tools, architecture
  --dry-run   preflight (without credentials), build and DMG, stop before signing/notarizing
  --tag TAG   release tag; defaults to GITHUB_REF_NAME, then the exact tag of HEAD
EOF
  exit 2
}

fail() {
  printf 'release: %s\n' "$*" >&2
  exit 1
}

note() {
  printf 'release: %s\n' "$*"
}

while [ "$#" -gt 0 ]; do
  case $1 in
    --check) MODE=check ;;
    --dry-run) MODE=dry-run ;;
    --tag)
      [ "$#" -ge 2 ] || usage
      TAG=$2
      shift
      ;;
    -h | --help) usage ;;
    *) usage ;;
  esac
  shift
done

# --- preflight -------------------------------------------------------------------------------

resolve_tag() {
  if [ -n "$TAG" ]; then
    return 0
  fi
  if [ -n "${GITHUB_REF_NAME:-}" ]; then
    TAG=$GITHUB_REF_NAME
    return 0
  fi
  TAG=$(git -C "$ROOT" describe --tags --exact-match 2>/dev/null) ||
    fail "no tag: pass --tag vX.Y.Z, set GITHUB_REF_NAME or check out the release tag"
}

json_version() {
  sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$1" | head -n 1
}

cargo_version() {
  sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$1" | head -n 1
}

check_tag() {
  case $TAG in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    *) fail "tag '$TAG' is not vX.Y.Z (prerelease suffixes like -alpha.12 are allowed)" ;;
  esac
  # Strict semver shape: reject v1.2.3foo, v01.2.3, empty or trailing prerelease parts.
  printf '%s\n' "$TAG" |
    grep -Eq '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$' ||
    fail "tag '$TAG' is not vX.Y.Z (prerelease suffixes like -alpha.12 are allowed)"
}

check_versions() {
  expected=${TAG#v}
  for source in package.json src-tauri/tauri.conf.json; do
    file="$ROOT/$source"
    [ -f "$file" ] || fail "missing version source $source"
    got=$(json_version "$file")
    [ "$got" = "$expected" ] || fail "$source says '$got' but the tag says '$expected'"
  done
  for source in src-tauri/Cargo.toml crates/host/Cargo.toml crates/host-http/Cargo.toml; do
    file="$ROOT/$source"
    [ -f "$file" ] || fail "missing version source $source"
    got=$(cargo_version "$file")
    [ "$got" = "$expected" ] || fail "$source says '$got' but the tag says '$expected'"
  done
}

check_tag_on_main() {
  git -C "$ROOT" rev-parse --git-dir >/dev/null 2>&1 ||
    fail "$ROOT is not a git checkout: cannot verify the tag is on main"
  git -C "$ROOT" rev-parse -q --verify "refs/tags/$TAG" >/dev/null ||
    fail "tag $TAG does not exist in this checkout (create it before releasing)"
  git -C "$ROOT" rev-parse -q --verify origin/main >/dev/null ||
    fail "origin/main is not fetched; run 'git fetch origin main' so the tag can be checked against main"
  git -C "$ROOT" merge-base --is-ancestor "refs/tags/$TAG^{commit}" origin/main ||
    fail "tag $TAG is not on main: release only commits that are already merged"
}

check_notes_heading() {
  file="$ROOT/docs/RELEASE-ALPHA.md"
  [ -f "$file" ] || fail "missing docs/RELEASE-ALPHA.md (release notes)"
  heading=$(sed -n '1p' "$file")
  case $heading in
    "# $PRODUCT ${TAG#v}"*) ;;
    *) fail "docs/RELEASE-ALPHA.md starts with '$heading'; expected '# $PRODUCT ${TAG#v}'" ;;
  esac
}

check_credentials() {
  missing=
  for name in $REQUIRED_CREDENTIALS $REQUIRED_IDENTITY; do
    eval "value=\${$name:-}"
    [ -n "$value" ] || missing="$missing $name"
  done
  [ -z "$missing" ] ||
    fail "missing credentials:$missing (names only; values belong in GitHub secrets/variables)"
}

check_tools() {
  missing=
  for tool in $REQUIRED_TOOLS; do
    command -v "$tool" >/dev/null 2>&1 || missing="$missing $tool"
  done
  [ -z "$missing" ] || fail "missing tools on PATH:$missing"
}

check_arch() {
  arch=$(uname -m)
  [ "$arch" = arm64 ] || fail "uname -m is '$arch'; the release targets Apple Silicon (arm64) only"
}

preflight() {
  strict=$1
  resolve_tag
  check_tag
  # The tag must exist and be merged before anything else: a stray tag is not a release.
  check_tag_on_main
  check_versions
  check_notes_heading
  [ "$strict" = relaxed ] || check_credentials
  check_tools
  check_arch
  note "preflight ok: $TAG (${TAG#v}), architecture arm64"
  if [ "$strict" = relaxed ]; then
    note "preflight relaxed: credentials not required (dry run)"
  fi
  return 0
}

case $MODE in
  check) preflight strict ;;
  dry-run) fail "--dry-run is not implemented yet (tickets-release-ci/03)" ;;
  full) fail "the signing path is not implemented yet (tickets-release-ci/04)" ;;
esac
