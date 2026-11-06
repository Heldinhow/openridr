#!/bin/sh
# Release pipeline for OpenRidr (macOS arm64): one script for both the manual release and the
# GitHub Actions workflow, so the published artifact is the same code path either way.
#
#   sh scripts/release-macos.sh --check [--tag vX.Y.Z]   preflight only, no side effects
#   sh scripts/release-macos.sh --dry-run                build + DMG, stop before signing
#   sh scripts/release-macos.sh --dmg-only --app <app>   assemble and verify a DMG, no signing
#   sh scripts/release-macos.sh --audit <app>            only the Mach-O signature audit
#   sh scripts/release-macos.sh                          full path: build, sign, notarize, staple
#
# Credentials come from the environment and are never printed, written to the repo or passed as
# command-line values (the notary API key is passed as a path):
#   APPLE_SIGNING_IDENTITY (repo variable), APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD,
#   APPLE_API_KEY (base64 of the .p8), APPLE_API_KEY_ID, APPLE_API_ISSUER (Team API keys only).
# On a Mac with the `notary` keychain profile, NOTARY_KEYCHAIN_PROFILE replaces the API key trio.
# Temporary work stays in $RUNNER_TEMP (default: ${TMPDIR:-/tmp}).
set -eu

ROOT=${RELEASE_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)}
WORK=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
PRODUCT=OpenRidr

MODE=full
TAG=
AUDIT_APP=
# The private .NET runtime ships one binary signed by Microsoft; everything else is ours.
MICROSOFT_IDENTITY="Developer ID Application: Microsoft Corporation (UBF8T346G9)"
NOTARY_KEY_FILE=
NOTARY_ZIP=

# Names only: the preflight reports what is missing, never a value.
REQUIRED_CERTIFICATE="APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD"
REQUIRED_IDENTITY=APPLE_SIGNING_IDENTITY
REQUIRED_TOOLS="bun cargo xcrun codesign hdiutil ditto shasum gh"

usage() {
  cat >&2 <<'EOF'
usage: sh scripts/release-macos.sh [--check | --dry-run | --dmg-only | --audit APP] [--tag vX.Y.Z]
                                   [--app APP]

  --check     preflight only: tag, versions, notes heading, credentials, tools, architecture
  --dry-run   preflight (without credentials), build and DMG, stop before signing/notarizing
  --dmg-only  assemble and verify the DMG from an existing app bundle, no build, no signing
  --audit APP audit every Mach-O of APP (our Developer ID, hardened runtime, timestamp)
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
    --audit)
      [ "$#" -ge 2 ] || usage
      MODE=audit
      AUDIT_APP=$2
      shift
      ;;
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
  # What gets built must be what the tag points at: a checked-out branch is not a release.
  head=$(git -C "$ROOT" rev-parse HEAD)
  tagged=$(git -C "$ROOT" rev-parse "refs/tags/$TAG^{commit}")
  [ "$head" = "$tagged" ] ||
    fail "HEAD is $head but tag $TAG points at $tagged; check out the tag before releasing"
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
    "# $PRODUCT "*)
      rest=${heading#"# $PRODUCT "}
      heading_version=${rest%% *}
      [ "$heading_version" = "${TAG#v}" ] ||
        fail "docs/RELEASE-ALPHA.md heading says '$heading_version'; expected ${TAG#v} (exact)"
      ;;
    *) fail "docs/RELEASE-ALPHA.md starts with '$heading'; expected '# $PRODUCT ${TAG#v}'" ;;
  esac
}

