#!/bin/sh
# The pull request body is the only record of what a change proves, so the governance job refuses a
# body that does not reference an issue or does not carry evidence.
#
# This is a script, not YAML inline, because scripts/ci-pr-body-selfcheck.sh has to exercise every
# rejection locally; the governance job runs that self-check before the real body.
#
# Usage: sh scripts/ci-pr-body.sh <body-file> [author]
# Exit: 0 accepted, 1 rejected (message on stderr), 2 usage/IO error.
set -eu

if [ "$#" -lt 1 ]; then
  echo 'usage: sh scripts/ci-pr-body.sh <body-file> [author]' >&2
  exit 2
fi
body=$1
author=${2:-}

# Dependency bumps open without a ticket and without evidence, and a bot cannot fix either; the gate
# exists for agent-authored pull requests.
case "$author" in
  'dependabot[bot]'|'renovate[bot]')
    echo "pr body: skipped for $author"
    exit 0
    ;;
esac

if [ ! -f "$body" ]; then
  echo "pr body: $body does not exist" >&2
  exit 2
fi

# Any of the linking keywords GitHub understands, plus `Part of`/`Refs` for a ticket that does not
# close its parent.
if ! grep -Eqi '(^|[^[:alnum:]])(closes|fixes|resolves|part of|refs)[[:space:]]*#[0-9]+' "$body"; then
  echo 'pr body: no issue reference (expected "Closes #N", "Part of #N" or "Refs #N")' >&2
  exit 1
fi

# The evidence section runs from its heading to the next `##` heading. Both spellings are accepted
# because the template is in English while the repo's docs are in Portuguese; the accented form is a
# separate pattern so a C locale cannot turn `ê` into a byte range.
heading=$(grep -Ein \
  -e '^##[[:space:]]+evidence[[:space:]]*$' \
  -e '^##[[:space:]]+evidência[[:space:]]*$' \
  "$body" | head -1 | cut -d: -f1)
if [ -z "$heading" ]; then
  echo 'pr body: no "## Evidence" section (say what you ran, or "not verified: why")' >&2
  exit 1
fi

# Drop blank lines and single-line HTML comments: the template ships a comment as a placeholder, and
# a section that still has only the placeholder proves nothing.
block=$(tail -n "+$((heading + 1))" "$body" \
  | awk '/^##[[:space:]]/ { exit } { print }' \
  | sed -e '/^[[:space:]]*$/d' -e '/^[[:space:]]*<!--/d')

if [ -z "$block" ]; then
  echo 'pr body: the evidence section is empty' >&2
  exit 1
fi

if ! printf '%s\n' "$block" | grep -Eqi -v '^[[:space:]]*(none|n/a|todo|tbd)\.?[[:space:]]*$'; then
  echo 'pr body: the evidence section is only a placeholder ("none", "n/a", "todo")' >&2
  exit 1
fi

echo "pr body: issue reference and evidence found"
