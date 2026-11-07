#!/bin/sh
# Self-check for scripts/dev-app.sh: runs the script from a fake repository root, with fake `bun`,
# `pgrep` and `tauri` on PATH, so nothing is compiled, launched or downloaded.
# Usage: sh scripts/dev-app-selfcheck.sh. Work dir is removed on exit.
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
script="$repo/scripts/dev-app.sh"
work=$(mktemp -d "${TMPDIR:-/tmp}/dev-app-selfcheck.XXXXXX")
# TMPDIR on macOS ends with a slash, so mktemp hands back a path with `//`, and the script resolves
# paths physically; normalize once so every expectation below is the string the script prints.
work=$(CDPATH= cd -- "$work" && pwd -P)
trap 'rm -rf "$work"' EXIT INT TERM

fail() {
  echo "self-check failed: $1" >&2
  exit 1
}

# A developer may have the overrides exported; every case below sets what it needs.
unset OPENRIDR_BUNDLED_CSHARP_LS OPENRIDR_BUNDLED_DOTNET_ROOT OPENRIDR_BUNDLED_NETCOREDBG OPENRIDR_APP

# Fake repository root: the script derives everything from its own path.
mkdir -p "$work/scripts" "$work/bin" "$work/node_modules/.bin"
cp "$script" "$work/scripts/dev-app.sh"
# A copy whose /Applications fallback points at a path that does not exist, so "no pair at all"
# is reachable on a machine that has the app installed.
sed "s#/Applications/OpenRidr.app#$work/absent/OpenRidr.app#" "$script" >"$work/scripts/dev-app-noapp.sh"
grep -q "$work/absent/OpenRidr.app" "$work/scripts/dev-app-noapp.sh" || fail "fallback rewrite"

log="$work/tauri.log"
export DEV_APP_SC_LOG="$log"
# One line per argument, so a shifted or split argument cannot pass as the expected text.
cat >"$work/node_modules/.bin/tauri" <<'EOF'
#!/bin/sh
{
  printf 'csharp=%s\n' "$OPENRIDR_BUNDLED_CSHARP_LS"
  printf 'runtime=%s\n' "$OPENRIDR_BUNDLED_DOTNET_ROOT"
  printf 'netcoredbg=%s\n' "$OPENRIDR_BUNDLED_NETCOREDBG"
  for arg in "$@"; do printf 'arg=%s\n' "$arg"; done
} >"$DEV_APP_SC_LOG"
EOF
chmod +x "$work/node_modules/.bin/tauri"
printf '#!/bin/sh\nexit 0\n' >"$work/bin/bun"
printf '#!/bin/sh\nexit 1\n' >"$work/bin/pgrep"
chmod +x "$work/bin/bun" "$work/bin/pgrep"
mkdir "$work/bin-busy"
printf '#!/bin/sh\nexit 0\n' >"$work/bin-busy/pgrep"
chmod +x "$work/bin-busy/pgrep"
mkdir "$work/bin-nobun"
cp "$work/bin/pgrep" "$work/bin-nobun/pgrep"
export PATH="$work/bin:$PATH"

# A complete pair, in the layout the host accepts.
make_pair() {
  mkdir -p "$1/csharp-ls" "$1/dotnet/host/fxr/10.0.14" \
    "$1/dotnet/shared/Microsoft.NETCore.App/10.0.14" "$1/netcoredbg"
  : >"$1/csharp-ls/CSharpLanguageServer.dll"
  : >"$1/csharp-ls/CSharpLanguageServer.deps.json"
  : >"$1/dotnet/dotnet"
  : >"$1/netcoredbg/netcoredbg"
  chmod +x "$1/dotnet/dotnet" "$1/netcoredbg/netcoredbg"
}

make_pair "$work/src-tauri/resources"
target="$work/OrderDesk.sln"
: >"$target"
# The script resolves the workspace physically, so the expectation does too.
target=$(CDPATH= cd -- "$(dirname -- "$target")" && pwd -P)/$(basename -- "$target")

# 1. Vendored pair + workspace: exports point at the vendored tree, and the argv is exactly
#    `dev --config <json> -- <absolute workspace>`.
sh "$work/scripts/dev-app.sh" "$target" 2>"$work/stderr" || fail "vendored pair exits 1"
grep -Fqx 'csharp='"$work"'/src-tauri/resources/csharp-ls' "$log" || fail "vendored csharp: $(cat "$log")"
grep -Fqx 'runtime='"$work"'/src-tauri/resources/dotnet' "$log" || fail "vendored runtime: $(cat "$log")"
grep -Fqx 'netcoredbg='"$work"'/src-tauri/resources/netcoredbg' "$log" || fail "vendored netcoredbg: $(cat "$log")"
grep -Fqx 'arg=dev' "$log" || fail "no dev argument: $(cat "$log")"
grep -Fqx 'arg=--config' "$log" || fail "no --config argument: $(cat "$log")"
grep -Fqx 'arg={"bundle":{"resources":null}}' "$log" || fail "wrong config argument: $(cat "$log")"
grep -Fqx 'arg=--' "$log" || fail "no -- before the workspace: $(cat "$log")"
grep -Fqx "arg=$target" "$log" || fail "workspace argument: $(cat "$log")"
grep -q "pair from $work/src-tauri/resources (vendored)" "$work/stderr" || fail "vendored pair is not printed"

# 2. A workspace path with spaces stays one argument.
mkdir -p "$work/My Solutions"
spaced="$work/My Solutions/Order Desk.sln"
: >"$spaced"
spaced=$(CDPATH= cd -- "$(dirname -- "$spaced")" && pwd -P)/$(basename -- "$spaced")
sh "$work/scripts/dev-app.sh" "$spaced" 2>/dev/null || fail "spaced workspace exits 1"
grep -Fqx "arg=$spaced" "$log" || fail "spaced workspace argument: $(cat "$log")"

