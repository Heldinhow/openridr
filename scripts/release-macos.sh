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
usage: sh scripts/release-macos.sh [--check | --dry-run | --dmg-only] [--tag vX.Y.Z] [--app APP]

  --check     preflight only: tag, versions, notes heading, credentials, tools, architecture
  --dry-run   preflight (without credentials), build and DMG, stop before signing/notarizing
  --dmg-only  assemble and verify the DMG from an existing app bundle, no build, no signing
  --tag TAG   release tag; defaults to GITHUB_REF_NAME, then the exact tag of HEAD
  --app APP   app bundle to use instead of the built one
EOF
  exit 2
}

fail() {
  printf 'release: %s\n' "$*" >&2
  exit 1
}

# Diagnostics go to stderr so command substitutions can capture only the artifact facts.
note() {
  printf 'release: %s\n' "$*" >&2
}

while [ "$#" -gt 0 ]; do
  case $1 in
    --check) MODE=check ;;
    --dry-run) MODE=dry-run ;;
    --dmg-only) MODE=dmg-only ;;
    --app)
      [ "$#" -ge 2 ] || usage
      RELEASE_APP=$2
      export RELEASE_APP
      shift
      ;;
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

# --- build, DMG and outputs -------------------------------------------------------------------

identity() {
  printf '%s' "${APPLE_SIGNING_IDENTITY:-}"
}

release_version() {
  if [ -n "${RELEASE_VERSION:-}" ]; then
    printf '%s' "$RELEASE_VERSION"
  else
    printf '%s' "${TAG#v}"
  fi
}

resolve_app() {
  if [ -n "${RELEASE_APP:-}" ]; then
    printf '%s' "$RELEASE_APP"
    return 0
  fi
  for candidate in \
    "$ROOT/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/$PRODUCT.app" \
    "$ROOT/src-tauri/target/release/bundle/macos/$PRODUCT.app"; do
    if [ -d "$candidate" ]; then
      printf '%s' "$candidate"
      return 0
    fi
  done
  printf '%s' "$ROOT/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/$PRODUCT.app"
}

build_app() {
  if [ -n "${RELEASE_APP:-}" ] || [ "${RELEASE_SKIP_BUILD:-0}" = 1 ]; then
    app=$(resolve_app)
    [ -d "$app" ] || fail "no app bundle at $app (RELEASE_APP/RELEASE_SKIP_BUILD set)"
    note "using existing app: $app"
    return 0
  fi
  if [ ! -d "$ROOT/node_modules" ]; then
    note "installing JS dependencies (bun install --frozen-lockfile)"
    (cd "$ROOT" && bun install --frozen-lockfile)
  fi
  note "building $PRODUCT ${TAG#v} (aarch64-apple-darwin, --bundles app)"
  (cd "$ROOT" && bun run tauri build --target aarch64-apple-darwin --bundles app)
  app=$(resolve_app)
  [ -d "$app" ] || fail "the build produced no app bundle at $app"
}

dmg_name() {
  printf '%s_%s_aarch64.dmg' "$PRODUCT" "$(release_version)"
}

assemble_dmg() {
  app=$1
  version=$(release_version)
  [ -n "$version" ] || fail "no version to name the DMG (pass --tag or RELEASE_VERSION)"
  [ -d "$app" ] || fail "no app bundle at $app"
  [ -x "$app/Contents/MacOS/openridr" ] || fail "$app has no Contents/MacOS/openridr"
  dmg="$WORK/$(dmg_name)"
  stage=$(mktemp -d "$WORK/release-dmg.XXXXXX")
  rm -f "$dmg"
  ditto "$app" "$stage/$PRODUCT.app"
  ln -s /Applications "$stage/Applications"
  note "assembling $dmg"
  hdiutil create -volname "$PRODUCT" -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$dmg" >/dev/null
  rm -rf "$stage"
  printf '%s' "$dmg"
}

