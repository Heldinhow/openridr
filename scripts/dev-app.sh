#!/bin/sh
# Starts the OpenRidr dev app with the embedded analysis pair (csharp-ls + private .NET 10
# runtime + netcoredbg). A fresh worktree has neither the vendored resources nor an app bundle:
# `tauri dev` stops at the bundle.resources check before it can compile, and a dev binary is not
# inside an .app, so `resources_root()` does not exist and the pair is only found through the
# three OPENRIDR_BUNDLED_* overrides. See docs/SPEC-DEV-APP.md.
# Usage: dev-app.sh [<path-to-.sln|.slnx|.csproj|folder>]   (no path: the Welcome screen)
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
resources="$repo/src-tauri/resources"
config='{"bundle":{"resources":null}}'

usage() {
  echo "usage: dev-app.sh [<path-to-.sln|.slnx|.csproj|folder>]" >&2
  exit 2
}

[ "$#" -le 1 ] || usage

workspace=""
if [ "$#" -eq 1 ]; then
  [ -e "$1" ] || { echo "dev-app.sh: path does not exist: $1" >&2; exit 1; }
  if [ ! -d "$1" ]; then
    case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
    *.sln | *.slnx | *.csproj) ;;
    *)
      echo "dev-app.sh: not a .sln, .slnx or .csproj: $1" >&2
      echo "dev-app.sh: open a .sln, a .slnx, a .csproj or a directory that contains one." >&2
      exit 2
      ;;
    esac
  fi
  workspace=$(CDPATH= cd -- "$(dirname -- "$1")" && pwd -P)/$(basename -- "$1")
fi

command -v bun >/dev/null 2>&1 || {
  echo "dev-app.sh: bun is not on PATH; the dev server runs 'bun run dev'." >&2
  exit 1
}
tauri="$repo/node_modules/.bin/tauri"
[ -x "$tauri" ] || {
  echo "dev-app.sh: $tauri is missing; run 'bun install' in $repo first." >&2
  exit 1
}
# The single-instance plugin hands the workspace to a live OpenRidr and this process exits, so a
# second instance would look like "the dev app never started".
if pgrep -x openridr >/dev/null 2>&1; then
  echo "dev-app.sh: an OpenRidr is already running; close it first." >&2
  exit 1
fi

# A runtime directory the host accepts: `10.<minor>[.<patch>...]`, digits only, no prerelease.
# `Version::parse` also drops build metadata, so a `+` suffix is not a prerelease.
is_stable_10() {
  case "$1" in
  10.*) ;;
  *) return 1 ;;
  esac
  name=${1%%+*}
  case "$name" in
  *-*) return 1 ;;
  esac
  rest=${name#10.}
  case "$rest" in
  '' | .* | *. | *..*) return 1 ;;
  esac
  case "$rest" in
  *[!0-9.]*) return 1 ;;
  esac
  return 0
}

