#!/bin/sh
# Print the body of a GitHub release: the human notes of docs/RELEASE-ALPHA.md followed by a
# "## Verificação" block generated from the artifact this run built. The SHA-256 and the byte count
# come from the workflow, never from the notes file, so they cannot drift from the uploaded DMG.
#
# Usage: sh scripts/release-notes.sh <version> <sha256> <bytes> [notes-file]
# Environment (optional, printed only when set): RELEASE_VERIFY_CODESIGN, RELEASE_VERIFY_SPCTL,
# RELEASE_VERIFY_STAPLER — free-text results of the artifact checks in this run.
set -eu

ROOT=${RELEASE_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)}
PRODUCT=OpenRidr

fail() {
  printf 'release-notes: %s\n' "$*" >&2
  exit 2
}

usage() {
  cat >&2 <<'EOF'
usage: sh scripts/release-notes.sh <version> <sha256> <bytes> [notes-file]
EOF
  exit 2
}

[ "$#" -ge 3 ] || usage
version=$1
sha256=$2
bytes=$3
notes=${4:-$ROOT/docs/RELEASE-ALPHA.md}

[ -n "$version" ] || usage
case $version in
  *[!0-9A-Za-z.-]* | "") usage ;;
esac
printf '%s\n' "$sha256" | grep -Eq '^[0-9a-f]{64}$' ||
  fail "sha256 '$sha256' is not 64 lowercase hex characters"
printf '%s\n' "$bytes" | grep -Eq '^[1-9][0-9]*$' || fail "bytes '$bytes' is not a positive integer"
[ -f "$notes" ] || fail "missing notes file $notes"

heading=$(sed -n '1p' "$notes")
case $heading in
  "# $PRODUCT "*)
    rest=${heading#"# $PRODUCT "}
    heading_version=${rest%% *}
    [ "$heading_version" = "$version" ] ||
      fail "$notes heading says '$heading_version'; expected '$version' (exact)"
    ;;
  *) fail "$notes starts with '$heading'; expected '# $PRODUCT $version'" ;;
esac

# The hash belongs to this run: a hand-written one in the notes is stale by construction.
if grep -Eqi '(^|[^0-9a-f])[0-9a-f]{64}([^0-9a-f]|$)' "$notes"; then
  fail "$notes already contains a SHA-256; the release body generates it, remove the hand-written one"
fi

dmg="${PRODUCT}_${version}_aarch64.dmg"

# Preserve the notes verbatim, then the generated block.
cat "$notes"
printf '\n## Verificação\n\n'
printf -- '- DMG: `%s` — %s bytes\n' "$dmg" "$bytes"
printf -- '- SHA-256: `%s`\n\n' "$sha256"
printf '```text\n%s  %s\n```\n\n' "$sha256" "$dmg"
printf 'Confira com `shasum -a 256 <arquivo>.dmg`.\n'

verify_line() {
  if [ -n "$2" ]; then
    printf -- '- %s: %s\n' "$1" "$2"
  fi
}

if [ -n "${RELEASE_VERIFY_CODESIGN:-}${RELEASE_VERIFY_SPCTL:-}${RELEASE_VERIFY_STAPLER:-}" ]; then
  printf '\nAssinatura e notarização conferidas no run que publicou esta release:\n\n'
  verify_line "codesign" "${RELEASE_VERIFY_CODESIGN:-}"
  verify_line "spctl" "${RELEASE_VERIFY_SPCTL:-}"
  verify_line "stapler" "${RELEASE_VERIFY_STAPLER:-}"
fi
