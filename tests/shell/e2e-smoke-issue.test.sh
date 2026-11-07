#!/bin/sh
# Focused tests for scripts/e2e-smoke-issue.sh, the script that opens, rewrites and closes the nightly
# smoke tracking issue. No network and no GitHub: a stub `gh` records the argv of every call and answers
# `issue list` from a fixture, and the real `jq` filters the real Playwright report.
#
#   sh tests/shell/e2e-smoke-issue.test.sh
#
#   SMOKE_SCRIPT=<path> sh tests/shell/e2e-smoke-issue.test.sh   # run the cases against another copy of
#                                                               # the script (the falha-antes proof)
#   SMOKE_SHELL=dash sh tests/shell/e2e-smoke-issue.test.sh      # run the script under dash, the
#                                                               # runner's /bin/sh
#
# Covers: red with no report, red with the real report, red with the issue already open (rewrite, never a
# second issue), red with an unrelated open issue, an unreadable report, a report with no failing spec (a
# failing `globalSetup`), green with the issue open (comment, then close), green with no issue (no write
# at all), a failing `gh` in the list/create/close calls, and the usage errors.
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
script=${SMOKE_SCRIPT:-$repo/scripts/e2e-smoke-issue.sh}
report=$repo/tests/shell/fixtures/playwright-report.json

failures=0
pass() { echo "ok   - $*"; }
fail() { echo "FAIL - $*"; failures=$((failures + 1)); }

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-smoke-issue-test.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

GH_LOG=$scratch/gh.log
issues=$scratch/issues.json
run_url=https://github.com/Heldinhow/openridr/actions/runs/42364384217
sha=0123456789abcdef0123456789abcdef01234567

# A stub gh: it appends the argv of every call to $GH_LOG, one argument per line behind a `---` marker,
# so the call count is `grep -c '^---$'` and a whole argument is `grep -x`. `issue list` prints
# $FAKE_ISSUES; the write subcommands print a URL and write nothing. FAKE_FAIL names the subcommand that
# fails, so the test can break one call at a time.
cat >"$scratch/gh" <<'STUB'
#!/bin/sh
printf '%s\n' '---' >>"$GH_LOG"
for arg in "$@"; do printf '%s\n' "$arg" >>"$GH_LOG"; done
verb=$1
sub=${2:-}
if [ -n "${FAKE_FAIL:-}" ] && [ "$FAKE_FAIL" = "$sub" ]; then
  echo "stub gh: HTTP 502 on $verb $sub" >&2
  exit 1
fi
case "$verb $sub" in
  "issue list") cat "$FAKE_ISSUES" ;;
  "issue create" | "issue edit" | "issue comment" | "issue close")
    echo 'https://github.com/Heldinhow/openridr/issues/1'
    ;;
  *)
    echo "stub gh: unexpected command '$*'" >&2
    exit 1
    ;;
esac
STUB
chmod +x "$scratch/gh"

no_issue() { printf '[]\n' >"$issues"; }
issue_open() { printf '[{"number":42,"title":"[nightly] e2e-smoke is red"}]\n' >"$issues"; }

# run <environment assignment>...: keeps the exit code, stdout and stderr, and starts from an empty call
# log. REPORT defaults to empty so a case that does not pass one really runs without a report.
run() {
  : >"$GH_LOG"
  set +e
  env GH="$scratch/gh" GH_LOG="$GH_LOG" FAKE_ISSUES="$issues" REPORT= \
    REPO="Heldinhow/openridr" RUN_URL="$run_url" REF=refs/heads/main SHA="$sha" \
    "$@" "${SMOKE_SHELL:-sh}" "$script" >"$scratch/stdout" 2>"$scratch/stderr"
  code=$?
  set -e
  out=$(cat "$scratch/stdout")
  err=$(cat "$scratch/stderr")
}

# expect <exit> <gh calls> <name>: the case's exit code and how many times `gh` was called. A case that
# fails here skips its content checks, so the output stays one line per broken case.
case_ok=1
expect() {
  want_code=$1
  want_calls=$2
  name=$3
  if [ "$code" -ne "$want_code" ]; then
    case_ok=0
    fail "$name: expected exit $want_code, got $code ($err)"
    return
  fi
  got_calls=$(grep -c '^---$' "$GH_LOG" || true)
  if [ "$got_calls" -ne "$want_calls" ]; then
    case_ok=0
    fail "$name: expected $want_calls gh call(s), got $got_calls"
    return
  fi
  case_ok=1
  pass "$name"
}

# arg <value>: some call passed exactly this argument (`create`, `edit`, `42`, the title).
arg() {
  [ "$case_ok" = 1 ] || return 0
  if grep -qxF -- "$1" "$GH_LOG"; then
    pass "  gh got the argument '$1'"
  else
    fail "gh did not get the argument '$1'"
  fi
}

# has <fragment>: some argument the script passed to `gh` contains the fragment (the body of the issue).
has() {
  [ "$case_ok" = 1 ] || return 0
  if grep -qF -- "$1" "$GH_LOG"; then
    pass "  gh got '$1'"
  else
    fail "gh did not get '$1'"
  fi
}

