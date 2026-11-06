#!/bin/sh
# Only the maintainer may merge a high-risk pull request. "The maintainer" is a review approval from
# someone with write access (OWNER/MEMBER/COLLABORATOR) submitted at or after the newest commit of the
# pull request: an approval of an older revision does not carry over, which mirrors
# `dismiss_stale_reviews_on_push` in the ruleset.
#
# The label `risk:*` is never an input here: editing a label cannot lower the tier or stand in for a
# review.
#
# Usage: REPO=<owner/repo> PR_NUMBER=<n> TIER=<low|medium|high> sh scripts/ci-risk-gate.sh
#   GH        path to the gh CLI (default: `gh`); the tests point it at a stub.
#   GH_TOKEN  credential for `gh api`.
# Exit: 0 nothing to gate, or an approval of the current revision exists; 1 the approval is missing
#       (the step has to fail); 2 usage/IO error.
set -eu

usage() {
  echo 'usage: REPO=<owner/repo> PR_NUMBER=<n> TIER=<low|medium|high> sh scripts/ci-risk-gate.sh' >&2
  exit 2
}

tier=${TIER:-}
[ -n "$tier" ] || usage
case "$tier" in
  low | medium)
    echo "risk gate: $tier risk needs no maintainer approval"
    exit 0
    ;;
  high) ;;
  *)
    echo "risk gate: '$tier' is not a risk tier" >&2
    exit 2
    ;;
esac

[ -n "${REPO:-}" ] || usage
[ -n "${PR_NUMBER:-}" ] || usage

gh=${GH:-gh}
command -v "$gh" >/dev/null 2>&1 || {
  echo "risk gate: $gh is not executable" >&2
  exit 2
}

# Newest commit of the pull request. A merge of the base branch into the branch is a commit too, so it
# also invalidates an earlier approval — the maintainer approves again, which is one click.
latest=$("$gh" api --paginate "repos/$REPO/pulls/$PR_NUMBER/commits" \
  --jq '.[].commit.committer.date' | LC_ALL=C sort | tail -1)

if [ -z "$latest" ]; then
  echo "risk gate: could not read the commits of #$PR_NUMBER" >&2
  exit 2
fi

# One line per review: state, author association, submitted_at, login. `--paginate` keeps long review
# histories; the filter is awk so that a paginated stream needs no JSON reassembly.
reviews=$("$gh" api --paginate "repos/$REPO/pulls/$PR_NUMBER/reviews" \
  --jq '.[] | [.state, .author_association, .submitted_at, .user.login] | @tsv')

approved=$(printf '%s\n' "$reviews" | awk -F'\t' -v latest="$latest" '
  $1 == "APPROVED" &&
  ($2 == "OWNER" || $2 == "MEMBER" || $2 == "COLLABORATOR") &&
  $3 >= latest { printf "%s approved %s at %s\n", $4, "this revision", $3; exit }')

if [ -n "$approved" ]; then
  echo "risk gate: high risk, $approved"
  exit 0
fi

echo "::error::high-risk pull request: it needs an approval from the maintainer of the current revision (last commit ${latest}). Approve the pull request, then re-run the failed jobs with: gh run rerun --failed <run-id>"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo
    echo '**Merge blocked by the risk gate**: this revision (`'"$latest"'`) still needs a maintainer approval.'
    echo
    echo 'Approve the pull request, then re-run the failed jobs: gh run rerun --failed <run-id>'
  } >>"$GITHUB_STEP_SUMMARY"
fi
exit 1