check_credentials() {
  missing=
  if [ -z "${APPLE_API_KEY:-}" ] && [ -z "${NOTARY_KEYCHAIN_PROFILE:-}" ]; then
    missing="$missing APPLE_API_KEY(or NOTARY_KEYCHAIN_PROFILE)"
  fi
  if [ -n "${APPLE_API_KEY:-}" ] && [ -z "${APPLE_API_KEY_ID:-}" ]; then
    missing="$missing APPLE_API_KEY_ID"
  fi
  # CI imports the certificate from secrets; a Mac using the notary keychain profile already has it
  # in the login keychain, so the .p12 variables are not its business.
  if [ -z "${NOTARY_KEYCHAIN_PROFILE:-}" ]; then
    for name in $REQUIRED_CERTIFICATE; do
      eval "value=\${$name:-}"
      [ -n "$value" ] || missing="$missing $name"
    done
  fi
  for name in $REQUIRED_IDENTITY; do
    eval "value=\${$name:-}"
    [ -n "$value" ] || missing="$missing $name"
  done
  [ -z "$missing" ] ||
    fail "missing credentials:$missing (names only; values belong in GitHub secrets/variables)"
  case ${APPLE_SIGNING_IDENTITY} in
    "Developer ID Application: "*" ("*")") ;;
    *) fail "APPLE_SIGNING_IDENTITY must be 'Developer ID Application: <name> (<TEAMID>)', got a value that is not a Developer ID identity" ;;
  esac
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
  ident=$(identity)
  note "building $PRODUCT ${TAG#v} (aarch64-apple-darwin, --bundles app, identity ${ident:-ad-hoc (-)})"
  # The identity is passed explicitly so a dry run can clear it and stay ad-hoc (the vendor scripts
  # and the Tauri bundler both read it).
  (cd "$ROOT" && APPLE_SIGNING_IDENTITY="$ident" bun run tauri build --target aarch64-apple-darwin --bundles app)
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
  expect_signed=${3:-0}
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
    if [ "$expect_signed" = 1 ]; then
      codesign --verify --deep --strict "$app" ||
        fail "the app inside $dmg does not verify with codesign --deep --strict"
      if ! spctl --assess --type execute -vv "$app" >"$mp.spctl" 2>&1; then
        cat "$mp.spctl" >&2
        fail "the app inside $dmg was not accepted by spctl"
      fi
      rm -f "$mp.spctl"
    fi
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

# --- signing, Mach-O audit, notarization ------------------------------------------------------

audit_identity() {
  ident=$(identity)
  [ -n "$ident" ] || ident="Developer ID Application: Helder Santos (K6M85697D4)"
  printf '%s' "$ident"
}

# `file` parses the load commands, so it sees every Mach-O magic (32/64-bit, both endiannesses,
# fat32/fat64) and does not mistake a data file whose first bytes happen to look like a magic.
macho_files() {
  find "$1" -type f | while IFS= read -r file; do
    case $(file -b "$file" 2>/dev/null) in
      *Mach-O*) printf '%s\n' "$file" ;;
    esac
  done
}

# Every Mach-O inside the bundle must carry our Developer ID, hardened runtime and a timestamp.
# The one exception is the .NET runtime's createdump, which ships signed by Microsoft.
audit_app() {
  app=$1
  ident=$2
  [ -d "$app" ] || fail "no app bundle at $app"
  list=$(mktemp "$WORK/macho.XXXXXX")
  macho_files "$app" >"$list"
  total=$(wc -l <"$list" | tr -d ' ')
  if [ "$total" = 0 ]; then
    rm -f "$list"
    fail "no Mach-O binary found under $app"
  fi
  ours=0
  microsoft=0
  problem=
  while IFS= read -r file; do
    relative=${file#"$app"/}
    if ! codesign --verify --strict "$file" 2>/dev/null; then
      problem="$relative does not verify with codesign --verify --strict"
      break
    fi
    if ! meta=$(codesign -dv --verbose=4 "$file" 2>&1); then
      problem="cannot read the signature of $relative"
      break
    fi
    case $meta in
      *"flags=0x10000(runtime)"*) ;;
      *)
        problem="$relative has no hardened runtime (flags=0x10000(runtime) missing)"
        break
        ;;
    esac
    case $meta in
      *"Timestamp="*) ;;
      *)
        problem="$relative has no secure timestamp"
        break
        ;;
    esac
    if [ "$(basename "$file")" = createdump ]; then
      case $meta in
        *"Authority=$MICROSOFT_IDENTITY"*) microsoft=$((microsoft + 1)) ;;
        *)
          problem="$relative is not signed by Microsoft ($MICROSOFT_IDENTITY); the runtime must ship it that way"
          break
          ;;
      esac
    else
      case $meta in
        *"Authority=$ident"*) ours=$((ours + 1)) ;;
        *)
          problem="$relative is not signed with '$ident'"
          break
          ;;
      esac
    fi
  done <"$list"
  rm -f "$list"
  if [ -n "$problem" ]; then
    fail "$problem"
  fi
  if [ "$microsoft" -gt 1 ]; then
    fail "$microsoft binaries claim to be createdump; only the runtime ships one"
  fi
  note "audit ok: $total Mach-O ($ours with '$ident', $microsoft createdump from Microsoft)"
}

