#!/bin/sh
# Self-check for scripts/vendor-analysis.sh: a generated tree that loses or changes a file must
# be rebuilt from the pinned artifact, never reported as current.
#
# Operates only on this worktree's generated output (`src-tauri/resources/{dotnet,csharp-ls}`),
# which the same script rebuilds. It never touches the vendor cache, the user's app home, other
# worktrees or the project fixtures. Requires the pinned downloads (network or `vendor/` cache).
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
RESOURCES="$ROOT/src-tauri/resources"
CSHARP="$RESOURCES/csharp-ls"
RUNTIME="$RESOURCES/dotnet"
SCRIPT="$ROOT/scripts/vendor-analysis.sh"

dependency="$CSHARP/Microsoft.CodeAnalysis.CSharp.dll"
runtime_component="$RUNTIME/shared/Microsoft.NETCore.App/10.0.14/System.Private.CoreLib.dll"

fail() {
  echo "self-check failed: $1" >&2
  exit 1
}

[ -f "$SCRIPT" ] || fail "missing $SCRIPT"

echo "self-check: first prepare"
sh "$SCRIPT" >/dev/null
[ -f "$dependency" ] || fail "no $dependency after prepare"
[ -f "$runtime_component" ] || fail "no $runtime_component after prepare"
dependency_hash=$(shasum -a 256 "$dependency" | awk '{print $1}')
runtime_hash=$(shasum -a 256 "$runtime_component" | awk '{print $1}')

echo "self-check: second prepare is a content-verified no-op"
out=$(sh "$SCRIPT")
printf '%s\n' "$out" | grep -q "current (content verified)" || fail "reuse is not content-verified: $out"

echo "self-check: tamper a managed dependency and remove a runtime component, keeping markers"
printf 'tampered' >>"$dependency"
rm -f "$runtime_component"
sh "$SCRIPT" >/dev/null || fail "prepare did not repair the damaged trees"

[ "$(shasum -a 256 "$dependency" | awk '{print $1}')" = "$dependency_hash" ] ||
  fail "the tampered dependency was not restored from the pinned artifact"
[ -f "$runtime_component" ] || fail "the removed runtime component was not restored"
[ "$(shasum -a 256 "$runtime_component" | awk '{print $1}')" = "$runtime_hash" ] ||
  fail "the removed runtime component came back with different content"
codesign --verify --strict "$RUNTIME/dotnet" || fail "the muxer is not signed after repair"

echo "self-check: a repaired tree is current again"
out=$(sh "$SCRIPT")
printf '%s\n' "$out" | grep -q "current (content verified)" || fail "repaired tree is not current: $out"

echo "ok: vendor-analysis self-check passed"
