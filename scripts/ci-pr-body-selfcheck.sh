#!/bin/sh
# Exercises every rejection of scripts/ci-pr-body.sh with synthetic bodies, in the style of
# scripts/openridr-selfcheck.sh: one case per line of output, non-zero exit on the first mismatch.
#
# Usage: sh scripts/ci-pr-body-selfcheck.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
gate="$here/ci-pr-body.sh"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-pr-body.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

cases=0

# expect <accepted|rejected> <name> <author> <body>
expect() {
  want=$1
  name=$2
  author=$3
  file="$scratch/$name.md"
  cat >"$file"
  cases=$((cases + 1))
  if sh "$gate" "$file" "$author" >"$scratch/$name.out" 2>&1; then
    got=accepted
  else
    got=rejected
  fi
  if [ "$got" != "$want" ]; then
    echo "FAIL $name: expected $want, got $got"
    sed 's/^/    /' "$scratch/$name.out"
    exit 1
  fi
  printf 'ok   %-28s %s\n' "$name" "$got"
}

expect accepted complete "" <<'BODY'
Closes #56

## Summary

Adds the gate.

## Evidence

`bun run check` and `cargo test -p openridr-host`: green.

## Merge danger

None.
BODY

expect accepted portuguese-heading "" <<'BODY'
Part of #56

## Resumo

Gate do corpo.

## Evidência

`sh scripts/ci-pr-body-selfcheck.sh`: 6/6.

## Risco de merge

Nenhum.
BODY

expect accepted refs-without-closing "" <<'BODY'
Refs #81

## Evidence

Ran the self-check.
BODY

expect rejected no-issue "" <<'BODY'
## Summary

Adds the gate.

## Evidence

`bun run check`: green.
BODY

expect rejected issue-but-no-evidence "" <<'BODY'
Closes #56

## Summary

Adds the gate.
BODY

expect rejected placeholder-only "" <<'BODY'
Closes #56

## Evidence

<!-- what you ran, or "not verified: why" -->
BODY

expect rejected none-only "" <<'BODY'
Closes #56

## Evidence

n/a
BODY

expect accepted dependabot 'dependabot[bot]' <<'BODY'
Bump @types/node from 22.15.0 to 22.16.0
BODY

expect accepted renovate 'renovate[bot]' <<'BODY'
chore(deps): update dependency vite to v6
BODY

echo "ci-pr-body self-check: $cases cases passed"
