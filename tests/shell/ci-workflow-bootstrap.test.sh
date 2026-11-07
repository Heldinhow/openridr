#!/bin/sh
# The bootstrap policy lives in .github/workflows/ci.yml, not in a script, because a script on the pull
# request's tree is exactly what the gate must not execute. That makes the YAML the code under test: this
# file extracts the marked blocks from the workflow and runs them for real, with a stub `gh` and fixture
# JSON, so a broken bootstrap rule fails here instead of in a pull request that nobody can merge.
#
#   sh tests/shell/ci-workflow-bootstrap.test.sh
#
# Covers: the trusted-blob check (regular file, non-empty, symlink, tree, missing), the bootstrap gate rule
# (tier handling, a pull request authored by the owner, the same pull request from another author, an
# approval on the head, an approval on an older commit, a withdrawn approval, a multi-page review history,
# an unreadable pull request, an unreadable review list) and the bootstrap aggregate rule (green, a failed
# job, a skipped PR-only job inside a pull request, a missing PR-only job, an empty `needs`).
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
workflow="$repo/.github/workflows/ci.yml"

failures=0
pass() { echo "ok   - $*"; }
fail() { echo "FAIL - $*"; failures=$((failures + 1)); }

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-workflow-bootstrap.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

[ -f "$workflow" ] || {
  echo "FAIL - $workflow not found"
  exit 1
}

# extract <start marker> <end marker>: the shell lines between the two marker comments, dedented by the
# workflow's own indentation. The markers are a contract with this test; renaming one breaks it loudly.
extract() {
  awk -v start="$1" -v end="$2" '
    index($0, start) { inside = 1; next }
    index($0, end) { inside = 0; next }
    inside {
      line = $0
      sub(/^          /, "", line)
      print line
    }' "$workflow"
}

trusted_blob=$(extract '# --- trusted blob check' '# --- end trusted blob check ---')
gate_rule=$(extract '# --- bootstrap gate rule' '# --- end bootstrap gate rule ---')
aggregate_rule=$(extract '# --- bootstrap aggregate rule' '# --- end bootstrap aggregate rule ---')

for block in trusted_blob gate_rule aggregate_rule; do
  eval "body=\$$block"
  if [ -z "$body" ]; then
    fail "$block: the block is empty or its markers moved"
  else
    pass "$block: extracted ($(printf '%s\n' "$body" | wc -l | tr -d ' ') lines)"
  fi
done

# --- trusted blob check ----------------------------------------------------------------------------

blob_repo="$scratch/blob-repo"
mkdir -p "$blob_repo/scripts"
git -C "$blob_repo" init -q
git -C "$blob_repo" config user.name fixture
git -C "$blob_repo" config user.email fixture@example.com
git -C "$blob_repo" config commit.gpgsign false
printf 'echo real\n' >"$blob_repo/scripts/real.sh"
printf '' >"$blob_repo/scripts/empty.sh"
ln -s /etc/passwd "$blob_repo/scripts/link.sh"
mkdir -p "$blob_repo/scripts/dir.sh"
printf 'echo other\n' >"$blob_repo/scripts/other.sh"
git -C "$blob_repo" add -A
git -C "$blob_repo" commit -qm seed

check_blob() { # <name> <expected: usable|refused> <path>
  name=$1
  want=$2
  path=$3
  set +e
  (cd "$blob_repo" && eval "$trusted_blob" && trusted_blob HEAD "$path" "$scratch/out.sh") >"$scratch/blob.out" 2>&1
  code=$?
  set -e
  got=refused
  if [ "$code" -eq 0 ]; then got=usable; fi
  if [ "$got" = "$want" ]; then
    pass "blob-$name: $got"
  else
    fail "blob-$name: expected $want, got $got (exit $code)"
  fi
}

check_blob regular usable scripts/real.sh
check_blob empty refused scripts/empty.sh
check_blob symlink refused scripts/link.sh
check_blob tree refused scripts/dir.sh
check_blob missing refused scripts/nope.sh

