#!/bin/sh
# Print the update manifest (`latest.json`) the installed app checks: the version, the human notes
# and, for darwin-aarch64, the URL of the signed archive plus the content of its `.sig`.
#
# The signature is the only thing that proves the artifact came from us, so a manifest built from a
# missing, empty or non-minisign signature is refused here instead of at the user's machine.
#
# Usage: sh scripts/release-updater-manifest.sh <version> <archive-url> <sig-file> [notes-file]
#   <version>      semver without the leading `v` (0.3.0-alpha.12)
#   <archive-url>  https URL of the `OpenRidr.app.tar.gz` published for this version
#   <sig-file>     the `.sig` the signing step wrote next to the archive
#   [notes-file]   defaults to docs/RELEASE-ALPHA.md
set -eu

ROOT=${RELEASE_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)}

fail() {
  printf 'updater-manifest: %s\n' "$*" >&2
  exit 2
}

usage() {
  cat >&2 <<'EOF'
usage: sh scripts/release-updater-manifest.sh <version> <archive-url> <sig-file> [notes-file]
EOF
  exit 2
}

[ "$#" -ge 3 ] || usage
version=$1
url=$2
sig=$3
notes=${4:-$ROOT/docs/RELEASE-ALPHA.md}

# Same strict shape the release preflight uses for the tag, without the leading `v`.
printf '%s\n' "$version" |
  grep -Eq '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$' ||
  fail "version '$version' is not semver (0.3.0-alpha.12)"

case $url in
  https://*) ;;
  *) fail "url '$url' is not https" ;;
esac

[ -f "$sig" ] || fail "missing signature file $sig"
[ -s "$sig" ] || fail "signature file $sig is empty"
# The `.sig` is base64 of a minisign signature box; a file that does not decode to one would make
# every installed app refuse the update long after this run.
sig_text=$(cat "$sig")
printf '%s' "$sig_text" | openssl base64 -d -A 2>/dev/null | grep -q '^untrusted comment' ||
  fail "$sig is not a base64 minisign signature"

[ -f "$notes" ] || fail "missing notes file $notes"
command -v jq >/dev/null 2>&1 || fail "jq is required to build the manifest"

# The endpoint response is not signed, so the date is the only freshness hint; RFC 3339 as the
# plugin expects.
pub_date=$(date -u +%Y-%m-%dT%H:%M:%SZ)

jq -n \
  --arg version "$version" \
  --arg url "$url" \
  --arg date "$pub_date" \
  --arg signature "$sig_text" \
  --rawfile notes "$notes" \
  '{
    version: $version,
    notes: $notes,
    pub_date: $date,
    platforms: {
      "darwin-aarch64": { signature: $signature, url: $url }
    }
  }'
