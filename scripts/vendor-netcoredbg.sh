#!/bin/sh
# Extract and re-sign the pinned netcoredbg asset for the app bundle.
# The zip hash is the release pin: a mismatch fails the build instead of shipping
# an unverified adapter. Re-running is a no-op once the tree matches the pin.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
ZIP="$ROOT/vendor/netcoredbg/netcoredbg-osx-arm64-3.2.0-1092.zip"
PIN=f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea
DEST="$ROOT/src-tauri/resources/netcoredbg"
ENTITLEMENTS="$DEST/netcoredbg.entitlements"
IDENTITY=${APPLE_SIGNING_IDENTITY:-}
MARKER="$DEST.vendor-state"
ENTITLEMENTS_STATE=missing
if [ -f "$ENTITLEMENTS" ]; then
  ENTITLEMENTS_STATE=$(shasum -a 256 "$ENTITLEMENTS" | awk '{print $1}')
fi
STATE="sha256:$PIN identity:${IDENTITY:--} entitlements:$ENTITLEMENTS_STATE"

if [ ! -f "$ZIP" ]; then
  echo "vendor-netcoredbg: missing $ZIP" >&2
  exit 1
fi
ACTUAL=$(shasum -a 256 "$ZIP" | awk '{print $1}')
if [ "$ACTUAL" != "$PIN" ]; then
  echo "vendor-netcoredbg: SHA-256 mismatch for $ZIP" >&2
  echo "  expected $PIN" >&2
  echo "  actual   $ACTUAL" >&2
  exit 1
fi

if [ -f "$MARKER" ] && [ "$(cat "$MARKER")" = "$STATE" ] \
  && [ -x "$DEST/netcoredbg" ] && [ -f "$DEST/libdbgshim.dylib" ]; then
  echo "vendor-netcoredbg: $DEST is current (3.2.0-1092, ${IDENTITY:-ad-hoc}); nothing to do."
  exit 0
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
