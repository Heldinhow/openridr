#!/bin/sh
# Focused tests for scripts/ci-risk-gate.sh, the rule that only the maintainer may merge a high-risk pull
# request. No network and no GitHub: a stub `gh` answers the `gh api` call the gate makes, with the real
# `jq` applying the real `--jq` expression to the fixture.
#
#   sh tests/shell/ci-risk-gate.test.sh
#
# Covers: no gate for low/medium, a pull request authored by the owner (no review needed), the same pull
# request from another author (review needed), an approval of the current revision, an approval of an
# older revision, an approval withdrawn by a later CHANGES_REQUESTED or by a dismissal, an approval from
# someone without write access, a reviewer whose latest review is a comment, an approval at the head of a
# mixed history, the gate reading the right pull request, an unreadable pull request, an unreadable review
# list, an unknown tier and the missing inputs.
#
#   GATE_SCRIPT=<path> sh tests/shell/ci-risk-gate.test.sh   # run the cases against another copy of the
#                                                            # script (the falha-antes proof)
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
script=${GATE_SCRIPT:-$repo/scripts/ci-risk-gate.sh}

failures=0
pass() { echo "ok   - $*"; }
fail() { echo "FAIL - $*"; failures=$((failures + 1)); }

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-risk-gate-test.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

owner=Heldinhow
repo_name=openridr
pr=74
head=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
older=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa

# A stub gh: it answers the two `gh api <url> [--jq <expr>]` calls the gate makes — the pull request and
# its reviews — and refuses anything else. It checks the URL, so a gate that read the wrong pull request
# fails here.
cat >"$scratch/gh" <<'STUB'
#!/bin/sh
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
  "repos/$EXPECT_REPO/pulls/$EXPECT_PR/reviews")
    if [ -n "${FAKE_FAIL:-}" ]; then
      echo 'stub gh: HTTP 502' >&2
      exit 1
    fi
    file=$FAKE_REVIEWS
    ;;
  "repos/$EXPECT_REPO/pulls/$EXPECT_PR")
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

reviews() { # writes $scratch/reviews.json from stdin
  cat >"$scratch/reviews.json"
}

# The author of the pull request. The default is not the owner, so every case below that is about the
# review rule keeps testing the review rule.
pr_author() { # <author_association>
  printf '{"author_association":"%s"}\n' "$1" >"$scratch/pr.json"
}
pr_author CONTRIBUTOR

# run <tier> [environment assignments...]: keeps exit code, stdout and stderr for the assertions.
run() {
  tier=$1
  shift
  set +e
  env GH="$scratch/gh" REPO="$owner/$repo_name" PR_NUMBER="$pr" TIER="$tier" HEAD_SHA="$head" \
    EXPECT_REPO="$owner/$repo_name" EXPECT_PR="$pr" FAKE_REVIEWS="$scratch/reviews.json" \
    FAKE_PR="$scratch/pr.json" \
    "$@" sh "$script" >"$scratch/stdout" 2>"$scratch/stderr"
  code=$?
  set -e
  out=$(cat "$scratch/stdout")
  err=$(cat "$scratch/stderr")
}

# expect <exit> <name> <tier> <stdout fragment> <stderr fragment> [env assignment...]
expect() {
  want_code=$1
  name=$2
  tier=$3
  out_fragment=$4
  err_fragment=$5
  shift 5
  run "$tier" "$@"
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

reviews <<'JSON'
[]
JSON

# No maintainer, no gate: low and medium go through without reading a single review.
expect 0 low-no-gate low 'needs no maintainer approval' ''
expect 0 medium-no-gate medium 'needs no maintainer approval' ''

# The author owns the repository: GitHub refuses an approval on your own pull request (HTTP 422), so the
# owner's own pull request passes with no review at all and the review list is not even read.
pr_author OWNER
reviews <<'JSON'
[]
JSON
expect 0 owner-author-no-review high 'the pull request author is the owner' ''

# The owner's own pull request passes whatever the review history says: the point of the rule is that no
# review can exist, not that the reviews happen to agree.
reviews <<JSON
[
  { "state": "CHANGES_REQUESTED", "author_association": "MEMBER", "submitted_at": "2026-11-06T13:00:00Z", "commit_id": "$head", "user": { "login": "member" } }
]
JSON
expect 0 owner-author-ignores-a-change-request high 'the pull request author is the owner' ''

# Only the owner: a member or a collaborator who writes the pull request still needs the approval.
pr_author MEMBER
reviews <<'JSON'
[]
JSON
expect 1 member-author-no-review high 'high-risk pull request' ''

reviews <<JSON
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T12:30:00Z", "commit_id": "$head", "user": { "login": "Heldinhow" } }
]
JSON
expect 0 member-author-with-approval high 'Heldinhow approved' ''

pr_author CONTRIBUTOR