# lacks <fragment>: nothing the script passed to `gh` contains it. This is what proves an issue was not
# created twice and that a passing test is not listed.
lacks() {
  [ "$case_ok" = 1 ] || return 0
  if grep -qF -- "$1" "$GH_LOG"; then
    fail "gh got '$1' and should not have"
  else
    pass "  gh did not get '$1'"
  fi
}

out_has() {
  [ "$case_ok" = 1 ] || return 0
  if printf '%s' "$out" | grep -qF -- "$1"; then
    pass "  stdout has '$1'"
  else
    fail "stdout lacks '$1' ($out)"
  fi
}

err_has() {
  [ "$case_ok" = 1 ] || return 0
  if printf '%s' "$err" | grep -qF -- "$1"; then
    pass "  stderr has '$1'"
  else
    fail "stderr lacks '$1' ($err)"
  fi
}

# order <first> <second>: the first subcommand is called before the second.
order() {
  [ "$case_ok" = 1 ] || return 0
  first=$(grep -nxF -- "$1" "$GH_LOG" | cut -d: -f1)
  second=$(grep -nxF -- "$2" "$GH_LOG" | cut -d: -f1)
  if [ -n "$first" ] && [ -n "$second" ] && [ "$first" -lt "$second" ]; then
    pass "  '$1' before '$2'"
  else
    fail "'$1' (line $first) is not before '$2' (line $second)"
  fi
}

# --- red, no report: the run failed before the specs ran --------------------------------------------
no_issue
run RESULT=red
expect 0 2 red-no-report-creates
arg create
arg '[nightly] e2e-smoke is red'
has 'e2e-smoke in:title'
has 'failed before the specs ran'
has "$run_url"
has 'refs/heads/main'
has "$sha"
lacks edit

# --- red with the real Playwright report ------------------------------------------------------------
no_issue
run RESULT=red REPORT="$report"
expect 0 2 red-with-report-creates
arg create
has 'workspace.spec.ts'
has 'reveals the close action'
# The nested suite carries its own `file` and it is wrong: the filter has to inherit the top-level one,
# so the body never shows the `../../../../private/tmp/...` path the report really contains.
lacks 'private/tmp'
# Only the specs with `ok == false`: the two passing tests are not in the body.
lacks 'opens the fixture asking for a Trust decision'
lacks 'saves the edited file'

# --- red with the issue already open: rewrite the body, never a second issue -------------------------
issue_open
run RESULT=red REPORT="$report"
expect 0 2 red-open-issue-rewrites-body
arg edit
arg 42
has 'workspace.spec.ts'
lacks create

# --- red with an unrelated open issue: the title match is exact -------------------------------------
printf '[{"number":7,"title":"e2e-smoke is slow"}]\n' >"$issues"
run RESULT=red
expect 0 2 red-unrelated-title-creates
arg create
arg '[nightly] e2e-smoke is red'
lacks edit

# --- red with an unreadable report: same as no report -----------------------------------------------
no_issue
run RESULT=red REPORT="$scratch/missing.json"
expect 0 2 red-unreadable-report
has 'failed before the specs ran'

# --- red, report present but no failing spec: a failing `globalSetup` -------------------------------
# Derived from the real report: a globalSetup failure leaves no suite and puts the error in `errors`,
# with `stats.unexpected` at 0. That is what tells this apart from a report that never existed.
jq '.suites = [] | .stats.expected = 0 | .stats.unexpected = 0
    | .errors = [{"message": "Error: globalSetup failed: could not install csharp-ls"}]' \
  "$report" >"$scratch/global-setup.json"
no_issue
run RESULT=red REPORT="$scratch/global-setup.json"
expect 0 2 red-report-without-failing-spec
has 'no failed spec'
lacks 'failed before the specs ran'

# --- green with the issue open: comment with the run, then close ------------------------------------
issue_open
run RESULT=green
expect 0 3 green-open-issue-comments-and-closes
arg comment
arg close
arg 42
has "$run_url"
order comment close
lacks create
lacks edit

# --- green with no issue: nothing is written --------------------------------------------------------
no_issue
run RESULT=green
expect 0 1 green-no-issue-writes-nothing
out_has 'nothing to write'

# --- a failing gh: non-zero and ::error:: -----------------------------------------------------------
no_issue
run RESULT=red FAKE_FAIL=list
expect 1 1 gh-list-failure
err_has '::error::'

no_issue
run RESULT=red FAKE_FAIL=create
expect 1 2 gh-create-failure
err_has '::error::'

issue_open
run RESULT=green FAKE_FAIL=close
expect 1 3 gh-close-failure
err_has '::error::'

# --- usage errors -----------------------------------------------------------------------------------
run
expect 2 0 missing-result

run RESULT=blue
expect 2 0 unknown-result

if [ "$failures" -ne 0 ]; then
  echo "e2e-smoke-issue tests: $failures failure(s)"
  exit 1
fi

echo "e2e-smoke-issue tests: all cases passed"
