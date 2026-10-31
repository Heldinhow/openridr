#!/bin/sh
# Builds the local macOS .app (ad-hoc signed, not notarized) and installs it into /Applications.
# Run again after each version bump to update the installed copy. See docs/install-app.md.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

bun run tauri build --bundles app

bundle="$root/src-tauri/target/release/bundle/macos/OpenRidr.app"
if [ ! -d "$bundle" ]; then
  echo "install-app: expected bundle not found: $bundle" >&2
  exit 1
fi

rm -rf /Applications/OpenRidr.app
cp -R "$bundle" /Applications/OpenRidr.app
# A locally built app is not quarantined; clearing the flag only matters for copies made elsewhere.
xattr -dr com.apple.quarantine /Applications/OpenRidr.app 2>/dev/null || true
echo "install-app: installed $(defaults read /Applications/OpenRidr.app/Contents/Info CFBundleShortVersionString) at /Applications/OpenRidr.app"