verify_dmg() {
  dmg=$1
  version=$2
  mp=$(mktemp -d "$WORK/release-mnt.XXXXXX")
  hdiutil attach "$dmg" -nobrowse -readonly -mountpoint "$mp" -quiet ||
    fail "cannot attach $dmg"
  (
    app="$mp/$PRODUCT.app"
    [ -d "$app" ] || fail "$dmg has no $PRODUCT.app"
    [ -L "$mp/Applications" ] || fail "$dmg has no Applications symlink"
    [ "$(readlink "$mp/Applications")" = /Applications ] ||
      fail "$dmg Applications symlink does not point to /Applications"
    binary="$app/Contents/MacOS/openridr"
    [ -x "$binary" ] || fail "$dmg app has no Contents/MacOS/openridr"
    file -b "$binary" | grep -q 'Mach-O 64-bit executable arm64' ||
      fail "$binary is not a Mach-O arm64 executable: $(file -b "$binary")"
    got=$(plutil -extract CFBundleShortVersionString raw -o - "$app/Contents/Info.plist")
    [ "$got" = "$version" ] || fail "$dmg app says version '$got', expected '$version'"
  ) && status=0 || status=$?
  hdiutil detach "$mp" -quiet || hdiutil detach "$mp" -force -quiet
  rm -rf "$mp"
  [ "$status" = 0 ] || exit 1
  note "DMG layout verified: $PRODUCT.app, Applications -> /Applications, version $version"
}

artifact_outputs() {
  dmg=$1
  sha=$(shasum -a 256 "$dmg" | awk '{print $1}')
  bytes=$(stat -f%z "$dmg")
  printf 'dmg=%s\n' "$dmg"
  printf 'sha256=%s\n' "$sha"
  printf 'bytes=%s\n' "$bytes"
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    {
      printf 'version=%s\n' "$(release_version)"
      printf 'dmg=%s\n' "$dmg"
      printf 'sha256=%s\n' "$sha"
      printf 'bytes=%s\n' "$bytes"
    } >>"$GITHUB_OUTPUT"
  fi
}

print_plan() {
  ident=$(identity)
  [ -n "$ident" ] || ident='Developer ID Application: <identity from APPLE_SIGNING_IDENTITY>'
  app=$(resolve_app)
  cat <<EOF
release: dry run plan (nothing is signed, notarized or published)
  audit every Mach-O under $app with '$ident' (hardened runtime, timestamp)
  ditto -c -k --keepParent --sequesterRsrc "$app" \$RUNNER_TEMP/$PRODUCT.zip
  xcrun notarytool submit \$RUNNER_TEMP/$PRODUCT.zip --key \$RUNNER_TEMP/AuthKey.p8 --key-id \$APPLE_API_KEY_ID [--issuer \$APPLE_API_ISSUER] --wait
  xcrun stapler staple "$app" && xcrun stapler validate "$app"
  hdiutil create -volname $PRODUCT -srcfolder <stage> -fs HFS+ -format UDZO "\$dmg"
  codesign --force --sign '$ident' --timestamp "\$dmg"
  xcrun notarytool submit "\$dmg" --key ... --wait
  xcrun stapler staple "\$dmg" && xcrun stapler validate "\$dmg"
  codesign --verify --deep --strict "$app"; spctl e stapler no app e no DMG montado
EOF
}

# --- modes ------------------------------------------------------------------------------------

case $MODE in
  check) preflight strict ;;
  dmg-only)
    [ -n "$(release_version)" ] || fail "--dmg-only needs --tag vX.Y.Z or RELEASE_VERSION"
    app=$(resolve_app)
    dmg=$(assemble_dmg "$app")
    verify_dmg "$dmg" "$(release_version)"
    artifact_outputs "$dmg"
    ;;
  dry-run)
    preflight relaxed
    build_app
    print_plan
    dmg=$(assemble_dmg "$(resolve_app)")
    verify_dmg "$dmg" "$(release_version)"
    artifact_outputs "$dmg"
    note "dry run complete: the DMG above is NOT signed, NOT notarized and was not published"
    ;;
  full) fail "the signing path is not implemented yet (#65)" ;;
esac
