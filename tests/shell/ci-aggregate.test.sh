#!/bin/sh
# Focused tests for scripts/ci-aggregate.sh, the logic behind the required `ci` check. No network and no
# GitHub: every case is a `needs` JSON, an event name and the list of pull-request-only jobs.
#
#   sh tests/shell/ci-aggregate.test.sh
#
# Covers: a pull request with everything green, a `push`/`workflow_dispatch` with the PR-only jobs
# skipped, a PR-only job skipped inside a pull request, a failed and a cancelled job, a non-PR job
# skipped by accident, a PR-only job missing from `needs` on a pull request, an empty `needs` object and
# the missing-input errors.
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
script="$repo/scripts/ci-aggregate.sh"

failures=0
pass() { echo "ok   - $*"; }
fail() { echo "FAIL - $*"; failures=$((failures + 1)); }

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-aggregate-test.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

# run <event> <pr_only> <json>: keeps exit code, stdout and stderr for the assertions.
run() {
  set +e
  NEEDS=$3 EVENT=$1 PR_ONLY=$2 sh "$script" >"$scratch/stdout" 2>"$scratch/stderr"
  code=$?
  set -e
  out=$(cat "$scratch/stdout")
  err=$(cat "$scratch/stderr")
}

# expect <pass|fail> <exit> <name> <event> <pr_only> <json> [expected stderr fragment]
expect() {
  want=$1
  want_code=$2
  name=$3
  shift 3
  run "$@"
  fragment=${4:-}
  if [ "$code" -ne "$want_code" ]; then
    fail "$name: expected exit $want_code, got $code ($err)"
    return
  fi
  if [ "$want" = pass ] && [ "$code" -ne 0 ]; then
    fail "$name: expected pass, got failure ($err)"
    return
  fi
  if [ "$want" = fail ] && [ "$code" -eq 0 ]; then
    fail "$name: expected failure, got pass ($out)"
    return
  fi
  if [ -n "$fragment" ] && ! printf '%s' "$err" | grep -qF "$fragment"; then
    fail "$name: stderr does not mention '$fragment' ($err)"
    return
  fi
  pass "$name"
}

all_green='{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"success"},"security":{"result":"success"},"risk":{"result":"success"}}'

# A pull request where every job passed.
expect pass 0 pr-all-green pull_request governance,risk "$all_green"

# Outside a pull request the PR-only jobs are skipped, and that is not a failure.
expect pass 0 push-pr-only-skipped push governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"skipped"},"security":{"result":"success"},"risk":{"result":"skipped"}}'
expect pass 0 dispatch-pr-only-skipped workflow_dispatch governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"skipped"},"security":{"result":"success"},"risk":{"result":"skipped"}}'

# Inside a pull request a skipped PR-only job is a bug in the workflow, not a pass.
expect fail 1 pr-governance-skipped pull_request governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"skipped"},"security":{"result":"success"},"risk":{"result":"success"}}' \
  'governance: skipped'
expect fail 1 pr-risk-skipped pull_request governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"success"},"security":{"result":"success"},"risk":{"result":"skipped"}}' \
  'risk: skipped'

# A real failure, and a cancellation (not `success`, so it may never read as green).
expect fail 1 pr-rust-failure pull_request governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"failure"},"governance":{"result":"success"},"security":{"result":"success"},"risk":{"result":"success"}}' \
  'rust: failure'
expect fail 1 pr-security-cancelled pull_request governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"success"},"security":{"result":"cancelled"},"risk":{"result":"success"}}' \
  'security: cancelled'

# A job that is not PR-only and is skipped by accident is never a pass.
expect fail 1 push-rust-skipped push governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"skipped"},"governance":{"result":"skipped"},"security":{"result":"success"},"risk":{"result":"skipped"}}' \
  'rust: skipped'

# Dropping a PR-only job from `needs` would silently drop its gate: on a pull request the aggregator
# requires every one of them to be present.
expect fail 1 pr-risk-missing pull_request governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"success"},"security":{"result":"success"}}' \
  'risk'

# Misconfiguration must be loud, never "everything passed".
expect fail 2 empty-needs pull_request governance,risk '{}'
expect fail 2 needs-not-an-object pull_request governance,risk '[]'
expect fail 2 missing-pr-only pull_request '' "$all_green"
expect fail 2 missing-event '' governance,risk "$all_green"

if [ "$failures" -ne 0 ]; then
  echo "ci-aggregate tests: $failures failure(s)"
  exit 1
fi

echo "ci-aggregate tests: all cases passed"
