#!/bin/sh
# Only the maintainer may merge a high-risk pull request. "The maintainer" is a review approval from
# someone with write access (OWNER/MEMBER/COLLABORATOR) submitted **on the current revision**: the review
# carries the commit it was submitted against, and that commit must be the pull request head. An approval
# of an older revision does not carry over, which mirrors `dismiss_stale_reviews_on_push` in the ruleset.
#
# The reviewer's **latest** review is the one that counts. A later CHANGES_REQUESTED or a dismissed
# review withdraws an earlier approval, even though the old APPROVED is still in the history.
#
# Neither the label `risk:*` nor a commit timestamp is an input here: editing a label cannot lower the
# tier, and committer dates are author-controlled.
#
# Usage: REPO=<owner/repo> PR_NUMBER=<n> TIER=<low|medium|high> HEAD_SHA=<sha> sh scripts/ci-risk-gate.sh
#   GH        path to the gh CLI (default: `gh`); the tests point it at a stub.
#   GH_TOKEN  credential for `gh api`.
# Exit: 0 nothing to gate, or an approval of the current revision exists; 1 the approval is missing
#       (the step has to fail); 2 usage/IO error.
set -eu

usage() {
  echo 'usage: REPO=<owner/repo> PR_NUMBER=<n> TIER=<low|medium|high> HEAD_SHA=<sha> sh scripts/ci-risk-gate.sh' >&2
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
head_sha=${HEAD_SHA:-}
[ -n "$head_sha" ] || usage

gh=${GH:-gh}
command -v "$gh" >/dev/null 2>&1 || {
  echo "risk gate: $gh is not executable" >&2
  exit 2
}

# One line per review: state, author association, submitted_at, the commit it was submitted against,
# login. `--paginate` keeps long review histories; the per-reviewer reduction is awk over that stream.
if ! reviews=$("$gh" api --paginate "repos/$REPO/pulls/$PR_NUMBER/reviews" \
  --jq '.[] | [.state, .author_association, .submitted_at, (.commit_id // ""), .user.login] | @tsv'); then
  echo "risk gate: could not read the reviews of #$PR_NUMBER" >&2
  exit 2
fi

# The latest review per login wins, so a withdrawn approval cannot be resurrected by an older APPROVED
# earlier in the history. `$3` is an ISO-8601 UTC timestamp, so a string comparison orders it.
approved=$(printf '%s\n' "$reviews" | awk -F'\t' -v head="$head_sha" '
  NF >= 5 {
    login = $5
    if (!(login in seen) || $3 >= submitted[login]) {
      seen[login] = 1
      state[login] = $1
      association[login] = $2
      submitted[login] = $3
      commit[login] = $4
    }
  }
  END {
    for (login in seen) {
      if (state[login] == "APPROVED" && commit[login] == head &&
          (association[login] == "OWNER" || association[login] == "MEMBER" ||
           association[login] == "COLLABORATOR")) {
        printf "%s approved %s at %s\n", login, head, submitted[login]
        found = 1
      }
    }
    exit(found ? 0 : 1)
  }') || approved=

if [ -n "$approved" ]; then
  echo "risk gate: high risk, $approved"
  exit 0
fi

echo "::error::high-risk pull request: it needs an approval from the maintainer of the current revision ($head_sha). Approve the pull request, then re-run the failed jobs with: gh run rerun --failed <run-id>"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo
    echo '**Merge blocked by the risk gate**: this revision (`'"$head_sha"'`) still needs a maintainer approval.'
    echo
    echo 'Approve the pull request, then re-run the failed jobs: gh run rerun --failed <run-id>'
  } >>"$GITHUB_STEP_SUMMARY"
fi
exit 1
