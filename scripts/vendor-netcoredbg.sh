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
# Test seam (tests/shell/vendor-netcoredbg.test.sh only): OPENRIDR_VENDOR_NETCOREDBG_ARCHIVE and
# OPENRIDR_VENDOR_NETCOREDBG_PIN substitute the archive and its digest, never the checks. The
# build never sets them.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
DEST="$ROOT/src-tauri/resources/netcoredbg"
MARKER="$DEST.vendor-state"

# Same limits as the host's runtime extractor (crates/host/src/debugger.rs).
MAX_ENTRIES=200
MAX_BYTES=$((200 * 1024 * 1024))

fail() {
  echo "vendor-netcoredbg: $*" >&2
  exit 1
}

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    fail "no sha256sum or shasum on PATH"
  fi
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
  archive="$ROOT/vendor/netcoredbg/$asset-3.2.0-1092.tar.gz"
  if [ -n "${OPENRIDR_VENDOR_NETCOREDBG_ARCHIVE:-}" ] || [ -n "${OPENRIDR_VENDOR_NETCOREDBG_PIN:-}" ]; then
    if [ -z "${OPENRIDR_VENDOR_NETCOREDBG_ARCHIVE:-}" ] || [ -z "${OPENRIDR_VENDOR_NETCOREDBG_PIN:-}" ]; then
      fail "set both OPENRIDR_VENDOR_NETCOREDBG_ARCHIVE and OPENRIDR_VENDOR_NETCOREDBG_PIN, or neither"
    fi
    archive=$OPENRIDR_VENDOR_NETCOREDBG_ARCHIVE
    pin=$OPENRIDR_VENDOR_NETCOREDBG_PIN
    asset=$(basename -- "$archive")
  fi
  [ -f "$archive" ] || fail "missing $archive"
  actual=$(sha256 "$archive")
  if [ "$actual" != "$pin" ]; then
    echo "vendor-netcoredbg: SHA-256 mismatch for $archive" >&2
    echo "  expected $pin" >&2
    echo "  actual   $actual" >&2
    exit 1
  fi
  STATE="sha256:$pin"
  if [ -f "$MARKER" ] && [ "$(cat "$MARKER")" = "$STATE" ] \
    && [ -x "$DEST/netcoredbg" ] && [ -f "$DEST/libdbgshim.so" ]; then
    echo "vendor-netcoredbg: $DEST is current ($asset); nothing to do."
    return 0
  fi

  # Validate the whole listing before a byte is written: an entry that escapes the destination,
  # a symlink, a hardlink or a special file rejects the archive, and the tree stays bounded.
  names=$(tar -tzf "$archive") || fail "$archive is not a readable .tar.gz"
  count=$(printf '%s\n' "$names" | wc -l | tr -d ' ')
  [ "$count" -le "$MAX_ENTRIES" ] \
    || fail "$archive has $count entries, above the $MAX_ENTRIES-entry limit"
  if printf '%s\n' "$names" | grep -Eq '(^/|(^|/)\.\.(/|$))'; then
    fail "$archive has an absolute path or a parent segment; nothing is extracted"
  fi
  if tar --numeric-owner -tvzf "$archive" | cut -c1 | grep -Eqv '^[-d]$'; then
    fail "$archive has a symlink, hardlink or special file; nothing is extracted"
  fi
  total=$(tar --numeric-owner -tvzf "$archive" | awk '{sum += $3} END {print sum + 0}')
  [ "$total" -le "$MAX_BYTES" ] \
    || fail "$archive would extract $total bytes, above the $MAX_BYTES-byte limit"

  TMP="$DEST.extract"
  rm -rf "$TMP"
  mkdir -p "$TMP"
  tar -xzf "$archive" --no-same-owner --no-same-permissions -C "$TMP" \
    || fail "could not extract $archive"
  if [ -n "$(find "$TMP" -type l -print -quit)" ]; then
    rm -rf "$TMP"
    fail "$archive extracted a symlink; nothing is installed"
  fi
  if [ ! -f "$TMP/netcoredbg/netcoredbg" ] || [ ! -f "$TMP/netcoredbg/libdbgshim.so" ]; then
    rm -rf "$TMP"
    fail "$archive has no netcoredbg/ payload"
  fi
  mkdir -p "$DEST"
  # Keep the tracked entitlements file; everything else is generated.
  find "$DEST" -mindepth 1 -maxdepth 1 ! -name "netcoredbg.entitlements" -exec rm -rf {} +
  mv "$TMP/netcoredbg"/* "$DEST"/
  rm -rf "$TMP"
  chmod +x "$DEST/netcoredbg"
  printf '%s\n' "$STATE" >"$MARKER"

  echo "vendor-netcoredbg: extracted $asset to $DEST"
  echo "vendor-netcoredbg: no signing on Linux; the build verifies the pinned SHA-256 of the archive"
}

case "$(uname -s)" in
Darwin) vendor_macos ;;
Linux) vendor_linux ;;
*) fail "unsupported build host $(uname -s): only macOS and Linux bundle netcoredbg" ;;
esac