has_10x() {
  for dir in "$1"/*; do
    [ -d "$dir" ] || continue
    is_stable_10 "$(basename -- "$dir")" && return 0
  done
  return 1
}

# The script changes to the repository root before starting the app, so every path it exports
# has to be absolute first.
absolute() {
  case "$1" in
  /*) printf '%s\n' "$1" ;;
  *) printf '%s\n' "$(CDPATH= cd -- "$(dirname -- "$1")" && pwd -P)/$(basename -- "$1")" ;;
  esac
}

# The host walks the adapter root, so a nested binary is a valid pair.
netcoredbg_bin() {
  if [ -f "$1/netcoredbg" ]; then
    printf '%s\n' "$1/netcoredbg"
  else
    find "$1" -type f -name netcoredbg 2>/dev/null | head -1
  fi
}

# Same conditions the host calls a complete pair: entry dll + deps, muxer + 10.x fxr + 10.x app
# runtime, adapter binary. Prints every missing path, one per line.
pair_gaps() {
  [ -f "$1/CSharpLanguageServer.dll" ] || echo "$1/CSharpLanguageServer.dll"
  [ -f "$1/CSharpLanguageServer.deps.json" ] || echo "$1/CSharpLanguageServer.deps.json"
  [ -x "$2/dotnet" ] || echo "$2/dotnet"
  has_10x "$2/host/fxr" || echo "$2/host/fxr/10.x"
  has_10x "$2/shared/Microsoft.NETCore.App" || echo "$2/shared/Microsoft.NETCore.App/10.x"
  [ -n "$(netcoredbg_bin "$3")" ] || echo "$3/netcoredbg"
}

csharp=${OPENRIDR_BUNDLED_CSHARP_LS:-}
runtime=${OPENRIDR_BUNDLED_DOTNET_ROOT:-}
netcoredbg=${OPENRIDR_BUNDLED_NETCOREDBG:-}
exported=0
[ -n "$csharp" ] && exported=$((exported + 1))
[ -n "$runtime" ] && exported=$((exported + 1))
[ -n "$netcoredbg" ] && exported=$((exported + 1))

if [ "$exported" -eq 3 ]; then
  source_note="the OPENRIDR_BUNDLED_* environment"
elif [ "$exported" -ne 0 ]; then
  echo "dev-app.sh: OPENRIDR_BUNDLED_* is partially set; set all three or none." >&2
  exit 1
else
  source_note=""
  if [ -d "$resources/csharp-ls" ] || [ -d "$resources/dotnet" ] || [ -f "$resources/netcoredbg/netcoredbg" ]; then
    csharp="$resources/csharp-ls"
    runtime="$resources/dotnet"
    netcoredbg="$resources/netcoredbg"
    gaps=$(pair_gaps "$csharp" "$runtime" "$netcoredbg")
    if [ -z "$gaps" ]; then
      source_note="$resources (vendored)"
    else
      echo "dev-app.sh: the vendored pair in $resources is incomplete; looking for an app bundle." >&2
      printf '%s\n' "$gaps" | sed 's/^/  missing: /' >&2
      csharp="" runtime="" netcoredbg=""
    fi
  fi
  if [ -z "$source_note" ]; then
    app=${OPENRIDR_APP:-}
    if [ -z "$app" ] && [ -d /Applications/OpenRidr.app ]; then
      app=/Applications/OpenRidr.app
    fi
    if [ -z "$app" ]; then
      echo "dev-app.sh: no embedded pair found." >&2
      echo "  install the app: bun run app:install" >&2
      echo "  or vendor the pair: sh scripts/vendor-netcoredbg.sh && sh scripts/vendor-analysis.sh" >&2
      echo "  or point OPENRIDR_APP at an OpenRidr.app." >&2
      exit 1
    fi
    case "$app" in
    *.app) ;;
    *)
      echo "dev-app.sh: OPENRIDR_APP must be the path of an OpenRidr.app: $app" >&2
      exit 1
      ;;
    esac
    [ -d "$app" ] || { echo "dev-app.sh: no app bundle at $app" >&2; exit 1; }
    app=$(absolute "$app")
    csharp="$app/Contents/Resources/csharp-ls"
    runtime="$app/Contents/Resources/dotnet"
    netcoredbg="$app/Contents/Resources/netcoredbg"
    version=$(/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" "$app/Contents/Info.plist" 2>/dev/null || true)
    source_note="$app ${version:-unknown version}"
  fi
fi

gaps=$(pair_gaps "$csharp" "$runtime" "$netcoredbg")
if [ -n "$gaps" ]; then
  echo "dev-app.sh: the embedded pair is incomplete; missing:" >&2
  printf '%s\n' "$gaps" | sed 's/^/  /' >&2
  echo "  install the app (bun run app:install) or vendor the pair (scripts/vendor-*.sh)." >&2
  exit 1
fi

csharp=$(absolute "$csharp")
runtime=$(absolute "$runtime")
netcoredbg=$(absolute "$netcoredbg")
echo "dev-app.sh: pair from $source_note" >&2
echo "dev-app.sh: workspace ${workspace:-<welcome>}" >&2
cd "$repo"
if [ -n "$workspace" ]; then
  exec env OPENRIDR_BUNDLED_CSHARP_LS="$csharp" OPENRIDR_BUNDLED_DOTNET_ROOT="$runtime" \
    OPENRIDR_BUNDLED_NETCOREDBG="$netcoredbg" "$tauri" dev --config "$config" -- "$workspace"
fi
exec env OPENRIDR_BUNDLED_CSHARP_LS="$csharp" OPENRIDR_BUNDLED_DOTNET_ROOT="$runtime" \
  OPENRIDR_BUNDLED_NETCOREDBG="$netcoredbg" "$tauri" dev --config "$config"