# Writes the .p8 and records its path in this shell (never in a $(...) subshell), so the EXIT trap
# can always remove it. Returns 1 when the environment uses a keychain profile instead.
prepare_notary_key() {
  if [ -n "$NOTARY_KEY_FILE" ]; then
    return 0
  fi
  [ -n "${APPLE_API_KEY:-}" ] || return 1
  [ -n "${APPLE_API_KEY_ID:-}" ] || fail "APPLE_API_KEY is set without APPLE_API_KEY_ID"
  old_umask=$(umask)
  umask 077
  NOTARY_KEY_FILE=$(mktemp "$WORK/AuthKey_${APPLE_API_KEY_ID}.XXXXXX")
  umask "$old_umask"
  printf '%s' "$APPLE_API_KEY" | tr -d '\n' | openssl base64 -d -A >"$NOTARY_KEY_FILE" ||
    fail "APPLE_API_KEY is not valid base64 of a .p8"
  [ -s "$NOTARY_KEY_FILE" ] || fail "APPLE_API_KEY decoded to an empty file"
  return 0
}

# notarytool with the credentials from the environment; the key is a path, never a value in argv.
notary() {
  action=$1
  shift
  if prepare_notary_key; then
    if [ -n "${APPLE_API_ISSUER:-}" ]; then
      xcrun notarytool "$action" "$@" --key "$NOTARY_KEY_FILE" --key-id "$APPLE_API_KEY_ID" \
        --issuer "$APPLE_API_ISSUER"
    else
      xcrun notarytool "$action" "$@" --key "$NOTARY_KEY_FILE" --key-id "$APPLE_API_KEY_ID"
    fi
  else
    xcrun notarytool "$action" "$@" --keychain-profile "$NOTARY_KEYCHAIN_PROFILE"
  fi
}

json_field() {
  printf '%s' "$1" | tr ',' '\n' |
    sed -n "s/.*\"$2\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" | head -n 1
}

notarize() {
  file=$1
  out=$(notary submit "$file" --wait --output-format json --no-progress) || {
    printf '%s\n' "$out" >&2
    fail "notarytool submit failed for $(basename "$file")"
  }
  status=$(json_field "$out" status)
  id=$(json_field "$out" id)
  if [ "$status" != "Accepted" ]; then
    printf '%s\n' "$out" >&2
    notary log "$id" >&2 || true
    fail "Apple rejected $(basename "$file") (status '${status:-unknown}', id '${id:-unknown}')"
  fi
  note "notarization accepted: $(basename "$file") ($id)"
}

staple_and_validate() {
  target=$1
  xcrun stapler staple "$target" >/dev/null || fail "stapler failed on $target"
  xcrun stapler validate "$target" >/dev/null || fail "stapler validate failed on $target"
  note "stapled and validated: $target"
}

