#!/bin/sh
# Installs OpenRidr into a user prefix on Linux without root: the binary, the netcoredbg payload,
# the desktop entry and the icon. The default prefix is $HOME/.local; --prefix installs somewhere
# else (the shell test uses a temporary one). Nothing is written outside the prefix, so removing
# the app is `rm -rf <prefix>/bin/openridr <prefix>/lib/openridr` plus the two shared files.
# See docs/install-app.md.
#
# Usage: sh scripts/install-app-linux.sh [--prefix <dir>]
#   OPENRIDR_INSTALL_BINARY   install this binary instead of building one (the shell test's seam;
#                             it also skips the build, which is the only slow step)
#   OPENRIDR_INSTALL_PAYLOAD  netcoredbg payload to install (default: src-tauri/resources/netcoredbg)
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)

usage() {
  echo "usage: install-app-linux.sh [--prefix <dir>]" >&2
}

prefix=$HOME/.local
while [ $# -gt 0 ]; do
  case $1 in
    --prefix)
      if [ $# -lt 2 ]; then
        echo "install-app-linux: --prefix needs a directory" >&2
        exit 2
      fi
      prefix=$2
      shift 2
      ;;
    --prefix=*)
      prefix=${1#--prefix=}
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "install-app-linux: unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

case $prefix in
  /*) ;;
  *)
    echo "install-app-linux: --prefix must be an absolute path (got: $prefix)" >&2
    exit 2
    ;;
esac

binary=${OPENRIDR_INSTALL_BINARY:-}
if [ -z "$binary" ]; then
  ( cd "$repo" && bun run tauri build --no-bundle )
  binary=$repo/src-tauri/target/release/openridr
fi
payload=${OPENRIDR_INSTALL_PAYLOAD:-$repo/src-tauri/resources/netcoredbg}
icons=$repo/src-tauri/icons
template=$repo/src-tauri/linux/openridr.desktop

if [ ! -f "$binary" ]; then
  echo "install-app-linux: the build produced no binary at $binary" >&2
  exit 1
fi
if [ ! -f "$payload/netcoredbg" ]; then
  echo "install-app-linux: the netcoredbg payload is missing at $payload" >&2
  exit 1
fi
if [ ! -f "$template" ]; then
  echo "install-app-linux: the desktop entry template is missing at $template" >&2
  exit 1
fi

mkdir -p "$prefix/bin" "$prefix/lib/openridr" "$prefix/share/applications" \
  "$prefix/share/icons/hicolor"
install -m 755 "$binary" "$prefix/bin/openridr"

# The payload is replaced, never merged: a stale adapter from an older build would be reported as
# the bundled one.
rm -rf "$prefix/lib/openridr/netcoredbg"
cp -R "$payload" "$prefix/lib/openridr/netcoredbg"

# 128x128@2x.png is the 256 px icon; the bundler derives the same three sizes for the deb.
for size in 32x32 128x128; do
  mkdir -p "$prefix/share/icons/hicolor/$size/apps"
  install -m 644 "$icons/$size.png" "$prefix/share/icons/hicolor/$size/apps/openridr.png"
done
mkdir -p "$prefix/share/icons/hicolor/256x256/apps"
install -m 644 "$icons/128x128@2x.png" \
  "$prefix/share/icons/hicolor/256x256/apps/openridr.png"

# One definition of the entry, shared with the deb: only the Exec line changes here, because a
# graphical session rarely has <prefix>/bin on PATH.
sed "s|^Exec=.*|Exec=$prefix/bin/openridr %F|" "$template" \
  >"$prefix/share/applications/openridr.desktop"

echo "install-app-linux: installed openridr at $prefix/bin/openridr"
echo "install-app-linux: desktop entry at $prefix/share/applications/openridr.desktop"