# 3. No workspace: the app still starts, without a trailing `--`.
sh "$work/scripts/dev-app.sh" 2>/dev/null || fail "no workspace exits 1"
grep -Fqx 'arg=dev' "$log" || fail "no workspace argv: $(cat "$log")"
if grep -Fqx 'arg=--' "$log"; then fail "no workspace still passes --"; fi

# 4. App bundle as the source: exports come from Contents/Resources and the version is printed.
app="$work/OpenRidr.app"
make_pair "$app/Contents/Resources"
cat >"$app/Contents/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>CFBundleShortVersionString</key><string>9.9.9-test</string></dict></plist>
EOF
rm -rf "$work/src-tauri/resources"
OPENRIDR_APP="$app" sh "$work/scripts/dev-app.sh" "$target" 2>"$work/stderr" || fail "app pair exits 1"
grep -Fqx "csharp=$app/Contents/Resources/csharp-ls" "$log" || fail "app pair csharp: $(cat "$log")"
grep -Fqx "netcoredbg=$app/Contents/Resources/netcoredbg" "$log" || fail "app pair netcoredbg: $(cat "$log")"
grep -q "pair from $app 9.9.9-test" "$work/stderr" || fail "app version is not printed: $(cat "$work/stderr")"

# 5. A relative source is resolved before the script changes directory.
(cd "$work" && OPENRIDR_APP=OpenRidr.app sh "$work/scripts/dev-app.sh" "$target" 2>/dev/null) \
  || fail "relative app exits 1"
grep -Fqx "csharp=$app/Contents/Resources/csharp-ls" "$log" || fail "relative app is not absolute: $(cat "$log")"

# 6. An incomplete pair fails before compiling, naming the missing path.
rm -rf "$app/Contents/Resources/csharp-ls/CSharpLanguageServer.deps.json"
OPENRIDR_APP="$app" sh "$work/scripts/dev-app.sh" "$target" 2>"$work/stderr" && fail "incomplete app pair exits 0"
grep -q "CSharpLanguageServer.deps.json" "$work/stderr" || fail "incomplete pair does not name the gap"

# 7. A runtime directory the host rejects (prerelease, non-numeric) is not a complete pair.
make_pair "$work/src-tauri/resources"
rm -rf "$work/src-tauri/resources/dotnet/host/fxr/10.0.14"
mkdir "$work/src-tauri/resources/dotnet/host/fxr/10.0.14-preview"
sh "$work/scripts/dev-app-noapp.sh" "$target" 2>"$work/stderr" && fail "prerelease runtime exits 0"
grep -q "host/fxr/10.x" "$work/stderr" || fail "prerelease runtime is not named"
rm -rf "$work/src-tauri/resources/dotnet/host/fxr/10.0.14-preview"
mkdir "$work/src-tauri/resources/dotnet/host/fxr/10.invalid"
sh "$work/scripts/dev-app-noapp.sh" "$target" 2>"$work/stderr" && fail "non-numeric runtime exits 0"
grep -q "host/fxr/10.x" "$work/stderr" || fail "non-numeric runtime is not named"

# 8. No pair at all: the message names both ways to get one.
rm -rf "$work/src-tauri/resources"
sh "$work/scripts/dev-app-noapp.sh" "$target" 2>"$work/stderr" && fail "no pair exits 0"
grep -q "bun run app:install" "$work/stderr" || fail "no pair does not offer app:install"
grep -q "scripts/vendor-netcoredbg.sh" "$work/stderr" || fail "no pair does not offer vendor scripts"

# 9. Partially exported overrides are refused instead of half-used.
OPENRIDR_BUNDLED_CSHARP_LS="$app/Contents/Resources/csharp-ls" sh "$work/scripts/dev-app-noapp.sh" "$target" 2>"$work/stderr" \
  && fail "partial overrides exit 0"
grep -q "partially set" "$work/stderr" || fail "partial overrides are not named"

# 10. A live OpenRidr is refused before anything is resolved.
PATH="$work/bin-busy:$PATH" sh "$work/scripts/dev-app.sh" "$target" 2>"$work/stderr" && fail "live instance exits 0"
grep -q "already running" "$work/stderr" || fail "live instance is not named"

# 11. Workspace validation mirrors scripts/openridr.
status=0
sh "$work/scripts/dev-app.sh" "$work/missing.sln" 2>"$work/stderr" || status=$?
[ "$status" -eq 1 ] || fail "missing path exit code $status"
grep -q "path does not exist" "$work/stderr" || fail "missing path is not named"
: >"$work/notes.txt"
status=0
sh "$work/scripts/dev-app.sh" "$work/notes.txt" 2>"$work/stderr" || status=$?
[ "$status" -eq 2 ] || fail "unsupported extension exit code $status"
grep -q "not a .sln, .slnx or .csproj" "$work/stderr" || fail "unsupported extension is not named"

# 12. Missing prerequisites ask for the fix instead of downloading anything.
mkdir -p "$work/nonm/scripts"
cp "$script" "$work/nonm/scripts/dev-app.sh"
PATH="$work/bin-nobun:/usr/bin:/bin:/usr/sbin:/sbin" sh "$work/nonm/scripts/dev-app.sh" "$target" 2>"$work/stderr" && fail "missing bun exits 0"
grep -q "bun is not on PATH" "$work/stderr" || fail "missing bun is not named"
PATH="$work/bin:$PATH" sh "$work/nonm/scripts/dev-app.sh" "$target" 2>"$work/stderr" && fail "missing tauri exits 0"
grep -q "bun install" "$work/stderr" || fail "missing tauri does not ask for bun install"

echo "dev-app self-check: ok"
