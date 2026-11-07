#!/bin/sh
# How much of what we merged had to be reverted? One number per month, from the repository's own history:
# the merged pull requests in the window and the revert commits that point back at them.
#
# Usage: sh scripts/revert-rate.sh [YYYY-MM] [--month YYYY-MM] [--merged FILE] [--reverts FILE]
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
usage: sh scripts/revert-rate.sh [YYYY-MM] [--month YYYY-MM] [--merged FILE] [--reverts FILE]

  YYYY-MM           month to measure, positionally (same as --month)
  --month YYYY-MM   month to measure (default: the previous calendar month)
  --merged FILE     merged pull requests as JSON (number, mergedAt), skips the network
  --reverts FILE    commits on main as JSON (commit.committer.date, commit.message), skips the network
EOF
  exit 2
}

month=
merged_file=
reverts_file=
limit=500
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
    -*)
      echo "revert-rate: unknown argument $1" >&2
      usage
      ;;
    *)
      # The ticket spelled the month as a positional argument; accept both spellings.
      if [ -n "$month" ]; then
        echo "revert-rate: the month was given twice ('$month' and '$1')" >&2
        usage
      fi
      month=$1
      shift
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

case "$mm" in
  01 | 03 | 05 | 07 | 08 | 10 | 12) last=31 ;;
  04 | 06 | 09 | 11) last=30 ;;
  02)
    if [ $((year % 4)) -eq 0 ] && { [ $((year % 100)) -ne 0 ] || [ $((year % 400)) -eq 0 ]; }; then
      last=29
    else
      last=28
    fi
    ;;
esac

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
    # The search is bounded on both ends with the real last day of the month: an open lower bound plus a
    # result cap would let newer pull requests crowd out the month being measured.
    gh pr list --state merged --limit "$limit" \
      --search "merged:$month-01..$month-$last" \
      --json number,title,mergedAt >"$merged_file" || {
      echo 'revert-rate: could not list the merged pull requests' >&2
      exit 2
    }
    if [ "$(jq 'length' "$merged_file")" -ge "$limit" ]; then
      echo "revert-rate: the search returned $limit merged pull requests, which is the cap; the month may be truncated. Raise --limit in the script." >&2
      exit 2
    fi
  fi
  if [ -z "$reverts_file" ]; then
    reverts_file=$scratch/reverts.json
    # `--paginate` prints one JSON array per page. The pages are captured first and normalised after,
    # never in a pipeline: in POSIX sh the pipeline status is `jq`'s, so a `gh` failure after a partial
    # page would produce a successful but incomplete count.
    pages=$scratch/pages.json
    if ! gh api --paginate "repos/$repo/commits?sha=main&since=$start&until=$end&per_page=100" \
      >"$pages"; then
      echo 'revert-rate: could not list the commits on main' >&2
      exit 2
    fi
    if ! jq -s 'add // []' "$pages" >"$reverts_file"; then
      echo 'revert-rate: the commit list is not JSON' >&2
      exit 2
    fi
  fi
fi

# Both inputs are normalised to a single JSON array: a fixture may be one array, or several concatenated
# arrays standing in for paginated pages. The two outputs have fixed, distinct names: deriving them from
# the input basename would let two fixtures with the same name overwrite each other.
merged_normalised=$scratch/merged.normalised.json
reverts_normalised=$scratch/reverts.normalised.json
for file in "$merged_file" "$reverts_file"; do
  [ -f "$file" ] || {
    echo "revert-rate: $file does not exist" >&2
    exit 2
  }
  case "$file" in
    "$merged_file") normalised=$merged_normalised ;;
    *) normalised=$reverts_normalised ;;
  esac
  if ! jq -s 'add // []' "$file" >"$normalised" 2>/dev/null; then
    echo "revert-rate: $file is not JSON" >&2
    exit 2
  fi
  if ! jq -e 'type == "array"' "$normalised" >/dev/null 2>&1; then
    echo "revert-rate: $file is not a JSON array" >&2
    exit 2
  fi
  case "$file" in
    "$merged_file") merged_file=$merged_normalised ;;
    *) reverts_file=$reverts_normalised ;;
  esac
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
