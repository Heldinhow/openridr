#!/bin/sh
# Self-check for scripts/openridr: runs it with a fake `open` first on a temporary PATH.
# Usage: sh scripts/openridr-selfcheck.sh. Never calls the real `open`; work dir is removed on exit.
set -eu
# A developer may export OPENRIDR_APP; the launcher under test must resolve the app itself.
unset OPENRIDR_APP

repo=$(cd "$(dirname "$0")/.." && pwd)
launcher="$repo/scripts/openridr"
work=$(mktemp -d "${TMPDIR:-/tmp}/openridr-selfcheck.XXXXXX")
trap 'rm -rf "$work"' EXIT INT TERM

mkdir "$work/bin"
log="$work/open.log"
cat > "$work/bin/open" <<EOF
#!/bin/sh
printf '%s\n' "\$@" > "$log"
EOF
chmod +x "$work/bin/open"
export PATH="$work/bin:$PATH"

target="$work/App.sln"
: > "$target"
fail() {
  echo "self-check failed: $1" >&2
  exit 1
}

sh "$launcher" "$target"
[ "$(cat "$log")" = "$(printf -- '-a\nOpenRidr\n%s' "$target")" ] || fail "default app argv"

OPENRIDR_APP="Open Ridr Dev" sh "$launcher" "$target"
[ "$(cat "$log")" = "$(printf -- '-a\nOpen Ridr Dev\n%s' "$target")" ] || fail "OPENRIDR_APP with spaces"

rm -f "$log"
if sh "$launcher" >/dev/null 2>&1; then fail "zero arguments exited 0"; fi
if sh "$launcher" "$target" "$target" >/dev/null 2>&1; then fail "two arguments exited 0"; fi
[ ! -e "$log" ] || fail "open was called for invalid usage"

if sh "$launcher" "$work/missing.sln" 2>"$work/stderr"; then fail "missing path exited 0"; fi
grep -q "does not exist" "$work/stderr" || fail "missing path has no message on stderr"
[ ! -e "$log" ] || fail "open was called for a missing path"

echo "ok: openridr self-check passed"
