#!/bin/sh
# Focused tests for the Linux half of scripts/vendor-netcoredbg.sh: the pinned SHA-256 gate and
# the archive checks that stand in for the host's extract_zip at build time (traversal, absolute
# path, symlink, special file, entry and size limits). The real 3.4 MB assets are not needed: the
# helper is sourced and called with a controlled archive and its digest, exactly as the build
# script calls it with the pinned ones.
#
#   sh tests/shell/vendor-netcoredbg.test.sh
#
# The Linux branch only runs on a Linux build host, so the whole file skips elsewhere.
set -eu

if [ "$(uname -s)" != "Linux" ]; then
  echo "skip - vendor-netcoredbg: the Linux branch only runs on a Linux build host"
  exit 0
fi

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)

# shellcheck source=scripts/vendor-netcoredbg-linux.sh
. "$repo/scripts/vendor-netcoredbg-linux.sh"

failures=0
pass() { echo "ok   - $*"; }
fail() {
  echo "FAIL - $*"
  failures=$((failures + 1))
}

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-vendor-netcoredbg.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

digest() {
  vendor_netcoredbg_sha256 "$1"
}

payload() {
  mkdir -p "$1/netcoredbg"
  printf '#!/bin/sh\nprintf "NET Core debugger 3.2.0-1\\n"\n' >"$1/netcoredbg/netcoredbg"
  chmod +x "$1/netcoredbg/netcoredbg"
  printf 'shim\n' >"$1/netcoredbg/libdbgshim.so"
}

# install <dest> <archive> <pin>: keeps the exit code, stdout and stderr for the assertions.
install() {
  set +e
  (
    vendor_netcoredbg_linux_install "$2" "$3" "$1" "$1.vendor-state"
  ) >"$scratch/out" 2>"$scratch/err"
  code=$?
  set -e
  out=$(cat "$scratch/out")
  err=$(cat "$scratch/err")
}

# expect <pass|fail> <name> <archive> <pin> [expected stderr fragment]
expect() {
  want=$1
  name=$2
  archive=$3
  pin=$4
  fragment=${5:-}
  dest="$scratch/dest-$name"
  rm -rf "$dest" "$dest.vendor-state"
  install "$dest" "$archive" "$pin"
  if [ "$want" = pass ]; then
    if [ "$code" -ne 0 ]; then
      fail "$name: exit $code, expected 0 (stderr: $err)"
      return 0
    fi
    if [ ! -x "$dest/netcoredbg" ] || [ ! -f "$dest/libdbgshim.so" ]; then
      fail "$name: the payload is not at $dest"
      return 0
    fi
    pass "$name"
    return 0
  fi
  if [ "$code" -eq 0 ]; then
    fail "$name: exit 0, expected a refusal"
    return 0
  fi
  if [ -n "$fragment" ] && ! printf '%s' "$err" | grep -Fq "$fragment"; then
    fail "$name: stderr does not contain '$fragment' (got: $err)"
    return 0
  fi
  if [ -e "$dest/netcoredbg" ]; then
    fail "$name: a rejected archive still installed $dest/netcoredbg"
    return 0
  fi
  pass "$name"
}

# --- a good archive is verified, extracted and reused -------------------------------
good="$scratch/good"
payload "$good"
tar -czf "$scratch/good.tar.gz" -C "$good" netcoredbg
good_pin=$(digest "$scratch/good.tar.gz")
expect pass "a pinned archive is extracted" "$scratch/good.tar.gz" "$good_pin"

dest="$scratch/dest-a pinned archive is extracted"
install "$dest" "$scratch/good.tar.gz" "$good_pin"
if [ "$code" -eq 0 ] && printf '%s' "$out" | grep -Fq "is current"; then
  pass "a second run reuses the extracted tree"
else
  fail "a second run reuses the extracted tree (exit $code, out: $out, err: $err)"
fi

# --- the pin is the gate ------------------------------------------------------------
expect fail "a digest mismatch installs nothing" "$scratch/good.tar.gz" \
  "0000000000000000000000000000000000000000000000000000000000000000" "SHA-256 mismatch"
expect fail "a missing archive installs nothing" "$scratch/absent.tar.gz" "$good_pin" \
  "missing"

# --- the archive checks -------------------------------------------------------------
# A parent segment, the shape a traversal entry has.
tar -czf "$scratch/traversal.tar.gz" --transform='s|^|../|' -C "$good" netcoredbg
expect fail "a parent segment is refused" "$scratch/traversal.tar.gz" \
  "$(digest "$scratch/traversal.tar.gz")" "parent segment"

# An absolute path.
mkdir -p "$scratch/abs"
printf 'x\n' >"$scratch/abs/file"
tar -czPf "$scratch/absolute.tar.gz" "$scratch/abs/file"
expect fail "an absolute path is refused" "$scratch/absolute.tar.gz" \
  "$(digest "$scratch/absolute.tar.gz")" "absolute path"

# A symlink, even one that points outside the tree.
mkdir -p "$scratch/link"
ln -s /etc/passwd "$scratch/link/escape"
tar -czf "$scratch/symlink.tar.gz" -C "$scratch/link" escape
expect fail "a symlink is refused" "$scratch/symlink.tar.gz" \
  "$(digest "$scratch/symlink.tar.gz")" "symlink, hardlink or special file"

# A special file.
mkdir -p "$scratch/special"
mkfifo "$scratch/special/fifo"
tar -czf "$scratch/special.tar.gz" -C "$scratch/special" fifo
expect fail "a special file is refused" "$scratch/special.tar.gz" \
  "$(digest "$scratch/special.tar.gz")" "symlink, hardlink or special file"

# More entries than the limit.
mkdir -p "$scratch/many/many"
i=0
while [ "$i" -lt 201 ]; do
  : >"$scratch/many/many/f$i"
  i=$((i + 1))
done
tar -czf "$scratch/many.tar.gz" -C "$scratch/many" many
expect fail "too many entries are refused" "$scratch/many.tar.gz" \
  "$(digest "$scratch/many.tar.gz")" "above the 200-entry limit"

# More bytes than the limit, before anything is written.
mkdir -p "$scratch/big"
truncate -s 201M "$scratch/big/blob"
tar -czf "$scratch/big.tar.gz" -C "$scratch/big" blob
expect fail "an oversized tree is refused" "$scratch/big.tar.gz" \
  "$(digest "$scratch/big.tar.gz")" "above the 209715200-byte limit"

if [ "$failures" -ne 0 ]; then
  echo "vendor-netcoredbg: $failures failure(s)"
  exit 1
fi
echo "vendor-netcoredbg: all checks passed"
