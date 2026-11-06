#!/bin/sh
# The pull request body is the only record of what a change proves, so the governance job refuses a
# body that does not reference an issue or does not carry evidence.
#
# This is a script, not YAML inline, because scripts/ci-pr-body-selfcheck.sh has to exercise every
# rejection locally; the governance job runs that self-check before the real body.
#
# Usage: sh scripts/ci-pr-body.sh <body-file> [author]
# Exit: 0 accepted, 1 rejected (message on stderr), 2 usage/IO error.
set -eu

if [ "$#" -lt 1 ]; then
  echo 'usage: sh scripts/ci-pr-body.sh <body-file> [author]' >&2
  exit 2
fi
body=$1
author=${2:-}

# Dependency bumps open without a ticket and without evidence, and a bot cannot fix either; the gate
# exists for agent-authored pull requests.
case "$author" in
  'dependabot[bot]'|'renovate[bot]')
    echo "pr body: skipped for $author"
    exit 0
    ;;
esac

if [ ! -f "$body" ]; then
  echo "pr body: $body does not exist" >&2
  exit 2
fi

# HTML comments are instructions, not content: the template ships its guidance inside them, so a body
# left untouched must fail. State machine rather than a regex because a comment can span lines and a
# line can hold text, a comment and more text.
clean=$(awk '
  BEGIN { incomment = 0 }
  {
    line = $0
    out = ""
    while (1) {
      if (incomment) {
        found = index(line, "-->")
        if (found == 0) { line = ""; break }
        line = substr(line, found + 3)
        incomment = 0
      } else {
        found = index(line, "<!--")
        if (found == 0) { out = out line; break }
        out = out substr(line, 1, found - 1)
        line = substr(line, found + 4)
        incomment = 1
      }
    }
    print out
  }' "$body")

# Fenced code is code, not a claim: `Refs #56` or a `## Evidence` heading quoted inside a fence must not
# count. Both views keep the line count, so a line number means the same line in either of them.
#   outside - the fence and everything inside it are blanked (issue reference, headings)
#   body    - only the delimiters are blanked, so a real command inside the Evidence section is still
#             evidence
# A fence closes on the same character with at least the opening length: `~~~` never closes a ``` fence,
# and a longer fence can contain a shorter one.
fences() {
  awk -v mode="$1" '
    function run(value, character,   count) {
      count = 0
      while (substr(value, count + 1, 1) == character) count++
      return count
    }
    {
      line = $0
      stripped = line
      sub(/^[ \t]+/, "", stripped)
      character = substr(stripped, 1, 1)
      marker = (character == "`" || character == "~") ? run(stripped, character) : 0
      delimiter = (marker >= 3)
      if (!inside) {
        if (delimiter) {
          inside = 1
          kind = character
          opened = marker
          print ""
          next
        }
        print line
        next
      }
      closing = 0
      if (delimiter && character == kind && marker >= opened) {
        tail = substr(stripped, marker + 1)
        gsub(/[ \t]/, "", tail)
        if (tail == "") closing = 1
      }
      if (closing) {
        inside = 0
        print ""
        next
      }
      print (mode == "body" ? line : "")
    }'
}

outside_fences=$(printf '%s\n' "$clean" | fences outside)
evidence_text=$(printf '%s\n' "$clean" | fences body)

# Any of the linking keywords GitHub understands, plus `Part of`/`Refs` for a ticket that does not
# close its parent.
if ! printf '%s\n' "$outside_fences" \
  | grep -Eqi '(^|[^[:alnum:]])(closes|fixes|resolves|part of|refs)[[:space:]]*#[0-9]+'; then
  echo 'pr body: no issue reference outside code blocks (expected "Closes #N", "Part of #N" or "Refs #N")' >&2
  exit 1
fi

# The evidence section runs from its heading to the next `##` heading. Headings are looked up in the
# fence-blanked view, so a `## Evidence` written inside a code block is not a section. Both spellings are
# accepted because the template is in English while the repo's docs are in Portuguese; the accented form
# is a separate pattern so a C locale cannot turn `ê` into a byte range.
heading=$(printf '%s\n' "$outside_fences" \
  | grep -Ein \
    -e '^##[[:space:]]+evidence[[:space:]]*$' \
    -e '^##[[:space:]]+evidência[[:space:]]*$' \
  | head -1 | cut -d: -f1)
if [ -z "$heading" ]; then
  echo 'pr body: no "## Evidence" section (say what you ran, or "not verified: why")' >&2
  exit 1
fi

# The section ends at the next heading that is not inside a fence (0 = none).
end=$(printf '%s\n' "$outside_fences" \
  | awk -v start="$heading" 'NR > start && /^##[[:space:]]/ { print NR; exit }')

# Comments are gone and fence delimiters are blanked; what is left is claimed evidence, and blank lines
# are formatting. A section with only fences is empty.
block=$(printf '%s\n' "$evidence_text" \
  | awk -v start="$heading" -v end="${end:-0}" 'NR > start && (end == 0 || NR < end) { print }' \
  | sed -e '/^[[:space:]]*$/d')

if [ -z "$block" ]; then
  echo 'pr body: the evidence section is empty' >&2
  exit 1
fi

if ! printf '%s\n' "$block" | grep -Eqi -v '^[[:space:]]*(none|n/a|todo|tbd)\.?[[:space:]]*$'; then
  echo 'pr body: the evidence section is only a placeholder ("none", "n/a", "todo")' >&2
  exit 1
fi

echo "pr body: issue reference and evidence found"
