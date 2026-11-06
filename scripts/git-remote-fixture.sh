#!/bin/sh
# Disposable local bare remote + two clones of the public fixtures/OrderDesk content, for Git
# acceptance work (docs/SPEC-GIT-REMOTE-ACCEPTANCE.md). No network, no login, no global git config:
# every setting lives inside these throwaway repositories.
#
#   user   <state>/user   repository root == workspace root; the app opens <state>/user/OrderDesk.sln
#   peer   <state>/peer   the counterpart; publish commits and branches from here with plain git
#   origin <state>/origin.git   the bare remote both clones share
#
# Usage: scripts/git-remote-fixture.sh [state-dir]
#   default state-dir: ${TMPDIR:-/tmp}/openridr-git-remote-<uid>; re-running recreates it.
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=${TMPDIR:-/tmp}; tmp=${tmp%/}
state=${1:-$tmp/openridr-git-remote-$(id -u)}
name="OpenRidr Acceptance"
email="acceptance@openridr.invalid"

case "$state" in
'') echo "git-remote-fixture: empty state path" >&2; exit 2 ;;
"$repo" | "$repo"/*) echo "git-remote-fixture: refusing to use a path inside the checkout: $state" >&2; exit 2 ;;
esac

for tool in git rsync; do
  command -v "$tool" >/dev/null 2>&1 || { echo "git-remote-fixture: $tool is required" >&2; exit 1; }
done

# Recreate instead of reusing: the fixture is disposable by contract.
rm -rf "$state"
mkdir -p "$state/user"

# Copy the public fixture into the clone root so the repository root is the workspace root.
rsync -a --exclude bin --exclude obj "$repo/fixtures/OrderDesk/" "$state/user/"

git init --bare --quiet --initial-branch=main "$state/origin.git"

git -C "$state/user" init --quiet --initial-branch=main
git -C "$state/user" config user.name "$name"
git -C "$state/user" config user.email "$email"
git -C "$state/user" add -A .
git -C "$state/user" commit --quiet -m "Baseline for issue 50 Git remote acceptance"
git -C "$state/user" remote add origin "$state/origin.git"
git -C "$state/user" push --quiet --set-upstream origin main

git clone --quiet "$state/origin.git" "$state/peer"
git -C "$state/peer" config user.name "$name"
git -C "$state/peer" config user.email "$email"

baseline=$(git -C "$state/user" rev-parse HEAD)
[ "$baseline" = "$(git -C "$state/origin.git" rev-parse refs/heads/main)" ] \
  || { echo "git-remote-fixture: origin/main does not match the baseline" >&2; exit 1; }
[ "$baseline" = "$(git -C "$state/peer" rev-parse HEAD)" ] \
  || { echo "git-remote-fixture: peer does not share the baseline" >&2; exit 1; }

echo "state $state"
echo "origin $state/origin.git"
echo "solution $state/user/OrderDesk.sln"
echo "peer $state/peer"
echo "baseline $baseline"
echo "launch VERIFY_OPENRIDR_DESKTOP_SOLUTION=$state/user/OrderDesk.sln"