# --- bootstrap gate rule ---------------------------------------------------------------------------

head=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
older=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa

cat >"$scratch/gh" <<'STUB'
#!/bin/sh
# Answers the two calls the bootstrap gate makes: `gh api <pull url> --jq <expr>` and
# `gh api --paginate <reviews url>` printing the pages. Any other URL is a bug in the rule.
url=
expr=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --paginate) shift ;;
    --jq) expr=$2; shift 2 ;;
    *) url=$1; shift ;;
  esac
done
case "$url" in
  */reviews)
    if [ -n "${FAKE_FAIL:-}" ]; then
      echo 'stub gh: HTTP 502' >&2
      exit 1
    fi
    file=$FAKE_REVIEWS
    ;;
  */pulls/7)
    if [ -n "${FAKE_PR_FAIL:-}" ]; then
      echo 'stub gh: HTTP 502' >&2
      exit 1
    fi
    file=$FAKE_PR
    ;;
  *)
    echo "stub gh: unexpected url '$url'" >&2
    exit 1
    ;;
esac
if [ -n "$expr" ]; then jq -r "$expr" "$file"; else cat "$file"; fi
STUB
chmod +x "$scratch/gh"

pr_author() { # <author_association>
  printf '{"author_association":"%s"}\n' "$1" >"$scratch/fixture-pr.json"
}
pr_author CONTRIBUTOR

run_gate() { # <tier> [env assignments...]
  tier=$1
  shift
  set +e
  env PATH="$scratch:$PATH" GH_TOKEN=x REPO=owner/repo PR_NUMBER=7 TIER="$tier" HEAD_SHA="$head" \
    RUNNER_TEMP="$scratch" FAKE_REVIEWS="$scratch/fixture-reviews.json" \
    FAKE_PR="$scratch/fixture-pr.json" \
    "$@" sh -c "$gate_rule" >"$scratch/gate.out" 2>&1
  code=$?
  set -e
  out=$(cat "$scratch/gate.out")
}

expect_gate() { # <exit> <name> <tier> <fragment> [env...]
  want=$1
  name=$2
  tier=$3
  fragment=$4
  shift 4
  run_gate "$tier" "$@"
  if [ "$code" -ne "$want" ]; then
    fail "gate-$name: expected exit $want, got $code ($out)"
    return
  fi
  if [ -n "$fragment" ] && ! printf '%s' "$out" | grep -qF "$fragment"; then
    fail "gate-$name: output does not contain '$fragment' ($out)"
    return
  fi
  pass "gate-$name"
}

cat >"$scratch/fixture-reviews.json" <<JSON
[
  {"state":"APPROVED","author_association":"OWNER","submitted_at":"2026-11-07T01:00:00Z","commit_id":"$head","user":{"login":"Heldinhow"}}
]
JSON
expect_gate 0 approval-on-head high 'an approval of'
expect_gate 0 low-no-approval-needed low 'needs no maintainer approval'
expect_gate 0 medium-no-approval-needed medium 'needs no maintainer approval'
expect_gate 2 unknown-tier dodgy 'is not a risk tier'

# The owner writes the pull request: no review can exist (GitHub refuses a self-approval), so it passes.
pr_author OWNER
cat >"$scratch/fixture-reviews.json" <<'JSON'
[]
JSON
expect_gate 0 owner-author-no-review high 'the pull request author is the owner'

# Anyone else still needs the approval.
pr_author MEMBER
expect_gate 1 member-author-no-review high 'needs an approval from the maintainer'

pr_author CONTRIBUTOR
expect_gate 2 unreadable-pull-request high 'could not read pull request' FAKE_PR_FAIL=1

