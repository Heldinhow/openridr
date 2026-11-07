#!/bin/sh
# The nightly E2E smoke reports into one tracking issue: a red run creates it or rewrites its body with
# the last result, a green run comments the run link and closes it. The workflow calls this twice (one
# step per result) and keeps no decision of its own.
#
# Usage: RESULT=<red|green> RUN_URL=<url> REF=<ref> SHA=<sha> REPO=<owner/repo> sh scripts/e2e-smoke-issue.sh
#   REPORT    path to the Playwright JSON report (`--reporter=json`). On a red run, absent or unreadable
#             means the run failed before the specs ran.
#   GH        path to the gh CLI (default: `gh`); the tests point it at a stub.
#   GH_TOKEN  credential for `gh`.
# Exit: 0 the issue was created, rewritten, commented and closed, or there was nothing to do;
#       1 a `gh` call failed (the step has to fail, with `::error::`); 2 usage error.
#
# shortcut: a job that hits its 60 min timeout or is cancelled runs no further step, so neither report
# happens; the fix is a second job (`needs: smoke`, `if: always()`) fed by an output, which is the plan B
# in docs/SPEC-NIGHTLY-SMOKE.md and costs one multi-line output plus one job.
set -eu

title='[nightly] e2e-smoke is red'

usage() {
  echo 'usage: RESULT=<red|green> RUN_URL=<url> REF=<ref> SHA=<sha> REPO=<owner/repo> sh scripts/e2e-smoke-issue.sh' >&2
  exit 2
}

fail() {
  echo "::error::$*" >&2
  exit 1
}

result=${RESULT:-}
[ -n "$result" ] || usage
[ -n "${RUN_URL:-}" ] || usage
[ -n "${REF:-}" ] || usage
[ -n "${SHA:-}" ] || usage
[ -n "${REPO:-}" ] || usage
case "$result" in
  red | green) ;;
  *)
    echo "e2e-smoke issue: '$result' is not red or green" >&2
    exit 2
    ;;
esac

gh=${GH:-gh}
command -v "$gh" >/dev/null 2>&1 || {
  echo "e2e-smoke issue: $gh is not executable" >&2
  exit 2
}

# One tracking issue at most, found by its exact title. The search is by `e2e-smoke` because GitHub's
# search syntax reads the brackets of the title as a qualifier; the exact match happens here. If more
# than one open issue matches, the lowest number wins and the others are left alone (a state that only
# manual intervention creates).
if ! open=$("$gh" issue list --repo "$REPO" --state open --search 'e2e-smoke in:title' \
  --json number,title); then
  fail "could not list the open issues of $REPO"
fi
if ! number=$(printf '%s' "$open" |
  jq -r --arg title "$title" '[.[] | select(.title == $title)] | sort_by(.number) | .[0].number // empty'); then
  fail "could not read the issue list of $REPO"
fi

now=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# The failed specs and tests, straight from the Playwright report. The `file` of the top-level suite is
# inherited down the recursion: nested suites carry their own `file` and it is wrong (a `../..` path from
# the report's rootDir), so `has("file") | .file` would print a broken path. The `file` is relative to
# `testDir`, which is why the body says "relative to tests/e2e/" instead of prefixing it here.
failed='{}'
report_read=0
if [ -n "${REPORT:-}" ] && [ -r "$REPORT" ]; then
  if failed=$(jq -c '
    def walk($file):
      ((.specs // [])[] | {file: $file, title: .title, ok: .ok}),
      ((.suites // [])[] | walk($file));
    [ .suites[] | .file as $file | walk($file) | select(.ok == false) ] as $failed
    | {
        specs: ($failed | map(.file) | unique),
        tests: ($failed | map(.title) | unique),
        unexpected: (.stats.unexpected // 0),
        errors: ((.errors // []) | length)
      }' "$REPORT" 2>/dev/null); then
    report_read=1
  else
    failed='{}'
  fi
fi
specs=$(printf '%s' "$failed" | jq -r '.specs[]?')
tests=$(printf '%s' "$failed" | jq -r '.tests[]?')
unexpected=$(printf '%s' "$failed" | jq -r '.unexpected // 0')
errors=$(printf '%s' "$failed" | jq -r '.errors // 0')

# The backticks below are markdown in the issue body, not command substitution.
# shellcheck disable=SC2016
red_body() {
  printf '%s\n\n' 'The nightly E2E smoke is **red**.'
  printf '%s\n' "- Run: $RUN_URL" "- Ref: \`$REF\`" "- Commit: \`$SHA\`" "- Reported at: $now"
  printf '\n'
  if [ -n "$specs" ]; then
    printf '%s\n\n' 'Failed specs (relative to `tests/e2e/`):'
    printf '%s\n' "$specs" | sed 's/^/- `/; s/$/`/'
    printf '\n%s\n\n' 'Failed tests:'
    printf '%s\n' "$tests" | sed 's/^/- /'
    printf '\n'
  elif [ "$report_read" = 1 ]; then
    printf '%s\n\n' "The Playwright report lists no failed spec ($unexpected unexpected test(s), $errors error(s) outside a test): the run failed before the specs reported, which is what a failing \`globalSetup\` looks like. Check the run log."
  else
    printf '%s\n\n' 'The run failed before the specs ran: there is no Playwright report to list the failed specs from. Check the run log for the step that failed (install, browser download, bootstrap).'
  fi
  printf '%s\n' 'The workflow closes this issue on the next green run.'
}

green_body() {
  printf '%s\n' "The nightly E2E smoke is **green** again: $RUN_URL"
  printf '\n'
  printf '%s\n' "- Ref: \`$REF\`" "- Commit: \`$SHA\`" "- Reported at: $now"
}

if [ "$result" = green ]; then
  if [ -z "$number" ]; then
    echo "e2e-smoke issue: green, and no open tracking issue in $REPO; nothing to write"
    exit 0
  fi
  body=$(green_body)
  if ! "$gh" issue comment "$number" --repo "$REPO" --body "$body"; then
    fail "could not comment on #$number"
  fi
  if ! "$gh" issue close "$number" --repo "$REPO"; then
    fail "could not close #$number"
  fi
  echo "e2e-smoke issue: green, commented on #$number with the run and closed it"
  exit 0
fi

body=$(red_body)
if [ -n "$number" ]; then
  if ! "$gh" issue edit "$number" --repo "$REPO" --body "$body"; then
    fail "could not rewrite the body of #$number"
  fi
  echo "e2e-smoke issue: red, rewrote the body of #$number"
  exit 0
fi
if ! "$gh" issue create --repo "$REPO" --title "$title" --body "$body"; then
  fail "could not create the tracking issue in $REPO"
fi
echo "e2e-smoke issue: red, created the tracking issue in $REPO"
