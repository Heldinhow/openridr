#!/bin/sh
# Which risk tier does a pull request carry? The tier comes from the paths it touches, never from who
# wrote it: documentation, tests and stylesheets are low; product code is medium; CI, release, signing,
# the trust model and dependency changes are high. A mixed pull request gets the highest tier among its
# paths.
#
# Usage: sh scripts/ci-risk-tier.sh <base-ref> [head-ref]
#   stdout: low | medium | high (a single line)
#   stderr: one reason per changed path, so a human can see what decided the tier
# Exit: 0 the tier was computed; 2 usage, git or empty-diff error. The caller must fail closed to
# `high` on a non-zero exit: an error here is not a low-risk pull request.
#
# The script knows nothing about GitHub: it takes git refs and prints. That is what lets
# scripts/ci-risk-tier-selfcheck.sh exercise every rule against synthetic repositories.
set -eu

usage() {
  echo 'usage: sh scripts/ci-risk-tier.sh <base-ref> [head-ref]' >&2
  exit 2
}

[ "$#" -ge 1 ] || usage
base=$1
head=${2:-HEAD}

# A ref that does not resolve is a caller bug, not "no changes".
if [ -z "$(git rev-parse --verify --quiet "$base^{commit}" 2>/dev/null)" ]; then
  echo "risk tier: $base is not a commit in this repository" >&2
  exit 2
fi
if [ -z "$(git rev-parse --verify --quiet "$head^{commit}" 2>/dev/null)" ]; then
  echo "risk tier: $head is not a commit in this repository" >&2
  exit 2
fi

# `...` is the pull-request diff (merge base to head), not the two-tip diff: commits that landed on the
# base branch after the branch started are not this pull request's risk. `--no-renames` reports a rename
# as a deletion plus an addition, so both the old and the new path get classified (deleting
# `trusted.rs` must not read as "an unknown path").
paths=$(git diff --name-only --no-renames "$base...$head") || {
  echo "risk tier: could not diff $base...$head" >&2
  exit 2
}

if [ -z "$paths" ]; then
  echo "risk tier: $base...$head changes no path" >&2
  exit 2
fi

# Dependency tables of a TOML file, one line per entry, sorted: section headers are kept so that moving
# a dependency between tables still counts, and reordering inside a table does not.
toml_deps() {
  awk '
    {
      line = $0
      sub(/\r$/, "", line)
      if (line ~ /^[ \t]*\[/) {
        name = line
        sub(/^[ \t]*\[+/, "", name)
        sub(/\]+[ \t]*$/, "", name)
        gsub(/[ \t]/, "", name)
        keep = (name ~ /^(dependencies|dev-dependencies|build-dependencies|patch)$/ ||
                name ~ /^(dependencies|dev-dependencies|build-dependencies|patch)\./ ||
                name ~ /\.(dependencies|dev-dependencies|build-dependencies)$/)
        if (keep) print "[" name "]"
        next
      }
      if (keep) {
        sub(/[ \t]+$/, "", line)
        if (line != "" && line !~ /^[ \t]*#/) print line
      }
    }' | LC_ALL=C sort
}

# Only the dependency-carrying objects of a package.json; `version`, `scripts` and the rest of the
# manifest are not dependency news.
json_deps() {
  jq -S 'if type == "object" then
           with_entries(select(.key |
             IN("dependencies", "devDependencies", "peerDependencies",
                "optionalDependencies", "overrides", "resolutions")))
         else . end'
}

# Returns 0 when the dependency tables of $1 differ between base and head (or when either side cannot be
# read: an added or deleted manifest is a dependency change), 1 when they are identical.
manifest_dependencies_changed() {
  before=$(git show "$base:$1" 2>/dev/null) || return 0
  after=$(git show "$head:$1" 2>/dev/null) || return 0
  case "$1" in
    *.json)
      left=$(printf '%s\n' "$before" | json_deps 2>/dev/null) || return 0
      right=$(printf '%s\n' "$after" | json_deps 2>/dev/null) || return 0
      ;;
    *)
      left=$(printf '%s\n' "$before" | toml_deps)
      right=$(printf '%s\n' "$after" | toml_deps)
      ;;
  esac
  [ "$left" = "$right" ] && return 1
  return 0
}

rank=0
bump() { # <tier> <path> <reason>
  case "$1" in
    high) value=2 ;;
    medium) value=1 ;;
    *) value=0 ;;
  esac
  if [ "$value" -gt "$rank" ]; then
    rank=$value
  fi
  printf '%s: %s (%s)\n' "$1" "$2" "$3" >&2
}

# Paths are split on newlines only: a path with a space stays one path. Globbing is off for the loop
# so that a path that happens to look like a pattern stays the literal path git reported.
old_ifs=$IFS
IFS='
'
set -f
for path in $paths; do
  case "$path" in
    .github/workflows/* | .github/rulesets/*)
      bump high "$path" 'CI workflow or branch ruleset'
      ;;
    AGENTS.md)
      bump high "$path" 'agent process rules'
      ;;
    scripts/release* | scripts/vendor-* | scripts/ci-*)
      bump high "$path" 'release, vendor or CI script'
      ;;
    src-tauri/tauri.conf.json | src-tauri/resources/*.entitlements)
      bump high "$path" 'signing, notarization or updater'
      ;;
    crates/host/src/trusted.rs)
      bump high "$path" 'trust model'
      ;;
    Cargo.lock | */Cargo.lock | bun.lock | */bun.lock)
      # Lockfiles only move when a dependency or a version does.
      bump high "$path" 'lockfile'
      ;;
    Cargo.toml | */Cargo.toml | package.json | */package.json)
      if manifest_dependencies_changed "$path"; then
        bump high "$path" 'dependency change'
      else
        bump medium "$path" 'manifest without a dependency change'
      fi
      ;;
    *.md | docs/*)
      bump low "$path" 'documentation'
      ;;
    *.css)
      bump low "$path" 'stylesheet'
      ;;
    tests/* | crates/*/tests/*)
      bump low "$path" 'tests'
      ;;
    src/* | crates/* | src-tauri/* | fixtures/* | rust-toolchain.toml)
      bump medium "$path" 'product code'
      ;;
    *)
      # A path no rule knows is not a documentation change: the default is the conservative one.
      bump medium "$path" 'unmatched path'
      ;;
  esac
done
IFS=$old_ifs
set +f

case "$rank" in
  2) tier=high ;;
  1) tier=medium ;;
  *) tier=low ;;
esac

echo "$tier"