cat >"$scratch/fixture-reviews.json" <<JSON
[
  {"state":"APPROVED","author_association":"OWNER","submitted_at":"2026-11-07T01:00:00Z","commit_id":"$older","user":{"login":"Heldinhow"}}
]
JSON
expect_gate 1 approval-on-older-commit high 'needs an approval from the maintainer'

cat >"$scratch/fixture-reviews.json" <<JSON
[
  {"state":"APPROVED","author_association":"OWNER","submitted_at":"2026-11-07T01:00:00Z","commit_id":"$head","user":{"login":"Heldinhow"}},
  {"state":"CHANGES_REQUESTED","author_association":"OWNER","submitted_at":"2026-11-07T02:00:00Z","commit_id":"$head","user":{"login":"Heldinhow"}}
]
JSON
expect_gate 1 withdrawn-approval high 'needs an approval from the maintainer'

# A review history longer than one page: `gh api --paginate` prints one array per page, and the rule has
# to reduce the whole stream. Reading a page at a time produced "1\n0" and broke the numeric test.
cat >"$scratch/fixture-reviews.json" <<JSON
[
  {"state":"APPROVED","author_association":"OWNER","submitted_at":"2026-11-07T01:00:00Z","commit_id":"$older","user":{"login":"Heldinhow"}}
]
[
  {"state":"APPROVED","author_association":"OWNER","submitted_at":"2026-11-07T02:00:00Z","commit_id":"$head","user":{"login":"Heldinhow"}}
]
JSON
expect_gate 0 approval-on-second-page high 'an approval of'

# The latest decision wins across pages too: an approval on page one and a change request on page two.
cat >"$scratch/fixture-reviews.json" <<JSON
[
  {"state":"APPROVED","author_association":"OWNER","submitted_at":"2026-11-07T01:00:00Z","commit_id":"$head","user":{"login":"Heldinhow"}}
]
[
  {"state":"CHANGES_REQUESTED","author_association":"OWNER","submitted_at":"2026-11-07T02:00:00Z","commit_id":"$head","user":{"login":"Heldinhow"}}
]
JSON
expect_gate 1 withdrawn-across-pages high 'needs an approval from the maintainer'

expect_gate 2 unreadable-reviews high 'could not read the reviews' FAKE_FAIL=1

# --- bootstrap aggregate rule ----------------------------------------------------------------------

run_aggregate() { # <event> <pr_only> <needs json>
  set +e
  env NEEDS="$3" EVENT="$1" PR_ONLY="$2" sh -c "$aggregate_rule" >"$scratch/agg.out" 2>&1
  code=$?
  set -e
  out=$(cat "$scratch/agg.out")
}

expect_aggregate() { # <exit> <name> <event> <pr_only> <needs> [fragment]
  want=$1
  name=$2
  shift 2
  run_aggregate "$@"
  if [ "$code" -ne "$want" ]; then
    fail "aggregate-$name: expected exit $want, got $code ($out)"
    return
  fi
  pass "aggregate-$name"
}

all_green='{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"success"},"risk":{"result":"success"},"security":{"result":"success"}}'
expect_aggregate 0 all-green pull_request governance,risk "$all_green"
expect_aggregate 0 push-pr-only-skipped push governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"skipped"},"risk":{"result":"skipped"},"security":{"result":"success"}}'
expect_aggregate 1 pr-risk-failure pull_request governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"success"},"risk":{"result":"failure"},"security":{"result":"success"}}'
expect_aggregate 1 pr-risk-skipped pull_request governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"success"},"risk":{"result":"skipped"},"security":{"result":"success"}}'
expect_aggregate 1 pr-risk-missing pull_request governance,risk \
  '{"typescript":{"result":"success"},"rust":{"result":"success"},"governance":{"result":"success"},"security":{"result":"success"}}'
expect_aggregate 2 empty-needs pull_request governance,risk '{}'

if [ "$failures" -ne 0 ]; then
  echo "workflow bootstrap tests: $failures failure(s)"
  exit 1
fi

echo "workflow bootstrap tests: all cases passed"
