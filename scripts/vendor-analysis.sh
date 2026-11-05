#!/bin/sh
# Extract, verify and sign the analysis tools that ship inside the app bundle:
#   * the private .NET 10 runtime that executes csharp-ls (never the project SDK);
#   * the csharp-ls 0.28.0 payload (managed, no NuGet shim).
# Both artifacts are pinned by SHA-512 of the official download; a mismatch fails the build
# instead of shipping an unverified analyzer. Re-running is a no-op once the trees match.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
RESOURCES="$ROOT/src-tauri/resources"
VENDOR="$ROOT/vendor"

RUNTIME_VERSION=10.0.14
RUNTIME_FILE="dotnet-runtime-$RUNTIME_VERSION-osx-arm64.tar.gz"
RUNTIME_URL="https://builds.dotnet.microsoft.com/dotnet/Runtime/$RUNTIME_VERSION/$RUNTIME_FILE"
# Official SHA-512 published by Microsoft (releases-index.json -> 10.0 channel, runtime asset).
RUNTIME_SHA512=19c5781dc2f87600865886b8774a4c5851caf1f5cf3f591185b922b0e51dea6b8c0ed0bf80ae4d3be2e083db662124258743e779fab6885322d641eff52b0382
RUNTIME_DEST="$RESOURCES/dotnet"
RUNTIME_CACHE="$VENDOR/dotnet-runtime/$RUNTIME_FILE"
RUNTIME_MARKER="$RESOURCES/dotnet.vendor-state"

CSHARP_VERSION=0.28.0
CSHARP_FILE="csharp-ls.$CSHARP_VERSION.nupkg"
CSHARP_URL="https://api.nuget.org/v3-flatcontainer/csharp-ls/$CSHARP_VERSION/$CSHARP_FILE"
# Same pin the installer uses (crates/host/src/tools.rs, PACKAGE_SHA512).
CSHARP_SHA512=a138b9c8f9de9dd6343e9b0d0c8f4d51637bba7e38606b7c2b08b1d91e8eefa39f12123057f6625f425812b4361e52841bd2856704ec127ff50144ce05170a99
CSHARP_DEST="$RESOURCES/csharp-ls"
CSHARP_CACHE="$VENDOR/csharp-ls/$CSHARP_FILE"
CSHARP_MARKER="$RESOURCES/csharp-ls.vendor-state"

IDENTITY=${APPLE_SIGNING_IDENTITY:-}
ENTITLEMENTS="$RESOURCES/dotnet.entitlements"
ENTITLEMENTS_STATE=missing
if [ -f "$ENTITLEMENTS" ]; then
  ENTITLEMENTS_STATE=$(shasum -a 256 "$ENTITLEMENTS" | awk '{print $1}')
fi
STATE="identity:${IDENTITY:--} runtime:sha512:$RUNTIME_SHA512 csharp-ls:sha512:$CSHARP_SHA512 entitlements:$ENTITLEMENTS_STATE"

fetch() {
  url=$1
  dest=$2
  pin=$3
  if [ ! -f "$dest" ]; then
    mkdir -p "$(dirname -- "$dest")"
    echo "vendor-analysis: downloading $url"
    curl -fL --retry 3 --retry-delay 2 -o "$dest.part" "$url"
    mv "$dest.part" "$dest"
  fi
  actual=$(shasum -a 512 "$dest" | awk '{print $1}')
  if [ "$actual" != "$pin" ]; then
    echo "vendor-analysis: SHA-512 mismatch for $dest" >&2
    echo "  expected $pin" >&2
    echo "  actual   $actual" >&2
    exit 1
  fi
}

sign() {
  if [ -n "$IDENTITY" ]; then
    codesign --force --options runtime --timestamp --sign "$IDENTITY" "$@"
  else
    codesign --force --options runtime --sign - "$@"
  fi
}

if [ -f "$RUNTIME_MARKER" ] && [ "$(cat "$RUNTIME_MARKER")" = "$STATE" ] \
  && [ -x "$RUNTIME_DEST/dotnet" ] \
  && [ -d "$RUNTIME_DEST/host/fxr/$RUNTIME_VERSION" ] \
  && [ -d "$RUNTIME_DEST/shared/Microsoft.NETCore.App/$RUNTIME_VERSION" ]; then
  echo "vendor-analysis: runtime $RUNTIME_VERSION is current; nothing to do."
else
  fetch "$RUNTIME_URL" "$RUNTIME_CACHE" "$RUNTIME_SHA512"
  TMP="$RUNTIME_DEST.extract"
  rm -rf "$TMP"
  mkdir -p "$TMP"
  tar -xzf "$RUNTIME_CACHE" -C "$TMP"
  rm -rf "$RUNTIME_DEST"
  mkdir -p "$RUNTIME_DEST"
  mv "$TMP"/* "$RUNTIME_DEST"/
  rmdir "$TMP"
  chmod +x "$RUNTIME_DEST/dotnet"
  # Deepest first: every Mach-O the muxer may load must carry its own signature.
  for binary in $(find "$RUNTIME_DEST" -type f -name "*.dylib" | sort); do
    sign "$binary"
  done
  if [ -f "$ENTITLEMENTS" ]; then
    sign --entitlements "$ENTITLEMENTS" "$RUNTIME_DEST/dotnet"
  else
    sign "$RUNTIME_DEST/dotnet"
  fi
  for binary in $(find "$RUNTIME_DEST" -type f -name "*.dylib" | sort); do
    codesign --verify --strict "$binary"
  done
  codesign --verify --strict "$RUNTIME_DEST/dotnet"
  printf '%s\n' "$STATE" >"$RUNTIME_MARKER"
  echo "vendor-analysis: extracted runtime $RUNTIME_VERSION to $RUNTIME_DEST"
fi

if [ -f "$CSHARP_MARKER" ] && [ "$(cat "$CSHARP_MARKER")" = "$STATE" ] \
  && [ -f "$CSHARP_DEST/CSharpLanguageServer.dll" ]; then
  echo "vendor-analysis: csharp-ls $CSHARP_VERSION is current; nothing to do."
else
  fetch "$CSHARP_URL" "$CSHARP_CACHE" "$CSHARP_SHA512"
  TMP="$CSHARP_DEST.extract"
  rm -rf "$TMP"
  mkdir -p "$TMP"
  unzip -q "$CSHARP_CACHE" -d "$TMP"
  if [ ! -f "$TMP/tools/net10.0/any/CSharpLanguageServer.dll" ]; then
    echo "vendor-analysis: $CSHARP_CACHE has no tools/net10.0/any payload" >&2
    exit 1
  fi
  rm -rf "$CSHARP_DEST"
  mkdir -p "$CSHARP_DEST"
  # Only the runnable payload: the .nupkg copies and NuGet bookkeeping stay out of the app.
  cp -R "$TMP/tools/net10.0/any/." "$CSHARP_DEST/"
  rm -rf "$TMP"
  # The payload is managed-only today; sign anything native if a future package adds it.
  for binary in $(find "$CSHARP_DEST" -type f \( -name "*.dylib" -o -name "*.so" \) | sort); do
    sign "$binary"
  done
  printf '%s\n' "$STATE" >"$CSHARP_MARKER"
  echo "vendor-analysis: extracted csharp-ls $CSHARP_VERSION to $CSHARP_DEST"
fi

echo "vendor-analysis: signed with ${IDENTITY:-ad-hoc (-)}, hardened runtime$([ -n "$IDENTITY" ] && echo ", timestamp" || echo ", no timestamp")"
