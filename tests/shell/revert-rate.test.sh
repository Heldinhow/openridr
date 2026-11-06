#!/bin/sh
# Focused tests for scripts/revert-rate.sh: the classification is a pure function of two JSON arrays, so
# every case runs offline with the `--merged`/`--reverts` flags and never touches GitHub.
#
#   sh tests/shell/revert-rate.test.sh
#
# Covers: the month window on both sides (a PR merged next month, a revert committed next month),
# GitHub's `Revert "…"` message and the conventional `revert:` type, a revert of a revert counting
# against the revert PR, duplicate reverts of the same PR counting once, a revert that names a PR
# outside the window, a revert that names nothing, the row format, "nothing merged" (exit 1) and the
# input errors (exit 2).
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
script="$repo/scripts/revert-rate.sh"

failures=0
pass() { echo "ok   - $*"; }
fail() { echo "FAIL - $*"; failures=$((failures + 1)); }

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-revert-rate-test.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

# run <month> <merged file> <reverts file>: keeps exit code, stdout and stderr for the assertions.
run() {
  set +e
  sh "$script" --month "$1" --merged "$2" --reverts "$3" >"$scratch/stdout" 2>"$scratch/stderr"
  code=$?
  set -e
  out=$(cat "$scratch/stdout")
  err=$(cat "$scratch/stderr")
}

# expect <exit> <name> <stdout fragment> <stderr fragment> <month> <merged> <reverts>
expect() {
  want_code=$1
  name=$2
  out_fragment=$3
  err_fragment=$4
  shift 4
  run "$@"
  if [ "$code" -ne "$want_code" ]; then
    fail "$name: expected exit $want_code, got $code ($err)"
    return
  fi
  if [ -n "$out_fragment" ] && ! printf '%s' "$out" | grep -qF "$out_fragment"; then
    fail "$name: stdout does not contain '$out_fragment' ($out)"
    return
  fi
  if [ -n "$err_fragment" ] && ! printf '%s' "$err" | grep -qF "$err_fragment"; then
    fail "$name: stderr does not contain '$err_fragment' ($err)"
    return
  fi
  pass "$name"
}

cat >"$scratch/merged.json" <<'JSON'
[
  { "number": 1, "title": "feat: one", "mergedAt": "2026-10-02T10:00:00Z" },
  { "number": 2, "title": "feat: two", "mergedAt": "2026-10-05T10:00:00Z" },
  { "number": 3, "title": "fix: three", "mergedAt": "2026-10-10T10:00:00Z" },
  { "number": 4, "title": "docs: four", "mergedAt": "2026-10-20T10:00:00Z" },
  { "number": 5, "title": "chore: five", "mergedAt": "2026-10-31T23:00:00Z" },
  { "number": 6, "title": "feat: next month", "mergedAt": "2026-11-01T00:30:00Z" }
]
JSON

cat >"$scratch/reverts.json" <<'JSON'
[
  { "sha": "a1", "commit": { "message": "Revert \"feat: two (#2)\"\n\nThis reverts commit deadbeef.", "committer": { "date": "2026-10-06T10:00:00Z" } } },
  { "sha": "a2", "commit": { "message": "revert(ci)!: four (#4)\n\nThis reverts commit cafe.", "committer": { "date": "2026-10-21T10:00:00Z" } } },
  { "sha": "a3", "commit": { "message": "feat: unrelated, not a revert (#3)", "committer": { "date": "2026-10-11T10:00:00Z" } } },
  { "sha": "a4", "commit": { "message": "Revert \"feat: something (#99)\"", "committer": { "date": "2026-10-12T10:00:00Z" } } },
  { "sha": "a5", "commit": { "message": "Revert \"docs: no reference\"", "committer": { "date": "2026-10-13T10:00:00Z" } } },
  { "sha": "a6", "commit": { "message": "Revert \"feat: two again (#2)\"", "committer": { "date": "2026-10-25T10:00:00Z" } } },
  { "sha": "a7", "commit": { "message": "Revert \"chore: five (#5)\"", "committer": { "date": "2026-11-02T10:00:00Z" } } }
]
JSON

# 5 PRs in October (#6 is November): #2 and #4 reverted, #2 twice so it counts once → 2/5 = 40.0%.
expect 0 two-of-five '| 2026-10 | 5 | 2 | 40.0% |' 'unattributed=2' 2026-10 \
  "$scratch/merged.json" "$scratch/reverts.json"

expect 0 stats '' 'merged=5 reverted=2' 2026-10 "$scratch/merged.json" "$scratch/reverts.json"

# A month where the window matches nothing is not a zero rate: it is "nothing to measure".
expect 1 september '' 'no pull request was merged in 2026-09' 2026-09 \
  "$scratch/merged.json" "$scratch/reverts.json"

cat >"$scratch/merged-revert.json" <<'JSON'
[
  { "number": 2, "title": "feat: two", "mergedAt": "2026-10-05T10:00:00Z" },
  { "number": 7, "title": "Revert \"feat: two (#2)\"", "mergedAt": "2026-10-08T10:00:00Z" }
]
JSON

cat >"$scratch/reverts-revert.json" <<'JSON'
[
  { "sha": "b1", "commit": { "message": "Revert \"Revert \\\"feat: two (#2)\\\" (#7)\"\n\nThis reverts commit b0b0.", "committer": { "date": "2026-10-09T10:00:00Z" } } }
]
JSON

# Undoing a revert counts against the revert PR, which is the PR that actually changed main.
expect 0 revert-of-revert '| 2026-10 | 2 | 1 | 50.0% |' 'unattributed=0' 2026-10 \
  "$scratch/merged-revert.json" "$scratch/reverts-revert.json"

# A conventional `revert:` without the `This reverts commit` trailer still counts.
cat >"$scratch/reverts-conventional.json" <<'JSON'
[
  { "sha": "c1", "commit": { "message": "revert: undo the merge (#2)", "committer": { "date": "2026-10-07T10:00:00Z" } } }
]
JSON

expect 0 conventional-type '| 2026-10 | 2 | 1 | 50.0% |' '' 2026-10 \
  "$scratch/merged-revert.json" "$scratch/reverts-conventional.json"

# Inputs are validated, not guessed.
printf 'not an array\n' >"$scratch/bad.json"
expect 2 invalid-month '' 'is not a month in YYYY-MM form' 2026-13 "$scratch/merged.json" "$scratch/reverts.json"
expect 2 missing-file '' 'does not exist' 2026-10 "$scratch/nope.json" "$scratch/reverts.json"
expect 2 not-an-array '' 'is not a JSON array' 2026-10 "$scratch/bad.json" "$scratch/reverts.json"

if [ "$failures" -ne 0 ]; then
  echo "revert-rate tests: $failures failure(s)"
  exit 1
fi

echo "revert-rate tests: all cases passed"
