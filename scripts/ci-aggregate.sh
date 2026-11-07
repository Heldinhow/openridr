#!/bin/sh
# The `ci` job is the single required check of the main ruleset, so its logic is a contract: every job
# it waits for must really have passed, and a skipped job must never read as green. Two jobs only run
# for a pull request (`governance`, `risk`), so outside a pull request their `skipped` is expected —
# and inside one it is a bug (a bad `if`, a renamed job).
#
# The logic lives here, not in the workflow YAML, because tests/shell/ci-aggregate.test.sh has to
# exercise every combination of results without a GitHub run.
#
# Usage: NEEDS=<json> EVENT=<event> PR_ONLY=<csv> sh scripts/ci-aggregate.sh
#   NEEDS   toJSON(needs) of the aggregator job
#   EVENT   github.event_name
#   PR_ONLY comma-separated jobs that only run for a pull request
# Exit: 0 every required job passed, 1 a job did not pass (listed on stderr), 2 usage/IO error.
set -eu

usage() {
  echo 'usage: NEEDS=<json> EVENT=<event> PR_ONLY=<csv> sh scripts/ci-aggregate.sh' >&2
  exit 2
}

if [ -z "${NEEDS:-}" ] || [ -z "${EVENT:-}" ] || [ -z "${PR_ONLY:-}" ]; then
  usage
fi

command -v jq >/dev/null 2>&1 || {
  echo 'aggregate: jq is required' >&2
  exit 2
}

if ! printf '%s' "$NEEDS" | jq -e 'type == "object"' >/dev/null 2>&1; then
  echo "aggregate: NEEDS is not a JSON object" >&2
  exit 2
fi

# An empty `needs` would make every check pass, which is the one failure mode this job exists to
# prevent: a misconfigured YAML must be loud.
if [ "$(printf '%s' "$NEEDS" | jq 'length')" -eq 0 ]; then
  echo 'aggregate: NEEDS is empty; the aggregator waits for nothing' >&2
  exit 2
fi

# A pull request must include every PR-only job, otherwise removing one from `needs` would silently
# remove the gate. Outside a pull request they are allowed to be absent (and are, when the job is new).
missing=$(printf '%s' "$NEEDS" | jq -r --arg event "$EVENT" --arg pr_only "$PR_ONLY" '
  if $event == "pull_request" then
    ($pr_only | split(",") | map(select(. != ""))) as $pr_only
    | (keys) as $present
    | $pr_only[] as $name
    | select(($present | index($name)) | not)
    | $name
  else empty end')

if [ -n "$missing" ]; then
  printf 'required jobs missing from needs (they run for every pull request):\n%s\n' "$missing" >&2
  exit 1
fi

failed=$(printf '%s' "$NEEDS" | jq -r --arg event "$EVENT" --arg pr_only "$PR_ONLY" '
  ($pr_only | split(",") | map(select(. != ""))) as $pr_only
  | to_entries[]
  | select(.value.result != "success"
      and ((.key as $key | $pr_only | index($key) | not)
        or (.value.result != "skipped")
        or ($event == "pull_request")))
  | "\(.key): \(.value.result)"')

if [ -n "$failed" ]; then
  printf 'required jobs did not pass:\n%s\n' "$failed" >&2
  exit 1
fi

echo "aggregate: every required job passed ($EVENT)"