# The approval must name the current head commit.
reviews <<JSON
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T12:30:00Z", "commit_id": "$head", "user": { "login": "Heldinhow" } }
]
JSON
expect 0 approval-on-head high 'Heldinhow approved' ''

reviews <<JSON
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T12:30:00Z", "commit_id": "$older", "user": { "login": "Heldinhow" } }
]
JSON
expect 1 approval-on-older-commit high 'high-risk pull request' ''

# A later CHANGES_REQUESTED withdraws the approval, even though the APPROVED stays in the history.
reviews <<JSON
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T12:30:00Z", "commit_id": "$head", "user": { "login": "Heldinhow" } },
  { "state": "CHANGES_REQUESTED", "author_association": "OWNER", "submitted_at": "2026-11-06T13:30:00Z", "commit_id": "$head", "user": { "login": "Heldinhow" } }
]
JSON
expect 1 withdrawn-by-changes-requested high 'high-risk pull request' ''

# A plain comment after the approval is not a decision: GitHub does not withdraw an approval because the
# reviewer wrote something afterwards, so the approval still stands.
reviews <<JSON
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T12:30:00Z", "commit_id": "$head", "user": { "login": "Heldinhow" } },
  { "state": "COMMENTED", "author_association": "OWNER", "submitted_at": "2026-11-06T13:30:00Z", "commit_id": "$head", "user": { "login": "Heldinhow" } }
]
JSON
expect 0 comment-after-approval high 'Heldinhow approved' ''

# A dismissal (what a push does when stale reviews are dismissed) withdraws it too.
reviews <<JSON
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T12:30:00Z", "commit_id": "$head", "user": { "login": "Heldinhow" } },
  { "state": "DISMISSED", "author_association": "OWNER", "submitted_at": "2026-11-06T13:30:00Z", "commit_id": "$head", "user": { "login": "Heldinhow" } }
]
JSON
expect 1 withdrawn-by-dismissal high 'high-risk pull request' ''

# Re-approving after the change of mind counts again: the latest review is the approval.
reviews <<JSON
[
  { "state": "CHANGES_REQUESTED", "author_association": "OWNER", "submitted_at": "2026-11-06T12:30:00Z", "commit_id": "$head", "user": { "login": "Heldinhow" } },
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T13:30:00Z", "commit_id": "$head", "user": { "login": "Heldinhow" } }
]
JSON
expect 0 reapproved high 'Heldinhow approved' ''

reviews <<JSON
[
  { "state": "APPROVED", "author_association": "CONTRIBUTOR", "submitted_at": "2026-11-06T13:00:00Z", "commit_id": "$head", "user": { "login": "someone" } }
]
JSON
expect 1 approval-without-write-access high 'high-risk pull request' ''

reviews <<JSON
[
  { "state": "COMMENTED", "author_association": "MEMBER", "submitted_at": "2026-11-06T13:00:00Z", "commit_id": "$head", "user": { "login": "member" } }
]
JSON
expect 1 comment-is-not-approval high 'high-risk pull request' ''

reviews <<'JSON'
[]
JSON
expect 1 no-reviews high 'high-risk pull request' ''

# One valid approval is enough even when another reviewer asked for changes, and the withdrawn one does
# not hide it.
reviews <<JSON
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T09:00:00Z", "commit_id": "$older", "user": { "login": "Heldinhow" } },
  { "state": "CHANGES_REQUESTED", "author_association": "MEMBER", "submitted_at": "2026-11-06T13:00:00Z", "commit_id": "$head", "user": { "login": "member" } },
  { "state": "APPROVED", "author_association": "COLLABORATOR", "submitted_at": "2026-11-06T14:00:00Z", "commit_id": "$head", "user": { "login": "collaborator" } }
]
JSON
expect 0 mixed-reviewers high 'collaborator approved' ''

# The gate must read the pull request it was told about (the stub gh fails on any other URL).
reviews <<JSON
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T12:30:00Z", "commit_id": "$head", "user": { "login": "Heldinhow" } }
]
JSON
expect 2 wrong-pull-request high '' 'could not read pull request' EXPECT_PR=999

# An unreadable pull request is an error, never a pass: the author is what decides the short circuit.
expect 2 unreadable-pull-request high '' 'could not read pull request' FAKE_PR_FAIL=1

# An unreadable review list is an error, never an approval and never a pass.
expect 2 api-failure high '' 'could not read the reviews' FAKE_FAIL=1

# Inputs are validated.
expect 2 unknown-tier dodgy '' 'is not a risk tier'
expect 2 missing-repo high '' 'usage:' REPO=
expect 2 missing-head high '' 'usage:' HEAD_SHA=

if [ "$failures" -ne 0 ]; then
  echo "ci-risk-gate tests: $failures failure(s)"
  exit 1
fi

echo "ci-risk-gate tests: all cases passed"
