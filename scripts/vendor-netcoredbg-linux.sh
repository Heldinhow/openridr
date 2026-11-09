#!/bin/sh
# Linux half of scripts/vendor-netcoredbg.sh, kept in its own file so tests/shell can call the
# extraction checks with a controlled archive. Sourced, never executed: the build path always
# passes the pinned archive and digest from the script's own table, so a caller cannot substitute
# the asset a normal build bundles.
#
# `vendor_netcoredbg_linux_install <archive> <pin> <dest> <marker>` verifies the SHA-256 of the
# official .tar.gz, validates the whole listing before writing a byte (no absolute path, no `..`
# segment, no symlink/hardlink/special file, bounded entry count and size — the same limits as
# the host's extract_zip), extracts, and installs the netcoredbg/ payload into <dest>.

# Same limits as the host's runtime extractor (crates/host/src/debugger.rs).
VENDOR_NETCOREDBG_MAX_ENTRIES=200
VENDOR_NETCOREDBG_MAX_BYTES=$((200 * 1024 * 1024))

vendor_netcoredbg_fail() {
  echo "vendor-netcoredbg: $*" >&2
  exit 1
}

vendor_netcoredbg_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    vendor_netcoredbg_fail "no sha256sum or shasum on PATH"
  fi
}

vendor_netcoredbg_linux_install() {
  archive=$1
  pin=$2
  dest=$3
  marker=$4

  [ -f "$archive" ] || vendor_netcoredbg_fail "missing $archive"
  actual=$(vendor_netcoredbg_sha256 "$archive")
  if [ "$actual" != "$pin" ]; then
    echo "vendor-netcoredbg: SHA-256 mismatch for $archive" >&2
    echo "  expected $pin" >&2
    echo "  actual   $actual" >&2
    exit 1
  fi
  state="sha256:$pin"
  if [ -f "$marker" ] && [ "$(cat "$marker")" = "$state" ] \
    && [ -x "$dest/netcoredbg" ] && [ -f "$dest/libdbgshim.so" ]; then
    echo "vendor-netcoredbg: $dest is current ($(basename -- "$archive")); nothing to do."
    return 0
  fi

  names=$(tar -tzf "$archive") || vendor_netcoredbg_fail "$archive is not a readable .tar.gz"
  count=$(printf '%s\n' "$names" | wc -l | tr -d ' ')
  [ "$count" -le "$VENDOR_NETCOREDBG_MAX_ENTRIES" ] \
    || vendor_netcoredbg_fail "$archive has $count entries, above the $VENDOR_NETCOREDBG_MAX_ENTRIES-entry limit"
  if printf '%s\n' "$names" | grep -Eq '(^/|(^|/)\.\.(/|$))'; then
    vendor_netcoredbg_fail "$archive has an absolute path or a parent segment; nothing is extracted"
  fi
  if tar --numeric-owner -tvzf "$archive" | cut -c1 | grep -Eqv '^[-d]$'; then
    vendor_netcoredbg_fail "$archive has a symlink, hardlink or special file; nothing is extracted"
  fi
  total=$(tar --numeric-owner -tvzf "$archive" | awk '{sum += $3} END {print sum + 0}')
  [ "$total" -le "$VENDOR_NETCOREDBG_MAX_BYTES" ] \
    || vendor_netcoredbg_fail "$archive would extract $total bytes, above the $VENDOR_NETCOREDBG_MAX_BYTES-byte limit"

  tmp="$dest.extract"
  rm -rf "$tmp"
  mkdir -p "$tmp"
  tar -xzf "$archive" --no-same-owner --no-same-permissions -C "$tmp" \
    || vendor_netcoredbg_fail "could not extract $archive"
  if [ -n "$(find "$tmp" -type l -print -quit)" ]; then
    rm -rf "$tmp"
    vendor_netcoredbg_fail "$archive extracted a symlink; nothing is installed"
  fi
  if [ ! -f "$tmp/netcoredbg/netcoredbg" ] || [ ! -f "$tmp/netcoredbg/libdbgshim.so" ]; then
    rm -rf "$tmp"
    vendor_netcoredbg_fail "$archive has no netcoredbg/ payload"
  fi
  mkdir -p "$dest"
  # Keep the tracked entitlements file; everything else is generated.
  find "$dest" -mindepth 1 -maxdepth 1 ! -name "netcoredbg.entitlements" -exec rm -rf {} +
  mv "$tmp/netcoredbg"/* "$dest"/
  rm -rf "$tmp"
  chmod +x "$dest/netcoredbg"
  printf '%s\n' "$state" >"$marker"

  echo "vendor-netcoredbg: extracted $(basename -- "$archive") to $dest"
  echo "vendor-netcoredbg: no signing on Linux; the build verifies the pinned SHA-256 of the archive"
}