notarize_app() {
  app=$1
  NOTARY_ZIP="$WORK/$PRODUCT-notary.$$.zip"
  ditto -c -k --keepParent --sequesterRsrc "$app" "$NOTARY_ZIP" || fail "cannot zip $app"
  notarize "$NOTARY_ZIP"
  rm -f "$NOTARY_ZIP"
  NOTARY_ZIP=
  staple_and_validate "$app"
}

verify_artifact() {
  app=$1
  dmg=$2
  ident=$(identity)
  codesign --verify --deep --strict --verbose=2 "$app" || fail "codesign --verify failed on $app"
  meta=$(codesign -dv --verbose=4 "$app" 2>&1)
  case $meta in
    *"Authority=$ident"*) ;;
    *) fail "the app is not signed by '$ident'" ;;
  esac
  case $meta in
    *"flags=0x10000(runtime)"*) ;;
    *) fail "the app has no hardened runtime" ;;
  esac
  if ! spctl --assess --type execute -vv "$app" >"$WORK/spctl-app.log" 2>&1; then
    cat "$WORK/spctl-app.log" >&2
    fail "spctl rejected the app"
  fi
  if ! spctl --assess --type open --context context:primary-signature -vv "$dmg" >"$WORK/spctl-dmg.log" 2>&1; then
    cat "$WORK/spctl-dmg.log" >&2
    fail "spctl rejected the DMG"
  fi
  xcrun stapler validate "$app" >/dev/null || fail "stapler validate failed on the app"
  xcrun stapler validate "$dmg" >/dev/null || fail "stapler validate failed on the DMG"
  verify_dmg "$dmg" "$(release_version)" 1
  note "artifact verified: codesign, spctl and stapler on app and DMG"
}

verification_outputs() {
  [ -n "${GITHUB_OUTPUT:-}" ] || return 0
  {
    printf 'verify_codesign=%s\n' "app e app no DMG montado aceitos (codesign --deep --strict)"
    printf 'verify_spctl=%s\n' "Notarized Developer ID (app e DMG)"
    printf 'verify_stapler=%s\n' "app e DMG validados"
  } >>"$GITHUB_OUTPUT"
}

cleanup() {
  if [ -n "$NOTARY_KEY_FILE" ]; then
    rm -f "$NOTARY_KEY_FILE"
    NOTARY_KEY_FILE=
  fi
  if [ -n "$NOTARY_ZIP" ]; then
    rm -f "$NOTARY_ZIP"
    NOTARY_ZIP=
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
    # A rehearsal must build exactly like a local build: no Developer ID even if the environment has
    # one, so vendor tools and the bundler sign ad-hoc and nothing real can be notarized by accident.
    APPLE_SIGNING_IDENTITY=
    export APPLE_SIGNING_IDENTITY
    note "dry run: signing identity cleared, the build stays ad-hoc"
    build_app
    print_plan
    dmg=$(assemble_dmg "$(resolve_app)")
    verify_dmg "$dmg" "$(release_version)"
    artifact_outputs "$dmg"
    note "dry run complete: the DMG above is NOT signed, NOT notarized and was not published"
    ;;
  audit)
    [ -n "$AUDIT_APP" ] || fail "--audit needs an app bundle path"
    audit_app "$AUDIT_APP" "$(audit_identity)"
    ;;
  full)
    trap cleanup EXIT
    preflight strict
    build_app
    app=$(resolve_app)
    audit_app "$app" "$(identity)"
    notarize_app "$app"
    dmg=$(assemble_dmg "$app")
    note "signing $dmg with $(identity)"
    codesign --force --sign "$(identity)" --timestamp "$dmg" || fail "cannot sign $dmg"
    notarize "$dmg"
    staple_and_validate "$dmg"
    verify_artifact "$app" "$dmg"
    verification_outputs
    artifact_outputs "$dmg"
    note "release artifact ready: $dmg"
    ;;
esac
