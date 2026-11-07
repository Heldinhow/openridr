#!/bin/sh
# The installed app verifies every update with the public key in `src-tauri/tauri.conf.json`. A wrong
# key there fails no build and no release: it makes every app refuse every update, silently, and the
# only fix is a new release signed with the key the app already trusts. So the value is compared
# against the key the release is signed with — the local keypair when this machine has it, and a
# committed copy of the same string otherwise (CI, a fresh clone).
#
# Usage: sh scripts/updater-pubkey-selfcheck.sh
#   OPENRIDR_UPDATER_PUBKEY   path of the public key file (default ~/.tauri/openridr-updater.key.pub)
set -eu

ROOT=${RELEASE_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)}
CONF="$ROOT/src-tauri/tauri.conf.json"
LOCAL=${OPENRIDR_UPDATER_PUBKEY:-$HOME/.tauri/openridr-updater.key.pub}

# Committed copy of the key in `~/.tauri/openridr-updater.key.pub`; the fallback when the keypair is
# not on this machine. When the file is present, both must agree, so this cannot drift unnoticed.
EXPECTED='dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEQzMERFNUJBRkFFOTVDNUUKUldSZVhPbjZ1dVVOMC9hNTJxT3lEelFxWW5hdDFvT0VvdS9YOEowTHpoYkkwK1AralVYUXp4UTUK'

fail() {
  printf 'updater-pubkey: %s\n' "$*" >&2
  exit 1
}

[ -f "$CONF" ] || fail "missing $CONF"

configured=$(sed -n 's/.*"pubkey"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$CONF" | head -n 1)
[ -n "$configured" ] || fail "no plugins.updater.pubkey in $CONF"

# Shape first: a base64 minisign public key, which is what the plugin decodes before verifying.
printf '%s' "$configured" |
  openssl base64 -d -A 2>/dev/null | grep -q '^untrusted comment: minisign public key' ||
  fail "the pubkey in $CONF is not a base64 minisign public key"

if [ -f "$LOCAL" ]; then
  local_key=$(cat "$LOCAL")
  [ "$configured" = "$local_key" ] ||
    fail "the pubkey in $CONF does not match $LOCAL"
  [ "$configured" = "$EXPECTED" ] ||
    fail "the committed copy in this script is stale: set EXPECTED to the key in $LOCAL"
  printf 'ok: the configured pubkey matches %s and the committed copy\n' "$LOCAL"
else
  [ "$configured" = "$EXPECTED" ] ||
    fail "the pubkey in $CONF does not match the committed copy"
  printf 'ok: the configured pubkey matches the committed copy (%s not present)\n' "$LOCAL"
fi
