#!/bin/sh
# How much of what we merged had to be reverted? One number per month, from the repository's own history:
# the merged pull requests in the window and the revert commits that point back at them.
#
# Usage: sh scripts/revert-rate.sh [--month YYYY-MM] [--merged FILE] [--reverts FILE]
#   No arguments: the previous calendar month, fetched with `gh api`.
#   --merged FILE    JSON array of merged pull requests (`number`, `mergedAt`), instead of fetching.
#   --reverts FILE   JSON array of commits on main (`commit.committer.date`, `commit.message`), instead
#                    of fetching. This is what tests/shell/revert-rate.test.sh exercises: the
#                    classification is a pure function of the two arrays, with no network.
#   stdout: one row of docs/evidence/revert-rate.md, so a month is a paste, not a hand count.
# Exit: 0 the row was produced, 1 nothing to measure (no merged PR in the window), 2 usage/IO error.
set -eu

usage() {
  cat >&2 <<'EOF'
usage: sh scripts/revert-rate.sh [--month YYYY-MM] [--merged FILE] [--reverts FILE]

  --month YYYY-MM   month to measure (default: the previous calendar month)
  --merged FILE     merged pull requests as JSON (number, mergedAt), skips the network
  --reverts FILE    commits on main as JSON (commit.committer.date, commit.message), skips the network
EOF
  exit 2
}

month=
merged_file=
reverts_file=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --month | --merged | --reverts)
      [ "$#" -ge 2 ] || usage
      case "$1" in
        --month) month=$2 ;;
        --merged) merged_file=$2 ;;
        --reverts) reverts_file=$2 ;;
      esac
      shift 2
      ;;
    -h | --help) usage ;;
    *)
      echo "revert-rate: unknown argument $1" >&2
      usage
      ;;
  esac
done

if [ -z "$month" ]; then
  this_year=$(date -u +%Y)
  this_month=$(date -u +%m)
  # `${var#0}` instead of `10#`: base conversion is undefined in POSIX sh and the runner's sh is dash.
  if [ "$this_month" = 01 ]; then
    month=$((this_year - 1))-12
  else
    month=$this_year-$(printf '%02d' $((${this_month#0} - 1)))
  fi
fi

case "$month" in
  [0-9][0-9][0-9][0-9]-0[1-9] | [0-9][0-9][0-9][0-9]-1[0-2]) ;;
  *)
    echo "revert-rate: '$month' is not a month in YYYY-MM form" >&2
    exit 2
    ;;
esac

year=${month%-*}
mm=${month#*-}
if [ "$mm" = 12 ]; then
  end=$((year + 1))-01-01T00:00:00Z
else
  end=$year-$(printf '%02d' $((${mm#0} + 1)))-01T00:00:00Z
fi
start=$month-01T00:00:00Z

scratch=$(mktemp -d "${TMPDIR:-/tmp}/openridr-revert-rate.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

if [ -z "$merged_file" ] || [ -z "$reverts_file" ]; then
  command -v gh >/dev/null 2>&1 || {
    echo 'revert-rate: gh is required to fetch the month; pass --merged and --reverts instead' >&2
    exit 2
  }
  repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner) || {
    echo 'revert-rate: could not resolve the repository with gh' >&2
    exit 2
  }
  if [ -z "$merged_file" ]; then
    merged_file=$scratch/merged.json
    # `merged:` takes whole days and a fixed upper bound (`-31`) is wrong for a 30-day month; the
    # exact window is the one the jq program applies, so only the lower bound goes to the search.
    gh pr list --state merged --limit 500 \
      --search "merged:>=$month-01" \
      --json number,title,mergedAt >"$merged_file" || {
      echo 'revert-rate: could not list the merged pull requests' >&2
      exit 2
    }
  fi
  if [ -z "$reverts_file" ]; then
    reverts_file=$scratch/reverts.json
    gh api --paginate "repos/$repo/commits?sha=main&since=$start&until=$end&per_page=100" \
      >"$reverts_file" || {
      echo 'revert-rate: could not list the commits on main' >&2
      exit 2
    }
  fi
fi

for file in "$merged_file" "$reverts_file"; do
  [ -f "$file" ] || {
    echo "revert-rate: $file does not exist" >&2
    exit 2
  }
  jq -e 'type == "array"' "$file" >/dev/null 2>&1 || {
    echo "revert-rate: $file is not a JSON array" >&2
    exit 2
  }
done

# The classification, as one jq program over the two arrays:
#   - a revert is a commit on main whose message starts with `Revert "` (GitHub's button), starts with
#     the conventional `revert:` type, or carries `This reverts commit <sha>`;
#   - it is attributed to the pull request it names as `(#N)`, and only if that PR was merged in the
#     window: a revert of a revert therefore counts against the revert PR, which is what happened;
#   - a revert that names no PR in the window is counted as unattributed, never silently dropped.
result=$(jq -nr \
  --arg month "$month" --arg start "$start" --arg end "$end" \
  --slurpfile merged "$merged_file" --slurpfile reverts "$reverts_file" '
  def in_window: . >= $start and . < $end;
  def looks_like_revert:
    (.commit.message | test("^Revert \""; "i")) or
    (.commit.message | test("^[[:space:]]*revert(\\([^)]*\\))?!?:"; "i")) or
    (.commit.message | test("This reverts commit"; "i"));
  def referenced: [.commit.message | scan("\\(#([0-9]+)\\)") | .[0] | tonumber];

  ($merged[0] | map(select(.mergedAt != null and (.mergedAt | in_window)))) as $prs
  | ($prs | map(.number)) as $numbers
  | ($reverts[0]
      | map(select(((.commit.committer.date // .commit.author.date) // "") as $date
                   | $date != "" and ($date | in_window)))
      | map(select(looks_like_revert))) as $revs
  | ($revs | map([referenced[] | select(. as $n | $numbers | index($n))] | first)) as $hits
  | ($hits | map(select(. != null)) | unique) as $reverted
  | ($hits | map(select(. == null)) | length) as $unattributed
  | ($prs | length) as $merged_count
  | ($reverted | length) as $reverted_count
  | if $merged_count == 0 then
      "row=merged=0 reverted=\($reverted_count) unattributed=\($unattributed)",
      "empty"
    else
      (($reverted_count * 1000 / $merged_count) | round) as $tenths
      | "row=| \($month) | \($merged_count) | \($reverted_count) | \($tenths / 10 | floor).\($tenths % 10)% |",
        "stats=merged=\($merged_count) reverted=\($reverted_count) unattributed=\($unattributed)"
    end')

row=$(printf '%s\n' "$result" | sed -n 's/^row=| /| /p')
stats=$(printf '%s\n' "$result" | sed -n 's/^stats=//p')

if printf '%s\n' "$result" | grep -qx empty; then
  echo "revert-rate: no pull request was merged in $month" >&2
  exit 1
fi

printf '%s\n' "$row"
printf 'revert-rate: %s (%s)\n' "$stats" "$month" >&2
if [ -n "$stats" ]; then
  unattributed=$(printf '%s' "$stats" | sed -n 's/.*unattributed=\([0-9]*\).*/\1/p')
  if [ "${unattributed:-0}" -gt 0 ]; then
    printf 'revert-rate: %s revert commit(s) name no pull request merged in %s; they are not counted\n' \
      "$unattributed" "$month" >&2
  fi
fi
