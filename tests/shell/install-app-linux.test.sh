#!/bin/sh
# Installs into a temporary prefix through the script's public interface (--prefix) and checks the
# files it wrote: the binary, the netcoredbg payload, the desktop entry (absolute Exec, %F, no
# MimeType) and the three icon sizes. The Tauri build is replaced by a prebuilt binary through
# OPENRIDR_INSTALL_BINARY, so the test never builds and never writes to the real ~/.local.
#
#   sh tests/shell/install-app-linux.test.sh
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
script=$repo/scripts/install-app-linux.sh

failures=0
pass() { echo "ok   - $*"; }
fail() {
  echo "FAIL - $*"
  failures=$((failures + 1))
}

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-install-app-linux.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

# A stand-in for the build output: an executable binary and the netcoredbg payload.
mkdir -p "$scratch/build"
binary=$scratch/build/openridr
printf '#!/bin/sh\nexit 0\n' >"$binary"
chmod +x "$binary"
payload=$scratch/build/netcoredbg
mkdir -p "$payload"
printf '#!/bin/sh\nexit 0\n' >"$payload/netcoredbg"
chmod +x "$payload/netcoredbg"
printf 'shim\n' >"$payload/libdbgshim.so"

home=$scratch/home
mkdir -p "$home"

# The script's own shebang is `/bin/sh`; INSTALL_APP_LINUX_SHELL runs it under another shell, which
# is how a host whose `/bin/sh` is bash still exercises dash (the CI runner's `/bin/sh`).
if [ -n "${INSTALL_APP_LINUX_SHELL:-}" ]; then
  install_script() { "$INSTALL_APP_LINUX_SHELL" "$script" "$@"; }
else
  install_script() { "$script" "$@"; }
fi

# run <prefix> [binary]: keeps the exit code, stdout and stderr for the assertions.
run() {
  set +e
  HOME=$home OPENRIDR_INSTALL_BINARY=${2:-$binary} OPENRIDR_INSTALL_PAYLOAD=$payload \
    install_script --prefix "$1" >"$scratch/out" 2>"$scratch/err"
  code=$?
  set -e
  out=$(cat "$scratch/out")
  err=$(cat "$scratch/err")
}

# --- a full install ---------------------------------------------------------------
prefix=$scratch/prefix
run "$prefix"
if [ "$code" -eq 0 ]; then
  pass "the install exits 0"
else
  fail "the install exits 0 (exit $code, stderr: $err)"
fi
for file in \
  "$prefix/bin/openridr" \
  "$prefix/lib/openridr/netcoredbg/netcoredbg" \
  "$prefix/lib/openridr/netcoredbg/libdbgshim.so" \
  "$prefix/share/applications/openridr.desktop" \
  "$prefix/share/icons/hicolor/32x32/apps/openridr.png" \
  "$prefix/share/icons/hicolor/128x128/apps/openridr.png" \
  "$prefix/share/icons/hicolor/256x256/apps/openridr.png"; do
  if [ -f "$file" ]; then
    pass "installed ${file#"$prefix"/}"
  else
    fail "installed ${file#"$prefix"/}"
  fi
done
if [ -x "$prefix/bin/openridr" ]; then
  pass "the installed binary is executable"
else
  fail "the installed binary is executable"
fi

# --- the desktop entry ------------------------------------------------------------
desktop=$prefix/share/applications/openridr.desktop
if grep -Fq "Exec=$prefix/bin/openridr %F" "$desktop"; then
  pass "the Exec line is the absolute path with %F"
else
  fail "the Exec line is the absolute path with %F (got: $(grep '^Exec=' "$desktop" || true))"
fi
if grep -Fxq "Categories=Development;IDE;" "$desktop"; then
  pass "the entry declares the IDE categories"
else
  fail "the entry declares the IDE categories"
fi
if grep -Fq "MimeType" "$desktop"; then
  fail "the entry claims no MIME type (the Linux bundle has no file associations)"
else
  pass "the entry claims no MIME type"
fi
for key in Name Icon Terminal StartupWMClass Keywords Comment; do
  if grep -Fq "$key=" "$desktop"; then
    pass "the entry has $key"
  else
    fail "the entry has $key"
  fi
done

# --- nothing outside the prefix ---------------------------------------------------
if [ -e "$home/.local" ]; then
  fail "a custom --prefix leaves the real ~/.local alone"
else
  pass "a custom --prefix leaves the real ~/.local alone"
fi

# --- a second run replaces the payload instead of merging it ----------------------
printf 'stale\n' >"$prefix/lib/openridr/netcoredbg/stale-file"
run "$prefix"
if [ -e "$prefix/lib/openridr/netcoredbg/stale-file" ]; then
  fail "a reinstall replaces the payload (a stale file survived)"
else
  pass "a reinstall replaces the payload"
fi

# --- refusals ---------------------------------------------------------------------
run "relative/prefix"
if [ "$code" -ne 0 ] && printf '%s' "$err" | grep -Fq "absolute path"; then
  pass "a relative prefix is refused"
else
  fail "a relative prefix is refused (exit $code, stderr: $err)"
fi
run "$scratch/prefix" "$scratch/absent/openridr"
if [ "$code" -ne 0 ] && printf '%s' "$err" | grep -Fq "produced no binary"; then
  pass "a missing binary is refused with a clear message"
else
  fail "a missing binary is refused with a clear message (exit $code, stderr: $err)"
fi
set +e
install_script --prefix >"$scratch/out" 2>"$scratch/err"
code=$?
set -e
if [ "$code" -ne 0 ] && grep -Fq "needs a directory" "$scratch/err"; then
  pass "--prefix without a value is refused"
else
  fail "--prefix without a value is refused (exit $code, stderr: $(cat "$scratch/err"))"
fi
set +e
install_script --bogus >"$scratch/out" 2>"$scratch/err"
code=$?
set -e
if [ "$code" -ne 0 ] && grep -Fq "unknown argument" "$scratch/err"; then
  pass "an unknown argument is refused"
else
  fail "an unknown argument is refused (exit $code, stderr: $(cat "$scratch/err"))"
fi

if [ "$failures" -ne 0 ]; then
  echo "install-app-linux: $failures failure(s)"
  exit 1
fi
echo "install-app-linux: all checks passed"
