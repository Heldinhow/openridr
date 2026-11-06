#!/bin/sh
# Focused tests for scripts/ci-risk-gate.sh, the rule that only the maintainer may merge a high-risk pull
# request. No network and no GitHub: a stub `gh` answers the two `gh api` calls the gate makes, with the
# real `jq` applying the real `--jq` expression to the fixture.
#
#   sh tests/shell/ci-risk-gate.test.sh
#
# Covers: no gate for low/medium, an approval of the current revision, an approval of an older revision,
# an approval from someone without write access, a dismissed approval, no reviews at all, an approval at
# exactly the head timestamp, the gate reading the right pull request, an unreadable commit list, an
# unknown tier and the missing inputs.
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
script="$repo/scripts/ci-risk-gate.sh"

failures=0
pass() { echo "ok   - $*"; }
fail() { echo "FAIL - $*"; failures=$((failures + 1)); }

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-risk-gate-test.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

owner=Heldinhow
repo_name=openridr
pr=74

# A stub gh: it answers the two `gh api --paginate <url> --jq <expr>` calls the gate makes, and refuses
# anything else. It checks the URL, so a gate that read the wrong pull request fails here.
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
  "repos/$EXPECT_REPO/pulls/$EXPECT_PR/commits") file=$FAKE_COMMITS ;;
  "repos/$EXPECT_REPO/pulls/$EXPECT_PR/reviews") file=$FAKE_REVIEWS ;;
  *)
    echo "stub gh: unexpected url '$url'" >&2
    exit 1
    ;;
esac
if [ -n "$expr" ]; then jq -r "$expr" "$file"; else cat "$file"; fi
STUB
chmod +x "$scratch/gh"

cat >"$scratch/commits.json" <<'JSON'
[
  { "sha": "aaa", "commit": { "committer": { "date": "2026-11-06T10:00:00Z" } } },
  { "sha": "bbb", "commit": { "committer": { "date": "2026-11-06T12:00:00Z" } } },
  { "sha": "ccc", "commit": { "committer": { "date": "2026-11-06T11:00:00Z" } } }
]
JSON

reviews() { # writes $scratch/reviews.json from stdin
  cat >"$scratch/reviews.json"
}

# run <tier> [environment assignments...]: keeps exit code, stdout and stderr for the assertions.
run() {
  tier=$1
  shift
  set +e
  env GH="$scratch/gh" REPO="$owner/$repo_name" PR_NUMBER="$pr" TIER="$tier" \
    EXPECT_REPO="$owner/$repo_name" EXPECT_PR="$pr" \
    FAKE_COMMITS="$scratch/commits.json" FAKE_REVIEWS="$scratch/reviews.json" \
    "$@" sh "$script" >"$scratch/stdout" 2>"$scratch/stderr"
  code=$?
  set -e
  out=$(cat "$scratch/stdout")
  err=$(cat "$scratch/stderr")
}

# expect <pass|fail> <exit> <name> <tier> <stdout fragment> <stderr fragment> [env assignment...]
expect() {
  want_code=$2
  name=$3
  tier=$4
  out_fragment=$5
  err_fragment=$6
  shift 6
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

# No maintainer, no gate: low and medium go through without a single call to gh (the fixture files are
# not even needed).
expect pass 0 low-no-gate low 'needs no maintainer approval' ''
expect pass 0 medium-no-gate medium 'needs no maintainer approval' ''

# The head is the newest committer date (12:00Z), not the last element of the list (11:00Z).
reviews <<'JSON'
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T12:30:00Z", "user": { "login": "Heldinhow" } }
]
JSON
expect pass 0 approval-after-head high 'Heldinhow approved this revision at 2026-11-06T12:30:00Z' ''

reviews <<'JSON'
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T11:30:00Z", "user": { "login": "Heldinhow" } }
]
JSON
expect fail 1 approval-before-head high 'high-risk pull request' ''

reviews <<'JSON'
[
  { "state": "APPROVED", "author_association": "CONTRIBUTOR", "submitted_at": "2026-11-06T13:00:00Z", "user": { "login": "someone" } }
]
JSON
expect fail 1 approval-without-write-access high 'high-risk pull request' ''

reviews <<'JSON'
[
  { "state": "DISMISSED", "author_association": "OWNER", "submitted_at": "2026-11-06T13:00:00Z", "user": { "login": "Heldinhow" } }
]
JSON
expect fail 1 dismissed-approval high 'high-risk pull request' ''

reviews <<'JSON'
[
  { "state": "COMMENTED", "author_association": "MEMBER", "submitted_at": "2026-11-06T13:00:00Z", "user": { "login": "member" } }
]
JSON
expect fail 1 comment-is-not-approval high 'high-risk pull request' ''

reviews <<'JSON'
[]
JSON
expect fail 1 no-reviews high 'high-risk pull request' ''

# An approval submitted in the same second as the head commit counts: the approver saw that revision.
reviews <<'JSON'
[
  { "state": "APPROVED", "author_association": "MEMBER", "submitted_at": "2026-11-06T12:00:00Z", "user": { "login": "member" } }
]
JSON
expect pass 0 approval-at-head high 'member approved this revision' ''

# One valid approval is enough even when an older one comes first, and a stale one does not hide it.
reviews <<'JSON'
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T09:00:00Z", "user": { "login": "Heldinhow" } },
  { "state": "APPROVED", "author_association": "COLLABORATOR", "submitted_at": "2026-11-06T14:00:00Z", "user": { "login": "collaborator" } }
]
JSON
expect pass 0 mixed-histories high 'collaborator approved this revision' ''

# The gate must read the pull request it was told about (the stub gh fails on any other URL).
reviews <<'JSON'
[
  { "state": "APPROVED", "author_association": "OWNER", "submitted_at": "2026-11-06T12:30:00Z", "user": { "login": "Heldinhow" } }
]
JSON
expect fail 2 wrong-pull-request high '' 'could not read the commits' EXPECT_PR=999

# An unreadable commit list is an error, not an approval and not a pass.
printf '[]\n' >"$scratch/commits-empty.json"
expect fail 2 no-commits high '' 'could not read the commits' FAKE_COMMITS="$scratch/commits-empty.json"

# Inputs are validated.
expect fail 2 unknown-tier dodgy '' "is not a risk tier"
expect fail 2 missing-repo high '' 'usage:' REPO=

if [ "$failures" -ne 0 ]; then
  echo "ci-risk-gate tests: $failures failure(s)"
  exit 1
fi

echo "ci-risk-gate tests: all cases passed"
