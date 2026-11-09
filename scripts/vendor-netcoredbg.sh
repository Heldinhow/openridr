#!/bin/sh
# Extract and re-sign the pinned netcoredbg asset for the app bundle.
# macOS: the zip is the release pin; the extracted Mach-O binaries are ad-hoc (or identity)
# signed with the hardened runtime.
# Linux: the official .tar.gz for the build architecture is the pin. The build verifies its
# SHA-256 and extracts it with the same protections as the runtime zip path (no traversal,
# absolute path, symlink or special file; bounded size and entry count). Nothing is signed.
# A mismatch fails the build instead of shipping an unverified adapter. Re-running is a no-op
# once the tree matches the pin.
#
# The Linux extraction lives in scripts/vendor-netcoredbg-linux.sh so tests/shell can exercise
# the checks with a controlled archive; a normal build always passes the pinned archive and
# digest below, so the asset cannot be substituted.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
DEST="$ROOT/src-tauri/resources/netcoredbg"
MARKER="$DEST.vendor-state"

# shellcheck source=scripts/vendor-netcoredbg-linux.sh
. "$ROOT/scripts/vendor-netcoredbg-linux.sh"

fail() {
  echo "vendor-netcoredbg: $*" >&2
  exit 1
}

sha256() {
  vendor_netcoredbg_sha256 "$1"
}

vendor_macos() {
  ZIP="$ROOT/vendor/netcoredbg/netcoredbg-osx-arm64-3.2.0-1092.zip"
  PIN=f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea
  ENTITLEMENTS="$DEST/netcoredbg.entitlements"
  IDENTITY=${APPLE_SIGNING_IDENTITY:-}
  ENTITLEMENTS_STATE=missing
  if [ -f "$ENTITLEMENTS" ]; then
    ENTITLEMENTS_STATE=$(sha256 "$ENTITLEMENTS")
  fi
  STATE="sha256:$PIN identity:${IDENTITY:--} entitlements:$ENTITLEMENTS_STATE"

  if [ ! -f "$ZIP" ]; then
    fail "missing $ZIP"
  fi
  ACTUAL=$(sha256 "$ZIP")
  if [ "$ACTUAL" != "$PIN" ]; then
    echo "vendor-netcoredbg: SHA-256 mismatch for $ZIP" >&2
    echo "  expected $PIN" >&2
    echo "  actual   $ACTUAL" >&2
    exit 1
  fi

  if [ -f "$MARKER" ] && [ "$(cat "$MARKER")" = "$STATE" ] \
    && [ -x "$DEST/netcoredbg" ] && [ -f "$DEST/libdbgshim.dylib" ]; then
    echo "vendor-netcoredbg: $DEST is current (3.2.0-1092, ${IDENTITY:-ad-hoc}); nothing to do."
    return 0
  fi

  TMP="$DEST.extract"
  rm -rf "$TMP"
  mkdir -p "$TMP"
  unzip -q "$ZIP" -d "$TMP"
  rm -rf "$TMP/__MACOSX"
  mkdir -p "$DEST"
  # Keep the tracked entitlements file; everything else is generated.
  find "$DEST" -mindepth 1 -maxdepth 1 ! -name "netcoredbg.entitlements" -exec rm -rf {} +
  mv "$TMP/netcoredbg"/* "$DEST"/
  rm -rf "$TMP"
  chmod +x "$DEST/netcoredbg"

  sign() {
    if [ -n "$IDENTITY" ]; then
      codesign --force --options runtime --timestamp --sign "$IDENTITY" "$@"
    else
      codesign --force --options runtime --sign - "$@"
    fi
  }

  sign "$DEST/libdbgshim.dylib"
  if [ -f "$ENTITLEMENTS" ]; then
    sign --entitlements "$ENTITLEMENTS" "$DEST/netcoredbg"
  else
    sign "$DEST/netcoredbg"
  fi
  codesign --verify --strict "$DEST/libdbgshim.dylib"
  codesign --verify --strict "$DEST/netcoredbg"
  printf '%s\n' "$STATE" >"$MARKER"

  echo "vendor-netcoredbg: extracted 3.2.0-1092 to $DEST"
  echo "vendor-netcoredbg: signed libdbgshim.dylib and netcoredbg with ${IDENTITY:-ad-hoc (-)}, hardened runtime, $([ -n "$IDENTITY" ] && echo "timestamp" || echo "no timestamp")$([ -f "$ENTITLEMENTS" ] && echo ", entitlements netcoredbg.entitlements" || echo ", no entitlements")"
}

vendor_linux() {
  arch=$(uname -m)
  case "$arch" in
  aarch64 | arm64)
    asset=netcoredbg-linux-arm64
    pin=065ff49badec8a695dbea2de6ab6a330c774a191e426a217ab8cc05250627ccb
    ;;
  x86_64 | amd64)
    asset=netcoredbg-linux-amd64
    pin=080eb3b2d2152465f599d3b33d1ee6e747794e11cc0a3773ec689f5e5f2c5afa
    ;;
  *)
    fail "no pinned netcoredbg asset for Linux $arch; nothing is substituted"
    ;;
  esac
  vendor_netcoredbg_linux_install \
    "$ROOT/vendor/netcoredbg/$asset-3.2.0-1092.tar.gz" "$pin" "$DEST" "$MARKER"
}

case "$(uname -s)" in
Darwin) vendor_macos ;;
Linux) vendor_linux ;;
*) fail "unsupported build host $(uname -s): only macOS and Linux bundle netcoredbg" ;;
esac
